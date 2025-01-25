use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use sqlx::SqlitePool;

use crate::{
    blob_storage::BlobStorageController, config::TMDB_API_KEY, drive_controller::DriveController,
    tagging::importers::tmdb::TmdbImporter,
};

pub struct Application {
    db: Arc<SqlitePool>,
    blob_controller: Arc<BlobStorageController>,
    tmdb_importer: TmdbImporter,
    drives: HashMap<String, DriveController>,
}
impl Application {
    pub async fn new(db: Arc<SqlitePool>, path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let blob_controller = Arc::new(BlobStorageController::new(Arc::clone(&db), path).await?);
        let tmdb_importer = TmdbImporter::new(
            Arc::clone(&db),
            String::clone(&TMDB_API_KEY),
            Arc::clone(&blob_controller),
        )?;
        return Ok(Self {
            db,
            blob_controller,
            tmdb_importer,
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

    pub fn importer(&self) -> &TmdbImporter {
        return &self.tmdb_importer;
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
