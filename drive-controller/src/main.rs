use std::{
    collections::HashMap,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use clap::Parser;
use futures::StreamExt;
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, BufReader},
    process::Command,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};
use tracing_subscriber::EnvFilter;

use crate::{
    makemkv::{Makemkv, messaging::MakemkvMessage},
    proto::mediacorral::drive_coordinator::v1::{
        DiscoveryInfo, DriveConnectionRequest, DriveConnectionResponse, DriveStatus,
        DriveStatusTag, RipMediaCommand, RipStatus, RipStatusTag, TrayCommand, UploadFileRequest,
        UploadFileRequestHeader, drive_connection_request, drive_connection_response,
        drive_coordinator_service_client::DriveCoordinatorServiceClient, upload_file_request,
    },
};

mod makemkv;
mod proto;

const UPLOAD_CHUNK_SIZE: usize = 1024 * 1024 * 2; // 2MiB

pub struct Drive {
    id: String,
    path: String,
    name: String,
    ejector: Arc<eject::device::Device>,
}
impl std::fmt::Debug for Drive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_fmt(format_args!(
            "Drive {{ id: {:?}, path: {:?}, name: {:?} }}",
            self.id, self.path, self.name
        ));
    }
}

/// This object allows you to create a directory that will be automatically deleted when
/// the task ends.
#[derive(Debug)]
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
    rip_directory: PathBuf,
    coordinator_address: String,
    drives: Vec<DriveInfo>,
    max_upload_queue: usize,
    drive_poll_frequency_ms: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DriveInfo {
    id: String,
    name: String,
    path: String,
}

fn main() {
    // Set up logging with environment filter
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let config_file = std::fs::File::open(args.config).expect("Couldn't open config");
    let config: Arc<DriveControllerConfig> =
        Arc::new(serde_yaml::from_reader(config_file).expect("Couldn't read config"));

    let mut drives = Vec::new();

    for drive in config.drives.iter() {
        let drive_path = String::from(
            std::fs::canonicalize(&drive.path)
                .expect("Couldn't open drives")
                .to_str()
                .expect("Unable to process path"),
        );
        drives.push(Drive {
            id: drive.id.clone(),
            ejector: Arc::new(
                eject::device::Device::open(&drive.path).expect("Couldn't open drives"),
            ),
            path: drive_path,
            name: drive.name.clone(),
        });
    }

    let _ = std::fs::create_dir(&config.rip_directory);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .expect("Couldn't build tokio runtime")
        .block_on(async move {
            let endpoint = Endpoint::from_str(config.coordinator_address.as_str())
                .expect("Invalid coordinator_address")
                .connect_lazy();

            let coordinator_client = DriveCoordinatorServiceClient::new(endpoint);

            let (upload_queue_sender, upload_queue_receiver) =
                tokio::sync::mpsc::channel(config.max_upload_queue);

            for drive in drives {
                tokio::task::spawn(control_drive(
                    coordinator_client.clone(),
                    drive,
                    Arc::clone(&config),
                    upload_queue_sender.clone(),
                ));
            }

            // This blocks the main task so it never exits.
            rip_job_postprocessor(coordinator_client.clone(), upload_queue_receiver).await;
        });
}

