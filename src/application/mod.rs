use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use sqlx::SqlitePool;
use types::JobInfo;

use crate::{
    blob_storage::BlobStorageController,
    config::TMDB_API_KEY,
    db::{
        delete_rip_job, get_matches_from_rip, get_movies, get_rip_image_blobs, get_rip_job,
        get_rip_jobs_with_untagged_videos, get_rip_video_blobs, get_tv_episodes, get_tv_seasons,
        get_tv_shows, get_untagged_videos_from_job, get_videos_from_rip,
        schemas::{MoviesItem, RipJobsItem, TvEpisodesItem, TvSeasonsItem, TvShowsItem, VideoType},
        tag_video_file,
    },
    drive_controller::DriveController,
    tagging::importers::tmdb::TmdbImporter,
};

pub mod types;

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

    pub async fn get_job_info(&self, rip_job: i64) -> anyhow::Result<JobInfo> {
        let job_info = get_rip_job(&self.db, rip_job).await?;
        let video_files = get_videos_from_rip(&self.db, rip_job).await?;
        let matches = get_matches_from_rip(&self.db, rip_job).await?;

        return Ok(JobInfo {
            id: job_info.id,
            start_time: job_info.start_time,
            disc_title: job_info.disc_title,
            suspected_contents: job_info
                .suspected_contents
                .and_then(|contents| serde_json::from_str(&contents).ok()),
            video_files,
            matches,
        });
    }

    pub fn importer(&self) -> &TmdbImporter {
        return &self.tmdb_importer;
    }

    pub async fn list_movies(&self) -> anyhow::Result<Vec<MoviesItem>> {
        return Ok(get_movies(&self.db).await?);
    }

    pub async fn list_tv_series(&self) -> anyhow::Result<Vec<TvShowsItem>> {
        return Ok(get_tv_shows(&self.db).await?);
    }

    pub async fn list_tv_seasons(&self, series_id: i64) -> anyhow::Result<Vec<TvSeasonsItem>> {
        return Ok(get_tv_seasons(&self.db, series_id).await?);
    }

    pub async fn list_tv_episodes(&self, season_id: i64) -> anyhow::Result<Vec<TvEpisodesItem>> {
        return Ok(get_tv_episodes(&self.db, season_id).await?);
    }

    pub async fn tag_video(
        &self,
        video_id: i64,
        video_type: VideoType,
        match_id: i64,
    ) -> anyhow::Result<()> {
        tag_video_file(&self.db, video_id, video_type, match_id).await?;
        return Ok(());
    }

    pub async fn get_untagged_jobs(
        &self,
        skip: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<RipJobsItem>> {
        return Ok(get_rip_jobs_with_untagged_videos(&self.db, skip, limit).await?);
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
}

fn canonicalize_drive_path(drive_path: &Path) -> anyhow::Result<String> {
    return Ok(String::from(
        PathBuf::from(drive_path)
            .canonicalize()?
            .to_str()
            .context("Invalid path")?,
    ));
}
