use anyhow::Context;
use futures::{Stream, StreamExt, TryStreamExt};
use mediacorral_proto::mediacorral::{
    drive_controller::v1::{
        DriveStatusTag, GetDriveStateRequest, RipMediaRequest,
        drive_controller_service_client::DriveControllerServiceClient,
    },
    server::v1::SuspectedContents,
};
use prost::Message;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::{
    collections::HashMap,
    ffi::OsStr,
    io::ErrorKind,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReadDirStream;
use tonic::transport::{Channel, Endpoint};

use crate::{
    CoordinatorConfig,
    blob_storage::BlobStorageController,
    db,
    managers::{
        exports::{ExportsDirError, ExportsManager},
        opensubtitles::OpenSubtitles,
        tmdb::TmdbImporter,
    },
    workers::subtitles::vobsub::PartessCache,
};

pub struct Application {
    pub db: Arc<SqlitePool>,
    rip_dir: PathBuf,
    partess_cache: PartessCache,
    autorip_enabled: AtomicBool,
    pub blob_storage: BlobStorageController,
    pub tmdb_importer: TmdbImporter,
    pub ost_importer: OpenSubtitles,
    pub exports_manager: Mutex<ExportsManager>,
    pub drive_controllers: HashMap<String, DriveControllerServiceClient<Channel>>,
}
impl Application {
    pub async fn new(config: CoordinatorConfig) -> anyhow::Result<Self> {
        let rip_dir = Path::new(&config.data_directory).join("rips");
        let blob_dir = Path::new(&config.data_directory).join("blobs");
        let exports_dir = Path::new(&config.data_directory).join("exports");
        let sqlite_path = Path::new(&config.data_directory).join("database.sqlite");

        let db = Arc::new(
            sqlx::SqlitePool::connect_with(
                SqliteConnectOptions::new()
                    .filename(sqlite_path.to_str().context("Couldn't open database")?)
                    .create_if_missing(true),
            )
            .await
            .expect("Couldn't open sqlite database"),
        );
        // Populate database
        if let sqlx::Result::<(String,)>::Err(sqlx::Error::RowNotFound) = sqlx::query_as(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%';",
        )
        .fetch_one(&*db)
        .await
        {
            sqlx::query(include_str!("db/up.sql")).execute(&*db).await?;
        }

        for dir in [&rip_dir, &blob_dir, &exports_dir].into_iter() {
            if let Err(err) = tokio::fs::create_dir(dir).await {
                if err.kind() != ErrorKind::AlreadyExists {
                    Err(err)?;
                }
            }
        }
        if let Ok(false) = tokio::fs::try_exists(&sqlite_path).await {}

        let blob_storage = BlobStorageController::new(Arc::clone(&db), blob_dir)
            .await
            .context("Couldn't create blob controller")?;

        let tmdb_importer = TmdbImporter::new(Arc::clone(&db), config.tmdb_api_key)
            .context("Couldn't create TMDB importer")?;

        let ost_importer = OpenSubtitles::new(
            config.ost_login.api_key,
            config.ost_login.username,
            config.ost_login.password,
        );

        let exports_manager = Mutex::new(
            ExportsManager::new(Arc::clone(&db), exports_dir, config.exports_dirs)
                .await
                .context("Failed to initialize exports manager")?,
        );

        let mut drive_controllers = HashMap::new();
        for (id, controller) in config.drive_controllers.into_iter() {
            let drive_controller_endpoint = Endpoint::from_str(&controller)
                .expect("Invalid drive controller address")
                .connect_lazy();
            drive_controllers.insert(
                id,
                DriveControllerServiceClient::new(drive_controller_endpoint),
            );
        }

        return Ok(Self {
            db,
            rip_dir,
            partess_cache: PartessCache::new(),
            autorip_enabled: AtomicBool::new(config.enable_autorip),
            blob_storage,
            tmdb_importer,
            ost_importer,
            exports_manager,
            drive_controllers,
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
    pub async fn rip_media(
        &self,
        drive_controller: &str,
        drive_id: u32,
        suspected_contents: Option<SuspectedContents>,
        autoeject: bool,
    ) -> Result<i64, ApplicationError> {
        let mut controller = self
            .drive_controllers
            .get(drive_controller)
            .ok_or(ApplicationError::ControllerMissing)?
            .clone();

        let drive_state = controller
            .get_drive_state(GetDriveStateRequest { drive_id })
            .await?
            .into_inner();

        match drive_state.status() {
            DriveStatusTag::Unspecified => {
                return Err(ApplicationError::FailedPrecondition(String::from(
                    "The drive is in an unrecognized state. Please ensure the coordinator is up to date",
                )));
            }
            DriveStatusTag::Empty => {
                return Err(ApplicationError::FailedPrecondition(String::from(
                    "There is no disc in the drive. Please insert a disc and try again.",
                )));
            }
            DriveStatusTag::TrayOpen => {
                return Err(ApplicationError::FailedPrecondition(String::from(
                    "The drive tray is open. Please close the tray and try again.",
                )));
            }
            DriveStatusTag::NotReady => {
                return Err(ApplicationError::TemporaryFailure);
            }
            DriveStatusTag::DiscLoaded => {}
        }
        if drive_state.active_rip_job.is_some() {
            return Err(ApplicationError::FailedPrecondition(String::from(
                "The drive is already performing a rip job. Cannot start another.",
            )));
        }

        let job_id = db::insert_rip_jobs(
            &self.db,
            &db::schemas::RipJobsItem {
                id: None,
                start_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| tonic::Status::internal("System clock is incorrect"))?
                    .as_secs() as i64,
                disc_title: drive_state.disc_name,
                suspected_contents: suspected_contents.map(|item| item.encode_to_vec()),
                rip_finished: false,
                imported: false,
            },
        )
        .await?;

        controller
            .rip_media(RipMediaRequest {
                job_id,
                drive_id,
                autoeject,
            })
            .await?
            .into_inner();

        return Ok(job_id);
    }

    pub async fn import_job(&self, job_id: i64) -> Result<(), ApplicationError> {
        // 1. Mark rip job as finished
        db::mark_rip_job_finished(&self.db, job_id, true).await?;
        let mut dir_walker = self.walk_rip_dir(job_id).await?;

        // 2. Import video files
        while let Some(file_path) = dir_walker.try_next().await? {
            if let Err(err) = self
                .blob_storage
                .add_video_file(&self.partess_cache, &file_path, Some(job_id))
                .await
            {
                println!(
                    "An error occurred importing job {}, file {}:\n{}",
                    job_id,
                    file_path.to_string_lossy(),
                    err,
                );
            }
        }

        // 3. Mark rip job as imported
        db::mark_rip_job_imported(&self.db, job_id, true).await?;

        return Ok(());
    }

    async fn walk_rip_dir(
        &self,
        job_id: i64,
    ) -> std::io::Result<impl Stream<Item = std::io::Result<PathBuf>> + Send + Unpin> {
        let rip_dir = Arc::new(self.rip_dir.join(job_id.to_string()));
        let walker = tokio::fs::read_dir(&*rip_dir).await?;
        let walker = ReadDirStream::new(walker);
        return Ok(Box::pin(walker.filter_map(move |item| {
            let rip_dir = Arc::clone(&rip_dir);
            async move {
                match item {
                    Ok(item) => {
                        let file_path = rip_dir.join(item.file_name());
                        if file_path.extension() == Some(OsStr::new("mkv")) {
                            Some(Ok(file_path))
                        } else {
                            None
                        }
                    }
                    Err(err) => Some(Err(err)),
                }
            }
        })));
    }
}

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("The requested drive controller was not found.")]
    ControllerMissing,
    #[error("The requested resource is currently busy. Please try again.")]
    TemporaryFailure,
    #[error("Precondition failed:\n{0}")]
    FailedPrecondition(String),
    #[error("An unknown error occurred upstream: {0}")]
    TonicError(#[from] tonic::Status),
    #[error("There was an error querying the database:\n{0}")]
    DbError(#[from] sqlx::Error),
    #[error("An I/O eeor occurred:\n{0}")]
    Io(#[from] std::io::Error),
}
