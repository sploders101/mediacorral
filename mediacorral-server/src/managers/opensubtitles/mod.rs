use lazy_regex::regex;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::{cmp::Ordering, time::SystemTime};
use thiserror::Error;
use tokio::{
    io::AsyncReadExt,
    sync::{Mutex, oneshot},
};

use crate::{
    blob_storage::{BlobError, BlobStorageController},
    db::{self, schemas::VideoType},
};

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Error, Debug)]
pub enum OstError {
    #[error("No subtitles found")]
    NoSubtitlesFound,
    #[error("Couldn't find reliable subtitles")]
    UnreliableSubtitles,
    #[error("An unknown blob storage error occurred:\n{0}")]
    BlobError(#[from] BlobError),
    #[error(
        "An error occurred while deserializing {tag:?} response:\n{inner}\nOriginal text:\n{original_text}"
    )]
    DeserializeError {
        tag: &'static str,
        inner: serde_json::Error,
        original_text: String,
    },
    #[error("An unknown reqwest error occurred:\n{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("An unknown database error occurred:\n{0}")]
    Db(#[from] sqlx::Error),
    #[error("An unknown I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
}

type OstResult<T> = Result<T, OstError>;

pub struct OpenSubtitles {
    agent: reqwest::Client,
    api_key: String,
    username: String,
    password: String,
    auth_token: Mutex<Option<(SystemTime, String)>>,
}
impl OpenSubtitles {
    pub fn new(api_key: String, username: String, password: String) -> Self {
        return Self {
            agent: reqwest::Client::new(),
            api_key,
            username,
            password,
            auth_token: Mutex::new(None),
        };
    }
    async fn login(&self, auth_token: &mut Option<(SystemTime, String)>) -> Result<(), OstError> {
        let response = self
            .agent
            .post("https://api.opensubtitles.com/api/v1/login")
            .header("User-Agent", "Mediacorral v1.0.0")
            .header("Api-Key", &self.api_key)
            .json(&json!({
                "username": &self.username,
                "password": &self.password,
            }))
            .send()
            .await?
            .text()
            .await?;
        let response: LoginResponse = match serde_json::from_str(&response) {
            Ok(response) => response,
            Err(err) => {
                return Err(OstError::DeserializeError {
                    tag: "login",
                    inner: err,
                    original_text: response,
                });
            }
        };

        *auth_token = Some((SystemTime::now(), response.token));

        return Ok(());
    }

