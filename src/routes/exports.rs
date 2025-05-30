use rocket::{Route, State};
use std::sync::Arc;

use crate::{application::Application, AnyhowError};

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
