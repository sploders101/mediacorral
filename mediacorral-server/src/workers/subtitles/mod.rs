use std::io::{Read, Seek};

use image::{GrayImage, Pixel, RgbaImage};
use leptess::Variable;
use matroska_demuxer::{Frame, MatroskaFile, TrackType};
use ocr::{Partess, PartessError};
use tokio::sync::watch;
use utils::crop_image;
use vobsub::VobsubError;

mod ocr;
mod utils;
mod vobsub;

#[derive(thiserror::Error, Debug)]
pub enum ExtractSubtitlesError {
    #[error("An I/O error occurred:\n{0}")]
    Io(#[from] std::io::Error),
    #[error("The subrip subtitles are not valid UTF-8")]
    SubripInvalidUtf8,
    #[error("An error occurred while reading VobSub subtitles:\n{0}")]
    VobsubError(#[from] VobsubError),
    #[error("An error occurred while demuxing:\n{0}")]
    DemuxError(#[from] matroska_demuxer::DemuxError),
    #[error("No suitable subtitle tracks found.")]
    NoSuitableSubtitles,
    #[error("An error occurred while running OCR:\n{0}")]
    PartessError(#[from] PartessError),
}

pub async fn extract_subtitles<T>(
    mkv_file: T,
    progress: Option<watch::Sender<f64>>,
) -> Result<String, ExtractSubtitlesError>
where
    T: Read + Seek,
{
    // TODO: Add progress reporting
    let mut mkv_file = MatroskaFile::open(mkv_file)?;

    let candidates: Vec<_> = mkv_file
        .tracks()
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
    let track = match candidates.len() {
        0 => return Err(ExtractSubtitlesError::NoSuitableSubtitles),
        1 => candidates.into_iter().next().unwrap(),
        _ => {
            // Found multiple valid candidates. See if we can narrow down any further by
            // filtering on default tracks. If not, just take the first candidate we found.
            match candidates
                .iter()
                .cloned()
                .find(|track| track.flag_default())
            {
                Some(track) => track,
                None => candidates.into_iter().next().unwrap(),
            }
        }
    };

    // mkv_file.info.duration

    let track_number = track.track_number().get();
    let subs = match track.codec_id() {
        "S_SUBRIP" => {
            let mut subs: Vec<String> = Vec::new();
            let mut frame = Frame::default();
            while mkv_file.next_frame(&mut frame)? {
                if frame.track != track_number {
                    continue;
                }

                let data = String::from_utf8(std::mem::take(&mut frame.data))
                    .map_err(|_| ExtractSubtitlesError::SubripInvalidUtf8)?;
                subs.push(data);
            }
            subs
        }
        "S_VOBSUB" => {
            let partess = Partess::new(
                String::from("eng"),
                vec![
                    (Variable::ClassifyEnableLearning, String::from("1")),
                    (Variable::TesseditPagesegMode, String::from("6")),
                    (Variable::TesseditDoInvert, String::from("0")),
                    (Variable::TesseditCharBlacklist, String::from("|\\/`_~{}")),
                ],
            );
            let codec_private = track
                .codec_private()
                .ok_or(ExtractSubtitlesError::VobsubError(VobsubError::InvalidIdx))?;
            let idx_data = vobsub::parse_idx(codec_private)?;
            let mut subs: Vec<String> = Vec::new();
            let mut frame = Frame::default();
            while mkv_file.next_frame(&mut frame)? {
                if frame.track != track_number {
                    continue;
                }

                let image: GrayImage = process_image(vobsub::parse_frame(&idx_data, &frame.data)?);
                let sub = partess.get()?.ocr_image(image)?;
                subs.push(sub);
            }

            subs
        }
        // Other codecs should be filtered out above
        _ => unreachable!(),
    };

    return Ok(subs.join("\n"));
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
