use super::importers::opensubtitles::OpenSubtitles;
use crate::{
    blob_storage::BlobStorageController,
    db::{
        get_episode_id_from_tmdb, get_ost_download_items_by_match, get_rip_video_blobs,
        insert_match_info_item, purge_matches_from_rip,
        schemas::{MatchInfoItem, VideoType},
    },
    tagging::importers::opensubtitles::strip_subtitles,
};
use levenshtein::levenshtein;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use sqlx::SqlitePool;
use std::{io::Read, sync::Arc};
use tokio::io::AsyncReadExt;

macro_rules! try_skip {
    (return, $item:expr) => {
        match $item {
            Ok(result) => result,
            Err(err) => {
                eprintln!("{err}");
                return;
            }
        }
    };
    ($item:expr) => {
        match $item {
            Ok(result) => result,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        }
    };
}

/// This function downloads all subtitles for the given tmdb ids and compares them
/// against the files in the database, creating `match_info` records that can be used
/// for tagging.
///
/// This function is really gross, not that memory efficient, etc, but it should work
/// for now. I'm just trying to get this working. I really don't want to use the Xbox anymore...
pub async fn analyze_subtitles(
    db: Arc<SqlitePool>,
    ost: Arc<OpenSubtitles>,
    blobs: Arc<BlobStorageController>,
    rip_job: i64,
    tmdb_ids: Vec<i32>,
) {
    // First, clear out any existing results
    try_skip!(return, purge_matches_from_rip(&db, rip_job).await);

    // Then, grab new ones
    let videos = try_skip!(return, get_rip_video_blobs(&db, rip_job).await);

    for tmdb_id in tmdb_ids {
        let internal_id = try_skip!(get_episode_id_from_tmdb(&db, tmdb_id).await);
        let mut cached_subtitles = try_skip!(
            get_ost_download_items_by_match(&db, VideoType::TvEpisode, internal_id).await
        );
        let (subtitle_id, subtitles) = match cached_subtitles.len() {
            0 => {
                let (subtitle_name, subtitles) = try_skip!(ost.find_best_subtitles(tmdb_id).await);
                let subtitle_id = try_skip!(
                    blobs
                        .add_ost_subtitles(
                            VideoType::TvEpisode,
                            internal_id,
                            subtitle_name,
                            subtitles.clone(),
                        )
                        .await
                );
                (subtitle_id, subtitles)
            }
            _ => {
                let subtitle = cached_subtitles.swap_remove(0);
                let mut subtitles = String::new();
                let file_path = blobs.get_file_path(&subtitle.blob_id);
                let mut file = try_skip!(tokio::fs::File::open(&file_path).await);
                try_skip!(file.read_to_string(&mut subtitles).await);
                (subtitle.id.unwrap(), subtitles)
            }
        };
        let subtitles = strip_subtitles(&subtitles);
        let video_matches = {
            let blobs = Arc::clone(&blobs);
            videos.clone().into_par_iter().filter_map(move |video| {
                let subtitle_blob = match video.subtitle_blob {
                    Some(ref blob) => blob,
                    None => return None,
                };
                let subtitles_path = blobs.get_file_path(subtitle_blob);
                let mut file = match std::fs::File::open(subtitles_path) {
                    Ok(file) => file,
                    Err(_err) => return None,
                };
                let mut file_subtitles = String::new();
                if let Err(_err) = file.read_to_string(&mut file_subtitles) {
                    return None;
                };
                file_subtitles = strip_subtitles(&file_subtitles);
                let distance = levenshtein(&subtitles, &file_subtitles);
                let max_distance = subtitles.len().max(file_subtitles.len());
                return Some((video, distance, max_distance));
            })
        };
        let video_matches: Vec<_> = tokio::task::spawn_blocking(move || video_matches.collect())
            .await
            .unwrap();
        for (video_match, distance, max_distance) in video_matches {
            let _ = insert_match_info_item(
                &db,
                &MatchInfoItem {
                    id: None,
                    video_file_id: video_match.id,
                    ost_download_id: subtitle_id,
                    distance: distance as _,
                    max_distance: max_distance as _,
                },
            )
            .await;
        }
    }
}
