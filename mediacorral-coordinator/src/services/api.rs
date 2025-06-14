use std::fs::File;
use std::io::{ErrorKind, Read};
use std::sync::Arc;

use crate::application::Application;
use crate::managers::exports::ExportsDirError;
use crate::managers::tmdb::TmdbError;
use mediacorral_proto::mediacorral::coordinator::v1::AutoripStatus;
use mediacorral_proto::mediacorral::{
    coordinator::v1::{self as proto, coordinator_api_service_server::CoordinatorApiService},
    drive_controller::v1::RipUpdate,
};
use tokio_stream::wrappers::ReceiverStream;

trait ToTonic {
    type T;
    fn bubble(self) -> Result<Self::T, tonic::Status>;
}
impl<T> ToTonic for std::io::Result<T> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        return self.map_err(|err| match err.kind() {
            ErrorKind::NotFound => tonic::Status::not_found("The requested asset was not found"),
            _ => tonic::Status::unknown(format!("An unknown error occurred:\n{err}")),
        });
    }
}
impl<T> ToTonic for Result<T, TmdbError> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        return self.map_err(|err| match err {
            TmdbError::ReqwestError(err) => tonic::Status::from_error(Box::from(err)),
            TmdbError::DeserializeError(err) => {
                tonic::Status::unknown(format!("Unable to parse TMDB response:\n{err}"))
            }
        });
    }
}
impl<T> ToTonic for anyhow::Result<T> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        return self.map_err(|err| tonic::Status::unknown(format!("{err}")));
    }
}
impl<T> ToTonic for Result<T, ExportsDirError> {
    type T = T;
    fn bubble(self) -> Result<T, tonic::Status> {
        return self.map_err(|err| match err {
            ExportsDirError::DirNotFound => {
                tonic::Status::not_found("The requested exports directory was not found.")
            }
            ExportsDirError::Io(err) => tonic::Status::internal(format!("{err}")),
            ExportsDirError::Db(err) => tonic::Status::internal(format!("{err}")),
        });
    }
}

const CHUNK_SIZE_LIMIT: u64 = 2000000; // 2MB

pub struct ApiService {
    application: Arc<Application>,
}
impl ApiService {
    pub fn new(application: Arc<Application>) -> Self {
        return Self { application };
    }
}
#[tonic::async_trait]
impl CoordinatorApiService for ApiService {
    /// Gets textual subtitles
    async fn get_subtitles(
        &self,
        request: tonic::Request<proto::GetSubtitlesRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetSubtitlesResponse>, tonic::Status> {
        let request = request.into_inner();
        let file_path = self
            .application
            .blob_storage
            .get_file_path(&request.blob_id);
        let mut subtitle_file = File::open(file_path).bubble()?;
        let meta = subtitle_file.metadata().bubble()?;
        if meta.len() > CHUNK_SIZE_LIMIT {
            return Err(tonic::Status::failed_precondition(
                "Subtitles too large to send.",
            ));
        }

        let mut subtitles = String::new();
        subtitle_file.read_to_string(&mut subtitles).bubble()?;

        return Ok(tonic::Response::new(proto::GetSubtitlesResponse {
            subtitles,
        }));
    }

    /// Searches TheMovieDatabase for a given query
    async fn search_tmdb_multi(
        &self,
        request: tonic::Request<proto::SearchTmdbMultiRequest>,
    ) -> std::result::Result<tonic::Response<proto::SearchTmdbMultiResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = self
            .application
            .tmdb_importer
            .query_any(
                &request.query,
                request.language.as_ref().map(String::as_str),
                request.page.unwrap_or(1),
            )
            .await
            .bubble()?;

        return Ok(tonic::Response::new(proto::SearchTmdbMultiResponse {
            page: result.page,
            total_pages: result.total_pages,
            total_results: result.total_results,
            results: result.results.into_iter().map(Into::into).collect(),
        }));
    }

