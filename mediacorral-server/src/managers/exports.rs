use futures::StreamExt;
use sqlx::prelude::FromRow;
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;

use crate::{
    blob_storage::BlobStorageController,
    db::schemas::VideoType,
    export_settings::{ExportSettings, LinkType, MediaType},
};

#[derive(Error, Debug)]
pub enum ExportsDirError {
    #[error("The requested directory was not found.")]
    DirNotFound,
    #[error("I/O Error:\n{0}")]
    Io(#[from] std::io::Error),
    #[error("Database error")]
    Db(#[from] sqlx::Error),
}

/// Data type used to represent a result from the database
/// when building a TV exports directory.
#[derive(FromRow)]
pub struct TvEntry {
    tv_title: String,
    tv_release_year: String,
    tv_tmdb: i32,
    season_number: u16,
    episode_title: String,
    episode_number: u16,
    episode_tmdb: i32,
    episode_blob: String,
}

pub struct ExportsManager {
    exports_base: PathBuf,
    configured_exports: HashMap<String, ExportSettings>,
    db_connection: Arc<sqlx::SqlitePool>,
}
impl ExportsManager {
    pub async fn new(
        db_connection: Arc<sqlx::SqlitePool>,
        exports_path: impl Into<PathBuf>,
        configured_exports: HashMap<String, ExportSettings>,
    ) -> std::io::Result<Self> {
        let path: PathBuf = exports_path.into();
        if !path.exists() {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                "Given exports directory not found",
            ));
        }
        if !path.is_dir() {
            return Err(std::io::Error::new(
                ErrorKind::NotADirectory,
                "Given exports directory is not a directory",
            ));
        }

        return Ok(Self {
            exports_base: path,
            db_connection,
            configured_exports,
        });
    }

    pub async fn rebuild_dir(
        &mut self,
        export_name: &str,
        blob_controller: &BlobStorageController,
    ) -> Result<(), ExportsDirError> {
        let settings = self
            .configured_exports
            .get(export_name)
            .ok_or(ExportsDirError::DirNotFound)?;
        let exports_dir = self.exports_base.join(export_name);
        match tokio::fs::read_dir(&exports_dir).await {
            Ok(mut dir_reader) => {
                while let Some(dir) = dir_reader.next_entry().await? {
                    let file_type = dir.file_type().await?;
                    if file_type.is_dir() {
                        tokio::fs::remove_dir_all(dir.path()).await?;
                    } else {
                        tokio::fs::remove_file(dir.path()).await?;
                    }
                }
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                tokio::fs::create_dir(&exports_dir).await?
            }
            Err(err) => {
                Err(err)?;
            }
        }
        match settings.media_type {
            MediaType::Movies => todo!(),
            MediaType::TvShows => {
                let mut results = sqlx::query_as(
                    "
                        SELECT
                            tv_shows.title as tv_title,
                            tv_shows.original_release_year as tv_release_year,
                            tv_shows.tmdb_id as tv_tmdb,
                            tv_seasons.season_number as season_number,
                            tv_episodes.title as episode_title,
                            tv_episodes.episode_number as episode_number,
                            tv_episodes.tmdb_id as episode_tmdb,
                            video_files.blob_id as episode_blob
                        FROM video_files
                        JOIN tv_episodes ON
                            video_files.match_id = tv_episodes.id
                        JOIN tv_seasons ON
                            tv_episodes.tv_season_id = tv_seasons.id
                        JOIN tv_shows ON
                            tv_episodes.tv_show_id = tv_shows.id
                        WHERE video_type = 3
                        ORDER BY tv_episodes.id
                    ",
                )
                .fetch(self.db_connection.as_ref());
                while let Some(result) = results.next().await {
                    let result: TvEntry = result?;
                    add_tv_episode(blob_controller, &exports_dir, &result, settings).await?;
                }
            }
        }

        return Ok(());
    }
    pub async fn splice_content(
        &mut self,
        video_type: VideoType,
        video_id: i64,
        blob_controller: &BlobStorageController,
    ) -> anyhow::Result<()> {
        match video_type {
            VideoType::Untagged => {}       // Not tagged. Nothing to do
            VideoType::Movie => {}          // TODO: Not supported yet.
            VideoType::SpecialFeature => {} // TODO: Not supported yet.
            VideoType::TvEpisode => {
                let result: TvEntry = sqlx::query_as(
                    "
                        SELECT
                            tv_shows.title as tv_title,
                            tv_shows.original_release_year as tv_release_year,
                            tv_shows.tmdb_id as tv_tmdb,
                            tv_seasons.season_number as season_number,
                            tv_episodes.title as episode_title,
                            tv_episodes.episode_number as episode_number,
                            tv_episodes.tmdb_id as episode_tmdb,
                            video_files.blob_id as episode_blob
                        FROM video_files
                        JOIN tv_episodes ON
                            video_files.match_id = tv_episodes.id
                        JOIN tv_seasons ON
                            tv_episodes.tv_season_id = tv_seasons.id
                        JOIN tv_shows ON
                            tv_episodes.tv_show_id = tv_shows.id
                        WHERE
                            video_files.video_type = 3
                            AND video_files.id = ?
                        LIMIT 1
                    ",
                )
                .bind(video_id)
                .fetch_one(self.db_connection.as_ref())
                .await?;
                for (export_name, settings) in self.configured_exports.iter() {
                    let exports_dir = self.exports_base.join(export_name);
                    add_tv_episode(blob_controller, &exports_dir, &result, settings).await?;
                }
            }
        }
        return Ok(());
    }
}

fn make_pathsafe(input: &str) -> String {
    return lazy_regex::regex_replace!(r#"[/\\?%*:|"<>\x7F\x00-\x1F]"#, input, "-").into_owned();
}

async fn add_tv_episode(
    blob_controller: &BlobStorageController,
    exports_dir: &Path,
    item: &TvEntry,
    settings: &ExportSettings,
) -> Result<(), ExportsDirError> {
    let show_folder = exports_dir.join(&format!(
        "{} ({}) {{tmdb-{}}}",
        item.tv_title, item.tv_release_year, item.tv_tmdb
    ));
    let season_folder = show_folder.join(&format!("Season {:02}", item.season_number));
    match tokio::fs::create_dir_all(&season_folder).await {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
        Err(err) => Err(err)?,
    }
    let episode_filename = format!(
        "{} ({}) - S{:02}E{:02} - {} - {{tmdb-{}}}.mkv",
        make_pathsafe(&item.tv_title),
        make_pathsafe(&item.tv_release_year),
        item.season_number,
        item.episode_number,
        make_pathsafe(&item.episode_title),
        item.episode_tmdb
    );
    let episode_path = season_folder.join(&episode_filename);
    let result = match settings.link_type {
        LinkType::Symbolic => {
            blob_controller
                .symbolic_link(&item.episode_blob, episode_path)
                .await
        }
        LinkType::Hard => {
            blob_controller
                .hard_link(&item.episode_blob, episode_path)
                .await
        }
    };
    match result {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {
            println!(
                "Blob {} not found. Please re-rip {}.",
                item.episode_blob, episode_filename
            );
        }
        Err(err) => Err(err)?,
    }
    return Ok(());
}
