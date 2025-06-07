use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum SuspectedContents {
    Movie { tmdb_id: i32 },
    TvEpisodes { episode_tmdb_ids: Vec<i32> },
}

// Movie Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct MoviesItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub release_year: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct MoviesSpecialFeaturesItem {
    pub id: Option<i64>,
    pub movie_id: i64,
    pub thumbnail_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

// TV Show Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct TvShowsItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub original_release_year: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct TvSeasonsItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub tv_show_id: i64,
    pub season_number: u16,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct TvEpisodesItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub tv_show_id: i64,
    pub tv_season_id: i64,
    pub episode_number: u16,
    pub thumbnail_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct RipJobsItem {
    pub id: Option<i64>,
    pub start_time: i64,
    pub disc_title: Option<String>,
    pub suspected_contents: Option<String>,
    pub rip_finished: bool,
    pub imported: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, sqlx::Type)]
#[repr(i32)]
pub enum VideoType {
    Untagged = 0,
    Movie = 1,
    SpecialFeature = 2,
    TvEpisode = 3,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct VideoFilesItem {
    pub id: Option<i64>,
    ///  Video type:
    ///  0 => Untagged
    ///  1 => Movie
    ///  2 => Special Feature
    ///  3 => TV Episode
    pub video_type: VideoType,
    ///  Match ID: Identifies the specific movie, special feature, etc this video contains.
    pub match_id: Option<i64>,
    pub blob_id: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub length: u32,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub original_video_hash: Vec<u8>,
    pub rip_job: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct SubtitleFilesItem {
    pub id: Option<i64>,
    pub blob_id: String,
    pub video_file: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct OstDownloadsItem {
    pub id: Option<i64>,
    pub video_type: VideoType,
    pub match_id: i64,
    pub filename: String,
    pub blob_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct MatchInfoItem {
    pub id: Option<i64>,
    pub video_file_id: i64,
    pub ost_download_id: i64,
    pub distance: u32,
    pub max_distance: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct ImageFilesItem {
    pub id: Option<i64>,
    pub blob_id: String,
    pub mime_type: String,
    pub name: Option<String>,
    pub rip_job: Option<i64>,
}
