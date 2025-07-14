use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    db::{
        self, delete_blob, insert_image_file, insert_ost_download_item,
        schemas::{ImageFilesItem, OstDownloadsItem, SubtitleFilesItem, VideoFilesItem, VideoType},
    },
    workers::mkv_analysis::{
        ExtractDetailsError, extract_details, subtitles::vobsub::PartessCache,
    },
};

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("An error occurred while extracting details from the video file:\n{0}")]
    ExtractDetails(#[from] ExtractDetailsError),
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
    #[error("A database error occurred:\n{0}")]
    Db(#[from] sqlx::Error),
}
type BlobResult<T> = Result<T, BlobError>;

pub struct BlobStorageController {
    blob_dir: PathBuf,
    db_connection: Arc<sqlx::SqlitePool>,
}
impl Clone for BlobStorageController {
    fn clone(&self) -> Self {
        return Self {
            blob_dir: self.blob_dir.clone(),
            db_connection: Arc::clone(&self.db_connection),
        };
    }
}
impl BlobStorageController {
    pub async fn new(
        db_connection: Arc<sqlx::SqlitePool>,
        blob_dir: impl Into<PathBuf>,
    ) -> std::io::Result<Self> {
        let blob_dir: PathBuf = blob_dir.into();
        if !blob_dir.exists() {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                "Given blob directory not found",
            ));
        }
        if !blob_dir.is_dir() {
            return Err(std::io::Error::new(
                ErrorKind::NotADirectory,
                "Given blob directory is not a directory",
            ));
        }

        return Ok(Self {
            blob_dir,
            db_connection,
        });
    }

    pub async fn add_video_file(
        &self,
        partess_cache: &PartessCache,
        file_path: &PathBuf,
        rip_job: Option<i64>,
    ) -> BlobResult<()> {
        let uuid = Uuid::new_v4().to_string();
        let new_path = self.blob_dir.join(&uuid);
        if let Err(err) = tokio::fs::rename(&file_path, &new_path).await {
            if err.kind() == std::io::ErrorKind::CrossesDevices {
                tokio::fs::copy(&file_path, &new_path).await?;
                tokio::fs::remove_file(&file_path).await?;
            }
        }
        let id = db::insert_video_file(
            &self.db_connection,
            &VideoFilesItem {
                id: None,
                video_type: VideoType::Untagged,
                match_id: None,
                blob_id: uuid,
                resolution_width: None,
                resolution_height: None,
                length: None,
                original_video_hash: None,
                rip_job,
                extended_metadata: None,
            },
        )
        .await?;

        let result = {
            let new_path = new_path.clone();
            let partess_cache = partess_cache.clone();
            tokio::task::spawn_blocking(move || {
                let file = std::fs::File::open(new_path)?;
                extract_details(file, None, &partess_cache)
            })
            .await
            .unwrap()
        }?;

        db::add_video_metadata(
            &self.db_connection,
            id,
            result.resolution_width,
            result.resolution_height,
            result.duration,
            &result.video_hash,
            result.extended_metadata,
        )
        .await?;

        if let Some(subtitles) = result.subtitles {
            let subs_uuid = Uuid::new_v4().to_string();
            let mut file = tokio::fs::File::create(self.blob_dir.join(&subs_uuid)).await?;
            file.write_all(subtitles.as_bytes()).await?;
            db::insert_subtitle_file(
                &self.db_connection,
                &SubtitleFilesItem {
                    id: None,
                    blob_id: subs_uuid,
                    video_file: id,
                },
            )
            .await?;
        }

        return Ok(());
    }

    pub async fn add_subtitles_file(
        &self,
        video_file_id: i64,
        subtitles: String,
    ) -> Result<(), BlobError> {
        let subs_uuid = Uuid::new_v4().to_string();
        let mut file = tokio::fs::File::create(self.blob_dir.join(&subs_uuid)).await?;
        file.write_all(subtitles.as_bytes()).await?;
        db::insert_subtitle_file(
            &self.db_connection,
            &SubtitleFilesItem {
                id: None,
                blob_id: subs_uuid,
                video_file: video_file_id,
            },
        )
        .await?;
        return Ok(());
    }

    pub async fn delete_rip_job(&self, rip_job: i64) -> BlobResult<()> {
        let videos = db::get_videos_from_rip(&self.db_connection, rip_job).await?;
        for video in videos {
            self.delete_blob(&video.blob_id).await?;
        }

        let subtitles = db::get_disc_subs_from_rip(&self.db_connection, rip_job).await?;
        for subtitle in subtitles {
            self.delete_blob(&subtitle.subtitle_blob).await?;
        }

        db::delete_matches_from_rip(&self.db_connection, rip_job).await?;
        db::delete_rip_job(&self.db_connection, rip_job).await?;

        return Ok(());
    }

    pub async fn add_ost_subtitles(
        &self,
        video_type: VideoType,
        match_id: i64,
        filename: String,
        data: String,
    ) -> BlobResult<i64> {
        let uuid = Uuid::new_v4().to_string();
        let mut file = File::create(self.blob_dir.join(&uuid)).await?;
        file.write_all(data.as_bytes()).await?;
        let id = insert_ost_download_item(
            &self.db_connection,
            &OstDownloadsItem {
                id: None,
                video_type,
                match_id,
                filename,
                blob_id: uuid,
            },
        )
        .await?;

        return Ok(id);
    }

    pub async fn add_image(
        &self,
        name: Option<String>,
        mime_type: String,
    ) -> BlobResult<(i64, File)> {
        let uuid = Uuid::new_v4().to_string();
        let file = File::create(self.blob_dir.join(&uuid)).await?;
        let id = insert_image_file(
            &self.db_connection,
            &ImageFilesItem {
                id: None,
                blob_id: uuid,
                mime_type,
                name,
                rip_job: None,
            },
        )
        .await?;

        return Ok((id, file));
    }

    pub async fn delete_blob(&self, blob_id: &str) -> BlobResult<()> {
        delete_blob(&self.db_connection, blob_id).await?;
        let file_path = self.blob_dir.join(blob_id);
        tokio::fs::remove_file(file_path).await?;

        return Ok(());
    }

    pub fn get_file_path(&self, id: &str) -> PathBuf {
        return self.blob_dir.join(id);
    }

    /// Creates a hard link to a blob at `destination`.
    ///
    /// This is useful, for example, if your media center is running in a container
    /// that does not have access to the blobs directory.
    pub async fn hard_link(
        &self,
        id: &String,
        destination: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let source_path = self.get_file_path(id);
        return match tokio::fs::hard_link(&source_path, &destination).await {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                tokio::fs::remove_file(&destination).await?;
                tokio::fs::hard_link(&source_path, &destination).await
            }
            Err(err) => Err(err),
        };
    }

    /// Creates a symlink to a blob at `destination`
    pub async fn symbolic_link(
        &self,
        id: &String,
        destination: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        if !destination.as_ref().is_absolute() {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Destination must be absolute",
            ));
        }
        let source = self.get_file_path(id);
        let dest_dir = destination.as_ref().parent().unwrap();

        // I already verify that both paths are absolute. This shouldn't fail.
        let relative = pathdiff::diff_paths(&source, &dest_dir)
            .expect("Error finding relative path in symbolic_link");

        return match tokio::fs::symlink(&relative, &destination).await {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                tokio::fs::remove_file(&source).await?;
                tokio::fs::symlink(&relative, &destination).await
            }
            Err(err) => Err(err),
        };
    }
}
