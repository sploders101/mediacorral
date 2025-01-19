use serde::{Deserialize, Serialize};

// Movie Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesItem {
    pub id: i64,
    pub tmdb_id: Option<String>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesSpecialFeaturesItem {
    pub id: i64,
    pub movie_id: i64,
    pub thumbnail_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

// TV Show Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvShowsItem {
    pub id: i64,
    pub tmdb_id: Option<String>,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvSeasonsItem {
    pub id: i64,
    pub tv_show_id: i64,
    pub season_number: i64,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvEpisodesItem {
    pub id: i64,
    pub tv_show_id: i64,
    pub tv_season_id: i64,
    pub episode_number: i64,
    pub thumbnail_blob: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
}

// File tags (Movie, Special features, TV Episode)

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MovieFilesItem {
    pub blob_id: i64,
    pub movie_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MovieSpecialFeaturesFilesItem {
    pub blob_id: i64,
    pub movie_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvFilesItem {
    pub blob_id: i64,
    pub tv_show_id: i64,
    pub tv_season_id: i64,
    pub tv_episode_id: i64,
}

// Untagged Media

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UntaggedMediaItem {
    pub blob_id: i64,
    pub subtitle_id: Option<i64>,
}

// Video File Stream Info

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct VideoMetadataItem {
    pub blob_id: i64,
    pub resolution: String,
    pub resolution_width: i64,
    pub resolution_height: i64,
    pub video_format: String,
    pub length: i64,
    pub audio_hash: Vec<u8>,
}

// Subtitle file info

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SubtitleMetadataItem {
    pub blob_id: i64,
    pub video_blob_id: i64,
    pub language: Option<String>,
}

// File References

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BlobItem {
    pub id: i64,
    pub creation_time: i64,
    pub mime_type: Option<String>,
    pub hash: Vec<u8>,
    pub filename: String,
}
