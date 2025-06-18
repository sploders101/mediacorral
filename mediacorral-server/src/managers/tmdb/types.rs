use mediacorral_proto::mediacorral::common::tmdb::v1 as pb_types;
use serde::{Deserialize, Serialize};

/// Search result container
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbQueryResults<T> {
    pub page: u32,
    pub total_pages: u32,
    pub total_results: u32,
    pub results: Vec<T>,
}

/// Search result for a movie or TV show
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbAnyTitle {
    pub backdrop_path: Option<String>,
    pub id: i32,
    pub name: Option<String>,
    pub title: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub media_type: String,
}
impl Into<pb_types::TmdbAnyTitle> for TmdbAnyTitle {
    fn into(self) -> pb_types::TmdbAnyTitle {
        return pb_types::TmdbAnyTitle {
            id: self.id,
            title: self.title,
            backdrop_path: self.backdrop_path,
            poster_path: self.poster_path,
            overview: self.overview,
        };
    }
}

/// Search result for a movie
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbMovieResult {
    pub id: i32,
    pub original_language: Option<String>,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub video: bool,
}
impl Into<pb_types::TmdbMovieResult> for TmdbMovieResult {
    fn into(self) -> pb_types::TmdbMovieResult {
        return pb_types::TmdbMovieResult {
            id: self.id,
            title: self.title.or(self.name),
            release_date: self.release_date,
            original_language: self.original_language,
            overview: self.overview,
            poster_path: self.poster_path,
        };
    }
}

/// Search result for a TV show
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvResult {
    pub id: i32,
    pub origin_country: Option<Vec<String>>,
    pub original_language: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub first_air_date: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
}
impl Into<pb_types::TmdbTvResult> for TmdbTvResult {
    fn into(self) -> pb_types::TmdbTvResult {
        return pb_types::TmdbTvResult {
            id: self.id,
            name: self.name.or(self.title).or(self.original_name),
            origin_country: self.origin_country.unwrap_or_default(),
            original_language: self.original_language,
            overview: self.overview,
            poster_path: self.poster_path,
            first_air_date: self.first_air_date,
        };
    }
}

/// Describes a genre from a lookup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbGenre {
    pub id: i32,
    pub name: String,
}
impl Into<pb_types::TmdbGenre> for TmdbGenre {
    fn into(self) -> pb_types::TmdbGenre {
        return pb_types::TmdbGenre {
            id: self.id,
            name: self.name,
        };
    }
}

/// Describes a production company
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbProductionCompany {
    pub id: i32,
    pub logo_path: Option<String>,
    pub name: String,
    pub origin_country: Option<String>,
}
impl Into<pb_types::TmdbProductionCompany> for TmdbProductionCompany {
    fn into(self) -> pb_types::TmdbProductionCompany {
        return pb_types::TmdbProductionCompany {
            id: self.id,
            name: self.name,
            logo_path: self.logo_path,
            origin_country: self.origin_country,
        };
    }
}

/// Describes a country the content was produced in
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbProductionCountry {
    pub iso_3166_1: String,
    pub name: String,
}
impl Into<pb_types::TmdbProductionCountry> for TmdbProductionCountry {
    fn into(self) -> pb_types::TmdbProductionCountry {
        return pb_types::TmdbProductionCountry {
            iso_3166_1: self.iso_3166_1,
            name: self.name,
        };
    }
}

/// Describes a spoken language
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbSpokenLanguage {
    pub english_name: String,
    pub iso_639_1: String,
    pub name: String,
}
impl Into<pb_types::TmdbSpokenLanguage> for TmdbSpokenLanguage {
    fn into(self) -> pb_types::TmdbSpokenLanguage {
        return pb_types::TmdbSpokenLanguage {
            iso_639_1: self.iso_639_1,
            english_name: self.english_name,
            name: self.name,
        };
    }
}

