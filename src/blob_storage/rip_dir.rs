use super::util_funcs;
use crate::{media_helpers::extract_subtitles, tagging::types::SuspectedContents};
use async_stream::stream;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

#[must_use]
pub struct RipDirHandle {
    job_id: i64,
    blob_dir: PathBuf,
    directory: PathBuf,
    db: Arc<SqlitePool>,
}
impl RipDirHandle {
    pub async fn new(
        db: Arc<SqlitePool>,
        blob_dir: PathBuf,
        rip_dir: &Path,
        disc_title: Option<String>,
        suspected_contents: Option<SuspectedContents>,
    ) -> anyhow::Result<Self> {
        let uuid = Uuid::new_v4();
        let directory = rip_dir.join(uuid.to_string());
        tokio::fs::create_dir(&directory).await?;

        let job_id = util_funcs::create_rip_job(&db, disc_title, suspected_contents).await?;
        return Ok(Self {
            job_id,
            blob_dir,
            directory,
            db,
        });
    }
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
    pub async fn extract_subtitles(&self) -> anyhow::Result<()> {
        let stream = std::pin::pin!(stream! {
            let mut dir = tokio::fs::read_dir(self).await?;
            while let Some(file) = dir.next_entry().await? {
                let path = file.path();
                if let Some(extension) = path.extension() {
                    if &OsString::from("mkv") == extension {
                        yield Ok(path);
                    }
                }
            }
        });
        extract_subtitles(stream).await?;

        return Ok(());
    }

    pub fn job_id(&self) -> i64 {
        return self.job_id;
    }

    /// This imports the mkv files into the system and deletes the folder when finished.
    /// This process may involve extra work such as extracting subtitles, and could take
    /// a while, so it's probably best to spawn a task for this on the runtime.
    pub async fn import(self) -> anyhow::Result<()> {
        self.extract_subtitles().await?;
        let mut video_blobs = HashMap::<String, i64>::new();
        let mut upload_subtitles = Vec::<(String, PathBuf)>::new();

        let mut readdir = tokio::fs::read_dir(&self.directory).await?;
        while let Some(file) = readdir.next_entry().await? {
            if !file.file_type().await?.is_file() {
                continue;
            }

            let path = file.path();
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("mkv") => {
                    if let Some(title_name) = path.file_stem().and_then(|stem| stem.to_str()) {
                        let video_id = util_funcs::insert_video(
                            &self.db,
                            &path,
                            &self.blob_dir,
                            Some(self.job_id),
                        )
                        .await?;
                        video_blobs.insert(String::from(title_name), video_id);
                    }
                }
                Some("srt") => {
                    // Found a textual subtitle track. Postpone import until we have
                    // blob IDs for the mkv.
                    if let Some(file_name) = path.file_stem() {
                        upload_subtitles.push((file_name.to_string_lossy().to_string(), path));
                    }
                }
                _ => {}
            }
        }

        // Insert subtitles now that we have video IDs to link them to
        for (name, path) in upload_subtitles {
            if let Some(video_id) = video_blobs.get(&name) {
                util_funcs::insert_subtitles(&self.db, &path, &self.blob_dir, *video_id).await?;
            }
        }

        let _ = tokio::fs::remove_dir_all(self.directory).await;

        return Ok(());
    }
    pub async fn discard(self) {
        // TODO: Remove rip job
        let _ = tokio::fs::remove_dir_all(self.directory).await;
    }
}
