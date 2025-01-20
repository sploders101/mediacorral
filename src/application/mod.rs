use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use sqlx::SqlitePool;

use crate::{blob_storage::BlobStorageController, drive_controller::DriveController};

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

    pub fn register_drive(&mut self, drive_path: &Path) -> anyhow::Result<()> {
        let path = canonicalize_drive_path(drive_path)?;
        self.drives.insert(
            path.clone(),
            DriveController::new(path, Arc::clone(&self.blob_controller))?,
        );

        return Ok(());
    }

    pub fn get_drive(&self, drive_path: &Path) -> anyhow::Result<&DriveController> {
        let path = canonicalize_drive_path(drive_path)?;

        return Ok(self.drives.get(&path).context("Drive not found")?);
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
