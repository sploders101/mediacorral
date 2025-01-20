use crate::{config::BDSUP_PATH, task_queue::TaskQueue};
use anyhow::Context;
use futures::{Stream, StreamExt};
use matroska::{Track, Tracktype};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

/// Gets the track to be used for comparison with OST
pub async fn get_comparison_track(file: PathBuf) -> anyhow::Result<Option<Track>> {
    let mut tracks = tokio::task::spawn_blocking(move || {
        get_subtitle_tracks(&file)
    }).await??;
    if tracks.len() == 0 {
        return Ok(None);
    }
    let default_track = get_default_track(&tracks).cloned();

    return Ok(Some(match default_track {
        Some(default_track) => default_track,
        None => tracks.swap_remove(0),
    }));
}

/// Tries to narrow down which subtitles track is preferrable.
/// If we are able to narrow it down to exactly one, it is returned.
/// Otherwise, this function returns None.
fn get_default_track<'a>(tracks: &'a Vec<Track>) -> Option<&'a Track> {
    return match tracks.len() {
        0 => None,
        1 => Some(&tracks[0]),
        _ => {
            let mut default_tracks = tracks.iter().filter(|track| track.default);
            if let (Some(track), None) = (default_tracks.next(), default_tracks.next()) {
                Some(track)
            } else {
                None
            }
        }
    };
}

/// Gets a list of subtitle tracks from an MKV file
fn get_subtitle_tracks(file: &Path) -> anyhow::Result<Vec<Track>> {
    let vid = matroska::open(file).context("Couldn't open video file")?;
    let tracks: Vec<Track> = vid
        .tracks
        .into_iter()
        .filter(|track| track.tracktype == Tracktype::Subtitle && track.enabled)
        .filter(|track| {
            track
                .language
                .as_ref()
                .map(|lang| match lang {
                    matroska::Language::ISO639(lang) if lang == "eng" => true,
                    matroska::Language::IETF(lang) if lang == "en" => true,
                    matroska::Language::IETF(lang) if lang == "en-US" => true,
                    matroska::Language::IETF(lang) if lang == "en-GB" => true,
                    _ => false,
                })
                .unwrap_or(false)
        })
        .collect();
    return Ok(tracks);
}

pub async fn extract_subtitles(
    mut files: impl Stream<Item = anyhow::Result<PathBuf>> + Unpin,
) -> anyhow::Result<()> {
    let ocr_queue = TaskQueue::new();

    while let Some(file) = files.next().await {
        let file = file.context("Error getting file")?;
        let st_track = match get_comparison_track(file.clone()).await? {
            Some(track) => track,
            None => {
                println!(
                    "No suitable subtitles found for {}",
                    file.into_os_string()
                        .into_string()
                        .expect("Invalid file name")
                );
                continue;
            }
        };
        let track_file = file.with_extension(match st_track.codec_id.as_str() {
            "S_TEXT/UTF8" => "srt",
            "S_VOBSUB" => "sub",
            "S_HDMV/PGS" => "sup",
            _ => "dat",
        });
        let extract_result = Command::new("mkvextract")
            .arg("tracks")
            .arg(&file)
            .arg(format!(
                "{}:{}",
                st_track.number - 1,
                track_file.to_str().unwrap()
            ))
            .spawn()
            .unwrap()
            .wait()
            .await
            .unwrap();
        if !extract_result.success() {
            anyhow::bail!("Failed to extract subtitles");
        }

        ocr_queue.add_task(async move {
            let mut vobsubocr = false;
            match st_track.codec_id.as_str() {
                "S_VOBSUB" => {
                    vobsubocr = true;
                }
                "S_HDMV/PGS" => {
                    Command::new("java")
                        .args(["-jar", &BDSUP_PATH, "-o"])
                        .arg(file.with_extension("sub"))
                        .arg(file.with_extension("sup"))
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .spawn()
                        .unwrap()
                        .wait()
                        .await
                        .unwrap();

                    // This program seems to exit before it's actually finished. May need to do some bugfixing...
                    // if bdsup_result.success() {}
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    vobsubocr = true;
                }
                _ => {}
            }
            if vobsubocr {
                Command::new("vobsubocr")
                    .args(["-c", "tessedit_char_blacklist=|\\/`_~!", "-l", "eng", "-o"])
                    .arg(file.with_extension("srt"))
                    .arg(file.with_extension("idx"))
                    .spawn()
                    .unwrap()
                    .wait()
                    .await
                    .unwrap();
            }
        });
    }

    ocr_queue.wait_for_queued_tasks().await;

    return Ok(());
}
