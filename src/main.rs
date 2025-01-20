use std::{ops::Deref, path::Path, sync::Arc};

use application::Application;
use drive_controller::ActiveDriveCommand;

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

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
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
    application.register_drive(&Path::new("/dev/sr1")).await.unwrap();
    application.register_drive(&Path::new("/dev/sr2")).await.unwrap();
    let application = Arc::new(application);

    let mut sr1 = application.get_drive(&Path::new("/dev/sr1")).unwrap().watch_state();
    let mut sr2 = application.get_drive(&Path::new("/dev/sr2")).unwrap().watch_state();

    let mut autoripper = std::pin::pin!(application.create_autoripper());

    loop {
        tokio::select! {
            _ = &mut autoripper => {
                panic!("Somewhing went wrong. Restart mediacorral");
            }
            _ = sr1.changed() => {
                match &sr1.borrow_and_update().active_command {
                    ActiveDriveCommand::None => println!("sr1: Finished"),
                    ActiveDriveCommand::Error { message } => println!("sr1: Error:\n{message}"),
                    ActiveDriveCommand::Ripping {
                        cprog_title,
                        cprog_value,
                        tprog_title,
                        tprog_value,
                        max_prog_value,
                        logs,
                    } => {
                        let logs = logs.iter().fold(String::new(), |prev, curr| prev + "    " + curr + "\n");
                        println!("sr1:\n  cprog: {}/{} ({})\n  tprog: {}/{} ({})\n  logs: \n{}", cprog_value, max_prog_value, cprog_title, tprog_value, max_prog_value, tprog_title, logs);
                    },
                }
            }
            _ = sr2.changed() => {
                match &sr2.borrow_and_update().active_command {
                    ActiveDriveCommand::None => println!("sr2: Finished"),
                    ActiveDriveCommand::Error { message } => println!("sr2: Error:\n{message}"),
                    ActiveDriveCommand::Ripping {
                        cprog_title,
                        cprog_value,
                        tprog_title,
                        tprog_value,
                        max_prog_value,
                        logs,
                    } => {
                        let logs = logs.iter().fold(String::new(), |prev, curr| prev + "    " + curr + "\n");
                        println!("sr2:\n  cprog: {}/{} ({})\n  tprog: {}/{} ({})\n  logs: \n{}", cprog_value, max_prog_value, cprog_title, tprog_value, max_prog_value, tprog_title, logs);
                    },
                }
            }
        }
    }

    // rocket::build().mount("/", routes![index])
}
