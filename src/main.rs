#[macro_use] extern crate rocket;

mod makemkv;
mod drive_controller;
mod blob_storage;
mod db;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