/// When a rip job is finished, it needs to be uploaded to the server and finalized. This process can take a while, so
/// there's not much point making the drives wait for this process to complete before accepting a new disc. To account
/// for this, `control_drive` function can pass finished rip jobs off to this task for finalization so it can continue
/// operations on its own. Jobs are processed sequentially since they are IO-bound. Parallelization likely won't help
/// much here.
#[tracing::instrument]
async fn rip_job_postprocessor(
    mut client: DriveCoordinatorServiceClient<Channel>,
    mut upload_queue: tokio::sync::mpsc::Receiver<(RipStatus, RipDir)>,
) {
    while let Some((status, dir)) = upload_queue.recv().await {
        let rip_job = status.rip_job;
        let mut contents = match tokio::fs::read_dir(&dir.dir).await {
            Ok(contents) => contents,
            Err(err) => {
                tracing::error!("Failed to list contents of rip dir: {}", err);
                continue;
            }
        };
        while let Some(dir_entry) = rev_ro(contents.next_entry().await) {
            let dir_entry = match dir_entry {
                Ok(dir_entry) => dir_entry,
                Err(err) => {
                    tracing::error!("Failed to list contents of rip dir: {}", err);
                    continue;
                }
            };
            // Skip non-files
            match dir_entry.file_type().await {
                Ok(ftype) if !ftype.is_file() => {
                    continue;
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("Failed to get file type: {}", err);
                    continue;
                }
            }
            let path = dir_entry.path();
            // Skip non-mkvs
            if path.extension().and_then(|ext| ext.to_str()) != Some("mkv") {
                continue;
            }
            // Skip invalid filenames
            let file_name = match dir_entry.file_name().into_string() {
                Ok(file_name) => file_name,
                Err(_) => {
                    tracing::error!("Invalid file name");
                    continue;
                }
            };

            // Upload
            let file = match tokio::fs::File::open(&path).await {
                Ok(file) => file,
                Err(err) => {
                    tracing::error!("Failed to open mkv file: {}", err);
                    continue;
                }
            };
            let file_size = file
                .metadata()
                .await
                .map(|stats| stats.size())
                .ok()
                .unwrap_or_default();
            let mut file = BufReader::new(file);
            let (upload_sender, upload_receiver) = tokio::sync::mpsc::channel(1);
            tokio::task::spawn(async move {
                let mut buf = [0u8; UPLOAD_CHUNK_SIZE];
                let mut hasher = md5::Context::new();
                if let Err(err) = upload_sender
                    .send(UploadFileRequest {
                        message: Some(upload_file_request::Message::Header(
                            UploadFileRequestHeader {
                                rip_job,
                                file_name,
                                file_size,
                            },
                        )),
                    })
                    .await
                {
                    tracing::error!("Failed to upload file: {}", err);
                    return;
                }
                loop {
                    match file.read(&mut buf).await {
                        Ok(0) => {
                            let digest = hasher.finalize();
                            if let Err(err) = upload_sender
                                .send(UploadFileRequest {
                                    message: Some(upload_file_request::Message::Md5Hash(
                                        digest.0.into(),
                                    )),
                                })
                                .await
                            {
                                tracing::error!("Failed to upload file: {}", err);
                            };
                            return;
                        }
                        Ok(bytes_read) => {
                            hasher.consume(&buf[0..bytes_read]);
                            if let Err(err) = upload_sender
                                .send(UploadFileRequest {
                                    message: Some(upload_file_request::Message::DataChunk(
                                        buf[0..bytes_read].into(),
                                    )),
                                })
                                .await
                            {
                                tracing::error!("Failed to upload file: {}", err);
                                return;
                            };
                        }
                        Err(err) => {
                            tracing::error!("Error reading file: {}", err);
                            // Return immediately. This will prevent the hash from being sent
                            // and cancel the request.
                            return;
                        }
                    }
                }
            });
            if let Err(err) = client
                .upload_file(ReceiverStream::new(upload_receiver))
                .await
            {
                tracing::error!("Failed to upload file: {}", err);
            }
        }
    }
}

/// Reverses `Result<Option<T>, E>` types into `Option<Result<T, E>>`. Useful for async loops.
fn rev_ro<T, E>(item: Result<Option<T>, E>) -> Option<Result<T, E>> {
    return match item {
        Ok(Some(item)) => Some(Ok(item)),
        Ok(None) => None,
        Err(err) => Some(Err(err)),
    };
}

