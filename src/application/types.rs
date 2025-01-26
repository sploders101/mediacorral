use serde::{Deserialize, Serialize};

use crate::{
    db::schemas::{MatchInfoItem, VideoFilesItem},
    tagging::types::SuspectedContents,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobInfo {
    pub id: Option<i64>,
    pub start_time: i64,
    pub disc_title: Option<String>,
    pub suspected_contents: Option<SuspectedContents>,
    pub video_files: Vec<VideoFilesItem>,
    pub matches: Vec<MatchInfoItem>,
}
