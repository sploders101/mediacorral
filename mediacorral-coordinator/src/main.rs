use std::{path::PathBuf, sync::Arc};

use application::Application;
use clap::Parser;
use mediacorral_proto::mediacorral::coordinator::v1::{
    coordinator_api_service_server::CoordinatorApiServiceServer,
    coordinator_job_service_server::CoordinatorJobServiceServer,
    coordinator_notification_service_server::CoordinatorNotificationServiceServer,
};
use serde::Deserialize;
use services::{api::ApiService, jobs::JobService, notifications::NotificationService};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;

mod application;
mod blob_storage;
mod db;
mod managers;
mod services;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, short)]
    config: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CoordinatorConfig {
    shared_directory: PathBuf,
    serve_address: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_file = std::fs::File::open(args.config).expect("Couldn't open config");
    let config: CoordinatorConfig =
        serde_yaml::from_reader(config_file).expect("Couldn't read config");

    let application = Arc::new(Application::new());

    Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(CoordinatorJobServiceServer::new(JobService::new(
            Arc::clone(&application),
        )))
        .add_service(CoordinatorNotificationServiceServer::new(
            NotificationService::new(Arc::clone(&application)),
        ))
        .add_service(CoordinatorApiServiceServer::new(ApiService::new(
            application,
        )))
        .serve(
            config
                .serve_address
                .parse()
                .expect("Invalid serve address."),
        )
        .await
        .unwrap();
}
