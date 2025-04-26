use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use sqlx::SqlitePool;
use tokio::{sync::Mutex, task::JoinHandle};
use types::JobInfo;

use crate::{
    blob_storage::BlobStorageController,
    config::{OST_API_KEY, OST_PASSWORD, OST_USERNAME, TMDB_API_KEY},
    db::{
        add_suspicion, delete_rip_job, get_matches_from_rip, get_movie_by_tmdb_id, get_movies,
        get_ost_download_items_by_show_id, get_ost_subtitles_from_rip, get_rip_image_blobs,
        get_rip_job, get_rip_jobs_with_untagged_videos, get_rip_video_blobs, get_tv_episode_by_id,
        get_tv_episode_by_tmdb_id, get_tv_episodes, get_tv_seasons, get_tv_shows,
        get_untagged_videos_from_job, get_videos_from_rip, rename_rip_job,
        schemas::{MoviesItem, RipJobsItem, TvEpisodesItem, TvSeasonsItem, TvShowsItem, VideoType},
        tag_video_file,
    },
    drive_controller::DriveController,
    exports_manager::ExportsManager,
    tagging::{
        analyzers::analyze_subtitles,
        importers::{opensubtitles::OpenSubtitles, tmdb::TmdbImporter},
        types::SuspectedContents,
    },
};

pub mod types;

pub struct Application {
    db: Arc<SqlitePool>,
    blob_controller: Arc<BlobStorageController>,
    exports_manager: Mutex<ExportsManager>,
    tmdb_importer: TmdbImporter,
    opensubtitles: Arc<OpenSubtitles>,
    suspicion_analyzers: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
    drives: HashMap<String, DriveController>,
}
impl Application {
    pub async fn new(db: Arc<SqlitePool>, path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let path: PathBuf = path.into();
        let blob_controller =
            Arc::new(BlobStorageController::new(Arc::clone(&db), path.clone()).await?);
        let tmdb_importer = TmdbImporter::new(
            Arc::clone(&db),
            String::clone(&TMDB_API_KEY),
            Arc::clone(&blob_controller),
        )?;
        let opensubtitles = Arc::new(OpenSubtitles::new(
            String::clone(&OST_API_KEY),
            String::clone(&OST_USERNAME),
            String::clone(&OST_PASSWORD),
        ));
        let exports_manager = Mutex::new(
            ExportsManager::new(
                Arc::clone(&db),
                Arc::clone(&blob_controller),
                path.join("exports"),
            )
            .await?,
        );
        return Ok(Self {
            db,
            blob_controller,
            exports_manager,
            tmdb_importer,
            opensubtitles,
            suspicion_analyzers: Arc::new(Mutex::new(HashMap::new())),
            drives: HashMap::new(),
        });
    }

    /// Gets the path of a blob
    pub fn get_blob_path(&self, blob_id: &str) -> PathBuf {
        return self.blob_controller.get_file_path(blob_id);
    }

    /// Register a drive to be controlled by Mediacorral
    pub async fn register_drive(&mut self, drive_path: &Path) -> anyhow::Result<()> {
        let path = canonicalize_drive_path(drive_path)?;
        self.drives.insert(
            path.clone(),
            DriveController::new(path, Arc::clone(&self.blob_controller)).await?,
        );

        return Ok(());
    }

    /// Lists the drives in Mediacorral's control
    pub fn list_drives(&self) -> impl Iterator<Item = &DriveController> {
        return self.drives.values();
    }

    /// Gets the drive controller for the requested drive
    pub fn get_drive(&self, drive_path: &Path) -> anyhow::Result<&DriveController> {
        let path = canonicalize_drive_path(drive_path)?;

        return Ok(self.drives.get(&path).context("Drive not found")?);
    }

    /// Gets data for a specific rip job with all the info needed for tagging
    pub async fn get_job_info(&self, rip_job: i64) -> anyhow::Result<JobInfo> {
        let job_info = get_rip_job(&self.db, rip_job).await?;
        let video_files = get_videos_from_rip(&self.db, rip_job).await?;
        let matches = get_matches_from_rip(&self.db, rip_job).await?;
        let subtitle_maps = get_rip_video_blobs(&self.db, rip_job).await?;
        let ost_subtitle_files = get_ost_subtitles_from_rip(&self.db, rip_job).await?;

        return Ok(JobInfo {
            id: job_info.id,
            start_time: job_info.start_time,
            disc_title: job_info.disc_title,
            suspected_contents: job_info
                .suspected_contents
                .and_then(|contents| serde_json::from_str(&contents).ok()),
            video_files,
            matches,
            subtitle_maps,
            ost_subtitle_files,
        });
    }

    pub async fn rename_job(&self, rip_job: i64, new_name: &str) -> anyhow::Result<()> {
        rename_rip_job(&self.db, rip_job, new_name).await?;
        return Ok(());
    }

    /// Gets the metadata importer so we can import from TMDB
    pub fn importer(&self) -> &TmdbImporter {
        return &self.tmdb_importer;
    }

    /// Lists the movies we have in our metadata database
    pub async fn list_movies(&self) -> anyhow::Result<Vec<MoviesItem>> {
        return Ok(get_movies(&self.db).await?);
    }

    /// Gets a single movie by its TMDB ID
    pub async fn get_movie_by_tmdb_id(&self, tmdb_id: i32) -> anyhow::Result<MoviesItem> {
        return Ok(get_movie_by_tmdb_id(&self.db, tmdb_id).await?);
    }

    /// Lists the TV series we have in our metadata database
    pub async fn list_tv_series(&self) -> anyhow::Result<Vec<TvShowsItem>> {
        return Ok(get_tv_shows(&self.db).await?);
    }

