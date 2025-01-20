use anyhow::Context;
use matroska::{Track, Tracktype};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncReadExt, process::Command};

/// Gets a list of subtitle tracks from an MKV file
async fn get_video_tracks(file: &Path) -> anyhow::Result<(u64, Vec<Track>)> {
    let file = PathBuf::from(file);
    let vid = tokio::task::spawn_blocking(move || matroska::open(file))
        .await?
        .context("Couldn't open video file")?;
    let tracks: Vec<Track> = vid
        .tracks
        .into_iter()
        .filter(|track| track.tracktype == Tracktype::Video && track.enabled)
        .collect();
    return Ok((
        vid.info.duration.map(|dur| dur.as_secs()).unwrap_or(0),
        tracks,
    ));
}

pub struct VideoMeta {
    pub width: u64,
    pub height: u64,
    pub length: u64,
    pub hash: [u8; 16],
}

/// Collects metadata about the video track (including hash)
pub async fn get_video_info(file: &Path) -> anyhow::Result<VideoMeta> {
    let (length, vid_tracks) = get_video_tracks(file).await?;
    let vid_track = match vid_tracks.get(0) {
        Some(track) => track,
        None => anyhow::bail!("Missing video track"),
    };
    let vid_settings = match vid_track.settings {
        matroska::Settings::Video(ref settings) => settings,
        _ => unreachable!(),
    };
    let track_file = file.with_extension("rawvideo");
    let extract_result = Command::new("mkvextract")
        .arg("tracks")
        .arg(&file)
        .arg(format!(
            "{}:{}",
            vid_track.number - 1,
            track_file.to_str().unwrap()
        ))
        .spawn()
        .unwrap()
        .wait()
        .await
        .unwrap();
    if !extract_result.success() {
        anyhow::bail!("Failed to extract video track");
    }

    // Now that the video has been extracted, hash it!
    let mut file = File::open(track_file).await?;
    let mut md5_ctx = md5::Context::new();
    loop {
        let mut read_buf = [0u8; 8192];
        let bytes_read = file.read(&mut read_buf).await?;
        if bytes_read == 0 {
            break;
        }
        md5_ctx.consume(&read_buf[0..bytes_read]);
    }
    return Ok(VideoMeta {
        width: vid_settings.pixel_width,
        height: vid_settings.pixel_height,
        length,
        hash: md5_ctx.compute().0,
    });
}
