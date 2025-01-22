pub use rip_dir::RipDirHandle;
use std::{io::ErrorKind, path::PathBuf, sync::Arc};

use crate::tagging::types::SuspectedContents;

mod rip_dir;
mod util_funcs;

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
                        panic!("Someone else is managing the blobs directory. Please make sure there are no other instances running.");
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

    pub async fn create_rip_dir(
        &self,
        disc_title: Option<String>,
        suspected_contents: Option<SuspectedContents>,
    ) -> anyhow::Result<RipDirHandle> {
        return Ok(RipDirHandle::new(
            Arc::clone(&self.db_connection),
            self.blob_dir.clone(),
            &self.rip_dir,
            disc_title,
            suspected_contents,
        )
        .await?);
    }

    pub async fn add_ost_subtitles(
        &self,
        video_type: VideoType,
        match_id: i64,
        ost_url: String,
        filename: String,
        data: String,
    ) -> anyhow::Result<i64> {
        let uuid = Uuid::new_v4().to_string();
        let mut file = File::open(self.blob_dir.join(&uuid)).await?;
        file.write_all(data.as_bytes()).await?;
        let id = insert_ost_download_item(
            &self.db_connection,
            &OstDownloadsItem {
                id: None,
                video_type,
                match_id,
                filename,
                ost_url,
                blob_id: uuid,
            },
        )
        .await?;

        return Ok(id);
    }
}
