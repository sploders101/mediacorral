use serde::{Deserialize, Serialize};

use crate::{
    db::{
        schemas::{MatchInfoItem, OstDownloadsItem, VideoFilesItem},
        RipVideoBlobs,
    },
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
    pub subtitle_maps: Vec<RipVideoBlobs>,
    pub ost_subtitle_files: Vec<OstDownloadsItem>,
}
