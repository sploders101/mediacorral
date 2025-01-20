use std::{
    ffi::OsStr,
    os::{fd::AsFd, unix::fs::MetadataExt},
    time::Duration,
};

use async_stream::stream;
use futures::Stream;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use udev::Device;

pub async fn get_disc_name(device: &str) -> Option<String> {
    let devnum = tokio::fs::metadata(device).await.ok()?.rdev();
    return tokio::task::spawn_blocking(move || {
        let device = Device::from_devnum(udev::DeviceType::Block, devnum).ok()?;
        return device
            .property_value("ID_FS_LABEL")
            .and_then(|label| label.to_str())
            .map(String::from);
    })
    .await
    .ok()?;
}

pub struct DiscInsert {
    pub device: String,
    pub disc_name: String,
}

pub fn disc_insert_events() -> impl Stream<Item = DiscInsert> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(10);
    // Using unwraps here because it won't escape the thread. Maybe I'll improve this later.
    std::thread::spawn(move || {
        let watcher = udev::MonitorBuilder::new().unwrap().listen().unwrap();
        'thread_loop: loop {
            if sender.is_closed() {
                break;
            }
            poll(
                &mut [PollFd::new(watcher.as_fd(), PollFlags::POLLIN)],
                // Specify a timeout so we can periodically check if anyone is even listening
                PollTimeout::try_from(Duration::from_secs(5))
                    .expect("Invalid poll timeout constant"),
            )
            .unwrap();
            for item in watcher.iter() {
                let device = item.device();
                if device.property_value("ID_CDROM") == Some(&OsStr::new("1")) {
                    // Device is a disc
                    if let Some(label) = device.property_value("ID_FS_LABEL") {
                        match (
                            device.devnode().and_then(|node| node.to_str()),
                            label.to_str(),
                        ) {
                            (Some(device), Some(label)) => {
                                if let Err(_) = sender.blocking_send(DiscInsert {
                                    device: String::from(device),
                                    disc_name: String::from(label),
                                }) {
                                    break 'thread_loop;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    return stream! {
        while let Some(insert) = receiver.recv().await {
            yield insert;
        }
    };
}