/// The main drive control loop. One of these is spawned for each drive, and only fails on hardware connection errors.
/// This handles keeping state objects up to date and running commands from the server.
#[tracing::instrument]
async fn control_drive(
    coordinator_client: DriveCoordinatorServiceClient<Channel>,
    drive: Drive,
    config: Arc<DriveControllerConfig>,
    upload_queue: tokio::sync::mpsc::Sender<(RipStatus, RipDir)>,
) {
    let (command_sender, mut command_receiver) =
        tokio::sync::mpsc::channel::<DriveConnectionResponse>(50);

    let (drive_status_sender, drive_status_receiver) =
        tokio::sync::watch::channel(DriveStatus::default());

    let (rip_watcher_sender, mut rip_watcher_receiver) =
        tokio::sync::mpsc::channel::<tokio::sync::watch::Receiver<RipStatus>>(50);

    {
        let coordinator_client = coordinator_client.clone();
        tokio::task::spawn(async move {
            let mut rip_status_streams = HashMap::new();
            loop {
                let result = handle_connection(
                    coordinator_client.clone(),
                    drive.id.clone(),
                    drive.name.clone(),
                    command_sender.clone(),
                    drive_status_receiver.clone(),
                    &mut rip_watcher_receiver,
                    &mut rip_status_streams,
                )
                .await;
                if let Err(err) = result {
                    tracing::error!("Restarting `handle_registration`. Error: {err:?}");
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    let mut update_interval =
        tokio::time::interval(Duration::from_millis(config.drive_poll_frequency_ms));
    update_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    struct RipJobCtx {
        status: tokio::sync::watch::Sender<RipStatus>,
        rip_dir: RipDir,
        makemkv: Makemkv,
        autoeject: bool,
    }
    let mut rip_job: Option<RipJobCtx> = None;

    loop {
        #[derive(Debug)]
        enum SelectResult {
            Message(DriveConnectionResponse),
            MakemkvUpdate(std::io::Result<Option<MakemkvMessage>>),
            DriveStatusUpdate,
        }
        let result = tokio::select! {
            // None shouldn't be possible, since command_sender is held
            message = command_receiver.recv() => SelectResult::Message(message.unwrap()),
            message = next_mmkv_if_some(rip_job.as_mut().map(|job| &mut job.makemkv)) => SelectResult::MakemkvUpdate(message),
            _ = update_interval.tick() => SelectResult::DriveStatusUpdate,
        };
        match result {
            SelectResult::Message(message) => match message.message {
                Some(drive_connection_response::Message::TrayCommand(tray_command)) => {
                    match TrayCommand::try_from(tray_command).unwrap_or_default() {
                        TrayCommand::Unspecified => continue,
                        TrayCommand::OpenTray => {
                            let ejector = Arc::clone(&drive.ejector);
                            tokio::task::spawn_blocking(move || {
                                if let Err(err) = ejector.eject() {
                                    tracing::error!("Failed to eject drive: {err}");
                                }
                            });
                        }
                        TrayCommand::CloseTray => {
                            let ejector = Arc::clone(&drive.ejector);
                            tokio::task::spawn_blocking(move || {
                                if let Err(err) = ejector.retract() {
                                    tracing::error!("Failed to retract drive: {err}");
                                }
                            });
                        }
                    }
                }
                Some(drive_connection_response::Message::RipMedia(RipMediaCommand {
                    job_id,
                    autoeject,
                })) => {
                    if rip_job.is_some() {
                        // Might be a race condition.
                        // TODO: Check if the job id is different
                        continue;
                    }
                    let rip_dir = RipDir::new(&config.rip_directory, job_id)
                        .await
                        .expect("Unable to create rip directory");
                    let mmkv =
                        Makemkv::rip(&drive.path, &rip_dir.dir).expect("Unable to start rip job");
                    let status = RipStatus {
                        rip_job: job_id,
                        status: RipStatusTag::Running as _,
                        ..Default::default()
                    };
                    let (status, status_receiver) = tokio::sync::watch::channel(status);
                    rip_watcher_sender
                        .send(status_receiver)
                        .await
                        .expect("Rip status receiver dropped.");
                    rip_job = Some(RipJobCtx {
                        status,
                        rip_dir,
                        makemkv: mmkv,
                        autoeject,
                    });
                    // Trigger a status update
                    drive_status_sender.send_modify(|status| {
                        status.active_rip_job = Some(job_id);
                    });
                }
                None => continue,
            },
            SelectResult::MakemkvUpdate(update) => {
                let ctx = rip_job
                    .as_mut()
                    .expect("Rip job context missing in update handler");
                match update.unwrap() {
                    Some(MakemkvMessage::Message { message }) => {
                        ctx.status.send_modify(|status| status.logs.push(message));
                    }
                    Some(MakemkvMessage::ProgressTitle { bar, name, .. }) => {
                        ctx.status.send_modify(|status| match bar {
                            makemkv::messaging::ProgressBar::Current => status.cprog_title = name,
                            makemkv::messaging::ProgressBar::Total => status.tprog_title = name,
                        });
                    }
                    Some(MakemkvMessage::ProgressValue {
                        current,
                        total,
                        max,
                    }) => {
                        ctx.status.send_modify(|status| {
                            status.cprog_value = current as _;
                            status.tprog_value = total as _;
                            status.max_prog_value = max as _;
                        });
                    }
                    Some(_) => {}
                    None => {
                        // Job has ended. Time to pass it off.
                        let ctx = rip_job
                            .take()
                            .expect("Rip job context missing in update handler");
                        ctx.status
                            .send_modify(|job| job.set_status(RipStatusTag::Completed));
                        let status = ctx.status.borrow().clone();
                        let _ = upload_queue.send((status, ctx.rip_dir)).await;
                        if ctx.autoeject {
                            if let Err(err) = drive.ejector.eject() {
                                tracing::error!("Failed to eject drive: {}", err);
                            }
                        }
                    }
                }
            }
            SelectResult::DriveStatusUpdate => {
                // Update the drive status
                match drive.ejector.status() {
                    Ok(status) => {
                        let disc_name = get_disc_name(&drive.path).await;
                        tracing::debug!("Got disc name {disc_name:?}");
                        let status = DriveStatus {
                            status: match status {
                                eject::device::DriveStatus::Empty => DriveStatusTag::Empty,
                                eject::device::DriveStatus::TrayOpen => DriveStatusTag::TrayOpen,
                                eject::device::DriveStatus::NotReady => DriveStatusTag::NotReady,
                                eject::device::DriveStatus::Loaded => DriveStatusTag::DiscLoaded,
                            } as _,
                            disc_name,
                            active_rip_job: rip_job
                                .as_ref()
                                .map(|rip_job| rip_job.status.borrow().rip_job),
                        };
                        drive_status_sender.send_if_modified(move |oldval| {
                            let modified = status != *oldval;
                            *oldval = status;
                            return modified;
                        });
                    }
                    Err(err) => {
                        tracing::error!("Failed to get drive status: {err}");
                    }
                }
            }
        }
    }
}

/// Handles the connection to the server and relays messages to the main control loop. This process is made to be
/// ephemeral so server disconnects don't interrupt operation of the drive. This will handle watching updates and
/// relaying them to the server, as well as relaying commands from the server to the drive control loop.
///
/// TODO: Handle connection drops more gracefully. Currently, when the connection is re-established, updates can
/// be missed.
#[tracing::instrument]
async fn handle_connection(
    mut client: DriveCoordinatorServiceClient<Channel>,
    drive_id: String,
    drive_name: String,
    command_sender: tokio::sync::mpsc::Sender<DriveConnectionResponse>,
    mut drive_status_receiver: tokio::sync::watch::Receiver<DriveStatus>,
    rip_status_receiver: &mut tokio::sync::mpsc::Receiver<tokio::sync::watch::Receiver<RipStatus>>,
    rip_status_streams: &mut HashMap<i64, tokio::sync::watch::Receiver<RipStatus>>,
) -> anyhow::Result<()> {
    let (sender, receiver) = tokio::sync::mpsc::channel(50);
    let response = client
        .connect_drive(ReceiverStream::new(receiver))
        .await
        .context("Failed to connect to server")?;

    // Send discovery message
    sender
        .send(DriveConnectionRequest {
            message: Some(drive_connection_request::Message::Discovery(
                DiscoveryInfo {
                    drive_id,
                    drive_name,
                },
            )),
        })
        .await
        .context("Failed to send discovery packet")?;

    // Mark all values as unseen
    drive_status_receiver.mark_changed();
    for stream in rip_status_streams.values_mut() {
        stream.mark_changed();
    }

    // Handle communication between server and drive loop
    let mut response = response.into_inner();
    loop {
        enum SelectResult {
            Message(DriveConnectionResponse),
            SendDriveUpdate,
            WatchRipStatus(tokio::sync::watch::Receiver<RipStatus>),
            RipStatusUpdate(RipStatus),
        }
        let result = tokio::select! {
            result = response.next() => {
                match result {
                    Some(Ok(message)) => SelectResult::Message(message),
                    Some(Err(err)) => {
                        return Err(err).context("Error receiving message from server");
                    }
                    None => {
                        return Ok(());
                    }
                }
            },
            Ok(()) = drive_status_receiver.changed() => SelectResult::SendDriveUpdate,
            Some(receiver) = rip_status_receiver.recv() => SelectResult::WatchRipStatus(receiver),
            Some(rip_status) = watch_all(rip_status_streams.values_mut()) => SelectResult::RipStatusUpdate(rip_status),
        };
        match result {
            SelectResult::Message(message) => command_sender
                .send(message)
                .await
                .context("Command loop exited")?,
            SelectResult::SendDriveUpdate => {
                let status = drive_status_receiver.borrow_and_update().clone();
                sender
                    .send(DriveConnectionRequest {
                        message: Some(drive_connection_request::Message::DriveStatusUpdate(status)),
                    })
                    .await
                    .context("Failed to send update")?
            }
            SelectResult::WatchRipStatus(message) => {
                let rip_job = message.borrow().rip_job;
                rip_status_streams.insert(rip_job, message);
            }
            SelectResult::RipStatusUpdate(rip_status) => {
                let rip_job = rip_status.rip_job;
                let rip_finished = matches!(
                    rip_status.status(),
                    RipStatusTag::Completed | RipStatusTag::Error
                );
                sender
                    .send(DriveConnectionRequest {
                        message: Some(drive_connection_request::Message::RipStatusUpdate(
                            rip_status,
                        )),
                    })
                    .await
                    .context("Failed to send update")?;
                if rip_finished {
                    rip_status_streams.remove(&rip_job);
                }
            }
        }
    }
}

pub async fn get_disc_name(device: &str) -> Option<String> {
    let output = Command::new("blkid")
        .arg("-o")
        .arg("value")
        .arg("-s")
        .arg("LABEL")
        .arg(device)
        .output()
        .await
        .ok()?;

    let label: String = String::from_utf8_lossy(&output.stdout).trim().into();
    if label.len() == 0 {
        return None;
    }
    return Some(label);
}

async fn next_mmkv_if_some(
    makemkv: Option<&mut Makemkv>,
) -> std::io::Result<Option<MakemkvMessage>> {
    match makemkv {
        Some(makemkv) => makemkv.next_event().await,
        None => std::future::pending().await,
    }
}

async fn watch_all(
    receivers: impl Iterator<Item = &mut tokio::sync::watch::Receiver<RipStatus>>,
) -> Option<RipStatus> {
    let things = receivers
        .map(|receiver| {
            Box::pin(async move {
                if let Err(_) = receiver.changed().await {
                    return None;
                }
                Some(receiver.borrow().clone())
            })
        })
        .collect::<Vec<_>>();
    if things.len() == 0 {
        return None;
    }
    return futures::future::select_all(things).await.0;
}