    /// Searches TheMovieDatabase for a TV show
    async fn search_tmdb_tv(
        &self,
        request: tonic::Request<proto::SearchTmdbTvRequest>,
    ) -> std::result::Result<tonic::Response<proto::SearchTmdbTvResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = self
            .application
            .tmdb_importer
            .query_tv(
                &request.query,
                request.first_air_date_year.as_ref().map(String::as_str),
                request.language.as_ref().map(String::as_str),
                request.year.as_ref().map(String::as_str),
                request.page,
            )
            .await
            .bubble()?;

        return Ok(tonic::Response::new(proto::SearchTmdbTvResponse {
            page: result.page,
            total_pages: result.total_pages,
            total_results: result.total_results,
            results: result.results.into_iter().map(Into::into).collect(),
        }));
    }

    /// Searches TheMovieDatabase for a Movie
    async fn search_tmdb_movie(
        &self,
        request: tonic::Request<proto::SearchTmdbMovieRequest>,
    ) -> std::result::Result<tonic::Response<proto::SearchTmdbMovieResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = self
            .application
            .tmdb_importer
            .query_movies(
                &request.query,
                request.language.as_ref().map(String::as_str),
                request.primary_release_year.as_ref().map(String::as_str),
                request.region.as_ref().map(String::as_str),
                request.year.as_ref().map(String::as_str),
                request.page.unwrap_or(1),
            )
            .await
            .bubble()?;

        return Ok(tonic::Response::new(proto::SearchTmdbMovieResponse {
            page: result.page,
            total_pages: result.total_pages,
            total_results: result.total_results,
            results: result.results.into_iter().map(Into::into).collect(),
        }));
    }

    /// Imports a TV show from TheMovieDatabase
    async fn import_tmdb_tv(
        &self,
        request: tonic::Request<proto::ImportTmdbTvRequest>,
    ) -> std::result::Result<tonic::Response<proto::ImportTmdbTvResponse>, tonic::Status> {
        let request = request.into_inner();

        self.application
            .import_tmdb_tv(request.tmdb_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::ImportTmdbTvResponse {}));
    }

    /// Imports a Movie from TheMovieDatabase
    async fn import_tmdb_movie(
        &self,
        request: tonic::Request<proto::ImportTmdbMovieRequest>,
    ) -> std::result::Result<tonic::Response<proto::ImportTmdbMovieResponse>, tonic::Status> {
        let request = request.into_inner();

        self.application
            .import_tmdb_movie(request.tmdb_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::ImportTmdbMovieResponse {}));
    }

    /// Rebuild exports directory
    async fn rebuild_exports_dir(
        &self,
        request: tonic::Request<proto::RebuildExportsDirRequest>,
    ) -> std::result::Result<tonic::Response<proto::RebuildExportsDirResponse>, tonic::Status> {
        let request = request.into_inner();

        self.application
            .rebuild_exports_dir(&request.exports_dir)
            .await
            .bubble()?;

        return Ok(tonic::Response::new(proto::RebuildExportsDirResponse {}));
    }

    /// Gets/sets the status of the auto-ripper
    async fn autorip_status(
        &self,
        request: tonic::Request<proto::AutoripStatusRequest>,
    ) -> std::result::Result<tonic::Response<proto::AutoripStatusResponse>, tonic::Status> {
        let request = request.into_inner();

        let status = match AutoripStatus::try_from(request.status) {
            Ok(AutoripStatus::Enabled) => {
                self.application.set_autorip(true);
                true
            }
            Ok(AutoripStatus::Disabled) => {
                self.application.set_autorip(false);
                false
            }
            _ => self.application.get_autorip(),
        };

        return Ok(tonic::Response::new(proto::AutoripStatusResponse {
            status: match status {
                true => AutoripStatus::Enabled as _,
                false => AutoripStatus::Disabled as _,
            },
        }));
    }

    /// Lists the currently-registered drives
    async fn list_drives(
        &self,
        request: tonic::Request<proto::ListDrivesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListDrivesResponse>, tonic::Status> {
        todo!();
    }

    /// Starts a rip job
    async fn start_rip_job(
        &self,
        request: tonic::Request<proto::StartRipJobRequest>,
    ) -> std::result::Result<tonic::Response<proto::StartRipJobResponse>, tonic::Status> {
        todo!();
    }

    /// Gets the current status of a rip job
    async fn get_rip_job_status(
        &self,
        request: tonic::Request<proto::GetRipJobStatusRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetRipJobStatusResponse>, tonic::Status> {
        todo!();
    }

    /// Server streaming response type for the StreamRipJobUpdates method.
    type StreamRipJobUpdatesStream = ReceiverStream<Result<RipUpdate, tonic::Status>>;

    /// Streams status updates from a rip job.
    /// Initial state is always `RipStatus::default()`.
    async fn stream_rip_job_updates(
        &self,
        request: tonic::Request<proto::StreamRipJobUpdatesRequest>,
    ) -> std::result::Result<tonic::Response<Self::StreamRipJobUpdatesStream>, tonic::Status> {
        todo!();
    }

    /// Ejects a disc
    async fn eject(
        &self,
        request: tonic::Request<proto::EjectRequest>,
    ) -> std::result::Result<tonic::Response<proto::EjectResponse>, tonic::Status> {
        todo!();
    }

    /// Retracts a disc
    async fn retract(
        &self,
        request: tonic::Request<proto::RetractRequest>,
    ) -> std::result::Result<tonic::Response<proto::RetractResponse>, tonic::Status> {
        todo!();
    }

    /// Lists the movies in the database
    async fn list_movies(
        &self,
        request: tonic::Request<proto::ListMoviesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListMoviesResponse>, tonic::Status> {
        todo!();
    }

    /// Gets a movie from the database by its TMDB ID
    async fn get_movie_by_tmdb_id(
        &self,
        request: tonic::Request<proto::GetMovieByTmdbIdRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetMovieByTmdbIdResponse>, tonic::Status> {
        todo!();
    }

    /// Lists the TV shows in the database
    async fn list_tv_shows(
        &self,
        request: tonic::Request<proto::ListTvShowsRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvShowsResponse>, tonic::Status> {
        todo!();
    }

    /// Lists the seasons for a given TV show
    async fn list_tv_seasons(
        &self,
        request: tonic::Request<proto::ListTvSeasonsRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvSeasonsResponse>, tonic::Status> {
        todo!();
    }

    /// Lists the episodes for a given season
    async fn list_tv_episodes(
        &self,
        request: tonic::Request<proto::ListTvEpisodesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvEpisodesResponse>, tonic::Status> {
        todo!();
    }

    /// Gets a particular TV episode
    async fn get_tv_episode(
        &self,
        request: tonic::Request<proto::GetTvEpisodeRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetTvEpisodeResponse>, tonic::Status> {
        todo!();
    }

    /// Gets a particular TV episode by TMDB id
    async fn get_tv_episode_by_tmdb_id(
        &self,
        request: tonic::Request<proto::GetTvEpisodeByTmdbIdRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetTvEpisodeByTmdbIdResponse>, tonic::Status>
    {
        todo!();
    }

    /// Tags a video file with metadata
    async fn tag_file(
        &self,
        request: tonic::Request<proto::TagFileRequest>,
    ) -> std::result::Result<tonic::Response<proto::TagFileResponse>, tonic::Status> {
        todo!();
    }

    /// Gets a list of jobs containing untagged files
    async fn get_untagged_jobs(
        &self,
        request: tonic::Request<proto::GetUntaggedJobsRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetUntaggedJobsResponse>, tonic::Status> {
        todo!();
    }

    /// Gets all info needed to catalog a job
    async fn get_job_catalogue_info(
        &self,
        request: tonic::Request<proto::GetJobCatalogueInfoRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetJobCatalogueInfoResponse>, tonic::Status>
    {
        todo!();
    }
}
