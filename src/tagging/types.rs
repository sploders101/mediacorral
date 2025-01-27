use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum SuspectedContents {
    Movie {
        tmdb_id: i32,
    },
    TvEpisodes {
        episode_tmdb_ids: Vec<i32>,
    },
}
