use mediacorral_proto::mediacorral::server::v1::{self as proto, SuspectedContents};
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sqlx::prelude::FromRow;

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
impl Into<proto::Movie> for MoviesItem {
    fn into(self) -> proto::Movie {
        return proto::Movie {
            id: self.id.unwrap_or_default(),
            tmdb_id: self.tmdb_id,
            poster_blob: self.poster_blob,
            title: self.title,
            release_year: self.release_year,
            description: self.description,
        };
    }
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
impl Into<proto::TvShow> for TvShowsItem {
    fn into(self) -> proto::TvShow {
        return proto::TvShow {
            id: self.id.unwrap_or_default(),
            tmdb_id: self.tmdb_id,
            poster_blob: self.poster_blob,
            title: self.title,
            original_release_year: self.original_release_year,
            description: self.description,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct TvSeasonsItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub tv_show_id: i64,
    pub season_number: u32,
    pub poster_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}
impl Into<proto::TvSeason> for TvSeasonsItem {
    fn into(self) -> proto::TvSeason {
        return proto::TvSeason {
            id: self.id.unwrap_or_default(),
            tmdb_id: self.tmdb_id,
            tv_show_id: self.tv_show_id,
            season_number: self.season_number,
            poster_blob: self.poster_blob,
            title: self.title,
            description: self.description,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct TvEpisodesItem {
    pub id: Option<i64>,
    pub tmdb_id: Option<i32>,
    pub tv_show_id: i64,
    pub tv_season_id: i64,
    pub episode_number: u32,
    pub thumbnail_blob: Option<i64>,
    pub title: String,
    pub description: Option<String>,
}
impl Into<proto::TvEpisode> for TvEpisodesItem {
    fn into(self) -> proto::TvEpisode {
        return proto::TvEpisode {
            id: self.id.unwrap_or_default(),
            tmdb_id: self.tmdb_id,
            tv_show_id: self.tv_show_id,
            tv_season_id: self.tv_season_id,
            episode_number: self.episode_number,
            thumbnail_blob: self.thumbnail_blob,
            title: self.title,
            description: self.description,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct RipJobsItem {
    pub id: Option<i64>,
    pub start_time: i64,
    pub disc_title: Option<String>,
    pub suspected_contents: Option<Vec<u8>>,
    pub rip_finished: bool,
    pub imported: bool,
}
impl Into<proto::RipJob> for RipJobsItem {
    fn into(self) -> proto::RipJob {
        return proto::RipJob {
            id: self.id.unwrap_or_default(),
            start_time: self.start_time,
            disc_title: self.disc_title,
            suspected_contents: self.suspected_contents.and_then(|contents| {
                SuspectedContents::decode(std::io::Cursor::new(contents)).ok()
            }),
            rip_finished: self.rip_finished,
            imported: self.imported,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, sqlx::Type)]
#[repr(i32)]
pub enum VideoType {
    Untagged = 0,
    Movie = 1,
    SpecialFeature = 2,
    TvEpisode = 3,
}
impl Into<proto::VideoType> for VideoType {
    fn into(self) -> proto::VideoType {
        match self {
            Self::Untagged => proto::VideoType::Unspecified,
            Self::Movie => proto::VideoType::Movie,
            Self::SpecialFeature => proto::VideoType::SpecialFeature,
            Self::TvEpisode => proto::VideoType::TvEpisode,
        }
    }
}
impl From<proto::VideoType> for VideoType {
    fn from(value: proto::VideoType) -> Self {
        match value {
            proto::VideoType::Unspecified => Self::Untagged,
            proto::VideoType::Movie => Self::Movie,
            proto::VideoType::SpecialFeature => Self::SpecialFeature,
            proto::VideoType::TvEpisode => Self::TvEpisode,
        }
    }
}

#[serde_as]
#[derive(Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
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
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub length: Option<u32>,
    pub original_video_hash: Option<Vec<u8>>,
    pub rip_job: Option<i64>,
}
impl Into<proto::VideoFile> for VideoFilesItem {
    fn into(self) -> proto::VideoFile {
        return proto::VideoFile {
            id: self.id.unwrap_or_default(),
            video_type: Into::<proto::VideoType>::into(self.video_type).into(),
            match_id: self.match_id,
            blob_id: self.blob_id,
            resolution_width: self.resolution_width,
            resolution_height: self.resolution_height,
            length: self.length,
            original_video_hash: self.original_video_hash,
            rip_job: self.rip_job,
        };
    }
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
impl Into<proto::OstDownloadsItem> for OstDownloadsItem {
    fn into(self) -> proto::OstDownloadsItem {
        return proto::OstDownloadsItem {
            id: self.id.unwrap_or_default(),
            video_type: Into::<proto::VideoType>::into(self.video_type) as _,
            match_id: self.match_id,
            filename: self.filename,
            blob_id: self.blob_id,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct MatchInfoItem {
    pub id: Option<i64>,
    pub video_file_id: i64,
    pub ost_download_id: i64,
    pub distance: u32,
    pub max_distance: u32,
}
impl Into<proto::MatchInfoItem> for MatchInfoItem {
    fn into(self) -> proto::MatchInfoItem {
        return proto::MatchInfoItem {
            id: self.id.unwrap_or_default(),
            video_file_id: self.video_file_id,
            ost_download_id: self.ost_download_id,
            distance: self.distance,
            max_distance: self.max_distance,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, FromRow)]
pub struct ImageFilesItem {
    pub id: Option<i64>,
    pub blob_id: String,
    pub mime_type: String,
    pub name: Option<String>,
    pub rip_job: Option<i64>,
}
