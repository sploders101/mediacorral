use std::{collections::HashMap, path::PathBuf, sync::Arc};

use application::Application;
use clap::Parser;
use mediacorral_proto::mediacorral::server::v1::{
    coordinator_api_service_server::CoordinatorApiServiceServer,
    coordinator_notification_service_server::CoordinatorNotificationServiceServer,
};
use serde::Deserialize;
use services::{api::ApiService, notifications::NotificationService};
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;

mod application;
mod blob_storage;
mod db;
mod managers;
mod services;
mod workers;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, short)]
    config: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CoordinatorConfig {
    data_directory: PathBuf,
    tmdb_api_key: String,
    ost_login: OstCreds,
    serve_address: String,
    exports_dirs: HashMap<String, export_settings::ExportSettings>,
    enable_autorip: bool,
    drive_controllers: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OstCreds {
    api_key: String,
    username: String,
    password: String,
}

pub mod export_settings {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum MediaType {
        Movies,
        TvShows,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum LinkType {
        Symbolic,
        Hard,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ExportSettings {
        pub media_type: MediaType,
        pub link_type: LinkType,
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config_file = std::fs::File::open(args.config).expect("Couldn't open config");
    let config: CoordinatorConfig =
        serde_yaml::from_reader(config_file).expect("Couldn't read config");

    let address = config
        .serve_address
        .parse()
        .expect("Invalid serve address.");

    let application = Arc::new(
        Application::new(config)
            .await
            .expect("Couldn't start application"),
    );

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(mediacorral_proto::mediacorral::FILE_DESCRIPTOR_SET)
        .with_service_name("mediacorral.server.v1.CoordinatorNotificationService")
        .with_service_name("mediacorral.server.v1.CoordinatorApiService")
        .build_v1()
        .unwrap();

    Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(CoordinatorNotificationServiceServer::new(
            NotificationService::new(Arc::clone(&application)),
        ))
        .add_service(CoordinatorApiServiceServer::new(ApiService::new(
            application,
        )))
        .add_service(reflection)
        .serve(address)
        .await
        .unwrap();
}
