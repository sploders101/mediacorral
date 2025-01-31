use std::sync::Arc;

use rocket::{serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use crate::{
    application::Application,
    tagging::importers::tmdb::types::{
        TmdbAnyTitle, TmdbMovieResult, TmdbQueryResults, TmdbTvResult,
    },
    AnyError, AnyhowError,
};

#[delete("/ost/subtitles/show/<show_id>")]
async fn purge_ost_subtitles_for_show(
    application: &State<Arc<Application>>,
    show_id: i64,
) -> Result<(), AnyhowError> {
    application.purge_ost_subtitles_by_show(show_id).await?;
    return Ok(());
}

#[get("/tmdb/any/search?<query>&<language>&<page>")]
async fn get_search_tmdb_multi(
    application: &State<Arc<Application>>,
    query: String,
    language: Option<String>,
    page: Option<u32>,
) -> Result<Json<TmdbQueryResults<TmdbAnyTitle>>, AnyError> {
    let language = language.as_ref().map(|item| item.as_str());
    let page = page.unwrap_or(1);

    let results = application
        .importer()
        .query_any(&query, language, page)
        .await?;

    return Ok(Json(results));
}

#[get("/tmdb/tv/search?<query>&<first_air_date_year>&<language>&<year>&<page>")]
async fn get_search_tmdb_tv(
    application: &State<Arc<Application>>,
    query: String,
    first_air_date_year: Option<String>,
    language: Option<String>,
    year: Option<String>,
    page: Option<u32>,
) -> Result<Json<TmdbQueryResults<TmdbTvResult>>, AnyError> {
    let first_air_date_year = first_air_date_year.as_ref().map(|item| item.as_str());
    let language = language.as_ref().map(|item| item.as_str());
    let year = year.as_ref().map(|item| item.as_str());
    let page = page.unwrap_or(1);

    let results = application
        .importer()
        .query_tv(&query, first_air_date_year, language, year, page)
        .await?;

    return Ok(Json(results));
}

#[get("/tmdb/movie/search?<query>&<language>&<primary_release_year>&<region>&<year>&<page>")]
async fn get_search_tmdb_movies(
    application: &State<Arc<Application>>,
    query: String,
    language: Option<String>,
    primary_release_year: Option<String>,
    region: Option<String>,
    year: Option<String>,
    page: Option<u32>,
) -> Result<Json<TmdbQueryResults<TmdbMovieResult>>, AnyError> {
    let language = language.as_ref().map(|item| item.as_str());
    let primary_release_year = primary_release_year.as_ref().map(|item| item.as_str());
    let region = region.as_ref().map(|item| item.as_str());
    let year = year.as_ref().map(|item| item.as_str());
    let page = page.unwrap_or(1);

    let results = application
        .importer()
        .query_movies(&query, language, primary_release_year, region, year, page)
        .await?;

    return Ok(Json(results));
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvImport {
    tmdb_id: i32,
}

#[post("/tmdb/tv/import", data = "<data>")]
async fn post_import_tmdb_tv(
    application: &State<Arc<Application>>,
    data: Json<TmdbTvImport>,
) -> Result<(), AnyhowError> {
    application.importer().import_tv(data.tmdb_id).await?;
    return Ok(());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbMovieImport {
    tmdb_id: i32,
}

#[post("/tmdb/movie/import", data = "<data>")]
async fn post_import_tmdb_movie(
    application: &State<Arc<Application>>,
    data: Json<TmdbMovieImport>,
) -> Result<(), AnyhowError> {
    application.importer().import_movie(data.tmdb_id).await?;
    return Ok(());
}

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![
        get_search_tmdb_multi,
        get_search_tmdb_tv,
        get_search_tmdb_movies,
        post_import_tmdb_tv,
        post_import_tmdb_movie,
        purge_ost_subtitles_for_show,
    ];
}
