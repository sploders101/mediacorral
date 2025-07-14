use std::io::{Read, Seek};

use matroska_demuxer::{Frame, MatroskaFile, TrackType};
use mediacorral_proto::mediacorral::server::v1::{ChapterInfo, VideoExtendedMetadata};
use subtitles::{
    StContext, get_subtitle_track,
    ocr::{PartessCache, PartessError},
    pgs::PgsError,
    srt::format_subtitles_srt,
    vobsub::VobsubError,
};
use tokio::sync::watch;

pub mod subtitles;

#[derive(thiserror::Error, Debug)]
pub enum ExtractDetailsError {
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
    #[error("The content is missing a video track")]
    MissingVideoTrack,
    #[error("Missing required properties on file")]
    MissingRequiredProps,
    #[error("The subrip subtitles are not valid UTF-8")]
    SubripInvalidUtf8,
    #[error("An error occurred while reading PGS subtitles:\n{0}")]
    PgsError(#[from] PgsError),
    #[error("An error occurred while reading VobSub subtitles:\n{0}")]
    VobsubError(#[from] VobsubError),
    #[error("An error occurred while demuxing:\n{0}")]
    DemuxError(#[from] matroska_demuxer::DemuxError),
    #[error("An error occurred while running OCR:\n{0}")]
    PartessError(#[from] PartessError),
}

pub struct MediaDetails {
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub duration: u32,
    pub video_hash: Vec<u8>,
    pub subtitles: Option<String>,
    pub extended_metadata: Option<VideoExtendedMetadata>,
}

pub fn extract_details<T>(
    mkv_file: T,
    mut progress: Option<watch::Sender<f64>>,
    partess_cache: &PartessCache,
) -> Result<MediaDetails, ExtractDetailsError>
where
    T: Read + Seek,
{
    let mut mkv_file = MatroskaFile::open(mkv_file)?;
    let mut extended_metadata = VideoExtendedMetadata::default();

    // Video-related things
    let vid_track = mkv_file
        .tracks()
        .into_iter()
        .find(|track| track.track_type() == TrackType::Video)
        .ok_or(ExtractDetailsError::MissingVideoTrack)?;
    let vid_track_info = vid_track.video().unwrap();
    let vid_track_number = vid_track.track_number().get();

    let resolution_width: u32 = vid_track_info
        .display_width()
        .ok_or(ExtractDetailsError::MissingRequiredProps)?
        .get() as _;
    let resolution_height: u32 = vid_track_info
        .display_height()
        .ok_or(ExtractDetailsError::MissingRequiredProps)?
        .get() as _;

    let mut vid_hasher = md5::Context::new();

    // Subtitle-related things
    let st_track = get_subtitle_track(mkv_file.tracks())?;
    let st_track_number = st_track.map(|track| track.track_number().get());
    let mut st_ctx = match st_track {
        Some(st_track) => Some(StContext::new(st_track, partess_cache)?),
        None => None,
    };

    // Container-related things
    let info = mkv_file.info();

    let timestamp_scale = info.timestamp_scale().get();
    let duration: u64 = match info.duration() {
        Some(duration) => duration.round() as u64 * timestamp_scale,
        None => return Err(ExtractDetailsError::MissingRequiredProps),
    };
    let duration_secs = duration / 1_000_000_000;

    if let Some(chapters) = mkv_file.chapters() {
        let mut chapters = chapters
            .into_iter()
            .flat_map(|edition| edition.chapter_atoms().into_iter())
            .enumerate()
            .peekable();
        while let Some((i, chapter)) = chapters.next() {
            extended_metadata.chapter_info.push(ChapterInfo {
                chapter_number: (i + 1) as _,
                chapter_uid: chapter.uid().get(),
                chapter_start: chapter.time_start(),
                chapter_end: chapter
                    .time_end()
                    .or(chapters
                        .peek()
                        .map(|(_i, next_chapter)| next_chapter.time_start()))
                    .unwrap_or(duration),
                chapter_name: chapter
                    .displays()
                    .into_iter()
                    .find(|display| {
                        matches!(
                            display.language_ietf().or(display.language()),
                            Some("eng") | Some("en") | Some("en-US") | Some("en-GB")
                        )
                    })
                    .map(|display| display.string().into())
                    .unwrap_or_else(|| format!("Chapter {}", i + 1)),
            });
        }
    }

    // Frame processing loop
    let mut frame = Frame::default();
    while mkv_file.next_frame(&mut frame)? {
        frame.timestamp = frame.timestamp * timestamp_scale;
        frame.duration = frame.duration.map(|duration| duration * timestamp_scale);
        if frame.track == vid_track_number {
            // Process video
            vid_hasher.consume(&frame.data);
        } else if Some(frame.track) == st_track_number {
            // Process subtitles
            if let Some(ref mut st_ctx) = st_ctx {
                st_ctx.process_frame(&mut frame)?;
            }
        }

        // Update progress
        if let Some(ref mut progress) = progress.as_mut() {
            let progress_value = (frame.timestamp as f64 / duration as f64 * 100.0).round();
            let _ = progress.send_if_modified(|old_val| {
                if *old_val != progress_value {
                    *old_val = progress_value;
                    true
                } else {
                    false
                }
            });
        }
    }

    return Ok(MediaDetails {
        resolution_width,
        resolution_height,
        duration: duration_secs as u32,
        video_hash: vid_hasher.compute().to_vec(),
        subtitles: match st_ctx {
            Some(st_ctx) => Some(format_subtitles_srt(st_ctx.collect()?, duration)),
            None => None,
        },
        extended_metadata: Some(extended_metadata),
    });
}
