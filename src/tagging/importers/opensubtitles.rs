use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use std::time::SystemTime;
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

        let files: Vec<SubtitleSummary> = search_result
            .data
            .iter()
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
                    })
            })
            .collect();

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
            .authenticated(|| self.agent.post(&pointer.link))
            .await?
            .text()
            .await?);
    }

    /// Finds the best subtitles by grabbing up to 3 and comparing them. The one that
    /// matches more closely with the rest gets picked.
    pub async fn find_best_subtitles(&self, tmdb_id: i32) -> anyhow::Result<String> {
        let subtitle_results = self.find_subtitles(tmdb_id).await?.into_iter().take(3);

        let mut subtitles = Vec::with_capacity(3);
        for subtitle_summary in subtitle_results {
            subtitles.push(self.download_subtitles(subtitle_summary.file_id).await?);
        }

        match subtitles.len() {
            0 => anyhow::bail!("No subtitles found"),
            1 => return Ok(subtitles.pop().unwrap()),
            2 => {
                let file1 = subtitles.pop().unwrap();
                let file2 = subtitles.pop().unwrap();
                let distance = levenshtein::levenshtein(&file1, &file2);
                let max_distance = file1.len().max(file2.len());
                if distance > max_distance / 4 {
                    anyhow::bail!("Couldn't find reliable subtitles");
                }
                return Ok(file1);
            }
            3 => {
                let file1 = subtitles.pop().unwrap();
                let file2 = subtitles.pop().unwrap();
                let file3 = subtitles.pop().unwrap();
                let distance1 = levenshtein::levenshtein(&file1, &file2);
                let distance2 = levenshtein::levenshtein(&file2, &file3);
                let distance3 = levenshtein::levenshtein(&file3, &file1);
                let max_distance = file1.len().max(file2.len()).max(file3.len());

                let mut distances = vec![distance1, distance2, distance3];
                distances.sort();

                if distances[0] > max_distance / 4 {
                    anyhow::bail!("Couldn't find reliable subtitles");
                }

                if distances[0] == distance1 {
                    if distances[1] == distance2 {
                        return Ok(file2);
                    } else {
                        return Ok(file1);
                    }
                } else if distances[0] == distance2 {
                    if distances[1] == distance1 {
                        return Ok(file2);
                    } else {
                        return Ok(file3);
                    }
                } else if distances[0] == distance3 {
                    if distances[1] == distance1 {
                        return Ok(file1);
                    } else {
                        return Ok(file3);
                    }
                }

                todo!();
            }
            _ => unreachable!(),
        }
    }
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
struct SubtitleSummary {
    name: String,
    file_id: u32,
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
