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
        schemas::{ImageFilesItem, OstDownloadsItem, VideoFilesItem, VideoType},
    },
    workers::subtitles::{extract_details, vobsub::PartessCache},
};

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
    #[error("A database error occurred:\n{0}")]
    Db(#[from] sqlx::Error),
}

pub struct BlobStorageController {
    blob_dir: PathBuf,
    rip_dir: PathBuf,
    db_connection: Arc<sqlx::SqlitePool>,
}
impl BlobStorageController {
    pub async fn new(
        db_connection: Arc<sqlx::SqlitePool>,
        path: impl Into<PathBuf>,
    ) -> std::io::Result<Self> {
        let path: PathBuf = path.into();
        if !path.exists() {
            return Err(std::io::Error::new(
                ErrorKind::NotFound,
                "Given blob directory not found",
            ));
        }
        if !path.is_dir() {
            return Err(std::io::Error::new(
                ErrorKind::NotADirectory,
                "Given blob directory is not a directory",
            ));
        }

        // Make sure blobs dir exists
        let blob_dir = path.join("blobs");
        match tokio::fs::create_dir(&blob_dir).await {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => Err(err)?,
        }

        // Make sure rips dir exists and is clear
        let rip_dir = path.join("rips");
        match tokio::fs::read_dir(&rip_dir).await {
            Ok(mut dirlist) => {
                while let Ok(Some(item)) = dirlist.next_entry().await {
                    tokio::fs::remove_dir_all(item.path()).await?;
                }
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                match tokio::fs::create_dir(&rip_dir).await {
                    Ok(()) => {}
                    Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                        panic!(
                            "Someone else is managing the blobs directory. Please make sure there are no other instances running."
                        );
                    }
                    Err(err) => Err(err)?,
                }
            }
            Err(err) => Err(err)?,
        }

        return Ok(Self {
            blob_dir,
            rip_dir,
            db_connection,
        });
    }

    pub async fn add_video_file(
        &self,
        partess_cache: &PartessCache,
        file_path: &PathBuf,
        rip_job: Option<i64>,
    ) -> anyhow::Result<()> {
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
        )
        .await?;

        return Ok(());
    }

    pub async fn add_ost_subtitles(
        &self,
        video_type: VideoType,
        match_id: i64,
        filename: String,
        data: String,
    ) -> anyhow::Result<i64> {
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
    ) -> anyhow::Result<(i64, File)> {
        let uuid = Uuid::new_v4().to_string();
        let file = File::open(self.blob_dir.join(&uuid)).await?;
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

    pub async fn delete_blob(&self, blob_id: &str) -> anyhow::Result<()> {
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
