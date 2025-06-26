use std::{fs::File, time::Duration};
use tokio::sync::watch;
use workers::subtitles::{extract_details, vobsub::PartessCache};

mod workers;

#[tokio::main]
async fn main() {
    let file = std::env::args()
        .into_iter()
        .nth(1)
        .expect("Please specify a filename");
    let tess_cache = PartessCache::new();
    for _ in 0..2 {
        let file = File::open(&file).unwrap();
        let (sender, mut receiver) = watch::channel(0.0);
        let tess_cache = tess_cache.clone();
        let mut result_thread =
            tokio::task::spawn_blocking(move || extract_details(file, Some(sender), &tess_cache));
        let result = loop {
            tokio::select! {
                Ok(_) = receiver.changed() => {
                    let progress = *receiver.borrow();
                    dbg!(progress);
                }
                result = &mut result_thread => {
                    let result = result.unwrap();
                    break result;
                }
            }
        };
        let result = result.unwrap();
        println!("Subs:\n{}", result.subtitles.unwrap_or_default());
        println!("Video hash: {}", hex::encode(&result.video_hash));
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
