use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::Context;
use futures::lock::Mutex;

use crate::{
    CoordinatorConfig,
    blob_storage::BlobStorageController,
    managers::{
        exports::{ExportsDirError, ExportsManager},
        tmdb::TmdbImporter,
    },
};

pub struct Application {
    autorip_enabled: AtomicBool,
    pub blob_storage: BlobStorageController,
    pub tmdb_importer: TmdbImporter,
    pub exports_manager: Mutex<ExportsManager>,
}
impl Application {
    pub async fn new(config: CoordinatorConfig) -> anyhow::Result<Self> {
        let blob_dir = Path::new(&config.data_directory).join("storage");
        let exports_dir = Path::new(&config.data_directory).join("exports");
        let sqlite_path = Path::new(&config.data_directory).join("database.sqlite");

        let db = Arc::new(
            sqlx::SqlitePool::connect(sqlite_path.to_str().context("Couldn't open database")?)
                .await
                .expect("Couldn't open sqlite database"),
        );

        let blob_storage = BlobStorageController::new(Arc::clone(&db), blob_dir)
            .await
            .context("Couldn't create blob controller")?;

        let tmdb_importer = TmdbImporter::new(Arc::clone(&db), config.tmdb_api_key)
            .context("Couldn't create TMDB importer")?;

        let exports_manager = Mutex::new(
            ExportsManager::new(Arc::clone(&db), exports_dir, config.exports_dirs)
                .await
                .context("Failed to initialize exports manager")?,
        );

        return Ok(Self {
            autorip_enabled: AtomicBool::new(config.enable_autorip),
            blob_storage,
            tmdb_importer,
            exports_manager,
        });
    }

    pub async fn import_tmdb_tv(&self, tmdb_id: i32) -> anyhow::Result<()> {
        return self
            .tmdb_importer
            .import_tv(tmdb_id, Some(&self.blob_storage))
            .await;
    }

    pub async fn import_tmdb_movie(&self, tmdb_id: i32) -> anyhow::Result<()> {
        return self
            .tmdb_importer
            .import_movie(tmdb_id, Some(&self.blob_storage))
            .await;
    }

    pub async fn rebuild_exports_dir(&self, exports_dir: &String) -> Result<(), ExportsDirError> {
        return self
            .exports_manager
            .lock()
            .await
            .rebuild_dir(exports_dir, &self.blob_storage)
            .await;
    }

    pub fn get_autorip(&self) -> bool {
        return self.autorip_enabled.load(Ordering::Relaxed);
    }
    pub fn set_autorip(&self, value: bool) {
        self.autorip_enabled.store(value, Ordering::Relaxed);
    }
}
