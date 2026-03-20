//! Disclaimer: This was generated primarily by ChatGPT. It is meant only as a debugging tool for
//! the drive-controller until a proper server implementation has been written.

use std::{
    collections::HashMap,
    io::{self, BufRead},
    sync::{Arc, Mutex},
};

use futures::StreamExt;
use tokio::{sync::mpsc, time::Instant};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};

// --- IMPORT YOUR GENERATED PROTO HERE ---
use crate::proto::mediacorral::drive_coordinator::v1::{
    DriveConnectionRequest, DriveConnectionResponse, RipMediaCommand, TrayCommand,
    UploadFileRequest, UploadFileResponse, drive_connection_request, drive_connection_response,
    drive_coordinator_service_server::{DriveCoordinatorService, DriveCoordinatorServiceServer},
    upload_file_request,
};

// --- STATE ---

type DriveId = String;

#[derive(Clone)]
struct DriveHandle {
    sender: mpsc::Sender<Result<DriveConnectionResponse, Status>>,
}

#[derive(Default, Clone)]
struct ServerState {
    drives: Arc<Mutex<HashMap<DriveId, DriveHandle>>>,
}

// --- gRPC IMPLEMENTATION ---

#[tonic::async_trait]
impl DriveCoordinatorService for ServerState {
    type ConnectDriveStream = ReceiverStream<Result<DriveConnectionResponse, Status>>;

    async fn connect_drive(
        &self,
        request: Request<tonic::Streaming<DriveConnectionRequest>>,
    ) -> Result<Response<Self::ConnectDriveStream>, Status> {
        let mut stream = request.into_inner();

        let (tx, rx) = mpsc::channel(32);
        let state = self.clone();

        tokio::spawn(async move {
            let mut drive_id: Option<String> = None;

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(msg) => match msg.message {
                        Some(drive_connection_request::Message::Discovery(info)) => {
                            println!(
                                "🔌 Drive connected: {} ({})",
                                info.drive_id, info.drive_name
                            );

                            drive_id = Some(info.drive_id.clone());

                            state
                                .drives
                                .lock()
                                .unwrap()
                                .insert(info.drive_id.clone(), DriveHandle { sender: tx.clone() });
                        }

                        Some(drive_connection_request::Message::DriveStatusUpdate(status)) => {
                            println!("📀 Drive status update: {:?}", status);
                        }

                        Some(drive_connection_request::Message::RipStatusUpdate(status)) => {
                            println!("🎬 Rip status: {:?}", status);
                        }

                        _ => {}
                    },
                    Err(err) => {
                        println!("❌ Stream error: {:?}", err);
                        break;
                    }
                }
            }

            if let Some(id) = drive_id {
                state.drives.lock().unwrap().remove(&id);
                println!("🔌 Drive disconnected: {}", id);
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn upload_file(
        &self,
        request: Request<tonic::Streaming<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        let mut stream = request.into_inner();

        println!("📤 Receiving upload...");

        let mut tally: u64 = 0;
        let mut total: u64 = 0;
        let timer = Instant::now();

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(msg) => match msg.message {
                    Some(upload_file_request::Message::Header(h)) => {
                        println!("📁 File: {} (job {})", h.file_name, h.rip_job);
                        total = h.file_size;
                    }
                    Some(upload_file_request::Message::DataChunk(chunk)) => {
                        if total > 0 {
                            tally += chunk.len() as u64;
                            println!("📦 Chunk: {} bytes: {}%", chunk.len(), 100 * tally / total);
                        } else {
                            println!("📦 Chunk: {} bytes", chunk.len());
                        }
                    }
                    Some(upload_file_request::Message::Md5Hash(hash)) => {
                        println!("🔐 MD5: {:?} {:?}", hash, timer.elapsed());
                    }
                    _ => {}
                },
                Err(err) => {
                    println!("❌ Upload stream error: {:?}", err);
                    break;
                }
            }
        }

        println!("✅ Upload complete");
        Ok(Response::new(UploadFileResponse {}))
    }
}

// --- COMMAND HELPERS ---

impl ServerState {
    async fn send_tray_open(&self, drive_id: &str) {
        if let Some(handle) = self.drives.lock().unwrap().get(drive_id) {
            let _ = handle
                .sender
                .send(Ok(DriveConnectionResponse {
                    message: Some(drive_connection_response::Message::TrayCommand(
                        TrayCommand::OpenTray as i32,
                    )),
                }))
                .await;
        } else {
            println!("⚠️ Unknown drive: {}", drive_id);
        }
    }

    async fn send_tray_close(&self, drive_id: &str) {
        if let Some(handle) = self.drives.lock().unwrap().get(drive_id) {
            let _ = handle
                .sender
                .send(Ok(DriveConnectionResponse {
                    message: Some(drive_connection_response::Message::TrayCommand(
                        TrayCommand::CloseTray as i32,
                    )),
                }))
                .await;
        } else {
            println!("⚠️ Unknown drive: {}", drive_id);
        }
    }

    async fn send_rip(&self, drive_id: &str, job_id: i64) {
        if let Some(handle) = self.drives.lock().unwrap().get(drive_id) {
            let _ = handle
                .sender
                .send(Ok(DriveConnectionResponse {
                    message: Some(drive_connection_response::Message::RipMedia(
                        RipMediaCommand {
                            job_id,
                            autoeject: true,
                        },
                    )),
                }))
                .await;
        } else {
            println!("⚠️ Unknown drive: {}", drive_id);
        }
    }

    fn list_drives(&self) {
        let drives = self.drives.lock().unwrap();
        if drives.is_empty() {
            println!("(no drives connected)");
        } else {
            println!("Connected drives:");
            for k in drives.keys() {
                println!(" - {}", k);
            }
        }
    }
}

// --- MAIN ---

#[tokio::main]
async fn main() {
    let state = ServerState::default();
    let grpc_state = state.clone();

    let addr = "0.0.0.0:50051".parse().unwrap();

    println!("🚀 Dev Coordinator Server running on {}", addr);

    // Start gRPC server
    tokio::spawn(async move {
        Server::builder()
            .add_service(DriveCoordinatorServiceServer::new(grpc_state))
            .serve(addr)
            .await
            .unwrap();
    });

    println!("Commands:");
    println!("  list");
    println!("  open <drive_id>");
    println!("  close <drive_id>");
    println!("  rip <drive_id> <job_id>");

    // CLI loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let parts: Vec<_> = line.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["list"] => state.list_drives(),

            ["open", drive] => {
                state.send_tray_open(drive).await;
            }

            ["close", drive] => {
                state.send_tray_close(drive).await;
            }

            ["rip", drive, job] => match job.parse::<i64>() {
                Ok(job_id) => state.send_rip(drive, job_id).await,
                Err(_) => println!("Invalid job_id"),
            },

            _ => {
                println!("Unknown command");
            }
        }
    }
}

// Needed because of how tonic generates modules
pub mod proto;