    /// Lists the TV seasons from the given show from our metadata database
    pub async fn list_tv_seasons(&self, series_id: i64) -> anyhow::Result<Vec<TvSeasonsItem>> {
        return Ok(get_tv_seasons(&self.db, series_id).await?);
    }

    /// Lists TV episodes from the given season from our metadata database
    pub async fn list_tv_episodes(&self, season_id: i64) -> anyhow::Result<Vec<TvEpisodesItem>> {
        return Ok(get_tv_episodes(&self.db, season_id).await?);
    }

    /// Gets a single TV episode by its ID
    pub async fn get_tv_episode(&self, episode_id: i64) -> anyhow::Result<TvEpisodesItem> {
        return Ok(get_tv_episode_by_id(&self.db, episode_id).await?);
    }

    /// Gets a single TV episode by its TMDB ID
    pub async fn get_tv_episode_by_tmdb_id(&self, tmdb_id: i32) -> anyhow::Result<TvEpisodesItem> {
        return Ok(get_tv_episode_by_tmdb_id(&self.db, tmdb_id).await?);
    }

    /// Tags a video file, matching it with the metadata we have in our database
    pub async fn tag_video(
        &self,
        video_id: i64,
        video_type: VideoType,
        match_id: i64,
    ) -> anyhow::Result<()> {
        tag_video_file(&self.db, video_id, video_type, match_id).await?;
        self.splice_export_content(video_type, video_id).await?;
        return Ok(());
    }

    /// Gets a list of all untagged rip jobs
    pub async fn get_untagged_jobs(
        &self,
        skip: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<RipJobsItem>> {
        return Ok(get_rip_jobs_with_untagged_videos(&self.db, skip, limit).await?);
    }

    pub async fn purge_ost_subtitles_by_show(&self, show_id: i64) -> anyhow::Result<()> {
        let blob_ids = get_ost_download_items_by_show_id(&self.db, show_id)
            .await
            .context("Couldn't find ost items")?;
        for blob_id in blob_ids {
            self.blob_controller
                .delete_blob(&blob_id)
                .await
                .context("Couldn't delete blob")?;
        }

        return Ok(());
    }

    pub async fn is_analyzing(&self, rip_job: i64) -> bool {
        return self.suspicion_analyzers.lock().await.contains_key(&rip_job);
    }

    /// Adds suspected contents for the rip job, triggering the process of analysis
    pub async fn suspect_content(
        &self,
        rip_job: i64,
        suspected_contents: Option<SuspectedContents>,
    ) -> anyhow::Result<()> {
        let mut unlocked_suspicion_analyzers = self.suspicion_analyzers.lock().await;
        if let Some(analyzer) = unlocked_suspicion_analyzers.remove(&rip_job) {
            analyzer.abort();
        }
        add_suspicion(&self.db, rip_job, suspected_contents.as_ref()).await?;
        match suspected_contents {
            Some(SuspectedContents::Movie { tmdb_id }) => {
                // TODO: Add analyzer
            }
            Some(SuspectedContents::TvEpisodes { episode_tmdb_ids }) => {
                let opensubtitles = Arc::clone(&self.opensubtitles);
                let db = Arc::clone(&self.db);
                let blob_controller = Arc::clone(&self.blob_controller);
                let suspicion_analyzers = Arc::clone(&self.suspicion_analyzers);
                unlocked_suspicion_analyzers.insert(
                    rip_job,
                    tokio::task::spawn(async move {
                        analyze_subtitles(
                            db,
                            opensubtitles,
                            blob_controller,
                            rip_job,
                            episode_tmdb_ids,
                        )
                        .await;
                        suspicion_analyzers.lock().await.remove(&rip_job);
                    }),
                );
            }
            None => {}
        }

        return Ok(());
    }

    /// Deletes any untagged videos from a rip job
    pub async fn prune_rip_job(&self, rip_job: i64) -> anyhow::Result<()> {
        let untagged_blobs = get_untagged_videos_from_job(&self.db, rip_job).await?;
        for blobs in untagged_blobs {
            self.blob_controller.delete_blob(&blobs.video_blob).await?;
            if let Some(subtitle_blob) = blobs.subtitle_blob {
                self.blob_controller.delete_blob(&subtitle_blob).await?;
            }
        }

        return Ok(());
    }

    /// Deletes a rip job and everything that came from it.
    ///
    /// WARNING! This is a destructive action, and could result in loss of media.
    pub async fn delete_rip_job(&self, rip_job: i64) -> anyhow::Result<()> {
        let video_blobs = get_rip_video_blobs(&self.db, rip_job).await?;
        let image_blobs = get_rip_image_blobs(&self.db, rip_job).await?;
        for video_blob in video_blobs {
            self.blob_controller
                .delete_blob(&video_blob.video_blob)
                .await?;
            if let Some(subtitle_blob) = video_blob.subtitle_blob {
                self.blob_controller.delete_blob(&subtitle_blob).await?;
            }
        }
        for image_blob in image_blobs {
            self.blob_controller
                .delete_blob(&image_blob.image_blob)
                .await?;
        }
        delete_rip_job(&self.db, rip_job).await?;

        return Ok(());
    }

    /// Rebuilds the requested exports directory, deleting and replacing all files
    pub async fn rebuild_exports(&self, exports_dir: &str) -> anyhow::Result<()> {
        self.exports_manager
            .lock()
            .await
            .rebuild_dir(exports_dir)
            .await?;
        return Ok(());
    }

    /// Splice content into all configured exports directories
    pub async fn splice_export_content(
        &self,
        video_type: VideoType,
        video_id: i64,
    ) -> anyhow::Result<()> {
        self.exports_manager
            .lock()
            .await
            .splice_content(video_type, video_id)
            .await?;
        return Ok(());
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
