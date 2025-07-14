use image::{GrayAlphaImage, GrayImage, Pixel, RgbaImage};
use matroska_demuxer::{Frame, TrackEntry, TrackType};
use ocr::PartessCache;
use pgs::processor::PgsProcessor;
use vobsub::VobsubProcessor;

use super::ExtractDetailsError;

pub mod ocr;
pub mod pgs;
pub mod srt;
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

#[derive(Debug, Clone, Eq, PartialEq)]
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
