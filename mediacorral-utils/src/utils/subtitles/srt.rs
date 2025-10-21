#![allow(dead_code)]

use super::Subtitle;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SrtParseError {
    #[error("Timestamp format invalid")]
    InvalidTimestamp,
    #[error("Invalid sequence number found")]
    InvalidSequence,
    #[error("Got EOF while reading subtitle")]
    UnexpectedEof,
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
        formatted.push_str(subtitle.data.trim_end_matches("\n"));
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

/// Parses a single SRT timestamp (ie. 1 of 2 segments in a time range),
/// and returns a u64 in milliseconds.
fn parse_srt_timestamp(timestamp: &str) -> Result<u64, SrtParseError> {
    let mut ts_iter = timestamp.chars();
    let hours: u64 = (&mut ts_iter)
        .take_while(|char| *char != ':')
        .collect::<String>()
        .parse()
        .map_err(|_| SrtParseError::InvalidTimestamp)?;
    let minutes: u64 = (&mut ts_iter)
        .take_while(|char| *char != ':')
        .collect::<String>()
        .parse()
        .map_err(|_| SrtParseError::InvalidTimestamp)?;
    let seconds: u64 = (&mut ts_iter)
        .take_while(|char| *char != ',')
        .collect::<String>()
        .parse()
        .map_err(|_| SrtParseError::InvalidTimestamp)?;
    let milliseconds: u64 = (&mut ts_iter)
        .collect::<String>()
        .parse()
        .map_err(|_| SrtParseError::InvalidTimestamp)?;

    let timestamp = (hours * 3_600_000) + (minutes * 60_000) + (seconds * 1000) + milliseconds;

    return Ok(timestamp);
}

fn parse_srt_timerange(timerange: &str) -> Result<(u64, u64), SrtParseError> {
    let mut segment_iter = timerange.split_whitespace();
    let left_timestamp = segment_iter.next().ok_or(SrtParseError::InvalidTimestamp)?;
    let arrow = segment_iter.next().ok_or(SrtParseError::InvalidTimestamp)?;
    if arrow != "-->" {
        return Err(SrtParseError::InvalidTimestamp);
    }
    let right_timestamp = segment_iter.next().ok_or(SrtParseError::InvalidTimestamp)?;

    return Ok((
        parse_srt_timestamp(left_timestamp)?,
        parse_srt_timestamp(right_timestamp)?,
    ));
}

pub fn parse_srt_file<T: Iterator<Item = S>, S: AsRef<str>>(
    mut srt_lines: T,
) -> Result<Vec<Subtitle>, SrtParseError> {
    let mut subtitles = Vec::new();
    let mut last_seq = 0;

    while let Some(line) = srt_lines.next() {
        let line = line.as_ref();
        if line == "" {
            continue;
        }

        let sequence_number: usize = line
            .trim()
            .parse()
            .map_err(|_| SrtParseError::InvalidSequence)?;
        last_seq += 1;
        if sequence_number != last_seq {
            return Err(SrtParseError::InvalidSequence);
        }
        let timerange = srt_lines.next().ok_or(SrtParseError::UnexpectedEof)?;
        let (tr_start, tr_end) = parse_srt_timerange(timerange.as_ref().trim())?;
        let subtitle_str = (&mut srt_lines)
            .take_while(|i| i.as_ref().trim() != "")
            .enumerate()
            .fold(String::new(), |subs, (i, line)| {
                if i == 0 {
                    subs + line.as_ref().trim()
                } else {
                    subs + "\n" + line.as_ref().trim()
                }
            });
        subtitles.push(Subtitle {
            timestamp: tr_start,
            duration: Some(tr_end - tr_start),
            data: subtitle_str,
        });
    }

    return Ok(subtitles);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_srt_timestamp_1() {
        let result = parse_srt_timestamp("00:00:06,373").unwrap();
        assert_eq!(result, 6_373);
    }

    #[test]
    fn parse_srt_timestamp_2() {
        let result = parse_srt_timestamp("00:19:22,261").unwrap();
        assert_eq!(result, 1_162_261);
    }

    #[test]
    fn parse_srt_timestamp_3() {
        let result = parse_srt_timestamp("02:19:22,261").unwrap();
        assert_eq!(result, 8_362_261);
    }

    #[test]
    fn parse_srt_timestamp_4() {
        let result = parse_srt_timestamp("02,19:22,261");
        assert!(result.is_err());
    }

    #[test]
    fn parse_srt_timerange_1() {
        let result = parse_srt_timerange("00:00:06,373 --> 00:00:11,812").unwrap();
        assert_eq!(result, (6_373, 11_812));
    }

    #[test]
    fn parse_srt_1() {
        let data = r#"1
00:00:06,373 --> 00:00:11,812
Text 1

2
00:00:12,079 --> 00:00:14,104
Text 2

3
00:00:14,181 --> 00:00:16,707
Text 3

4
00:00:16,783 --> 00:00:18,376
Text 4"#;
        let subtitles = parse_srt_file(data.lines()).unwrap();
        assert_eq!(
            subtitles,
            vec![
                Subtitle {
                    timestamp: 6_373,
                    duration: Some(5_439),
                    data: String::from("Text 1"),
                },
                Subtitle {
                    timestamp: 12_079,
                    duration: Some(2_025),
                    data: String::from("Text 2"),
                },
                Subtitle {
                    timestamp: 14_181,
                    duration: Some(2_526),
                    data: String::from("Text 3"),
                },
                Subtitle {
                    timestamp: 16_783,
                    duration: Some(1_593),
                    data: String::from("Text 4"),
                },
            ]
        );
    }
}
