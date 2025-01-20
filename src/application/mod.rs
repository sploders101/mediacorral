use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use futures::StreamExt;
use sqlx::SqlitePool;

use crate::{
    async_udev::disc_insert_events, blob_storage::BlobStorageController,
    drive_controller::DriveController,
};

pub struct Application {
    db: Arc<SqlitePool>,
    blob_controller: Arc<BlobStorageController>,
    drives: HashMap<String, DriveController>,
}
impl Application {
    pub async fn new(db: Arc<SqlitePool>, path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        return Ok(Self {
            blob_controller: Arc::new(BlobStorageController::new(Arc::clone(&db), path).await?),
            db,
            drives: HashMap::new(),
        });
    }

    pub async fn register_drive(&mut self, drive_path: &Path) -> anyhow::Result<()> {
        let path = canonicalize_drive_path(drive_path)?;
        self.drives.insert(
            path.clone(),
            DriveController::new(path, Arc::clone(&self.blob_controller)).await?,
        );

        return Ok(());
    }

    pub fn list_drives(&self) -> impl Iterator<Item = &DriveController> {
        return self.drives.values();
    }

    pub fn get_drive(&self, drive_path: &Path) -> anyhow::Result<&DriveController> {
        let path = canonicalize_drive_path(drive_path)?;

        return Ok(self.drives.get(&path).context("Drive not found")?);
    }

    pub fn create_autoripper(self: &Arc<Self>) -> impl Future<Output = ()> {
        let this = Arc::clone(self);
        return async move {
            let mut events = std::pin::pin!(disc_insert_events());
            while let Some(insertion) = events.next().await {
                if let Ok(drive) = this.get_drive(&Path::new(&insertion.device)) {
                    println!("Ripping {}", insertion.disc_name);
                    drive.rip(Some(insertion.disc_name), None, true);
                }
            }
        };
    }
}

fn canonicalize_drive_path(drive_path: &Path) -> anyhow::Result<String> {
    return Ok(String::from(
        PathBuf::from(drive_path)
            .canonicalize()?
            .to_str()
            .context("Invalid path")?,
    ));
}
