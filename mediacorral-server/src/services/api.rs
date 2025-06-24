use std::fs::File;
use std::io::{ErrorKind, Read};
use std::sync::Arc;

use crate::application::{Application, ApplicationError};
use crate::db;
use crate::managers::exports::ExportsDirError;
use crate::managers::tmdb::TmdbError;
use mediacorral_proto::mediacorral::drive_controller::v1 as drive_controller;
use mediacorral_proto::mediacorral::{
    drive_controller::v1::{
        EjectRequest, GetDriveCountRequest, GetDriveMetaRequest, GetJobStatusRequest,
        RetractRequest, RipUpdate, WatchRipJobRequest,
    },
    server::v1::{self as proto, coordinator_api_service_server::CoordinatorApiService},
};
use prost::Message;

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
impl<T> ToTonic for Result<T, ApplicationError> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        self.map_err(|err| match err {
            ApplicationError::DbError(sqlx::Error::RowNotFound) => {
                tonic::Status::not_found(err.to_string())
            }
            ApplicationError::DbError(err) => tonic::Status::unknown(err.to_string()),
            ApplicationError::Io(err) => match err.kind() {
                ErrorKind::NotFound => tonic::Status::not_found(err.to_string()),
                _ => tonic::Status::unknown(err.to_string()),
            },
            ApplicationError::ControllerMissing => tonic::Status::not_found(err.to_string()),
            ApplicationError::TemporaryFailure => tonic::Status::unavailable(err.to_string()),
            ApplicationError::FailedPrecondition(err_str) => {
                tonic::Status::failed_precondition(err_str)
            }
            ApplicationError::TonicError(err) => err,
        })
    }
}
impl<T> ToTonic for Option<T> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        return self.ok_or_else(|| tonic::Status::not_found("The requested asset was not found"));
    }
}
impl<T> ToTonic for Result<T, sqlx::Error> {
    type T = T;
    fn bubble(self) -> Result<Self::T, tonic::Status> {
        return self.map_err(|err| match err {
            sqlx::Error::InvalidArgument(err) => tonic::Status::invalid_argument(err),
            sqlx::Error::Io(err) => tonic::Status::internal(format!("I/O error:\n{err}")),
            sqlx::Error::RowNotFound => {
                tonic::Status::not_found("The requested asset was not found in the database")
            }
            err => {
                tonic::Status::internal(format!("An unknown database error has occurred:\n{err}"))
            }
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

        let status = match proto::AutoripStatus::try_from(request.status) {
            Ok(proto::AutoripStatus::Enabled) => {
                self.application.set_autorip(true);
                true
            }
            Ok(proto::AutoripStatus::Disabled) => {
                self.application.set_autorip(false);
                false
            }
            _ => self.application.get_autorip(),
        };

        return Ok(tonic::Response::new(proto::AutoripStatusResponse {
            status: match status {
                true => proto::AutoripStatus::Enabled as _,
                false => proto::AutoripStatus::Disabled as _,
            },
        }));
    }

    /// Lists the currently-registered drives
    async fn list_drives(
        &self,
        _request: tonic::Request<proto::ListDrivesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListDrivesResponse>, tonic::Status> {
        let mut drives = Vec::<proto::DiscDrive>::new();
        for (controller_id, mut controller) in self
            .application
            .drive_controllers
            .iter()
            .map(|(id, controller)| (id.clone(), controller.clone()))
        {
            let drive_count = controller
                .get_drive_count(GetDriveCountRequest {})
                .await?
                .into_inner()
                .drive_count;
            for drive_id in 0..drive_count {
                let meta = controller
                    .get_drive_meta(GetDriveMetaRequest { drive_id })
                    .await?
                    .into_inner();
                drives.push(proto::DiscDrive {
                    controller: controller_id.clone(),
                    drive_id,
                    name: meta.name,
                });
            }
        }
        return Ok(tonic::Response::new(proto::ListDrivesResponse { drives }));
    }

    /// Starts a rip job
    async fn start_rip_job(
        &self,
        request: tonic::Request<proto::StartRipJobRequest>,
    ) -> std::result::Result<tonic::Response<proto::StartRipJobResponse>, tonic::Status> {
        let request = request.into_inner();
        let drive = request
            .drive
            .ok_or_else(|| tonic::Status::invalid_argument("Missing drive info"))?;
        let job_id = self
            .application
            .rip_media(
                &drive.controller,
                drive.drive_id,
                request.suspected_contents,
                request.autoeject,
            )
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::StartRipJobResponse { job_id }));
    }

    /// Gets the current status of a rip job
    async fn get_rip_job_status(
        &self,
        request: tonic::Request<proto::GetRipJobStatusRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetRipJobStatusResponse>, tonic::Status> {
        let request = request.into_inner();

        for mut controller in self
            .application
            .drive_controllers
            .iter()
            .map(|(_id, controller)| controller.clone())
        {
            if let Ok(status) = controller
                .get_job_status(GetJobStatusRequest {
                    job_id: request.job_id,
                })
                .await
            {
                return Ok(tonic::Response::new(proto::GetRipJobStatusResponse {
                    status: Some(status.into_inner()),
                }));
            }
        }

        return Err(tonic::Status::not_found("The requested job was not found."));
    }

    /// Server streaming response type for the StreamRipJobUpdates method.
    type StreamRipJobUpdatesStream = tonic::Streaming<RipUpdate>;

    /// Streams status updates from a rip job.
    /// Initial state is always `RipStatus::default()`.
    async fn stream_rip_job_updates(
        &self,
        request: tonic::Request<proto::StreamRipJobUpdatesRequest>,
    ) -> std::result::Result<tonic::Response<Self::StreamRipJobUpdatesStream>, tonic::Status> {
        let request = request.into_inner();

        for mut controller in self
            .application
            .drive_controllers
            .iter()
            .map(|(_id, controller)| controller.clone())
        {
            if let Ok(status) = controller
                .watch_rip_job(WatchRipJobRequest {
                    job_id: request.job_id,
                })
                .await
            {
                return Ok(tonic::Response::new(status.into_inner()));
            }
        }

        return Err(tonic::Status::not_found(
            "The requested rip job was not found",
        ));
    }

    /// Ejects a disc
    async fn eject(
        &self,
        request: tonic::Request<proto::EjectRequest>,
    ) -> std::result::Result<tonic::Response<proto::EjectResponse>, tonic::Status> {
        let request = request.into_inner();
        let drive = request
            .drive
            .ok_or_else(|| tonic::Status::invalid_argument("Missing drive ID"))?;
        let mut controller = self
            .application
            .drive_controllers
            .get(&drive.controller)
            .bubble()?
            .clone();
        controller
            .eject(EjectRequest {
                drive_id: drive.drive_id,
            })
            .await?;
        return Ok(tonic::Response::new(proto::EjectResponse {}));
    }

    /// Retracts a disc
    async fn retract(
        &self,
        request: tonic::Request<proto::RetractRequest>,
    ) -> std::result::Result<tonic::Response<proto::RetractResponse>, tonic::Status> {
        let request = request.into_inner();
        let drive = request
            .drive
            .ok_or_else(|| tonic::Status::invalid_argument("Missing drive ID"))?;
        let mut controller = self
            .application
            .drive_controllers
            .get(&drive.controller)
            .bubble()?
            .clone();
        controller
            .retract(RetractRequest {
                drive_id: drive.drive_id,
            })
            .await?;
        return Ok(tonic::Response::new(proto::RetractResponse {}));
    }

    /// Gets the current state of the drive
    async fn get_drive_state(
        &self,
        request: tonic::Request<proto::GetDriveStateRequest>,
    ) -> std::result::Result<tonic::Response<drive_controller::DriveState>, tonic::Status> {
        let request = request.into_inner();
        let mut controller = self
            .application
            .drive_controllers
            .get(&request.controller_id)
            .bubble()?
            .clone();
        return controller
            .get_drive_state(drive_controller::GetDriveStateRequest {
                drive_id: request.drive_id,
            })
            .await;
    }

    /// Lists the movies in the database
    async fn list_movies(
        &self,
        request: tonic::Request<proto::ListMoviesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListMoviesResponse>, tonic::Status> {
        let _request = request.into_inner();

        let movies = db::get_movies(&self.application.db).await.bubble()?;
        return Ok(tonic::Response::new(proto::ListMoviesResponse {
            movies: movies.into_iter().map(Into::into).collect(),
        }));
    }

    /// Gets a movie from the database by its TMDB ID
    async fn get_movie_by_tmdb_id(
        &self,
        request: tonic::Request<proto::GetMovieByTmdbIdRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetMovieByTmdbIdResponse>, tonic::Status> {
        let request = request.into_inner();

        let movie = db::get_movie_by_tmdb_id(&self.application.db, request.tmdb_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::GetMovieByTmdbIdResponse {
            movie: Some(movie.into()),
        }));
    }

    /// Lists the TV shows in the database
    async fn list_tv_shows(
        &self,
        request: tonic::Request<proto::ListTvShowsRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvShowsResponse>, tonic::Status> {
        let _request = request.into_inner();

        let movies = db::get_tv_shows(&self.application.db).await.bubble()?;
        return Ok(tonic::Response::new(proto::ListTvShowsResponse {
            tv_shows: movies.into_iter().map(Into::into).collect(),
        }));
    }

    /// Lists the seasons for a given TV show
    async fn list_tv_seasons(
        &self,
        request: tonic::Request<proto::ListTvSeasonsRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvSeasonsResponse>, tonic::Status> {
        let request = request.into_inner();

        let seasons = db::get_tv_seasons(&self.application.db, request.series_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::ListTvSeasonsResponse {
            series_id: request.series_id,
            tv_seasons: seasons.into_iter().map(Into::into).collect(),
        }));
    }

    /// Lists the episodes for a given season
    async fn list_tv_episodes(
        &self,
        request: tonic::Request<proto::ListTvEpisodesRequest>,
    ) -> std::result::Result<tonic::Response<proto::ListTvEpisodesResponse>, tonic::Status> {
        let request = request.into_inner();

        let episodes = db::get_tv_episodes(&self.application.db, request.tv_season_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::ListTvEpisodesResponse {
            tv_season_id: request.tv_season_id,
            tv_episodes: episodes.into_iter().map(Into::into).collect(),
        }));
    }

    /// Gets a particular TV episode
    async fn get_tv_episode(
        &self,
        request: tonic::Request<proto::GetTvEpisodeRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetTvEpisodeResponse>, tonic::Status> {
        let request = request.into_inner();

        let episode = db::get_tv_episode_by_id(&self.application.db, request.episode_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::GetTvEpisodeResponse {
            episode: Some(episode.into()),
        }));
    }

    /// Gets a particular TV episode by TMDB id
    async fn get_tv_episode_by_tmdb_id(
        &self,
        request: tonic::Request<proto::GetTvEpisodeByTmdbIdRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetTvEpisodeByTmdbIdResponse>, tonic::Status>
    {
        let request = request.into_inner();

        let episode = db::get_tv_episode_by_tmdb_id(&self.application.db, request.tmdb_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::GetTvEpisodeByTmdbIdResponse {
            episode: Some(episode.into()),
        }));
    }

    /// Tags a video file with metadata
    async fn tag_file(
        &self,
        request: tonic::Request<proto::TagFileRequest>,
    ) -> std::result::Result<tonic::Response<proto::TagFileResponse>, tonic::Status> {
        let request = request.into_inner();

        db::tag_video_file(
            &self.application.db,
            request.file,
            request.video_type().into(),
            request.match_id,
        )
        .await
        .bubble()?;

        return Ok(tonic::Response::new(proto::TagFileResponse {}));
    }

    /// Gets a particular job
    async fn get_job_info(
        &self,
        request: tonic::Request<proto::GetJobInfoRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetJobInfoResponse>, tonic::Status> {
        let request = request.into_inner();

        let rip_job = db::get_rip_job(&self.application.db, request.job_id)
            .await
            .bubble()?;
        return Ok(tonic::Response::new(proto::GetJobInfoResponse {
            details: Some(rip_job.into()),
        }));
    }

    /// Gets a list of jobs containing untagged files
    async fn get_untagged_jobs(
        &self,
        request: tonic::Request<proto::GetUntaggedJobsRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetUntaggedJobsResponse>, tonic::Status> {
        let request = request.into_inner();

        let rip_jobs = db::get_rip_jobs_with_untagged_videos(
            &self.application.db,
            request.skip,
            request.limit,
        )
        .await
        .bubble()?;
        return Ok(tonic::Response::new(proto::GetUntaggedJobsResponse {
            rip_jobs: rip_jobs.into_iter().map(Into::into).collect(),
        }));
    }

    /// Gets all info needed to catalog a job
    async fn get_job_catalogue_info(
        &self,
        request: tonic::Request<proto::GetJobCatalogueInfoRequest>,
    ) -> std::result::Result<tonic::Response<proto::GetJobCatalogueInfoResponse>, tonic::Status>
    {
        let request = request.into_inner();

        let job_info = db::get_rip_job(&self.application.db, request.job_id)
            .await
            .bubble()?;
        let video_files = db::get_videos_from_rip(&self.application.db, request.job_id)
            .await
            .bubble()?;
        let matches = db::get_matches_from_rip(&self.application.db, request.job_id)
            .await
            .bubble()?;
        let subtitle_maps = db::get_rip_video_blobs(&self.application.db, request.job_id)
            .await
            .bubble()?;
        let ost_subtitle_files =
            db::get_ost_subtitles_from_rip(&self.application.db, request.job_id)
                .await
                .bubble()?;

        return Ok(tonic::Response::new(proto::GetJobCatalogueInfoResponse {
            id: job_info.id.unwrap_or_default(),
            start_time: job_info.start_time,
            disc_title: job_info.disc_title,
            suspected_contents: job_info.suspected_contents.and_then(|contents| {
                proto::SuspectedContents::decode(std::io::Cursor::new(contents)).ok()
            }),
            video_files: video_files.into_iter().map(Into::into).collect(),
            matches: matches.into_iter().map(Into::into).collect(),
            subtitle_maps: subtitle_maps.into_iter().map(Into::into).collect(),
            ost_subtitle_files: ost_subtitle_files.into_iter().map(Into::into).collect(),
        }));
    }
}
