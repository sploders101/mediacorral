use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use async_udev::{disc_insert_events, get_disc_name};
use clap::Parser;
use futures::StreamExt;
use makemkv::{
    Makemkv,
    messaging::{MakemkvMessage, ProgressBar},
};
use mediacorral_proto::mediacorral::{
    drive_controller::v1::{
        DriveState, DriveStatusTag, EjectRequest, EjectResponse, GetDriveCountRequest,
        GetDriveCountResponse, GetDriveMetaRequest, GetDriveMetaResponse, GetDriveStateRequest,
        GetJobStatusRequest, JobStatus, Progress, ReapJobRequest, ReapJobResponse, RetractRequest,
        RetractResponse, RipMediaRequest, RipMediaResponse, RipStatus, RipUpdate,
        WatchRipJobRequest,
        drive_controller_service_server::{DriveControllerService, DriveControllerServiceServer},
        rip_update,
    },
    server::v1::{
        DiscInsertedRequest, RipFinishedRequest,
        coordinator_notification_service_client::CoordinatorNotificationServiceClient,
    },
};
use serde::Deserialize;
use tokio::{
    sync::{RwLock, watch},
    task::JoinHandle,
};
use tokio_stream::wrappers::WatchStream;
use tonic::transport::{Endpoint, Server};

mod async_udev;
mod makemkv;

macro_rules! try_ejector {
    (wrap $val:expr) => {
        try_ejector!(tokio::task::spawn_blocking(move || $val).await.unwrap())
    };
    ($val:expr) => {
        match ($val) {
            Ok(value) => value,
            Err(err) => match err.kind() {
                eject::error::ErrorKind::AccessDenied => {
                    return Err(tonic::Status::permission_denied(
                        "Got 'access denied' while ejecting the drive.",
                    ));
                }
                eject::error::ErrorKind::NotFound => {
                    return Err(tonic::Status::not_found(
                        "Got 'device not found' while ejecting the drive.",
                    ));
                }
                eject::error::ErrorKind::InvalidPath => {
                    return Err(tonic::Status::internal(
                        "Got 'invalid path' while ejecting the drive.",
                    ));
                }
                eject::error::ErrorKind::UnsupportedOperation => {
                    return Err(tonic::Status::unknown(
                        "This operation is not supported on the device.",
                    ));
                }
                _ => {
                    return Err(tonic::Status::unknown("An unknown error occurred."));
                }
            },
        }
    };
}

pub struct Drive {
    path: String,
    name: String,
    ejector: Arc<eject::device::Device>,
}

pub struct RipJob {
    job_id: i64,
    drive_id: usize,
    job_status: watch::Receiver<RipStatus>,
    #[allow(dead_code)]
    task_handle: JoinHandle<()>,
}

pub struct DriveController {
    id: String,
    coordinator_notifs: CoordinatorNotificationServiceClient<tonic::transport::Channel>,
    shared_directory: PathBuf,
    drives: Arc<Vec<Drive>>,
    rip_jobs: RwLock<HashMap<i64, RipJob>>,
}

#[tonic::async_trait]
impl DriveControllerService for DriveController {
    async fn get_drive_count(
        &self,
        _request: tonic::Request<GetDriveCountRequest>,
    ) -> std::result::Result<tonic::Response<GetDriveCountResponse>, tonic::Status> {
        return Ok(tonic::Response::new(GetDriveCountResponse {
            drive_count: self.drives.len() as _,
        }));
    }

