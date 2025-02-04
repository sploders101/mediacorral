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

#[post("/rebuild/<exports_dir>")]
async fn post_rebuild_exports_dir(
    application: &State<Arc<Application>>,
    exports_dir: String,
) -> Result<(), AnyhowError> {
    application.rebuild_exports(&exports_dir).await?;
    return Ok(());
}

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![post_rebuild_exports_dir];
}
