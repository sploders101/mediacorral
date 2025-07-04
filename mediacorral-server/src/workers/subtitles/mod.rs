use std::io::{Read, Seek};

use image::{GrayImage, Pixel, RgbaImage};
use matroska_demuxer::{Frame, MatroskaFile, TrackEntry, TrackType};
use ocr::PartessError;
use tokio::sync::watch;
use utils::crop_image;
use vobsub::{PartessCache, VobsubError, VobsubProcessor};

pub mod ocr;
pub mod utils;
pub mod vobsub;

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
}

fn get_subtitle_track(tracks: &[TrackEntry]) -> Result<Option<&TrackEntry>, ExtractDetailsError> {
    let candidates: Vec<_> = tracks
        .into_iter()
        .filter(|track| track.track_type() == TrackType::Subtitle)
        .filter(|track| track.flag_enabled())
        .filter(|track| matches!(track.codec_id(), "S_VOBSUB" | "S_SUBRIP"))
        .filter(|track| {
            track
                .language()
                .map(|lang| matches!(lang, "eng" | "en" | "en-US" | "en-GB"))
                .unwrap_or(false)
        })
        .collect();
    return Ok(match candidates.len() {
        0 => None,
        1 => Some(candidates.into_iter().next().unwrap()),
        _ => {
            // Found multiple valid candidates. See if we can narrow down any further by
            // filtering on default tracks. If not, just take the first candidate we found.
            match candidates
                .iter()
                .cloned()
                .find(|track| track.flag_default())
            {
                Some(track) => Some(track),
                None => Some(candidates.into_iter().next().unwrap()),
            }
        }
    });
}

pub struct Subtitle {
    timestamp: u64,
    duration: Option<u64>,
    data: String,
}

enum StContext {
    Subrip(Vec<Subtitle>),
    Vobsub(VobsubProcessor),
}
impl StContext {
    fn process_frame(
        &mut self,
        timestamp_scale: u64,
        frame: &mut Frame,
    ) -> Result<(), ExtractDetailsError> {
        match self {
            Self::Subrip(subs) => subs.push(Subtitle {
                timestamp: frame.timestamp * timestamp_scale,
                duration: frame.duration.map(|duration| duration * timestamp_scale),
                data: String::from_utf8(std::mem::take(&mut frame.data))
                    .map_err(|_| ExtractDetailsError::SubripInvalidUtf8)?,
            }),
            Self::Vobsub(vobs) => vobs.push_frame(
                frame.timestamp * timestamp_scale / 1000,
                frame.duration.map(|duration| duration * timestamp_scale),
                std::mem::take(&mut frame.data),
            ),
        }
        return Ok(());
    }

    fn collect(self) -> Result<Vec<Subtitle>, ExtractDetailsError> {
        match self {
            Self::Subrip(subs) => Ok(subs),
            StContext::Vobsub(vobs) => vobs.collect(),
        }
    }
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

    let vid_track = mkv_file
        .tracks()
        .into_iter()
        .find(|track| track.track_type() == TrackType::Video)
        .ok_or(ExtractDetailsError::MissingVideoTrack)?;
    let vid_track_info = vid_track.video().unwrap();
    let vid_track_number = vid_track.track_number().get();
    let mut vid_hasher = md5::Context::new();

    let st_track = get_subtitle_track(mkv_file.tracks())?;
    // let st_track = Option::<&TrackEntry>::None;
    let st_track_number = st_track.map(|track| track.track_number().get());
    let mut st_ctx = match st_track {
        Some(ref st_track) => Some(match st_track.codec_id() {
            "S_SUBRIP" => StContext::Subrip(Vec::new()),
            "S_VOBSUB" => StContext::Vobsub(VobsubProcessor::new(
                partess_cache,
                "eng",
                st_track.codec_private().unwrap_or(&[]),
            )?),
            // Other codecs should be filtered out above
            _ => unreachable!(),
        }),
        None => None,
    };

    let resolution_width: u32 = vid_track_info
        .display_width()
        .ok_or(ExtractDetailsError::MissingRequiredProps)?
        .get() as _;
    let resolution_height: u32 = vid_track_info
        .display_height()
        .ok_or(ExtractDetailsError::MissingRequiredProps)?
        .get() as _;

    let info = mkv_file.info();
    let timestamp_scale = info.timestamp_scale().get();
    let duration: u64 = match info.duration() {
        Some(duration) => duration.round() as u64 * timestamp_scale,
        None => return Err(ExtractDetailsError::MissingRequiredProps),
    };
    let duration_secs = duration / 1_000_000_000;
    if let Some(ref mut progress) = progress {
        let _ = progress.send(0.0);
    }

    let mut duration_tracker: u32 = 0;
    let mut frame = Frame::default();
    while mkv_file.next_frame(&mut frame)? {
        duration_tracker = duration_tracker.max(frame.timestamp as _);
        if frame.track == vid_track_number {
            // Process video
            vid_hasher.consume(&frame.data);
        } else if Some(frame.track) == st_track_number {
            // Process subtitles
            if let Some(ref mut st_ctx) = st_ctx {
                st_ctx.process_frame(timestamp_scale, &mut frame)?;
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
    });
}

/// Formats an iterator of subtitles as SRT text
pub fn format_subtitles_srt(
    subtitles: impl IntoIterator<Item = Subtitle>,
    duration: u64,
) -> String {
    let mut subtitles = subtitles.into_iter().enumerate().peekable();
    let mut formatted = String::new();
    while let Some((seq, subtitle)) = subtitles.next() {
        if seq != 0 {
            formatted.push_str("\n\n");
        }
        formatted.push_str(&(seq + 1).to_string());
        let start_time = subtitle.timestamp;
        let end_time = match subtitle.duration {
            Some(duration) => start_time + duration,
            None => match subtitles.peek() {
                Some((_i, subtitle)) => subtitle.timestamp,
                None => duration,
            },
        };
        formatted.push_str(&format!(
            "\n{} --> {}\n",
            format_srt_timestamp(start_time),
            format_srt_timestamp(end_time)
        ));
        formatted.push_str(&subtitle.data);
    }
    return formatted + "\n";
}

fn format_srt_timestamp(timestamp: u64) -> String {
    let timestamp_ms = timestamp / 1000;
    let ms = timestamp_ms % 1000;
    let timestamp_s = timestamp_ms / 1000;
    let s = timestamp_s % 60;
    let timestamp_m = timestamp_s / 60;
    let m = timestamp_m % 60;
    let timestamp_h = timestamp_m / 60;
    format!("{timestamp_h:02}:{m:02}:{s:02},{ms:03}")
}

pub fn process_image(image: RgbaImage) -> GrayImage {
    let cropped_image = crop_image(&image);
    drop(image);
    let mut gray_image: GrayImage = GrayImage::new(cropped_image.width(), cropped_image.height());
    for (src_pixel, dest_pixel) in cropped_image.pixels().zip(gray_image.pixels_mut()) {
        if src_pixel.0[3] == 0 {
            dest_pixel.0 = [255];
            continue;
        }
        let luminance = src_pixel.to_luma().0[0];
        dest_pixel.0 = [255 - luminance];
    }
    return gray_image;
}
