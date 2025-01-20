#[macro_use]
extern crate rocket;

mod application;
mod blob_storage;
mod config;
mod db;
mod drive_controller;
mod makemkv;
mod media_helpers;
mod task_queue;
mod tagging;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
