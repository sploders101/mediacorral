use anyhow::Context;
use futures::lock::Mutex;
use mediacorral_proto::mediacorral::{
    coordinator::v1::SuspectedContents,
    drive_controller::v1::{
        DriveStatusTag, GetDriveStateRequest, RipMediaRequest,
        drive_controller_service_client::DriveControllerServiceClient,
    },
};
use prost::Message;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    path::Path,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tonic::transport::{Channel, Endpoint};

use crate::{
    CoordinatorConfig,
    blob_storage::BlobStorageController,
    db,
    managers::{
        exports::{ExportsDirError, ExportsManager},
        tmdb::TmdbImporter,
    },
};

pub struct Application {
    pub db: Arc<SqlitePool>,
    autorip_enabled: AtomicBool,
    pub blob_storage: BlobStorageController,
    pub tmdb_importer: TmdbImporter,
    pub exports_manager: Mutex<ExportsManager>,
    pub drive_controllers: HashMap<String, DriveControllerServiceClient<Channel>>,
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
            autorip_enabled: AtomicBool::new(config.enable_autorip),
            blob_storage,
            tmdb_importer,
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
        .await
        .unwrap();

        controller
            .rip_media(RipMediaRequest { job_id, drive_id })
            .await?
            .into_inner();

        return Ok(job_id);
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
}
