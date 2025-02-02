use serde::{Deserialize, Serialize};

/// Search result container
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbQueryResults<T> {
    pub page: usize,
    pub total_pages: usize,
    pub total_results: usize,
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

/// Describes a genre from a lookup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbGenre {
    pub id: i32,
    pub name: String,
}

/// Describes a production company
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbProductionCompany {
    pub id: i32,
    pub logo_path: Option<String>,
    pub name: String,
    pub origin_country: String,
}

/// Describes a country the content was produced in
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbProductionCountry {
    pub iso_3166_1: String,
    pub name: String,
}

/// Describes a spoken language
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbSpokenLanguage {
    pub english_name: String,
    pub iso_639_1: String,
    pub name: String,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvCreator {
    pub id: i32,
    pub credit_id: String,
    pub name: String,
    pub gender: u8,
    pub profile_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvNetwork {
    pub id: i32,
    pub logo_path: Option<String>,
    pub name: String,
    pub origin_country: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeason {
    pub air_date: String,
    pub episode_count: u32,
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub season_number: u16,
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
    pub number_of_episodes: Option<u32>,
    pub number_of_seasons: Option<u16>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeasonDetails {
    pub air_date: Option<String>,
    pub episodes: Vec<TmdbTvEpisodeDetails>,
    pub name: String,
    pub overview: Option<String>,
    pub id: i32,
    pub poster_path: Option<String>,
    pub season_number: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbCrewMember {
    pub department: Option<String>,
    pub job: Option<String>,
    pub credit_id: Option<String>,
    pub id: i32,
    pub known_for_department: Option<String>,
    pub name: String,
    pub original_name: Option<String>,
    pub profile_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvEpisodeDetails {
    pub air_date: Option<String>,
    pub episode_number: u16,
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub runtime: Option<u32>,
    pub season_number: u16,
    pub show_id: i32,
    pub still_path: Option<String>,
    pub crew: Option<Vec<TmdbCrewMember>>,
    pub guest_stars: Option<Vec<TmdbCrewMember>>,
}