    async fn get_drive_meta(
        &self,
        request: tonic::Request<GetDriveMetaRequest>,
    ) -> Result<tonic::Response<GetDriveMetaResponse>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        return Ok(tonic::Response::new(GetDriveMetaResponse {
            drive_id: request.drive_id,
            name: drive.name.clone(),
        }));
    }

    async fn eject(
        &self,
        request: tonic::Request<EjectRequest>,
    ) -> Result<tonic::Response<EjectResponse>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        let ejector = Arc::clone(&drive.ejector);
        try_ejector!(wrap ejector.eject());

        return Ok(tonic::Response::new(EjectResponse {}));
    }

    async fn retract(
        &self,
        request: tonic::Request<RetractRequest>,
    ) -> Result<tonic::Response<RetractResponse>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        let ejector = Arc::clone(&drive.ejector);
        try_ejector!(wrap ejector.retract());

        return Ok(tonic::Response::new(RetractResponse {}));
    }

    async fn get_drive_state(
        &self,
        request: tonic::Request<GetDriveStateRequest>,
    ) -> Result<tonic::Response<DriveState>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        let disc_name = get_disc_name(&drive.path).await;

        let ejector = Arc::clone(&drive.ejector);
        let status = match try_ejector!(wrap ejector.status()) {
            eject::device::DriveStatus::Empty => DriveStatusTag::Empty,
            eject::device::DriveStatus::TrayOpen => DriveStatusTag::TrayOpen,
            eject::device::DriveStatus::NotReady => DriveStatusTag::NotReady,
            eject::device::DriveStatus::Loaded => DriveStatusTag::DiscLoaded,
        };

        let mut active_rip_job: Option<_> = None;
        for job in self.rip_jobs.read().await.values() {
            if job.drive_id == request.drive_id as usize {
                active_rip_job = Some(job.job_id);
            }
        }

        return Ok(tonic::Response::new(DriveState {
            drive_id: request.drive_id,
            status: status.into(),
            disc_name,
            active_rip_job,
        }));
    }

    async fn rip_media(
        &self,
        request: tonic::Request<RipMediaRequest>,
    ) -> Result<tonic::Response<RipMediaResponse>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        let mut jobs = self.rip_jobs.write().await;
        // Check for jobs with the same ID
        if jobs.contains_key(&request.job_id) {
            return Err(tonic::Status::already_exists(
                "The requested job ID already exists.",
            ));
        }
        // Check for jobs already running on the drive
        for job in jobs.values() {
            if job.drive_id == request.drive_id as _
                && job.job_status.borrow().status == JobStatus::Running.into()
            {
                return Err(tonic::Status::resource_exhausted(
                    "The requested drive is already undergoing a rip job.",
                ));
            }
        }

        let rip_dir = RipDir::new(&self.shared_directory, request.job_id)
            .await
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::AlreadyExists => {
                    tonic::Status::already_exists("The rip directory already exists.")
                }
                std::io::ErrorKind::NotFound => {
                    tonic::Status::not_found("The shared directory doesn't exist")
                }
                _ => tonic::Status::internal(format!(
                    "An error occurred while creating the rip directory:\n{err}"
                )),
            })?;
        let mut makemkv = Makemkv::rip(&drive.path, &rip_dir.dir).map_err(|err| {
            tonic::Status::internal(format!("Unknown error while spawning makemkv:\n{err}"))
        })?;

        let (sender, receiver) = watch::channel(RipStatus {
            job_id: request.job_id,
            status: JobStatus::Unspecified.into(),
            cprog_title: String::from("Starting Rip..."),
            tprog_title: String::from("Starting Rip..."),
            progress: Some(Progress {
                cprog_value: 0,
                tprog_value: 0,
                max_value: 1,
            }),
            logs: Vec::new(),
        });
        let mut notif_client = self.coordinator_notifs.clone();
        let controller_id = self.id.clone();
        let ejector = Arc::clone(&drive.ejector);
        let autoeject = request.autoeject;
        let task_handle = tokio::task::spawn(async move {
            while let Ok(Some(event)) = makemkv.next_event().await {
                match event {
                    MakemkvMessage::ProgressTitle { bar, name, .. } => {
                        sender.send_modify(|rip_status| match bar {
                            ProgressBar::Current => rip_status.cprog_title = name,
                            ProgressBar::Total => rip_status.tprog_title = name,
                        });
                    }
                    MakemkvMessage::ProgressValue {
                        current,
                        total,
                        max,
                    } => sender.send_modify(|rip_status| {
                        rip_status.progress = Some(Progress {
                            cprog_value: current as _,
                            tprog_value: total as _,
                            max_value: max as _,
                        })
                    }),
                    MakemkvMessage::Message { message } => sender.send_modify(|rip_status| {
                        rip_status.logs.push(message);
                    }),
                    _ => continue,
                }
            }
            match makemkv.finish().await {
                Ok(exit_status) if exit_status.success() => {
                    sender.send_modify(|rip_status| rip_status.set_status(JobStatus::Completed));
                }
                _ => {
                    sender.send_modify(|rip_status| rip_status.set_status(JobStatus::Error));
                }
            }
            rip_dir.complete();
            if autoeject {
                let _ = ejector.eject();
            }
            for _ in 0..15 {
                if let Ok(_) = notif_client
                    .rip_finished(RipFinishedRequest {
                        controller_id: controller_id.clone(),
                        job_id: request.job_id,
                    })
                    .await
                {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        jobs.insert(
            request.job_id,
            RipJob {
                job_id: request.job_id,
                drive_id: request.drive_id as _,
                job_status: receiver,
                task_handle,
            },
        );

        return Ok(tonic::Response::new(RipMediaResponse {}));
    }

    async fn get_job_status(
        &self,
        request: tonic::Request<GetJobStatusRequest>,
    ) -> Result<tonic::Response<RipStatus>, tonic::Status> {
        let request = request.into_inner();

        let jobs = self.rip_jobs.read().await;
        let job = jobs
            .get(&request.job_id)
            .ok_or_else(|| tonic::Status::not_found("The requested job was not found"))?;
        return Ok(tonic::Response::new(job.job_status.borrow().clone()));
    }

    type WatchRipJobStream = WatchRipJobStream;

    async fn watch_rip_job(
        &self,
        request: tonic::Request<WatchRipJobRequest>,
    ) -> Result<tonic::Response<Self::WatchRipJobStream>, tonic::Status> {
        let request = request.into_inner();

        let jobs = self.rip_jobs.read().await;
        let job = jobs
            .get(&request.job_id)
            .ok_or_else(|| tonic::Status::not_found("The requested job was not found"))?;
        let receiver = job.job_status.clone();
        drop(jobs);

        return Ok(tonic::Response::new(WatchRipJobStream {
            starting_value: RipStatus::default(),
            log_offset: 0,
            receiver: WatchStream::new(receiver),
            buffer: VecDeque::new(),
        }));
    }

    async fn reap_job(
        &self,
        request: tonic::Request<ReapJobRequest>,
    ) -> Result<tonic::Response<ReapJobResponse>, tonic::Status> {
        let request = request.into_inner();

        let mut jobs = self.rip_jobs.write().await;
        jobs.remove(&request.job_id);

        return Ok(tonic::Response::new(ReapJobResponse {}));
    }
}

pub struct WatchRipJobStream {
    starting_value: RipStatus,
    log_offset: usize,
    receiver: WatchStream<RipStatus>,
    buffer: VecDeque<rip_update::RipUpdate>,
}
impl futures::Stream for WatchRipJobStream {
    type Item = Result<RipUpdate, tonic::Status>;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            // Clear the cache...
            if let Some(item) = this.buffer.pop_front() {
                return std::task::Poll::Ready(Some(Ok(RipUpdate {
                    rip_update: Some(item),
                })));
            }

            // ... then go grab a new result
            // NOTE: This isn't the most efficient method, since WatchStream will clone the logs
            // on every change, but I really just want something that works right now. Go for this
            // when optimizing.
            let poll_result = this.receiver.poll_next_unpin(cx);
            match poll_result {
                std::task::Poll::Pending => return std::task::Poll::Pending,
                std::task::Poll::Ready(None) => {
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Ready(Some(new_value)) => {
                    if new_value.status != this.starting_value.status {
                        this.starting_value.status = new_value.status;
                        this.buffer
                            .push_back(rip_update::RipUpdate::Status(new_value.status));
                    }
                    if new_value.cprog_title != this.starting_value.cprog_title {
                        this.starting_value.cprog_title = new_value.cprog_title.clone();
                        this.buffer.push_back(rip_update::RipUpdate::CprogTitle(
                            new_value.cprog_title.clone(),
                        ));
                    }
                    if new_value.tprog_title != this.starting_value.tprog_title {
                        this.starting_value.tprog_title = new_value.tprog_title.clone();
                        this.buffer.push_back(rip_update::RipUpdate::TprogTitle(
                            new_value.tprog_title.clone(),
                        ));
                    }
                    if new_value.progress != this.starting_value.progress {
                        this.starting_value.progress = new_value.progress;
                        if let Some(progress) = new_value.progress {
                            this.buffer
                                .push_back(rip_update::RipUpdate::ProgressValues(progress));
                        }
                    }
                    for i in this.log_offset..new_value.logs.len() {
                        let message = new_value.logs[i].clone();
                        this.buffer
                            .push_back(rip_update::RipUpdate::LogMessage(message));
                    }
                    this.log_offset = new_value.logs.len();
                    drop(new_value);
                }
            }
        }
    }
}

/// This object allows you to create a directory that will be automatically deleted if
/// the task is cancelled.
pub struct RipDir {
    dir: PathBuf,
}
impl RipDir {
    /// Create a new rip directory for a specific job
    pub async fn new(rip_directory: &Path, job_id: i64) -> std::io::Result<Self> {
        let rip_dir = rip_directory.join("rips").join(job_id.to_string());
        tokio::fs::create_dir(&rip_dir).await?;

        return Ok(Self { dir: rip_dir });
    }
    /// Drops the directory without deleting its contents
    pub fn complete(self) {
        std::mem::forget(self);
    }
}
impl Drop for RipDir {
    fn drop(&mut self) {
        let dir = std::mem::take(&mut self.dir);
        tokio::task::spawn(async move {
            if let Err(err) = tokio::fs::remove_dir_all(&dir).await {
                println!("An error occurred while deleting RipDir {dir:?}:\n{err}");
            }
        });
    }
}

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, short)]
    config: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DriveControllerConfig {
    shared_directory: PathBuf,
    serve_address: String,
    coordinator_address: String,
    controller_id: String,
    drives: Vec<DriveInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DriveInfo {
    name: String,
    path: String,
}

fn main() {
    let args = Args::parse();

    let config_file = std::fs::File::open(args.config).expect("Couldn't open config");
    let config: DriveControllerConfig =
        serde_yaml::from_reader(config_file).expect("Couldn't read config");

    let mut drives = Vec::new();

    for drive in config.drives {
        drives.push(Drive {
            ejector: Arc::new(
                eject::device::Device::open(&drive.path).expect("Couldn't open drives"),
            ),
            path: drive.path,
            name: drive.name,
        });
    }

    let rip_dir = config.shared_directory.join("rips");
    let _ = std::fs::create_dir(&rip_dir);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .expect("Couldn't build tokio runtime")
        .block_on(async move {
            let coordinator_endpoint = Endpoint::from_str(&config.coordinator_address)
                .expect("Invalid coordinator address")
                .connect_lazy();
            let coordinator_client =
                CoordinatorNotificationServiceClient::new(coordinator_endpoint);

            let drives = Arc::new(drives);

            // Watch for disc insert events
            {
                let drives = Arc::clone(&drives);
                let mut coordinator_client = coordinator_client.clone();
                let controller_id = config.controller_id.clone();
                tokio::task::spawn(async move {
                    let mut stream = disc_insert_events();
                    while let Some(event) = stream.next().await {
                        for (i, drive) in drives.iter().enumerate() {
                            if event.device != drive.path {
                                continue;
                            }
                            let _ = coordinator_client
                                .disc_inserted(DiscInsertedRequest {
                                    controller_id: controller_id.clone(),
                                    drive_id: i as _,
                                    name: Some(event.disc_name),
                                })
                                .await;
                            break;
                        }
                    }
                });
            }

            let reflection = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(
                    mediacorral_proto::mediacorral::FILE_DESCRIPTOR_SET,
                )
                .with_service_name("mediacorral.drive_controller.v1.DriveControllerService")
                .build_v1()
                .unwrap();

            Server::builder()
                .add_service(reflection)
                .add_service(DriveControllerServiceServer::new(DriveController {
                    id: config.controller_id.clone(),
                    coordinator_notifs: coordinator_client,
                    shared_directory: config.shared_directory,
                    drives,
                    rip_jobs: RwLock::new(HashMap::new()),
                }))
                .serve(config.serve_address.parse().expect("Invalid address"))
                .await
                .unwrap();
        });
}
