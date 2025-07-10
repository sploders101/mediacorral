use image::{GrayAlphaImage, GrayImage};
use leptess::Variable;

use crate::{
    rayon_helpers::BackpressuredRayon,
    workers::subtitles::{
        ExtractDetailsError, Subtitle, ocr::Partess, process_graya_image, utils::crop_gray_image,
        vobsub::PartessCache,
    },
};

use super::{PgsError, PgsParser};

pub struct PgsProcessor {
    pgs_parser: PgsParser,
    rayon_pool: BackpressuredRayon<
        Box<
            dyn Fn(
                    (u64, Option<u64>, GrayAlphaImage),
                )
                    -> Result<Option<(u64, Option<u64>, String)>, ExtractDetailsError>
                + Send
                + Sync
                + 'static,
        >,
        (u64, Option<u64>, GrayAlphaImage),
        Result<Option<(u64, Option<u64>, String)>, ExtractDetailsError>,
    >,
}
impl PgsProcessor {
    pub fn new(partess_cache: &PartessCache, language: &str) -> Result<Self, ExtractDetailsError> {
        let mut cache = partess_cache.cache.lock().unwrap();
        let partess = match cache.get(language) {
            Some(partess) => partess.clone(),
            None => {
                let partess = Partess::new(
                    String::from(language),
                    vec![
                        // We want deterministic OCR for cross-referential analysis
                        (Variable::ClassifyEnableLearning, String::from("0")),
                        (Variable::TesseditPagesegMode, String::from("6")),
                        (Variable::TesseditDoInvert, String::from("0")),
                        (Variable::TesseditCharBlacklist, String::from("|\\/`_~{}")),
                    ],
                );
                cache.insert(String::from(language), partess.clone());
                partess
            }
        };
        drop(cache);
        return Ok(Self {
            pgs_parser: PgsParser::new(),
            rayon_pool: BackpressuredRayon::new(
                5,
                Box::new(move |(timestamp, duration, image)| {
                    let image = crop_gray_image(&image);
                    if image.width() == 0 || image.height() == 0 {
                        return Ok(None);
                    }
                    let image: GrayImage = process_graya_image(image);
                    let mut partess = partess.get()?;
                    let sub = partess.ocr_image(image)?;
                    return Ok(Some((timestamp, duration, sub)));
                }),
            ),
        });
    }
    pub fn push_frame(&mut self, frame: &matroska_demuxer::Frame) -> Result<(), PgsError> {
        if let Some(image) = self.pgs_parser.process_mkv_frame(frame)? {
            self.rayon_pool.push_data((
                frame.timestamp / 1000,
                frame.duration.map(|duration| duration / 1000),
                image,
            ));
        }
        return Ok(());
    }
    pub fn collect(self) -> Result<Vec<Subtitle>, ExtractDetailsError> {
        let mut subs: Vec<_> = self
            .rayon_pool
            .try_collect()?
            .into_iter()
            .filter_map(|item| item)
            .collect();
        subs.sort_by_key(|(timestamp, _duration, _sub)| *timestamp);
        return Ok(subs
            .into_iter()
            .map(|(timestamp, duration, data)| Subtitle {
                timestamp,
                duration,
                data,
            })
            .collect());
    }
}
