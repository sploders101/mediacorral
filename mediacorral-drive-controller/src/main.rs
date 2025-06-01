use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use async_udev::get_disc_name;
use clap::Parser;
use makemkv::{messaging::MakemkvMessage, Makemkv};
use mediacorral_proto::drive_controller::{
    DriveState, DriveStatusTag, EjectRequest, EjectResponse, GetDriveCountRequest,
    GetDriveCountResponse, GetDriveMetaRequest, GetDriveMetaResponse, GetDriveStateRequest,
    RetractRequest, RetractResponse, RipMediaRequest, RipMediaResponse, RipUpdate,
    drive_controller_service_server::{DriveControllerService, DriveControllerServiceServer},
    drive_state::ActiveCommand,
};
use serde::Deserialize;
use tokio::sync::RwLock;
use tonic::transport::Server;

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
    active_command: RwLock<Option<ActiveCommand>>,
}

pub struct DriveController {
    rip_directory: PathBuf,
    drives: Arc<Vec<Drive>>,
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

        let active_command = drive.active_command.read().await.clone();

        return Ok(tonic::Response::new(DriveState {
            drive_id: request.drive_id,
            status: status.into(),
            disc_name,
            active_command,
        }));
    }

    type RipMediaStream =
        Pin<Box<dyn futures::Stream<Item = Result<RipUpdate, tonic::Status>> + Send + 'static>>;

    async fn rip_media(
        &self,
        request: tonic::Request<RipMediaRequest>,
    ) -> Result<tonic::Response<Self::RipMediaStream>, tonic::Status> {
        let request = request.into_inner();

        let drive = match self.drives.get(request.drive_id as usize) {
            Some(drive) => drive,
            None => {
                return Err(tonic::Status::not_found(
                    "The requested drive was not found.",
                ));
            }
        };

        let rip_dir = RipDir::new(&self.rip_directory, request.job_id)
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
        let makemkv = Makemkv::rip(
            &drive.path,
            &self.rip_directory.join(request.job_id.to_string()),
        )
        .map_err(|err| {
            tonic::Status::internal(format!("Unknown error while spawning makemkv:\n{err}"))
        })?;

        while let Some(event) = makemkv
            .next_event()
            .await
            .map_err(|err| tonic::Status::internal(format!("{err}")))?
        {
            // TODO: Convert to RipUpdate
            match event {
            }
        }

        let output = async_stream::try_stream! {};

        return Ok(tonic::Response::new(
            Box::pin(output) as Self::RipMediaStream
        ));
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
        let rip_dir = rip_directory.join(job_id.to_string());
        tokio::fs::create_dir(&rip_dir).await?;

        return Ok(Self { dir: rip_dir });
    }
    /// Drops the directory without deleting its contents
    pub fn complete(self) {}
}
impl Drop for RipDir {
    fn drop(&mut self) {
        let dir = std::mem::take(&mut self.dir);
        tokio::task::spawn(async move {
            tokio::fs::remove_dir_all(dir).await;
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
    rip_directory: PathBuf,
    address: String,
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
            active_command: RwLock::new(None),
        });
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .expect("Couldn't build tokio runtime")
        .block_on(async move {
            Server::builder()
                .add_service(DriveControllerServiceServer::new(DriveController {
                    rip_directory: config.rip_directory,
                    drives: Arc::new(drives),
                }))
                .serve(config.address.parse().expect("Invalid address"))
                .await
                .unwrap();
        });
}
