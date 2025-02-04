use anyhow::Context;
use rocket::{http::ContentType, response::stream::ReaderStream, Route, State};
use std::sync::Arc;

use crate::{application::Application, AnyhowError};

#[get("/subtitles/<blob_id>")]
async fn get_subtitles(
    application: &State<Arc<Application>>,
    blob_id: String,
) -> Result<(ContentType, ReaderStream![tokio::fs::File]), AnyhowError> {
    let path = application.get_blob_path(&blob_id);
    let file = tokio::fs::File::open(path)
        .await
        .context("Couldn't open blob")?;
    return Ok((ContentType::Text, ReaderStream::one(file)));
}

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![get_subtitles];
}
