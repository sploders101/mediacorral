use serde::{Deserialize, Serialize};

// Movie Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesItem {
    id: usize,
    tmdb_id: Option<String>,
    poster_blob: Option<usize>,
    title: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MoviesSpecialFeaturesItem {
    id: usize,
    movie_id: usize,
    thumbnail_blob: Option<usize>,
    title: String,
    description: Option<String>,
}

// TV Show Metadata

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvShowsItem {
    id: usize,
    tmdb_id: Option<String>,
    poster_blob: Option<usize>,
    title: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvSeasonsItem {
    id: usize,
    tv_show_id: usize,
    season_number: usize,
    poster_blob: Option<usize>,
    title: String,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvEpisodesItem {
    id: usize,
    tv_show_id: usize,
    tv_season_id: usize,
    episode_number: usize,
    thumbnail_blob: Option<usize>,
    title: Option<String>,
    description: Option<String>,
}

// File tags (Movie, Special features, TV Episode)

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MovieFilesItem {
    blob_id: usize,
    movie_id: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MovieSpecialFeaturesFilesItem {
    blob_id: usize,
    movie_id: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct TvFilesItem {
    blob_id: usize,
    tv_show_id: usize,
    tv_season_id: usize,
    tv_episode_id: usize,
}

// Untagged Media

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UntaggedMediaItem {
    blob_id: usize,
    subtitle_id: Option<usize>,
}

// Video File Stream Info

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct VideoMetadataItem {
    blob_id: usize,
    resolution: String,
    resolution_width: usize,
    resolution_height: usize,
    video_format: String,
    length: usize,
    audio_hash: Vec<u8>,
}

// Subtitle file info

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct SubtitleMetadataItem {
    blob_id: usize,
    video_blob_id: usize,
    language: Option<String>,
}

// File References

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BlobItem {
    id: usize,
    creation_time: usize,
    mime_type: Option<String>,
    hash: Vec<u8>,
    filename: String,
}
