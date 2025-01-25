use futures::StreamExt;
use rocket::{
    response::stream::{Event, EventStream},
    serde::json::Json,
    Route, State,
};
use serde::Deserialize;
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    application::Application, async_udev::disc_insert_events, drive_controller::DriveState,
    tagging::types::SuspectedContents, AnyhowError, AutoripEnabler,
};

#[get("/autorip")]
async fn get_autorip(enabler: &State<AutoripEnabler>) -> Json<bool> {
    return Json(*enabler.inner().0.lock().await);
}

#[post("/autorip", data = "<data>")]
async fn post_autorip(enabler: &State<AutoripEnabler>, data: Json<bool>) {
    *enabler.inner().0.lock().await = data.0;
}

#[get("/list_drives")]
fn get_list_drives(application: &State<Arc<Application>>) -> Json<Vec<String>> {
    return Json(
        application
            .list_drives()
            .map(|item| String::from(item.get_devname()))
            .collect(),
    );
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

#[get("/rip_status?<device>")]
async fn get_rip_status(
    application: &State<Arc<Application>>,
    device: String,
) -> Json<Option<DriveState>> {
    let application = application.inner();
    return Json(match application.get_drive(&Path::new(&device)) {
        Ok(device) => Some(device.watch_state().borrow().clone()),
        Err(_err) => None,
    });
}

#[get("/rip_status?<device>&stream=true")]
async fn get_rip_status_stream(
    application: &State<Arc<Application>>,
    device: String,
) -> Result<EventStream![Event], AnyhowError> {
    let application = application.inner();
    let drive = application.get_drive(&Path::new(&device))?;
    let mut state = drive.watch_state();
    return Ok(EventStream! {
        let state_string = serde_json::to_string(&*state.borrow()).unwrap();
        yield Event::data(state_string);
        loop {
            if let Err(_) = state.changed().await {
                return;
            }
            let state_string = serde_json::to_string(&*state.borrow()).unwrap();
            yield Event::data(state_string);
        }
    });
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

pub fn get_routes() -> impl Into<Vec<Route>> {
    return routes![
        get_autorip,
        post_autorip,
        get_list_drives,
        post_rip,
        get_rip_status,
        get_rip_status_stream
    ];
}
