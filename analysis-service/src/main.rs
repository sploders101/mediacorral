use std::path::PathBuf;

use tonic::{async_trait, transport::Server};

use proto::mediacorral::analysis::v1 as pb;

use crate::utils::{ExtractDetailsError, extract_details, subtitles::ocr::PartessCache};

mod proto;
mod rayon_helpers;
mod utils;

struct MediaAnalysisServiceProvider {
    partess_cache: PartessCache,
    blob_dir: PathBuf,
}
impl MediaAnalysisServiceProvider {
    pub fn new(blob_dir: PathBuf) -> Self {
        return MediaAnalysisServiceProvider {
            partess_cache: PartessCache::new(),
            blob_dir,
        };
    }
}
#[async_trait]
impl pb::media_analysis_service_server::MediaAnalysisService for MediaAnalysisServiceProvider {
    async fn analyze_mkv(
        &self,
        request: tonic::Request<pb::AnalyzeMkvRequest>,
    ) -> tonic::Result<tonic::Response<pb::AnalyzeMkvResponse>> {
        let request = request.into_inner();
        if request
            .blob_id
            .chars()
            .any(|i| !matches!(i, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-'))
        {
            return Err(tonic::Status::invalid_argument("Invalid blob_id."));
        };
        let blob_path = self.blob_dir.join(&request.blob_id);
        let partess_cache = self.partess_cache.clone();
        let result = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(blob_path)?;
            return extract_details(file, &partess_cache, request.st_track_number);
        })
        .await
        .unwrap();

        return match result {
            Ok(result) => Ok(tonic::Response::new(result)),
            // Handle special-case I/O error for non-existent file
            Err(ExtractDetailsError::Io(io_err))
                if io_err.kind() == std::io::ErrorKind::NotFound =>
            {
                Err(tonic::Status::not_found(io_err.to_string()))
            }
            // Pull out I/O errors as they may not be safe to disclose
            Err(ExtractDetailsError::Io(err)) => {
                eprintln!("{err}");
                Err(tonic::Status::internal("An internal error occurred"))
            }
            // Other errors are safe to disclose
            Err(err) => {
                eprintln!("{err}");
                Err(tonic::Status::internal(err.to_string()))
            }
        };
    }
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let addr = match std::env::var("LISTEN_ADDR") {
        Ok(val) => val.parse()?,
        Err(_) => "[::]:50051".parse()?,
    };
    let blob_dir = match std::env::var("BLOB_DIR") {
        Ok(val) => PathBuf::from(val),
        Err(_) => PathBuf::from("/mnt/mediacorral/blobs"),
    };
    let provider = MediaAnalysisServiceProvider::new(blob_dir);

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::mediacorral::FILE_DESCRIPTOR_SET)
        .with_service_name("mediacorral.analysis.v1.MediaAnalysisService")
        .build_v1()
        .unwrap();

    Server::builder()
        .add_service(reflection)
        .add_service(pb::media_analysis_service_server::MediaAnalysisServiceServer::new(provider))
        .serve(addr)
        .await?;

    return Ok(());
}
