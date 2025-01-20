use serde::{Deserialize, Serialize};

// Movie Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<String>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesSpecialFeaturesItem {
    pub id: Option<i64>,
    pub movie_id: i64,
    pub thumbnail_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

// TV Show Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvShowsItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<String>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvSeasonsItem {
    pub id: Option<i64>,
    pub tv_show_id: i64,
    pub season_number: i64,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvEpisodesItem {
    pub id: Option<i64>,
    pub tv_show_id: i64,
    pub tv_season_id: i64,
    pub episode_number: i64,
    pub thumbnail_blob: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
}

pub struct RipJobsItem {
	pub id: Option<i64>,
	pub start_time: i64,
	pub disc_title: Option<String>,
	pub suspected_contents: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum VideoType {
    Untagged = 0,
    Movie = 1,
    SpecialFeature = 2,
    TvEpisode = 3,
}
impl VideoType {
    pub fn to_db(self) -> i64 {
        return match self {
            Self::Untagged => 0,
            Self::Movie => 1,
            Self::SpecialFeature => 2,
            Self::TvEpisode => 3,
        };
    }
    pub fn from_db(int: i64) -> Self {
        return match int {
            0 => Self::Untagged,
            1 => Self::Movie,
            2 => Self::SpecialFeature,
            3 => Self::TvEpisode,
            _ => Self::Untagged,
        };
    }
}

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
	pub original_mkv_hash: [u8; 16],
	pub rip_job: Option<i64>,
}

pub struct SubtitleFilesItem {
	pub id: Option<i64>,
	pub blob_id: String,
	pub video_file: i64,
}

pub struct ImageFilesItem {
	pub id: Option<i64>,
	pub blob_id: String,
	pub mime_type: String,
	pub name: Option<String>,
	pub rip_job: Option<i64>,
}
