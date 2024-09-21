#![allow(clippy::single_match)]

use futures::{stream::FuturesUnordered, StreamExt};
use image::DynamicImage;
use items::{
    cached_get_item_identifiers, cached_items_and_sets, items::Item, CacheError, ReqwestSerdeError,
};
use log_watcher::{watcher, LogEntry};
use relic_screen_parser::parse_relic_screen;
use state::State;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use thiserror::Error;
use tokio::{fs::create_dir_all, sync::mpsc::Sender, time::sleep};
use xcap::{Window, XCapError};

pub mod config;
pub mod items;
pub mod log_watcher;
pub mod ocr;
pub mod relic_screen_parser;
pub mod state;

pub struct Engine {
    tesseract_path: PathBuf,
    items: HashMap<String, Item>,
    debug: bool,
}

#[derive(Error, Debug)]
pub enum EngineCreateError {
    #[error("create cache path error")]
    CreateCachePathError(#[from] std::io::Error),
    #[error("invalid tesseract")]
    InvalidTesseract,
    #[error("create cache path error")]
    FetchError(#[from] CacheError<ReqwestSerdeError>),
}

#[derive(Error, Debug)]
pub enum EngineRunError {
    #[error("warframe not running")]
    WarframeNotRunning,
    #[error("xcap error")]
    XCapError(#[from] XCapError),
}

impl Engine {
    pub async fn new(
        tesseract_path: PathBuf,
        cache_path: PathBuf,
        debug: bool,
    ) -> Result<Self, EngineCreateError> {
        let (valid_path, cache_path_status) = tokio::join!(
            ocr::validate_path(&tesseract_path),
            create_dir_all(&cache_path)
        );
        cache_path_status?;
        if !valid_path {
            return Err(EngineCreateError::InvalidTesseract);
        }
        let item_identifiers = cached_get_item_identifiers(&cache_path).await?;
        let (items, _sets) = cached_items_and_sets(&cache_path, &item_identifiers).await?;
        Ok(Self {
            tesseract_path,
            items,
            debug,
        })
    }

    pub async fn run(self, sender: Sender<State>) -> Result<(), EngineRunError> {
        let windows = Window::all()?;
        let warframe = windows
            .into_iter()
            .find(|x| x.title() == "Warframe")
            .ok_or(EngineRunError::WarframeNotRunning)?;
        if self.debug {
            let image = warframe.capture_image().unwrap();
            let image = DynamicImage::ImageRgba8(image);
            image.save("initial_test.png").unwrap();
        }

        let mut reciever = watcher().await;

        let relic_screen_enabler = {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(100);

            let tesseract_path = self.tesseract_path.clone();
            let debug = self.debug;

            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    for i in 0..10 {
                        sleep(Duration::from_millis(1000)).await;
                        let image = warframe.capture_image().unwrap();
                        let image = DynamicImage::ImageRgba8(image);
                        if debug {
                            image.save(format!("reward_capture{i}.png")).unwrap();
                        }
                        let results =
                            parse_relic_screen(&image, 4, &tesseract_path, &self.items).await;
                        let finished = results.iter().filter(|x| x.is_some()).count() == 4;
                        let _ = sender
                            .send(State {
                                relic_rewards: results
                                    .into_iter()
                                    .map(|x| async move {
                                        let x = x?;
                                        Some((x.clone(), x.price().await.ok()?))
                                    })
                                    .collect::<FuturesUnordered<_>>()
                                    .collect::<Vec<_>>()
                                    .await,
                            })
                            .await;

                        if finished {
                            break;
                        }
                    }
                }
            });

            tx
        };

        while let Some(entry) = reciever.recv().await {
            match entry {
                LogEntry::ScriptInfo { script, content } => match script.as_str() {
                    "ProjectionRewardChoice" => match content.as_str() {
                        "Relic rewards initialized" => {
                            let _ = relic_screen_enabler.send(()).await;
                            println!("Got reward screen")
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
}
