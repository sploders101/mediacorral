use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ItemType {
    Disc,
    Title,
    Stream,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ItemAttribute {
    Unknown,
    Type,
    Name,
    LangCode,
    LangName,
    CodecId,
    CodecShort,
    CodecLong,
    ChapterCount,
    Duration,
    DiskSize,
    DiskSizeBytes,
    StreamTypeExtension,
    Bitrate,
    AudioChannelsCount,
    AngleInfo,
    SourceFileName,
    AudioSampleRate,
    AudioSampleSize,
    VideoSize,
    VideoAspectRatio,
    VideoFrameRate,
    StreamFlags,
    DateTime,
    OriginalTitleId,
    SegmentsCount,
    SegmentsMap,
    OutputFileName,
    MetadataLanguageCode,
    MetadataLanguageName,
    TreeInfo,
    PanelTitle,
    VolumeName,
    OrderWeight,
    OutputFormat,
    OutputFormatDescription,
    SeamlessInfo,
    PanelText,
    MkvFlags,
    MkvFlagsText,
    AudioChannelLayoutName,
    OutputCodecShort,
    OutputConversionType,
    OutputAudioSampleRate,
    OutputAudioSampleSize,
    OutputAudioChannelsCount,
    OutputAudioChannelLayoutName,
    OutputAudioChannelLayout,
    OutputAudioMixDescription,
    Comment,
    OffsetSequenceId,
}
impl From<usize> for ItemAttribute {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Type,
            2 => Self::Name,
            3 => Self::LangCode,
            4 => Self::LangName,
            5 => Self::CodecId,
            6 => Self::CodecShort,
            7 => Self::CodecLong,
            8 => Self::ChapterCount,
            9 => Self::Duration,
            10 => Self::DiskSize,
            11 => Self::DiskSizeBytes,
            12 => Self::StreamTypeExtension,
            13 => Self::Bitrate,
            14 => Self::AudioChannelsCount,
            15 => Self::AngleInfo,
            16 => Self::SourceFileName,
            17 => Self::AudioSampleRate,
            18 => Self::AudioSampleSize,
            19 => Self::VideoSize,
            20 => Self::VideoAspectRatio,
            21 => Self::VideoFrameRate,
            22 => Self::StreamFlags,
            23 => Self::DateTime,
            24 => Self::OriginalTitleId,
            25 => Self::SegmentsCount,
            26 => Self::SegmentsMap,
            27 => Self::OutputFileName,
            28 => Self::MetadataLanguageCode,
            29 => Self::MetadataLanguageName,
            30 => Self::TreeInfo,
            31 => Self::PanelTitle,
            32 => Self::VolumeName,
            33 => Self::OrderWeight,
            34 => Self::OutputFormat,
            35 => Self::OutputFormatDescription,
            36 => Self::SeamlessInfo,
            37 => Self::PanelText,
            38 => Self::MkvFlags,
            39 => Self::MkvFlagsText,
            40 => Self::AudioChannelLayoutName,
            41 => Self::OutputCodecShort,
            42 => Self::OutputConversionType,
            43 => Self::OutputAudioSampleRate,
            44 => Self::OutputAudioSampleSize,
            45 => Self::OutputAudioChannelsCount,
            46 => Self::OutputAudioChannelLayoutName,
            47 => Self::OutputAudioChannelLayout,
            48 => Self::OutputAudioMixDescription,
            49 => Self::Comment,
            50 => Self::OffsetSequenceId,
            _ => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProgressBar {
    Current,
    Total,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MakemkvMessage {
    Message {
        message: String,
    },
    ProgressTitle {
        bar: ProgressBar,
        code: i32,
        id: i32,
        name: String,
    },
    ProgressValue {
        current: usize,
        total: usize,
        max: usize,
    },
    DriveScan {
        index: i32,
        visible: i32,
        enabled: i32,
        flags: i32,
        drive_name: String,
        disc_name: String,
        device_path: Option<String>,
    },
    TitleCount {
        count: usize,
    },
    DiscInfo {
        item: ItemType,
        id: ItemAttribute,
        code: i32,
        value: String,
    },
}
impl MakemkvMessage {
    /// Parses a message from `makemkvcon`.
    ///
    /// This function attempts to parse a message from makemkvcon, returning
    /// `None` if it didn't recognize the message. This will follow their spec
    /// as defined at https://www.makemkv.com/developers/usage.txt as closely
    /// as possible, but it doesn't seem to be entirely accurate, so expect
    /// missing data and variations from the spec.
    pub fn from_iter(mut iter: impl Iterator<Item = String>) -> Option<Self> {
        let ident = iter.next()?;
        let mut ident_split = ident.split(':');
        let msg_type = ident_split.next()?;

        match msg_type {
            "MSG" => {
                let _code = ident_split.next()?;
                let _flags = iter.next()?;
                let _count = iter.next()?;
                let message = iter.next()?;
                // let _format = iter.next()?;
                // let _param_1 = iter.next()?;

                return Some(Self::Message { message });
            }
            "PRGC" | "PRGT" => {
                let code = ident_split.next()?.parse().ok()?;
                let id = iter.next()?.parse().ok()?;
                let name = iter.next()?;
                return Some(Self::ProgressTitle {
                    bar: match msg_type {
                        "PRGC" => ProgressBar::Current,
                        "PRGT" => ProgressBar::Total,
                        _ => unreachable!(),
                    },
                    code,
                    id,
                    name,
                });
            }
            "PRGV" => {
                let current = ident_split.next()?.parse().ok()?;
                let total = iter.next()?.parse().ok()?;
                let max = iter.next()?.parse().ok()?;
                return Some(Self::ProgressValue {
                    current,
                    total,
                    max,
                });
            }
            "DRV" => {
                let index = ident_split.next()?.parse().ok()?;
                let visible = iter.next()?.parse().ok()?;
                let enabled = iter.next()?.parse().ok()?;
                let flags = iter.next()?.parse().ok()?;
                let drive_name = iter.next()?;
                let disc_name = iter.next()?;
                let device_path = iter.next();
                return Some(Self::DriveScan {
                    index,
                    visible,
                    enabled,
                    flags,
                    drive_name,
                    disc_name,
                    device_path,
                });
            }
            "TCOUT" => {
                let count = ident_split.next()?.parse().ok()?;
                return Some(Self::TitleCount { count });
            }
            "CINFO" | "TINFO" | "SINFO" => {
                let id: usize = ident_split.next()?.parse().ok()?;
                let code = iter.next()?.parse().ok()?;
                let value = iter.next()?;
                return Some(Self::DiscInfo {
                    item: match msg_type {
                        "CINFO" => ItemType::Disc,
                        "TINFO" => ItemType::Title,
                        "SINFO" => ItemType::Stream,
                        _ => unreachable!(),
                    },
                    id: ItemAttribute::try_from(id).ok()?,
                    code,
                    value,
                });
            }
            _ => {
                return None;
            }
        }
    }
}
