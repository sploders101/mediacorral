use std::{
    collections::HashMap,
    io::{Read, Seek},
};

use crate::{proto::mediacorral::analysis::v1 as pb, utils::subtitles::srt::format_subtitles_srt};
use matroska_demuxer::{Frame, MatroskaFile, TrackType};
use subtitles::{
    StContext, get_subtitle_track,
    ocr::{PartessCache, PartessError},
    pgs::PgsError,
    vobsub::VobsubError,
};

pub mod subtitles;

#[derive(thiserror::Error, Debug)]
pub enum ExtractDetailsError {
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
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

fn map_stereo_mode(mode: matroska_demuxer::StereoMode) -> pb::VideoStereoMode {
    match mode {
        matroska_demuxer::StereoMode::Unknown => pb::VideoStereoMode::Unspecified,
        matroska_demuxer::StereoMode::Mono => pb::VideoStereoMode::Mono,
        matroska_demuxer::StereoMode::SideBySideLeftEyeFirst => {
            pb::VideoStereoMode::SideBySideLeftEyeFirst
        }
        matroska_demuxer::StereoMode::TopBottomRightEyeFirst => {
            pb::VideoStereoMode::TopBottomRightEyeFirst
        }
        matroska_demuxer::StereoMode::TopBottomLeftEyeFirst => {
            pb::VideoStereoMode::TopBottomLeftEyeFirst
        }
        matroska_demuxer::StereoMode::CheckboardRightEyeFirst => {
            pb::VideoStereoMode::CheckboardRightEyeFirst
        }
        matroska_demuxer::StereoMode::CheckboardLeftEyeFirst => {
            pb::VideoStereoMode::CheckboardLeftEyeFirst
        }
        matroska_demuxer::StereoMode::RowInterleavedRightEyeFirst => {
            pb::VideoStereoMode::RowInterleavedRightEyeFirst
        }
        matroska_demuxer::StereoMode::RowInterleavedLeftEyeFirst => {
            pb::VideoStereoMode::RowInterleavedLeftEyeFirst
        }
        matroska_demuxer::StereoMode::ColumnInterleavedRightEyeFirst => {
            pb::VideoStereoMode::ColumnInterleavedRightEyeFirst
        }
        matroska_demuxer::StereoMode::ColumnInterleavedLeftEyeFirst => {
            pb::VideoStereoMode::ColumnInterleavedLeftEyeFirst
        }
        matroska_demuxer::StereoMode::AnaglyphCyanRed => pb::VideoStereoMode::AnaglyphCyanRed,
        matroska_demuxer::StereoMode::SideBySideRightEyeFirst => {
            pb::VideoStereoMode::SideBySideRightEyeFirst
        }
        matroska_demuxer::StereoMode::AnaglyphGreenMagenta => {
            pb::VideoStereoMode::AnaglyphGreenMagenta
        }
        matroska_demuxer::StereoMode::LacedLeftEyeFirst => pb::VideoStereoMode::LacedLeftEyeFirst,
        matroska_demuxer::StereoMode::LacedRightEyeFirst => pb::VideoStereoMode::LacedRightEyeFirst,
    }
}

pub fn extract_details<T>(
    mkv_file: T,
    partess_cache: &PartessCache,
    st_track_number: u64,
) -> Result<pb::AnalyzeMkvResponse, ExtractDetailsError>
where
    T: Read + Seek,
{
    let mut mkv_file = MatroskaFile::open(mkv_file)?;
    let mut metadata = pb::MediaDetails::default();
    let info = mkv_file.info();

    // Collect container metadata
    metadata.name = info.title().unwrap_or_default().to_string();
    let timestamp_scale = info.timestamp_scale().get();
    let duration_ns: u64 = match info.duration() {
        Some(duration) => duration.round() as u64 * timestamp_scale,
        None => return Err(ExtractDetailsError::MissingRequiredProps),
    };
    let duration = duration_ns / 1_000_000_000;
    metadata.duration = duration as _;
    if let Some(chapters) = mkv_file.chapters() {
        let mut chapters = chapters
            .into_iter()
            .flat_map(|edition| edition.chapter_atoms().into_iter())
            .enumerate()
            .peekable();
        while let Some((i, chapter)) = chapters.next() {
            metadata.chapter_info.push(pb::ChapterInfo {
                chapter_number: (i + 1) as _,
                chapter_uid: chapter.uid().get(),
                chapter_start_ns: chapter.time_start(),
                chapter_end_ns: chapter
                    .time_end()
                    .or(chapters
                        .peek()
                        .map(|(_i, next_chapter)| next_chapter.time_start()))
                    .unwrap_or(duration_ns),
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

    // Collect track metadata
    let mut track_hashers: HashMap<u64, md5::Context> =
        HashMap::with_capacity(mkv_file.tracks().len());
    for track in mkv_file.tracks().into_iter() {
        match track.track_type() {
            TrackType::Video => {
                let video_details = match track.video() {
                    Some(value) => value,
                    None => {
                        eprintln!("Missing video track info. Skipping...");
                        continue;
                    }
                };
                track_hashers.insert(track.track_number().get(), md5::Context::new());
                metadata.video_tracks.push(pb::VideoTrack {
                    track_number: track.track_number().get(),
                    track_uid: track.track_uid().get(),
                    enabled: track.flag_enabled(),
                    default: track.flag_default(),
                    commentary: track.flag_commentary(),
                    original: track.flag_original(),
                    visual_impaired: track.flag_visual_impaired(),
                    name: track.name().map(String::from),
                    language: track.language().map(String::from),
                    codec_id: String::from(track.codec_id()),
                    stereo_mode: video_details
                        .stereo_mode()
                        .map(map_stereo_mode)
                        .unwrap_or(pb::VideoStereoMode::Mono) as _,
                    display_width: video_details.display_width().map(|i| i.get()).unwrap_or(0),
                    display_height: video_details.display_height().map(|i| i.get()).unwrap_or(0),
                    hash: Vec::new(), // Will get overwritten after frame processing
                });
            }
            TrackType::Audio => {
                // Instantiate track hasher
                track_hashers.insert(track.track_number().get(), md5::Context::new());
                let audio_details = match track.audio() {
                    Some(value) => value,
                    None => {
                        eprintln!("Missing audio track info. Skipping...");
                        continue;
                    }
                };
                metadata.audio_tracks.push(pb::AudioTrack {
                    track_number: track.track_number().get(),
                    track_uid: track.track_uid().get(),
                    enabled: track.flag_enabled(),
                    default: track.flag_default(),
                    commentary: track.flag_commentary(),
                    original: track.flag_original(),
                    visual_impaired: track.flag_visual_impaired(),
                    name: track.name().map(String::from),
                    language: track.language().map(String::from),
                    codec_id: String::from(track.codec_id()),
                    channels: audio_details.channels().get(),
                    hash: Vec::new(), // Will be overwritten after frame processing
                });
            }
            TrackType::Subtitle => {
                // Instantiate track hasher
                track_hashers.insert(track.track_number().get(), md5::Context::new());
                metadata.subtitle_tracks.push(pb::SubtitleTrack {
                    track_number: track.track_number().get(),
                    track_uid: track.track_uid().get(),
                    enabled: track.flag_enabled(),
                    default: track.flag_default(),
                    commentary: track.flag_commentary(),
                    original: track.flag_original(),
                    visual_impaired: track.flag_visual_impaired(),
                    name: track.name().map(String::from),
                    language: track.language().map(String::from),
                    codec_id: String::from(track.codec_id()),
                    hash: Vec::new(), // Will be overwritten after frame processing
                });
            }
            _ => {}
        }
    }

    // Get ideal subtitle track for analysis
    let st_track = if st_track_number == 0 {
        get_subtitle_track(mkv_file.tracks())?
    } else {
        mkv_file
            .tracks()
            .iter()
            .filter(StContext::supported_tracks)
            .find(|track| track.track_number().get() == st_track_number)
    };
    let st_track_number = st_track.map(|track| track.track_number().get());
    let mut st_ctx = match st_track {
        Some(st_track) => Some(StContext::new(st_track, partess_cache)?),
        None => None,
    };

    // Frame processing loop
    let mut frame = Frame::default();
    while mkv_file.next_frame(&mut frame)? {
        frame.timestamp = frame.timestamp * timestamp_scale;
        frame.duration = frame.duration.map(|duration| duration * timestamp_scale);
        // Process track
        if let Some(track_hasher) = track_hashers.get_mut(&frame.track) {
            track_hasher.consume(&frame.data)
        }
        if Some(frame.track) == st_track_number {
            // Process subtitles
            if let Some(ref mut st_ctx) = st_ctx {
                st_ctx.process_frame(&mut frame)?;
            }
        }
    }

    // Collect hashes
    for track in metadata.video_tracks.iter_mut() {
        if let Some(hasher) = track_hashers.remove(&track.track_number) {
            track.hash = hasher.compute().to_vec();
        }
    }
    for track in metadata.audio_tracks.iter_mut() {
        if let Some(hasher) = track_hashers.remove(&track.track_number) {
            track.hash = hasher.compute().to_vec();
        }
    }
    for track in metadata.subtitle_tracks.iter_mut() {
        if let Some(hasher) = track_hashers.remove(&track.track_number) {
            track.hash = hasher.compute().to_vec();
        }
    }

    return Ok(pb::AnalyzeMkvResponse {
        media_details: Some(metadata),
        aggregated_subtitles: match st_ctx {
            Some(st_ctx) => Some(pb::AggregatedSubtitles {
                subtitles: format_subtitles_srt(st_ctx.collect()?, duration),
                track_number: st_track_number.unwrap(),
            }),
            None => None,
        },
    });
}
