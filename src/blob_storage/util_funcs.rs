use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use matroska::Matroska;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    db::{self as db_funcs, schemas},
    media_helpers::get_video_info,
    tagging::types::SuspectedContents,
};

pub async fn insert_video(
    db: &SqlitePool,
    path: &Path,
    blob_dir: &Path,
    rip_job: Option<i64>,
) -> anyhow::Result<i64> {
    // Get metadata about the video
    let info = get_video_info(path).await?;

    // Add it to the database & blob folder
    let uuid = Uuid::new_v4().to_string();
    tokio::fs::rename(path, blob_dir.join(&uuid)).await?;
    let id = db_funcs::insert_video_file(
        db,
        &schemas::VideoFilesItem {
            id: None,
            video_type: schemas::VideoType::Untagged,
            match_id: None,
            blob_id: uuid,
            resolution_width: info.width as _,
            resolution_height: info.height as _,
            length: info.length as _,
            original_video_hash: info.hash,
            rip_job,
        },
    )
    .await?;

    return Ok(id);
}

pub async fn insert_subtitles(
    db: &SqlitePool,
    path: &Path,
    blob_dir: &Path,
    video_id: i64,
) -> anyhow::Result<i64> {
    let uuid = Uuid::new_v4().to_string();
    tokio::fs::rename(path, blob_dir.join(&uuid)).await?;
    let id = db_funcs::insert_subtitle_file(db, &schemas::SubtitleFilesItem {
        id: None,
        blob_id: uuid,
        video_file: video_id,
    }).await?;

    return Ok(id);
}

fn get_timestamp() -> i64 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is incorrect")
        .as_secs() as _;
}

pub async fn create_rip_job(
    db: &SqlitePool,
    disc_title: Option<String>,
    suspected_contents: Option<SuspectedContents>,
) -> anyhow::Result<i64> {
    let job_id = db_funcs::insert_rip_jobs(
        db,
        &schemas::RipJobsItem {
            id: None,
            start_time: get_timestamp(),
            disc_title,
            suspected_contents: suspected_contents
                .map(|value| serde_json::to_string(&value).unwrap()),
        },
    )
    .await?;

    return Ok(job_id);
}
