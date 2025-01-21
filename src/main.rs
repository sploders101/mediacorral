use application::Application;
use async_udev::disc_insert_events;
use futures::StreamExt;
use rocket::{
    http::{ContentType, Status},
    response::{stream::TextStream, Responder},
    serde::json::Json,
    Response, State,
};
use serde::Deserialize;
use std::{io::Cursor, path::Path, sync::Arc};
use tagging::types::SuspectedContents;
use tokio::sync::Mutex;

#[macro_use]
extern crate rocket;

mod application;
mod async_udev;
mod blob_storage;
mod config;
mod db;
mod drive_controller;
mod makemkv;
mod media_helpers;
mod tagging;
mod task_queue;

struct AnyhowError(anyhow::Error);
impl<'r, 'o: 'r> Responder<'r, 'o> for AnyhowError {
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
impl From<anyhow::Error> for AnyhowError {
    fn from(value: anyhow::Error) -> Self {
        return Self(value);
    }
}

struct AutoripEnabler(Arc<Mutex<bool>>);

#[get("/autorip")]
async fn get_autorip(enabler: &State<AutoripEnabler>) -> Json<bool> {
    return Json(*enabler.inner().0.lock().await);
}

#[post("/autorip", data = "<data>")]
async fn post_autorip(enabler: &State<AutoripEnabler>, data: Json<bool>) {
    *enabler.inner().0.lock().await = data.0;
}

#[derive(Deserialize)]
struct RipInstruction {
    device: String,
    disc_name: Option<String>,
    suspected_contents: Option<SuspectedContents>,
    autoeject: bool,
}

#[post("/rip", data = "<data>")]
async fn post_rip(
    application: &State<Arc<Application>>,
    data: Json<RipInstruction>,
) -> Result<(), AnyhowError> {
    application
        .inner()
        .get_drive(&Path::new(&data.0.device))?
        .rip(
            data.0.disc_name,
            data.0.suspected_contents,
            data.0.autoeject,
        );

    return Ok(());
}

#[get("/")]
fn index(application: &State<Arc<Application>>) -> TextStream![&str] {
    return TextStream! {
        for drive in application.list_drives() {
            yield drive.get_devname();
        }
    };
}

#[launch]
async fn rocket() -> _ {
    let data_dir = std::env::var("DATA_DIR").unwrap();

    let blob_dir = Path::new(&data_dir).join("storage");
    let sqlite_path = Path::new(&data_dir).join("database.sqlite");

    let db = Arc::new(
        sqlx::SqlitePool::connect(sqlite_path.to_str().unwrap())
            .await
            .expect("Couldn't open sqlite database"),
    );

    let mut application = Application::new(db, blob_dir).await.unwrap();
    let drives =
        std::env::var("DISC_DRIVES").expect("Missing comma-separated DISC_DRIVES variable");
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
        .mount("/", routes![index, get_autorip, post_autorip, post_rip])
}

pub fn create_autoripper(enabler: Arc<Mutex<bool>>, application: Arc<Application>) {
    tokio::task::spawn(async move {
        let mut events = std::pin::pin!(disc_insert_events());
        while let Some(insertion) = events.next().await {
            if !*enabler.lock().await {
                continue;
            }
            if let Ok(drive) = application.get_drive(&Path::new(&insertion.device)) {
                drive.rip(Some(insertion.disc_name), None, true);
            }
        }
    });
}