/// Movie details from ID lookup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbMovieDetails {
    pub genres: Option<Vec<TmdbGenre>>,
    pub id: i32,
    pub imdb_id: u32,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: u32,
    pub title: Option<String>,
    pub name: Option<String>,
}
impl Into<pb_types::TmdbMovieDetails> for TmdbMovieDetails {
    fn into(self) -> pb_types::TmdbMovieDetails {
        return pb_types::TmdbMovieDetails {
            id: self.id,
            imdb_id: self.imdb_id,
            title: self.title,
            genres: self
                .genres
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            poster_path: self.poster_path,
            release_date: self.release_date,
            overview: self.overview,
            runtime: self.runtime,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvCreator {
    pub id: i32,
    pub credit_id: String,
    pub name: String,
    pub gender: u8,
    pub profile_path: Option<String>,
}
impl Into<pb_types::TmdbTvCreator> for TmdbTvCreator {
    fn into(self) -> pb_types::TmdbTvCreator {
        return pb_types::TmdbTvCreator {
            id: self.id,
            credit_id: self.credit_id,
            name: self.name,
            profile_path: self.profile_path,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvNetwork {
    pub id: i32,
    pub logo_path: Option<String>,
    pub name: String,
    pub origin_country: Option<String>,
}
impl Into<pb_types::TmdbTvNetwork> for TmdbTvNetwork {
    fn into(self) -> pb_types::TmdbTvNetwork {
        return pb_types::TmdbTvNetwork {
            id: self.id,
            logo_path: self.logo_path,
            name: self.name,
            origin_country: self.origin_country,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeason {
    pub air_date: Option<String>,
    pub episode_count: u32,
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub season_number: u16,
}
impl Into<pb_types::TmdbTvSeason> for TmdbTvSeason {
    fn into(self) -> pb_types::TmdbTvSeason {
        return pb_types::TmdbTvSeason {
            id: self.id,
            season_number: self.season_number.into(),
            name: self.name,
            air_date: self.air_date,
            poster_path: self.poster_path,
            episode_count: self.episode_count,
            overview: self.overview,
        };
    }
}

/// TV Series details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeriesDetails {
    pub created_by: Vec<TmdbTvCreator>,
    pub first_air_date: Option<String>,
    pub genres: Option<Vec<TmdbGenre>>,
    pub id: i32,
    pub in_production: bool,
    pub languages: Option<Vec<String>>,
    pub name: String,
    pub networks: Vec<TmdbTvNetwork>,
    pub number_of_episodes: u32,
    pub number_of_seasons: u32,
    pub origin_country: Option<Vec<String>>,
    pub original_language: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub production_companies: Option<Vec<TmdbProductionCompany>>,
    pub production_countries: Option<Vec<TmdbProductionCountry>>,
    pub seasons: Vec<TmdbTvSeason>,
    pub spoken_languages: Option<Vec<TmdbSpokenLanguage>>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub tv_type: Option<String>,
}
impl Into<pb_types::TmdbTvSeriesDetails> for TmdbTvSeriesDetails {
    fn into(self) -> pb_types::TmdbTvSeriesDetails {
        return pb_types::TmdbTvSeriesDetails {
            id: self.id,
            name: self.name,
            poster_path: self.poster_path,
            r#type: self.tv_type,
            status: self.status,
            first_air_date: self.first_air_date,
            number_of_episodes: self.number_of_episodes,
            number_of_seasons: self.number_of_seasons,
            created_by: self.created_by.into_iter().map(Into::into).collect(),
            genres: self
                .genres
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            in_production: self.in_production,
            languages: self
                .languages
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            networks: self.networks.into_iter().map(Into::into).collect(),
            origin_country: self
                .origin_country
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            original_language: self.original_language,
            overview: self.overview,
            production_companies: self
                .production_companies
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            production_countries: self
                .production_countries
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            spoken_languages: self
                .spoken_languages
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            seasons: self.seasons.into_iter().map(Into::into).collect(),
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeasonDetails {
    pub air_date: Option<String>,
    pub episodes: Vec<TmdbTvEpisodeDetails>,
    pub name: String,
    pub overview: Option<String>,
    pub id: i32,
    pub poster_path: Option<String>,
    pub season_number: u32,
}
impl Into<pb_types::TmdbTvSeasonDetails> for TmdbTvSeasonDetails {
    fn into(self) -> pb_types::TmdbTvSeasonDetails {
        return pb_types::TmdbTvSeasonDetails {
            id: self.id,
            air_date: self.air_date,
            episodes: self.episodes.into_iter().map(Into::into).collect(),
            name: self.name,
            overview: self.overview,
            poster_path: self.poster_path,
            season_number: self.season_number,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvEpisodeDetails {
    pub air_date: Option<String>,
    pub episode_number: u32,
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub runtime: Option<u32>,
    pub season_number: u32,
    pub show_id: i32,
    pub still_path: Option<String>,
}
impl Into<pb_types::TmdbTvEpisodeDetails> for TmdbTvEpisodeDetails {
    fn into(self) -> pb_types::TmdbTvEpisodeDetails {
        return pb_types::TmdbTvEpisodeDetails {
            air_date: self.air_date,
            episode_number: self.episode_number,
            id: self.id,
            name: self.name,
            overview: self.overview,
            runtime: self.runtime,
            season_number: self.season_number,
            show_id: self.show_id,
            still_path: self.still_path,
        };
    }
}
