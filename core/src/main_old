use std::time::Duration;

use anyhow::anyhow;
use image::DynamicImage;
use log_watcher::{watcher, LogEntry};
use ocr::ocr;
use relic_screen_parser::parse_relic_screen;
use tokio::time::sleep;
use xcap::Window;

pub mod config;
pub mod items;
pub mod log_watcher;
pub mod ocr;
pub mod relic_screen_parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let windows = Window::all()?;
    let warframe = windows
        .into_iter()
        .find(|x| x.title() == "Warframe")
        .ok_or(anyhow!("warframe not running"))?;
    let image = warframe.capture_image().unwrap();
    let image = DynamicImage::ImageRgba8(image);
    image.save("initial_test.png").unwrap();
    let mut reciever = watcher().await;

    while let Some(entry) = reciever.recv().await {
        match entry {
            LogEntry::ScriptInfo { script, content } => match script.as_str() {
                "ProjectionRewardChoice" => match content.as_str() {
                    "Relic rewards initialized" => {
                        println!("At reward screen");
                        println!("Waiting a bit");
                        sleep(Duration::from_millis(1500)).await;
                        let image = warframe.capture_image().unwrap();
                        let image = DynamicImage::ImageRgba8(image);
                        image.save("reward_capture.png").unwrap();
                        let results = parse_relic_screen(&image, 4).await;
                        println!("{results:?}");
                    }
                    "Got rewards" => {
                        println!("Finished reward screen")
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}
