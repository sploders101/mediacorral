use std::ops::Deref;

use image::{GrayAlphaImage, GrayImage, Pixel, RgbaImage};
use matroska_demuxer::{Frame, TrackEntry, TrackType};
use pgs::processor::PgsProcessor;
use vobsub::{PartessCache, VobsubProcessor};

use super::ExtractDetailsError;

pub mod ocr;
pub mod pgs;
pub mod utils;
pub mod vobsub;

pub fn get_subtitle_track(
    tracks: &[TrackEntry],
) -> Result<Option<&TrackEntry>, ExtractDetailsError> {
    let candidates: Vec<_> = tracks
        .into_iter()
        .filter(StContext::supported_tracks)
        .filter(|track| track.track_type() == TrackType::Subtitle)
        .filter(|track| track.flag_enabled())
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

pub enum StContext {
    Subrip(Vec<Subtitle>),
    Vobsub(VobsubProcessor),
    Pgs(PgsProcessor),
}
impl StContext {
    #[inline]
    pub fn supported_tracks(track: &&TrackEntry) -> bool {
        return matches!(track.codec_id(), "S_VOBSUB" | "S_SUBRIP" | "S_HDMV/PGS")
            && matches!(
                track.language_bcp47().or(track.language()),
                Some("eng") | Some("en") | Some("en-US") | Some("en-GB")
            );
    }

    pub fn new(
        st_track: &TrackEntry,
        partess_cache: &PartessCache,
    ) -> Result<Self, ExtractDetailsError> {
        return Ok(match st_track.codec_id() {
            "S_SUBRIP" => StContext::Subrip(Vec::new()),
            "S_VOBSUB" => StContext::Vobsub(VobsubProcessor::new(
                partess_cache,
                "eng",
                st_track.codec_private().unwrap_or(&[]),
            )?),
            "S_HDMV/PGS" => StContext::Pgs(PgsProcessor::new(partess_cache, "eng")?),
            // Other codecs should be filtered out above
            _ => unreachable!(),
        });
    }

    pub fn process_frame(&mut self, frame: &mut Frame) -> Result<(), ExtractDetailsError> {
        match self {
            Self::Subrip(subs) => subs.push(Subtitle {
                timestamp: frame.timestamp / 1000,
                duration: frame.duration.map(|duration| duration / 1000),
                data: String::from_utf8(std::mem::take(&mut frame.data))
                    .map_err(|_| ExtractDetailsError::SubripInvalidUtf8)?,
            }),
            Self::Vobsub(vobs) => vobs.push_frame(
                frame.timestamp / 1000,
                frame.duration.map(|duration| duration / 1000),
                std::mem::take(&mut frame.data),
            ),
            Self::Pgs(processor) => processor.push_frame(&frame)?,
        }
        return Ok(());
    }

    pub fn collect(self) -> Result<Vec<Subtitle>, ExtractDetailsError> {
        match self {
            Self::Subrip(subs) => Ok(subs),
            Self::Vobsub(vobs) => vobs.collect(),
            Self::Pgs(processor) => processor.collect(),
        }
    }
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

pub fn process_graya_image(image: GrayAlphaImage) -> GrayImage {
    let mut gray_image: GrayImage = GrayImage::new(image.width(), image.height());
    for (src_pixel, dest_pixel) in image.pixels().zip(gray_image.pixels_mut()) {
        if src_pixel.0[1] == 0 {
            dest_pixel.0 = [255];
            continue;
        }
        dest_pixel.0 = [255 - src_pixel.0[0]];
    }
    return gray_image;
}

pub fn process_rgba_image(image: RgbaImage) -> GrayImage {
    let mut gray_image: GrayImage = GrayImage::new(image.width(), image.height());
    for (src_pixel, dest_pixel) in image.pixels().zip(gray_image.pixels_mut()) {
        if src_pixel.0[3] == 0 {
            dest_pixel.0 = [255];
            continue;
        }
        let luminance = src_pixel.to_luma().0[0];
        dest_pixel.0 = [255 - luminance];
    }
    return gray_image;
}
