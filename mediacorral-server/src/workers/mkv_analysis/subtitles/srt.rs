use super::Subtitle;

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
