use std::sync::Arc;

use rocket::{serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use crate::{
    application::{types::JobInfo, Application},
    db::schemas::{MoviesItem, RipJobsItem, TvEpisodesItem, TvSeasonsItem, TvShowsItem, VideoType},
    tagging::types::SuspectedContents,
    AnyhowError,
};

#[get("/metadata/movies/list")]
async fn get_list_movies(
    application: &State<Arc<Application>>,
) -> Result<Json<Vec<MoviesItem>>, AnyhowError> {
    return Ok(Json(application.list_movies().await?));
}

#[get("/metadata/tv/list")]
async fn get_list_tv(
    application: &State<Arc<Application>>,
) -> Result<Json<Vec<TvShowsItem>>, AnyhowError> {
    return Ok(Json(application.list_tv_series().await?));
}

#[get("/metadata/tv/<series_id>/seasons")]
async fn get_list_tv_seasons(
    application: &State<Arc<Application>>,
    series_id: i64,
) -> Result<Json<Vec<TvSeasonsItem>>, AnyhowError> {
    return Ok(Json(application.list_tv_seasons(series_id).await?));
}

#[get("/metadata/tv/seasons/<season_id>/episodes")]
async fn get_list_tv_episodes(
    application: &State<Arc<Application>>,
    season_id: i64,
) -> Result<Json<Vec<TvEpisodesItem>>, AnyhowError> {
    return Ok(Json(application.list_tv_episodes(season_id).await?));
}

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

#[post("/jobs/<job_id>/rename", data = "<new_name>")]
async fn post_rename(
    application: &State<Arc<Application>>,
    job_id: i64,
    new_name: Json<String>,
) -> Result<(), AnyhowError> {
    application.rename_job(job_id, &new_name).await?;
    return Ok(());
}

#[get("/jobs/<job_id>/analysis_status")]
async fn analyzing_job(application: &State<Arc<Application>>, job_id: i64) -> Json<bool> {
    return Json(application.is_analyzing(job_id).await);
}

#[post("/jobs/<job_id>/suspicion", data = "<contents>")]
async fn suspect_job(
    application: &State<Arc<Application>>,
    job_id: i64,
    contents: Json<Option<SuspectedContents>>,
) -> Result<(), AnyhowError> {
    application.suspect_content(job_id, contents.0).await?;
    return Ok(());
}

#[post("/jobs/<job_id>/prune")]
async fn prune_job(application: &State<Arc<Application>>, job_id: i64) -> Result<(), AnyhowError> {
    application.prune_rip_job(job_id).await?;
    return Ok(());
}

#[delete("/jobs/<job_id>")]
async fn delete_job(application: &State<Arc<Application>>, job_id: i64) -> Result<(), AnyhowError> {
    application.delete_rip_job(job_id).await?;
    return Ok(());
}

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![
        get_list_movies,
        get_list_tv,
        get_list_tv_seasons,
        get_list_tv_episodes,
        post_tag_file,
        get_untagged_jobs,
        get_job,
        post_rename,
        analyzing_job,
        suspect_job,
        prune_job,
        delete_job,
    ];
}
