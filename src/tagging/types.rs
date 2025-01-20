use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum SuspectedContents {
    Movie {
        tmdb_id: String,
    },
    TvEpisodes {
        tmdb_id: String,
        episodes: Vec<Episode>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Episode {
    season: u32,
    episode: u32,
}
