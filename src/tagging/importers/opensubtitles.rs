use lazy_regex::regex;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::{cmp::Ordering, time::SystemTime};
use tokio::sync::Mutex;

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

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
    async fn login(&self, auth_token: &mut Option<(SystemTime, String)>) -> reqwest::Result<()> {
        let response: LoginResponse = self
            .agent
            .post("https://api.opensubtitles.com/api/v1/login")
            .header("User-Agent", "plex-autotagger")
            .header("Api-Key", &self.api_key)
            .json(&json!({
                "username": &self.username,
                "password": &self.password,
            }))
            .send()
            .await?
            .json()
            .await?;

        *auth_token = Some((SystemTime::now(), response.token));

        return Ok(());
    }

    async fn authenticated(
        &self,
        req_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> reqwest::Result<reqwest::Response> {
        println!("Making OST request");
        let query_start = SystemTime::now();
        loop {
            let mut auth_token = self.auth_token.lock().await;
            if let Some((ref token_time, ref mut token)) = *auth_token {
                let response = req_fn()
                    .header("User-Agent", "plex-autotagger")
                    .header("Api-Key", &self.api_key)
                    .header("Authorization", String::from("Bearer ") + token)
                    .send()
                    .await?;
                if response.status() == StatusCode::UNAUTHORIZED {
                    if token_time < &query_start {
                        self.login(&mut auth_token).await?;
                        continue;
                    } else {
                        return Err(response.error_for_status().unwrap_err());
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

    pub async fn find_subtitles(&self, tmdb_id: i32) -> reqwest::Result<Vec<SubtitleSummary>> {
        let search_result: SearchResults = self
            .authenticated(|| {
                self.agent
                    .get("https://api.opensubtitles.com/api/v1/subtitles")
                    .query(&[("tmdb_id", &tmdb_id.to_string())])
            })
            .await?
            .json()
            .await?;

        let mut files: Vec<SubtitleSummary> = search_result
            .data
            .iter()
            .filter(|subtitle| &subtitle.attributes.language == "en")
            .flat_map(|subtitle| {
                subtitle
                    .attributes
                    .files
                    .iter()
                    .map(|file| SubtitleSummary {
                        name: format!(
                            "lang: {}, name: {}, uploader: {} ({})",
                            subtitle.attributes.language,
                            file.file_name,
                            subtitle.attributes.uploader.name,
                            subtitle.attributes.uploader.rank,
                        ),
                        file_id: file.file_id,
                        uploader: subtitle.attributes.uploader.clone(),
                    })
            })
            .collect();

        files.sort_by(|a, b| {
            match numeric_rank(&a.uploader.rank).cmp(&numeric_rank(&b.uploader.rank)) {
                Ordering::Less => return Ordering::Less,
                Ordering::Greater => return Ordering::Greater,
                Ordering::Equal => return Ordering::Equal,
            }
            // May add more criteria later. Not sure yet.
        });

        return Ok(files);
    }

    pub async fn download_subtitles(&self, file_id: u32) -> reqwest::Result<String> {
        let pointer: DownloadPointer = self
            .authenticated(|| {
                self.agent
                    .post("https://api.opensubtitles.com/api/v1/download")
                    .json(&json!({
                        "file_id": file_id,
                    }))
            })
            .await?
            .json()
            .await?;

        return Ok(self
            .authenticated(|| self.agent.get(&pointer.link))
            .await?
            .text()
            .await?);
    }

    /// Finds the best subtitles by grabbing up to 3 and comparing them. The one that
    /// matches more closely with the rest gets picked.
    /// returns `(file_name, subtitles)`
    pub async fn find_best_subtitles(&self, tmdb_id: i32) -> anyhow::Result<(String, String)> {
        let subtitle_results = self.find_subtitles(tmdb_id).await?.into_iter().take(3);

        let mut subtitles = Vec::with_capacity(3);
        for subtitle_summary in subtitle_results {
            subtitles.push((
                subtitle_summary.name,
                self.download_subtitles(subtitle_summary.file_id).await?,
            ));
        }

        return tokio::task::spawn_blocking(move || match subtitles.len() {
            0 => anyhow::bail!("No subtitles found"),
            1 => return Ok(subtitles.pop().unwrap()),
            2 => {
                let file1 = subtitles.pop().unwrap();
                let file2 = subtitles.pop().unwrap();
                let distance = levenshtein::levenshtein(&file1.1, &file2.1);
                let max_distance = file1.1.len().max(file2.1.len());
                if distance > max_distance / 4 {
                    anyhow::bail!("Couldn't find reliable subtitles");
                }
                return Ok(file1);
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

                if distances[0].0 > max_distance / 4 {
                    anyhow::bail!("Couldn't find reliable subtitles");
                }

                if distances[0].0 == 1 {
                    if distances[1].0 == 2 {
                        return Ok(file2);
                    } else {
                        return Ok(file1);
                    }
                } else if distances[0].0 == 2 {
                    if distances[1].0 == 1 {
                        return Ok(file2);
                    } else {
                        return Ok(file3);
                    }
                } else if distances[0].0 == 3 {
                    if distances[1].0 == 1 {
                        return Ok(file1);
                    } else {
                        return Ok(file3);
                    }
                }

                unreachable!();
            }
            _ => unreachable!(),
        })
        .await
        .unwrap();
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
    return match a.uploader.rank {
        "Administrator" => 100,
        "Application Developers" => 40,
        "Gold member" => 30,
        "Bronze Member" => 20,
        "anonymous" => 0,
        _ => 10,
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
    language: String,
    // download_count: u32,
    // new_download_count: u32,
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
