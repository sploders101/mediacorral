use std::sync::Arc;

use rocket::{serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use crate::{
    application::{types::JobInfo, Application},
    db::schemas::{RipJobsItem, VideoType},
    AnyhowError,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagFile {
    file: i64,
    video_type: VideoType,
    match_id: i64,
}

#[post("/tag_file", data = "<data>")]
async fn post_tag_file(
    application: &State<Arc<Application>>,
    data: Json<TagFile>,
) -> Result<(), AnyhowError> {
    application
        .tag_video(data.file, data.video_type, data.match_id)
        .await?;
    return Ok(());
}

#[get("/get_untagged_jobs?<skip>&<limit>")]
async fn get_untagged_jobs(
    application: &State<Arc<Application>>,
    skip: Option<u32>,
    limit: Option<u32>,
) -> Result<Json<Vec<RipJobsItem>>, AnyhowError> {
    let skip = skip.unwrap_or(0);
    let limit = limit.unwrap_or(1000);
    let results = application.get_untagged_jobs(skip, limit).await?;
    return Ok(Json(results));
}

#[get("/jobs/<job_id>")]
async fn get_job(
    application: &State<Arc<Application>>,
    job_id: i64,
) -> Result<Json<JobInfo>, AnyhowError> {
    let results = application.get_job_info(job_id).await?;

    return Ok(Json(results));
}

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![post_tag_file, get_untagged_jobs,];
}
