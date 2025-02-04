use application::Application;
use rocket::{
    fs::FileServer,
    http::{ContentType, Status},
    response::{content::RawHtml, stream::ReaderStream, Responder},
    Request, Response,
};
use routes::create_autoripper;
use std::{io::Cursor, path::Path, sync::Arc};
use tokio::sync::Mutex;

#[macro_use]
extern crate rocket;

mod application;
mod async_udev;
mod blob_storage;
mod config;
mod db;
mod drive_controller;
mod exports_manager;
mod makemkv;
mod media_helpers;
mod routes;
mod tagging;
mod task_queue;

struct AutoripEnabler(Arc<Mutex<bool>>);

/// This should be removed at some point, but is fine for an MVP.
/// This wraps any error type and provides a 500 response code, because
/// apparently, the default `Responder` implementation for `Result` doesn't.
/// Also, `anyhow::Error` apparently doesn't implement `std::error::Error`,
/// and blanket implementations don't mix with concrete ones for some ridiculous
/// reason, forcing this to exist as a separate type.
struct AnyhowError(anyhow::Error);
impl<'r, 'o: 'r> Responder<'r, 'o> for AnyhowError {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut response = Response::new();
        response.set_status(Status::InternalServerError);
        response.set_header(ContentType::Text);
        let body = format!("{:?}", self.0);
        let body = Vec::from(body.as_bytes());
        response.set_sized_body(body.len(), Cursor::new(body));
        return Ok(response);
    }
}
impl From<anyhow::Error> for AnyhowError {
    fn from(value: anyhow::Error) -> Self {
        return Self(value);
    }
}

/// This should be removed at some point, but is fine for an MVP.
/// This wraps any error type and provides a 500 response code, because
/// apparently, the default `Responder` implementation for `Result` doesn't.
struct AnyError(Box<dyn std::error::Error + Send + Sync + 'static>);
impl<'r, 'o: 'r> Responder<'r, 'o> for AnyError {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut response = Response::new();
        response.set_status(Status::InternalServerError);
        response.set_header(ContentType::Text);
        let body = format!("{}", self.0);
        let body = Vec::from(body.as_bytes());
        response.set_sized_body(body.len(), Cursor::new(body));
        return Ok(response);
    }
}
impl<T: std::error::Error + Send + Sync + 'static> From<T> for AnyError {
    fn from(value: T) -> Self {
        return Self(Box::from(value));
    }
}

#[launch]
async fn rocket() -> _ {
    let data_dir = std::env::var("DATA_DIR").unwrap();

    let blob_dir = Path::new(&data_dir).join("storage");
    let exports_dir = Path::new(&data_dir).join("exports");
    let sqlite_path = Path::new(&data_dir).join("database.sqlite");

    let db = Arc::new(
        sqlx::SqlitePool::connect(sqlite_path.to_str().unwrap())
            .await
            .expect("Couldn't open sqlite database"),
    );

    let mut application = Application::new(db, blob_dir).await.unwrap();
    let drives =
        std::env::var("DISC_DRIVES").expect("Missing whitespace-separated DISC_DRIVES variable");
    for drive in drives.split_whitespace() {
        application.register_drive(&Path::new(drive)).await.unwrap();
    }
    let application = Arc::new(application);
    let enable_autorip = std::env::var("ENABLE_AUTORIP")
        .map(|item| match item.as_str() {
            "1" | "yes" | "on" => true,
            "0" | "no" | "off" => false,
            _ => panic!("Invalid value for ENABLE_AUTORIP"),
        })
        .unwrap_or(false);
    let enable_autorip = Arc::new(Mutex::new(enable_autorip));
    create_autoripper(Arc::clone(&enable_autorip), Arc::clone(&application));

    rocket::build()
        .manage(application)
        .manage(AutoripEnabler(enable_autorip))
        .mount("/", FileServer::from("./frontend/dist"))
        .mount("/api/blobs", routes::blob_routes())
        .mount("/api/ripping", routes::ripping_routes())
        .mount("/api/data_imports", routes::data_imports_routes())
        .mount("/api/tagging", routes::tagging_routes())
        .mount("/api/exports", routes::exports_routes())
}
