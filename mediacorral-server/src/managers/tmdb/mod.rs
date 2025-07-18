use std::{borrow::Cow, sync::Arc};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use types::{
    TmdbAnyTitle, TmdbMovieDetails, TmdbMovieResult, TmdbQueryResults, TmdbTvResult,
    TmdbTvSeasonDetails, TmdbTvSeriesDetails,
};

use crate::{
    blob_storage::{BlobError, BlobStorageController},
    db::{
        self,
        schemas::{MoviesItem, TvShowsItem},
    },
};

mod types;

static USER_AGENT: &'static str = concat!("mediacorral@", env!("CARGO_PKG_VERSION"));

// TODO: Fetch this from the configuration endpoint
static IMAGE_BASE: &'static str = "https://image.tmdb.org/t/p/original";

#[derive(Error, Debug)]
pub enum TmdbError {
    #[error("An error occurred when performing the request:\n{0:?}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("An error occurred when parsing {0}:\n{1:?}")]
    DeserializeError(Cow<'static, str>, serde_json::Error),
    #[error("Invalid authorization token")]
    InvalidAuthToken,
    #[error("An blob error occurred:\n{0}")]
    Blob(#[from] BlobError),
    #[error("Unknown image format")]
    UnknownImageFormat,
    #[error("The requested content is missing a required field: {0}")]
    MissingField(&'static str),
    #[error("A database error occurred:\n{0}")]
    Db(#[from] sqlx::Error),
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
}
fn json_err(name: &'static str) -> impl FnOnce(serde_json::Error) -> TmdbError {
    return move |err| TmdbError::DeserializeError(name.into(), err);
}

pub type TmdbResult<T> = Result<T, TmdbError>;

pub struct TmdbImporter {
    db: Arc<SqlitePool>,
    agent: reqwest::Client,
}
impl TmdbImporter {
    pub fn new(db: Arc<SqlitePool>, api_key: String) -> TmdbResult<Self> {
        let agent = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(HeaderMap::from_iter(
                [(
                    HeaderName::from_static("authorization"),
                    HeaderValue::from_str(&format!("Bearer {api_key}"))
                        .map_err(|_| TmdbError::InvalidAuthToken)?,
                )]
                .into_iter(),
            ))
            .build()?;

        return Ok(Self { db, agent });
    }

    pub async fn query_any(
        &self,
        query: &str,
        language: Option<&str>,
        page: u32,
    ) -> TmdbResult<TmdbQueryResults<TmdbAnyTitle>> {
        // TODO: Configure a BASE_URL variable and use that
        let response = self
            .agent
            .get("https://api.themoviedb.org/3/search/multi")
            .query(&[
                ("query", Some(query)),
                ("language", language),
                ("page", Some(page.to_string().as_str())),
            ])
            .send()
            .await?
            .error_for_status()?;
        return serde_json::from_slice(&response.bytes().await?).map_err(json_err("multi search"));
    }

    pub async fn query_movies(
        &self,
        query: &str,
        language: Option<&str>,
        primary_release_year: Option<&str>,
        region: Option<&str>,
        year: Option<&str>,
        page: u32,
    ) -> TmdbResult<TmdbQueryResults<TmdbMovieResult>> {
        let response = self
            .agent
            .get("https://api.themoviedb.org/3/search/movie")
            .query(&[
                ("query", Some(query)),
                ("language", language),
                ("primary_release_year", primary_release_year),
                ("region", region),
                ("year", year),
                ("page", Some(page.to_string().as_str())),
            ])
            .send()
            .await?
            .error_for_status()?;
        return serde_json::from_slice(&response.bytes().await?).map_err(json_err("movie search"));
    }

    pub async fn query_tv(
        &self,
        query: &str,
        first_air_date_year: Option<&str>,
        language: Option<&str>,
        year: Option<&str>,
        page: u32,
    ) -> TmdbResult<TmdbQueryResults<TmdbTvResult>> {
        let response = self
            .agent
            .get("https://api.themoviedb.org/3/search/tv")
            .query(&[
                ("query", Some(query)),
                ("first_air_date_year", first_air_date_year),
                ("language", language),
                ("year", year),
                ("page", Some(page.to_string().as_str())),
            ])
            .send()
            .await?
            .error_for_status()?;
        return serde_json::from_slice(&response.bytes().await?).map_err(json_err("tv search"));
    }

    async fn get_poster(
        &self,
        poster_path: String,
        blob_storage: &BlobStorageController,
    ) -> TmdbResult<i64> {
        let mut response = self
            .agent
            .get(format!("{IMAGE_BASE}/{poster_path}"))
            .send()
            .await?;
        let mime_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .ok_or(TmdbError::UnknownImageFormat)?;
        let (id, mut file) = blob_storage
            .add_image(
                Some(String::from(poster_path.trim_start_matches('/'))),
                String::from(mime_type),
            )
            .await?;
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
        }

        return Ok(id);
    }

    /// Imports movie metadata for the given ID into the local database.
    pub async fn import_movie(
        &self,
        movie_id: i32,
        blob_storage: Option<&BlobStorageController>,
    ) -> TmdbResult<i64> {
        let response: TmdbMovieDetails = serde_json::from_slice(
            &self
                .agent
                .get(format!("https://api.themoviedb.org/3/movie/{movie_id}"))
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?,
        )
        .map_err(json_err("movie query"))?;

        let poster_blob = match blob_storage.zip(response.poster_path) {
            Some((blob_storage, poster_path)) => {
                Some(self.get_poster(poster_path, blob_storage).await?)
            }
            _ => None,
        };

        let title = response
            .title
            .or(response.name)
            .ok_or(TmdbError::MissingField("name"))?;
        let movie_id = db::insert_tmdb_movie(
            &self.db,
            &MoviesItem {
                id: None,
                tmdb_id: Some(movie_id),
                poster_blob,
                title,
                release_year: response
                    .release_date
                    .and_then(|item| item.split('-').next().map(String::from)),
                description: response.overview,
                runtime: response.runtime,
            },
        )
        .await?;

        return Ok(movie_id);
    }

    /// Imports TV metadata for the given ID into the local database.
    ///
    /// This function recurses into shows and episodes to get all data for the entire show.
    pub async fn import_tv(
        &self,
        tv_id: i32,
        blob_storage: Option<&BlobStorageController>,
    ) -> TmdbResult<i64> {
        let response: TmdbTvSeriesDetails = serde_json::from_slice(
            &self
                .agent
                .get(format!("https://api.themoviedb.org/3/tv/{tv_id}"))
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?,
        )
        .map_err(json_err("tv series query"))?;

        let poster_blob = match blob_storage.zip(response.poster_path) {
            Some((blob_storage, poster_path)) => {
                Some(self.get_poster(poster_path, blob_storage).await?)
            }
            _ => None,
        };

        // Loop over seasons and postpone database interaction until we have all the information
        // in case there's rate-limiting, since I'm not confident in the upsert functionality yet.
        let mut season_details_list = Vec::new();
        for season in response.seasons {
            let season_response: TmdbTvSeasonDetails = serde_json::from_slice(
                &self
                    .agent
                    .get(format!(
                        "https://api.themoviedb.org/3/tv/{}/season/{}",
                        tv_id, season.season_number
                    ))
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?,
            )
            .map_err(|err| {
                TmdbError::DeserializeError(
                    Cow::Owned(format!("series {tv_id}, season {}", season.season_number)),
                    err,
                )
            })?;
            season_details_list.push(season_response);
        }

        let series_id = db::insert_tmdb_tv_show(
            &self.db,
            &TvShowsItem {
                id: None,
                tmdb_id: Some(tv_id),
                poster_blob,
                title: response.name,
                original_release_year: response
                    .first_air_date
                    .and_then(|item| item.split('-').next().map(String::from)),
                description: response.overview,
            },
        )
        .await?;

        for season_details in season_details_list {
            let poster_blob = match blob_storage.zip(season_details.poster_path) {
                Some((blob_storage, poster_path)) => {
                    Some(self.get_poster(poster_path, blob_storage).await?)
                }
                _ => None,
            };
            let season_id = db::upsert_tv_season(
                &self.db,
                &db::schemas::TvSeasonsItem {
                    id: None,
                    tmdb_id: Some(season_details.id),
                    tv_show_id: series_id,
                    season_number: season_details.season_number,
                    poster_blob,
                    title: season_details.name,
                    description: season_details.overview,
                },
            )
            .await?;

            for episode in season_details.episodes {
                let thumbnail_blob = match blob_storage.zip(episode.still_path) {
                    Some((blob_storage, still_path)) => {
                        Some(self.get_poster(still_path, blob_storage).await?)
                    }
                    _ => None,
                };
                let _episode_id = db::upsert_tv_episode(
                    &self.db,
                    &db::schemas::TvEpisodesItem {
                        id: None,
                        tmdb_id: Some(episode.id),
                        tv_show_id: series_id,
                        tv_season_id: season_id,
                        episode_number: episode.episode_number,
                        thumbnail_blob,
                        title: episode.name,
                        description: episode.overview,
                        runtime: episode.runtime,
                    },
                )
                .await?;
            }
        }

        return Ok(series_id);
    }
}
