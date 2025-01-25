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
    pub backdrop_path: String,
    pub id: i32,
    pub title: String,
    pub original_language: String,
    pub original_title: String,
    pub overview: String,
    pub poster_path: String,
    pub media_type: String,
    pub genre_ids: Vec<u32>,
    pub popularity: f32,
    pub release_date: String,
    pub video: bool,
    pub vote_average: f32,
    pub vote_count: u32,
}

/// Search result for a movie
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbMovieResult {
    pub backdrop_path: String,
    pub genre_ids: Vec<u32>,
    pub id: i32,
    pub original_language: String,
    pub original_title: String,
    pub overview: String,
    pub popularity: f32,
    pub poster_path: String,
    pub release_date: String,
    pub title: String,
    pub video: bool,
    pub vote_average: f32,
    pub vote_count: u32,
}

/// Search result for a TV show
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvResult {
    pub backdrop_path: String,
    pub genre_ids: Vec<u32>,
    pub id: i32,
    pub origin_country: Vec<String>,
    pub original_language: String,
    pub original_name: String,
    pub overview: String,
    pub popularity: f32,
    pub poster_path: String,
    pub first_air_date: String,
    pub name: String,
    pub vote_average: f32,
    pub vote_count: u32,
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
    pub logo_path: String,
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
    pub backdrop_path: String,
    pub belongs_to_collection: String,
    pub budget: u32,
    pub genres: Vec<TmdbGenre>,
    pub homepage: String,
    pub id: i32,
    pub imdb_id: u32,
    pub original_language: String,
    pub original_title: String,
    pub overview: String,
    pub popularity: String,
    pub poster_path: String,
    pub production_companies: Vec<TmdbProductionCompany>,
    pub production_countries: Vec<TmdbProductionCountry>,
    pub release_date: String,
    pub revenue: i64,
    pub runtime: u32,
    pub spoken_languages: Vec<TmdbSpokenLanguage>,
    pub status: String,
    pub tagline: String,
    pub title: String,
    pub video: bool,
    pub vote_average: f32,
    pub vote_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbLastEpisode {
    pub id: i32,
    pub name: String,
    pub overview: String,
    pub vote_average: f32,
    pub vote_count: u32,
    pub air_date: String,
    pub episode_number: u32,
    pub production_code: String,
    pub runtime: u32,
    pub season_number: u16,
    pub show_id: i32,
    pub still_path: String,
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
    pub logo_path: String,
    pub name: String,
    pub origin_country: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeason {
    pub air_date: String,
    pub episode_count: u32,
    pub id: i32,
    pub name: String,
    pub overview: String,
    pub poster_path: String,
    pub season_number: u16,
    pub vote_average: f32,
}

/// TV Series details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeriesDetails {
    pub backdrop_path: String,
    pub created_by: Vec<TmdbTvCreator>,
    pub episode_run_time: Vec<u32>,
    pub first_air_date: String,
    pub genres: Vec<TmdbGenre>,
    pub homepage: String,
    pub id: i32,
    pub in_production: bool,
    pub languages: Vec<String>,
    pub last_air_date: String,
    pub last_episode_to_air: Option<TmdbLastEpisode>,
    pub name: String,
    pub next_episode_to_air: String,
    pub networks: Vec<TmdbTvNetwork>,
    pub number_of_episodes: u32,
    pub number_of_seasons: u16,
    pub origin_country: Vec<String>,
    pub original_language: String,
    pub original_name: String,
    pub overview: String,
    pub popularity: f32,
    pub poster_path: String,
    pub production_companies: Vec<TmdbProductionCompany>,
    pub production_countries: Vec<TmdbProductionCountry>,
    pub seasons: Vec<TmdbTvSeason>,
    pub spoken_languages: Vec<TmdbSpokenLanguage>,
    pub status: String,
    pub tagline: String,
    #[serde(rename = "type")]
    pub tv_type: String,
    pub vote_average: f32,
    pub vote_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvSeasonDetails {
    pub air_date: String,
    pub episodes: Vec<TmdbTvEpisodeDetails>,
    pub name: String,
    pub overview: String,
    pub id: i32,
    pub poster_path: String,
    pub season_number: u16,
    pub vote_average: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbCrewMember {
    pub department: String,
    pub job: String,
    pub credit_id: String,
    pub adult: bool,
    pub gender: u8,
    pub id: i32,
    pub known_for_department: String,
    pub name: String,
    pub original_name: String,
    pub popularity: f32,
    pub profile_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TmdbTvEpisodeDetails {
    pub air_date: String,
    pub episode_number: u16,
    pub id: i32,
    pub name: String,
    pub overview: String,
    pub production_code: String,
    pub runtime: u32,
    pub season_number: u16,
    pub show_id: i32,
    pub still_path: String,
    pub vote_average: f32,
    pub vote_count: u32,
    pub crew: Vec<TmdbCrewMember>,
    pub guest_stars: Vec<TmdbCrewMember>,
}