    async fn authenticated(
        &self,
        req_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, OstError> {
        let query_start = SystemTime::now();
        loop {
            let mut auth_token = self.auth_token.lock().await;
            if let Some((ref token_time, ref mut token)) = *auth_token {
                let response = req_fn()
                    .header("User-Agent", "Mediacorral v1.0.0")
                    .header("Api-Key", &self.api_key)
                    .header("Authorization", String::from("Bearer ") + token)
                    .send()
                    .await?;
                if response.status() == StatusCode::UNAUTHORIZED {
                    if token_time < &query_start {
                        self.login(&mut auth_token).await?;
                        continue;
                    } else {
                        return Err(response.error_for_status().unwrap_err().into());
                    }
                } else {
                    return Ok(response);
                }
            } else {
                self.login(&mut auth_token).await?;
                continue;
            }
        }
    }

    pub async fn find_subtitles(&self, tmdb_id: i32) -> OstResult<Vec<SubtitleSummary>> {
        let search_result = self
            .authenticated(|| {
                self.agent
                    .get("https://api.opensubtitles.com/api/v1/subtitles")
                    .query(&[("tmdb_id", &tmdb_id.to_string())])
            })
            .await?
            .error_for_status()?
            .text()
            .await?;
        let search_result: SearchResults = match serde_json::from_str(&search_result) {
            Ok(search_result) => search_result,
            Err(err) => {
                return Err(OstError::DeserializeError {
                    tag: "find_subtitles",
                    inner: err,
                    original_text: search_result,
                });
            }
        };

        let mut files: Vec<SubtitleSummary> = search_result
            .data
            .iter()
            .filter(|subtitle| {
                subtitle.attributes.language.as_ref().map(String::as_str) == Some("en")
            })
            .flat_map(|subtitle| {
                let lang = subtitle
                    .attributes
                    .language
                    .as_ref()
                    .expect("Language is null. This should have been filtered prior");
                subtitle
                    .attributes
                    .files
                    .iter()
                    .map(move |file| SubtitleSummary {
                        name: format!(
                            "lang: {}, name: {}, uploader: {} ({})",
                            lang,
                            file.file_name,
                            subtitle.attributes.uploader.name,
                            subtitle.attributes.uploader.rank,
                        ),
                        download_count: subtitle.attributes.download_count,
                        new_download_count: subtitle.attributes.new_download_count,
                        file_id: file.file_id,
                        uploader: subtitle.attributes.uploader.clone(),
                    })
            })
            .collect();

        files.sort_by(|a, b| {
            match (a.uploader.rank.as_str(), b.uploader.rank.as_str()) {
                ("Admin Warning", "Admin Warning") => {}
                (_, "Admin Warning") => return Ordering::Less,
                ("Admin Warning", _) => return Ordering::Greater,
                _ => {}
            }
            match &a.new_download_count.cmp(&b.new_download_count) {
                Ordering::Less => return Ordering::Greater,
                Ordering::Greater => return Ordering::Less,
                Ordering::Equal => {}
            }
            match &a.download_count.cmp(&b.download_count) {
                Ordering::Less => return Ordering::Greater,
                Ordering::Greater => return Ordering::Less,
                Ordering::Equal => {}
            }
            match numeric_rank(&a.uploader.rank).cmp(&numeric_rank(&b.uploader.rank)) {
                Ordering::Less => return Ordering::Greater,
                Ordering::Greater => return Ordering::Less,
                Ordering::Equal => return Ordering::Equal,
            }
            // May add more criteria later. Not sure yet.
        });

        return Ok(files);
    }

    pub async fn download_subtitles(&self, file_id: u32) -> OstResult<String> {
        let pointer = self
            .authenticated(|| {
                self.agent
                    .post("https://api.opensubtitles.com/api/v1/download")
                    .json(&json!({
                        "file_id": file_id,
                    }))
            })
            .await?
            .error_for_status()?
            .text()
            .await?;
        let pointer: DownloadPointer = match serde_json::from_str(&pointer) {
            Ok(pointer) => pointer,
            Err(err) => {
                return Err(OstError::DeserializeError {
                    tag: "download_subtitles",
                    inner: err,
                    original_text: pointer,
                });
            }
        };

        return Ok(self
            .authenticated(|| self.agent.get(&pointer.link))
            .await?
            .error_for_status()?
            .text()
            .await?);
    }

    /// Finds the best subtitles by grabbing up to 3 and comparing them. The one that
    /// matches more closely with the rest gets picked.
    /// returns `(file_name, subtitles)`
    pub async fn find_best_subtitles(&self, tmdb_id: i32) -> OstResult<(String, String)> {
        let subtitle_results = self.find_subtitles(tmdb_id).await?.into_iter().take(3);

        let mut subtitles = Vec::with_capacity(3);
        for subtitle_summary in subtitle_results {
            subtitles.push((
                subtitle_summary.name,
                self.download_subtitles(subtitle_summary.file_id).await?,
            ));
        }

        let (ret_chan_sender, ret_chan_recv) = oneshot::channel::<OstResult<(String, String)>>();
        tokio::task::spawn_blocking(move || match subtitles.len() {
            0 => {
                let _ = ret_chan_sender.send(Err(OstError::NoSubtitlesFound));
                return;
            }
            1 => {
                let _ = ret_chan_sender.send(Ok(subtitles.pop().unwrap()));
                return;
            }
            2 => {
                let file1 = subtitles.pop().unwrap();
                let file2 = subtitles.pop().unwrap();
                let mut distance = None;
                rayon::scope(|s| {
                    s.spawn(|_| distance = Some(levenshtein::levenshtein(&file1.1, &file2.1)));
                });
                let distance = distance.unwrap();
                let max_distance = file1.1.len().max(file2.1.len());
                if distance > max_distance / 2 {
                    let _ = ret_chan_sender.send(Err(OstError::UnreliableSubtitles));
                    return;
                }
                let _ = ret_chan_sender.send(Ok(file1));
                return;
            }
            3 => {
                let file1 = subtitles.pop().unwrap();
                let file2 = subtitles.pop().unwrap();
                let file3 = subtitles.pop().unwrap();
                let mut distance1: Option<usize> = None;
                let mut distance2: Option<usize> = None;
                let mut distance3: Option<usize> = None;
                let file1_stripped = strip_subtitles(&file1.1);
                let file2_stripped = strip_subtitles(&file2.1);
                let file3_stripped = strip_subtitles(&file3.1);
                rayon::scope(|s| {
                    s.spawn(|_| {
                        distance1 = Some(levenshtein::levenshtein(&file1_stripped, &file2_stripped))
                    });
                    s.spawn(|_| {
                        distance2 = Some(levenshtein::levenshtein(&file2_stripped, &file3_stripped))
                    });
                    s.spawn(|_| {
                        distance3 = Some(levenshtein::levenshtein(&file3_stripped, &file1_stripped))
                    });
                });
                let distance1 = distance1.unwrap();
                let distance2 = distance2.unwrap();
                let distance3 = distance3.unwrap();
                let max_distance = file1.1.len().max(file2.1.len()).max(file3.1.len());

                let mut distances = vec![(1, distance1), (2, distance2), (3, distance3)];
                distances.sort_by_key(|item| item.1);

                if distances[0].0 > max_distance / 2 {
                    let _ = ret_chan_sender.send(Err(OstError::UnreliableSubtitles));
                    return;
                }

                if distances[0].0 == 1 {
                    if distances[1].0 == 2 {
                        let _ = ret_chan_sender.send(Ok(file2));
                        return;
                    } else {
                        let _ = ret_chan_sender.send(Ok(file1));
                        return;
                    }
                } else if distances[0].0 == 2 {
                    if distances[1].0 == 1 {
                        let _ = ret_chan_sender.send(Ok(file2));
                        return;
                    } else {
                        let _ = ret_chan_sender.send(Ok(file3));
                        return;
                    }
                } else if distances[0].0 == 3 {
                    if distances[1].0 == 1 {
                        let _ = ret_chan_sender.send(Ok(file1));
                        return;
                    } else {
                        let _ = ret_chan_sender.send(Ok(file3));
                        return;
                    }
                }

                unreachable!();
            }
            _ => unreachable!(),
        });
        let thing = ret_chan_recv.await.unwrap()?;
        return Ok(thing);
    }

    pub async fn get_subtitles(
        &self,
        db: &SqlitePool,
        blob_controller: &BlobStorageController,
        video_type: VideoType,
        video_id: i64,
        tmdb_id: i32,
    ) -> OstResult<(i64, String)> {
        let existing_subs =
            match db::get_ost_download_items_by_match(db, video_type, video_id).await {
                Ok(row) => row.into_iter().next(),
                Err(sqlx::Error::RowNotFound) => None,
                Err(err) => return Err(err.into()),
            };

        if let Some(existing_subs) = existing_subs {
            let mut subs = String::new();
            let file_path = blob_controller.get_file_path(&existing_subs.blob_id);
            let mut file = tokio::fs::File::open(file_path).await?;
            file.read_to_string(&mut subs).await?;
            return Ok((
                existing_subs
                    .id
                    .expect("Primary key missing in query result"),
                subs,
            ));
        }

        let (filename, subs) = self.find_best_subtitles(tmdb_id).await?;
        let result = blob_controller
            .add_ost_subtitles(video_type, video_id, filename.clone(), subs.clone())
            .await?;

        return Ok((result, subs));
    }
}

/// Strips symbols from subtitles that may cause issues during comparison
pub fn strip_subtitles(subs: &str) -> String {
    let intermediate = regex!(
        r"(?:<\s*[^>]*>|<\s*/\s*a>)|(?:^.*-->.*$|^[0-9]+$|[^a-zA-Z0-9 ?\.,!\n]|^\s*-*\s*|\r)"m
    )
    .replace_all(subs, "");
    return regex!(r"[\n ]{1,}")
        .replace_all(&intermediate, " ")
        .into_owned();
}

/// Converts an uploader's rank into a numeric value for sorting
fn numeric_rank(rank: &str) -> usize {
    return match rank {
        "Administrator" => 0,
        "Application Developers" => 10,
        "Gold member" => 20,
        "Bronze Member" => 30,
        "anonymous" => 100,
        "Admin Warning" => 110,
        _ => 90,
    };
}

#[derive(Debug, Deserialize, Clone)]
struct SearchResults {
    // total_pages: u32,
    // total_count: u32,
    // per_page: u32,
    // page: u32,
    data: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, Clone)]
