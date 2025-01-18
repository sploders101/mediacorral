use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, watch};

use crate::makemkv::Makemkv;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum DriveCommand {
    Rip,
    Eject,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub enum ActiveDriveCommand {
    #[default]
    None,
    /// Actively ripping media
    Ripping {
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
    },
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Eq, PartialEq)]
pub struct DriveState {
    active_command: ActiveDriveCommand,
}

/// Interface for interacting with a disc drive.
///
/// This interface should not be instantiated more than once for a
/// single drive. These should be created at program init and re-used
/// for future requests.
pub struct DriveController {
    current_state: watch::Receiver<DriveState>,
    commander: mpsc::Sender<DriveCommand>,
    task: tokio::task::JoinHandle<()>,
}
impl DriveController {
    pub fn new(drive: PathBuf) -> std::io::Result<Self> {
        let (state_sender, current_state) = watch::channel(DriveState::default());
        let (commander, mut command_receiver) = mpsc::channel(1);
        let ejector = eject::device::Device::open(drive)?;

        let task = tokio::task::spawn(async move {
            loop {
                while let Some(command) = command_receiver.recv().await {
                }
            }
        });

        return Ok(DriveController {
            current_state,
            commander,
            task,
        });
    }
}
