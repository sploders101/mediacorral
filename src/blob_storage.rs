use std::{io::ErrorKind, path::{Path, PathBuf}, sync::Arc};

use uuid::Uuid;

#[must_use]
pub struct RipDirHandle {
    directory: PathBuf,
}
impl AsRef<PathBuf> for RipDirHandle {
    fn as_ref(&self) -> &PathBuf {
        return &self.directory;
    }
}
impl AsRef<Path> for RipDirHandle {
    fn as_ref(&self) -> &Path {
        return &self.directory;
    }
}
impl RipDirHandle {
    pub async fn import(self) {
        todo!();
    }
    pub async fn discard(self) {
        let _ = tokio::fs::remove_dir_all(self.directory).await;
    }
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

    pub async fn create_rip_dir(&self) -> std::io::Result<RipDirHandle> {
        let uuid = Uuid::new_v4();
        let rip_dir = self.rip_dir.join(uuid.to_string());
        tokio::fs::create_dir(&rip_dir).await?;
        return Ok(RipDirHandle {
            directory: rip_dir,
        });
    }
}
