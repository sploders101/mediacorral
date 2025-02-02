use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, watch};

use crate::{
    async_udev::get_disc_name,
    blob_storage::BlobStorageController,
    makemkv::{
        messaging::{MakemkvMessage, ProgressBar},
        Makemkv,
    },
    tagging::types::SuspectedContents,
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum DriveCommand {
    /// Rip the disc in the drive
    Rip {
        disc_name: Option<String>,
        suspected_contents: Option<SuspectedContents>,
        autoeject: bool,
    },
    /// Eject the drive
    Eject,
    /// Retract the drive
    Retract,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum ActiveDriveCommand {
    #[default]
    None,
    /// The last command resulted in an error
    Error { message: String },
    /// Actively ripping media
    Ripping {
        /// The ID for this job
        job_id: i64,
        /// Title for the "Current Progress" bar
        cprog_title: String,
        /// Value for the "Current Progress" bar
        cprog_value: usize,
        /// Title for the "Current Progress" bar
        tprog_title: String,
        /// Value for the "Current Progress" bar
        tprog_value: usize,
        /// Maximum progress value, used for calculating percentages
        max_prog_value: usize,
        /// Log messages
        logs: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum DriveStatus {
    #[default]
    Unknown,
    Empty,
    TrayOpen,
    NotReady,
    Loaded,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct DriveState {
    pub active_command: ActiveDriveCommand,
    pub status: DriveStatus,
}

macro_rules! setup_macros {
    ($state_sender:ident, $command:tt) => {
        /// Communicates an error back through the DriveController channel, optionally skipping the command (default).
        macro_rules! throw {
            (nocont $err:ident) => {
                {
                    $state_sender.send_modify(move |state| state.active_command = ActiveDriveCommand::Error {
                        message: format!("{}", $err),
                    });
                }
            };
            ($err:ident) => {
                {
                    throw!(nocont $err);
                    continue $command;
                }
            };
        }
        /// Extracts an `Ok` value. If the value was `Err`, the `rip_dir` is discarded and the command is skipped.
        macro_rules! try_skip {
            ($item:expr) => {
                match $item {
                    Ok(result) => result,
                    Err(err) => {
                        throw!(err);
                    }
                }
            };
            ($item:expr, discard $rip_dir:ident) => {
                match $item {
                    Ok(result) => result,
                    Err(err) => {
                        $rip_dir.discard().await;
                        throw!(err);
                    }
                }
            };
        }
    }
}

/// Interface for interacting with a disc drive.
///
/// This interface should not be instantiated more than once for a
/// single drive. These should be created at program init and re-used
/// for future requests.
pub struct DriveController {
    drive: String,
    current_state: watch::Receiver<DriveState>,
    commander: mpsc::Sender<DriveCommand>,
    task: tokio::task::JoinHandle<()>,
}
impl DriveController {
    pub async fn new(
        drive: String,
        blob_controller: Arc<BlobStorageController>,
    ) -> std::io::Result<Self> {
        let (state_sender, current_state) = watch::channel(DriveState::default());
        let (commander, mut command_receiver) = mpsc::channel(1);
        let mut ejector = eject::device::Device::open(&drive)?;

        let task = {
            let drive = drive.clone();
            tokio::task::spawn(async move {
                'command: loop {
                    // Give macros access to contextual state
                    setup_macros!(state_sender, 'command);

                    enum Event {
                        Stop,
                        PollTimeout,
                        Message(DriveCommand),
                    }

                    let result: Event = tokio::select! {
                        result = command_receiver.recv() => match result {
                            Some(result) => Event::Message(result),
                            None => Event::Stop,
                        },
                        _ = tokio::time::sleep(Duration::from_secs(1)) => Event::PollTimeout,
                    };
                    match result {
                        Event::Stop => {
                            break 'command;
                        }
                        Event::PollTimeout => {
                            let (status, ejector_return) =
                                tokio::task::spawn_blocking(move || (ejector.status(), ejector))
                                    .await
                                    .unwrap();
                            ejector = ejector_return;
                            state_sender.send_if_modified(move |value|{
                                let new_status = match status {
                                    Ok(eject::device::DriveStatus::Empty) => DriveStatus::Empty,
                                    Ok(eject::device::DriveStatus::TrayOpen) => DriveStatus::TrayOpen,
                                    Ok(eject::device::DriveStatus::NotReady) => DriveStatus::NotReady,
                                    Ok(eject::device::DriveStatus::Loaded) => DriveStatus::Loaded,
                                    Err(_) => DriveStatus::Unknown,
                                };
                                let was_changed = value.status != new_status;
                                value.status = new_status;
                                return was_changed;
                            });
                        }
                        Event::Message(DriveCommand::Rip {
                            disc_name,
                            suspected_contents,
                            autoeject,
                        }) => {
                            // Attempt to get disc name if unspecified
                            let disc_name = match disc_name {
                                Some(disc_name) => Some(disc_name),
                                None => get_disc_name(&drive).await,
                            };

                            // Allocate rip directory
                            let rip_dir = match blob_controller
                                .create_rip_dir(disc_name, suspected_contents)
                                .await
                            {
                                Ok(rip_dir) => rip_dir,
                                Err(err) => throw!(err),
                            };

                            // Start rip job and communicate status updates
                            let rip_job =
                                try_skip!(Makemkv::rip(&drive, &rip_dir.as_ref()), discard rip_dir);
                            try_skip!(handle_events(rip_dir.job_id(), rip_job, &state_sender).await, discard rip_dir);
                            tokio::task::spawn(rip_dir.import());
                            if autoeject {
                                try_skip!(ejector.eject());
                            }
                        }
                        Event::Message(DriveCommand::Eject) => try_skip!(ejector.eject()),
                        Event::Message(DriveCommand::Retract) => try_skip!(ejector.retract()),
                    }
                }
            })
        };

        return Ok(DriveController {
            drive,
            current_state,
            commander,
            task,
        });
    }

    pub fn get_devname(&self) -> &str {
        return &self.drive;
    }

    pub async fn get_disc_name(&self) -> Option<String> {
        return get_disc_name(&self.drive).await;
    }

    // TODO: Fix race condition (if two rip calls happen simultaneously, one should fail)
    /// Rip the disc in the drive and add its contents to storage, ready to catalogue.
    pub fn rip(
        &self,
        disc_name: Option<String>,
        suspected_contents: Option<SuspectedContents>,
        autoeject: bool,
    ) {
        let _ = self.commander.try_send(DriveCommand::Rip {
            disc_name,
            suspected_contents,
            autoeject,
        });
    }

    /// Ejects the drive tray
    pub fn eject(&self) {
        let _ = self.commander.try_send(DriveCommand::Eject);
    }

    /// Retracts the drive tray
    pub fn retract(&self) {
        let _ = self.commander.try_send(DriveCommand::Retract);
    }

    pub fn watch_state(&self) -> watch::Receiver<DriveState> {
        return self.current_state.clone();
    }
}

/// Handles events from a rip job and keeps the drive state updated
async fn handle_events(
    job_id: i64,
    mut rip_job: Makemkv,
    sender: &watch::Sender<DriveState>,
) -> anyhow::Result<()> {
    while let Some(message) = rip_job.next_event().await? {
        sender.send_modify(move |state| {
            match state.active_command {
                ActiveDriveCommand::Ripping { .. } => {}
                _ => {
                    state.active_command = ActiveDriveCommand::Ripping {
                        job_id,
                        cprog_title: String::new(),
                        cprog_value: 0,
                        tprog_title: String::new(),
                        tprog_value: 0,
                        max_prog_value: 1,
                        logs: Vec::new(),
                    }
                }
            }
            match state.active_command {
                ActiveDriveCommand::Ripping {
                    ref mut cprog_title,
                    ref mut cprog_value,
                    ref mut tprog_title,
                    ref mut tprog_value,
                    ref mut max_prog_value,
                    ref mut logs,
                    ..
                } => match message {
                    MakemkvMessage::Message { message } => logs.push(message),
                    MakemkvMessage::ProgressTitle {
                        bar: ProgressBar::Current,
                        name,
                        ..
                    } => *cprog_title = name,
                    MakemkvMessage::ProgressTitle {
                        bar: ProgressBar::Total,
                        name,
                        ..
                    } => *tprog_title = name,
                    MakemkvMessage::ProgressValue {
                        current,
                        total,
                        max,
                    } => {
                        *cprog_value = current;
                        *tprog_value = total;
                        *max_prog_value = max;
                    }
                    _ => {}
                },
                _ => unreachable!(),
            }
        });
    }
    sender.send_modify(|state| state.active_command = ActiveDriveCommand::None);
    return Ok(());
}
