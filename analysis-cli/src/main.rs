mod rayon_helpers;
mod utils;

use std::{
    fs::File,
    io::{Read, stdin, stdout},
    path::PathBuf,
};

use clap::{Parser, Subcommand};

use utils::{
    extract_details,
    subtitles::{ocr::PartessCache, srt::parse_srt_file},
};

#[derive(Parser, Debug, Clone)]
/// This is a collection of utilities for processing media files in mediacorral.
struct Args {
    #[command(subcommand)]
    command: McUtilsCommand,
}

#[derive(Subcommand, Debug, Clone)]
enum McUtilsCommand {
    /// Analyzes an MKV file and outputs a variety of metadata in JSON.
    ///
    /// This function also extracts subtitles and uses tesseract to convert graphical
    /// subtitles into SRT format.
    AnalyzeMkv {
        #[arg()]
        /// Path to the file that should be processed
        file: PathBuf,
    },
    /// Converts SRT subtitles into JSON
    Srt2json {
        #[arg()]
        /// Path to the file that should be processed. Passing `-` will read from stdin
        file: String,
    },
}

fn main() {
    let args = Args::parse();

    return match args.command {
        McUtilsCommand::AnalyzeMkv { file } => {
            let file = File::open(file).expect("Could not open file");
            let partess_cache = PartessCache::new();
            let details = extract_details(file, &partess_cache).expect("Could not analyze file");
            let stdout = stdout().lock();
            let _ = serde_json::to_writer(stdout, &details);
        }
        McUtilsCommand::Srt2json { file } => {
            let mut subtitles = String::new();
            if file == "-" {
                stdin()
                    .lock()
                    .read_to_string(&mut subtitles)
                    .expect("Failed to read subtitles from stdin");
            } else {
                File::open(file)
                    .expect("Failed to open file")
                    .read_to_string(&mut subtitles)
                    .expect("Failed to read subtitles from file");
            }
            let result = parse_srt_file(subtitles.lines()).expect("Failed to parse subtitles");
            let mut stdout = stdout().lock();
            let _ = serde_json::to_writer(&mut stdout, &result);
        }
    };
}