struct SearchResult {
    // id: String,
    // #[serde(rename = "type")]
    // result_type: String,
    attributes: STAttributes,
}

#[derive(Debug, Deserialize, Clone)]
struct STAttributes {
    // subtitle_id: String,
    language: Option<String>,
    download_count: u32,
    new_download_count: u32,
    // hearing_impaired: bool,
    // votes: u32,
    // ratings: f32,
    // from_trusted: bool,
    // foreign_parts_only: bool,
    // ai_translated: bool,
    // machine_translated: bool,
    // release: String,
    uploader: OSTUploader,
    files: Vec<STFile>,
}

#[derive(Debug, Deserialize, Clone)]
struct OSTUploader {
    // uploader_id: i32,
    name: String,
    rank: String,
}

#[derive(Debug, Deserialize, Clone)]
struct STFile {
    file_id: u32,
    file_name: String,
}

#[derive(Debug, Clone)]
pub struct SubtitleSummary {
    name: String,
    file_id: u32,
    download_count: u32,
    new_download_count: u32,
    uploader: OSTUploader,
}

#[derive(Debug, Deserialize, Clone)]
struct DownloadPointer {
    link: String,
    // file_name: String,
    // requests: u32,
    // remaining: u32,
    // message: String,
    // reset_time: String,
    // reset_time_utc: String,
}
