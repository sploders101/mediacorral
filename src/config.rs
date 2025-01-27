use lazy_static::lazy_static;

lazy_static! {
    pub static ref BDSUP_PATH: String = std::env::var("BDSUP2SUB_PATH")
        .expect("BDSup2Sub not found. Please specify it with BDSUP2SUB_PATH environment variable");
    pub static ref TMDB_API_KEY: String = std::env::var("TMDB_API_KEY")
        .expect("API key not found. Please specify it with TMDB_API_KEY environment variable");
    pub static ref OST_API_KEY: String = std::env::var("OST_API_KEY")
        .expect("API key not found. Please specify it with OST_API_KEY environment variable");
    pub static ref OST_USERNAME: String = std::env::var("OST_USERNAME")
        .expect("OST username not found. Please specify it with OST_USERNAME environment variable");
    pub static ref OST_PASSWORD: String = std::env::var("OST_PASSWORD")
        .expect("OST password not found. Please specify it with OST_PASSWORD environment variable");
}
