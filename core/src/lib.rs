#![allow(clippy::single_match)]

use futures::{stream::FuturesUnordered, StreamExt};
use image::DynamicImage;
use items::{
    cached_get_item_identifiers, cached_items_and_sets, items::Item, CacheError, ReqwestSerdeError,
};
use log_watcher::{watcher, LogEntry};
use relic_screen_parser::{parse_relic_screen, ItemOrForma};
use state::State;
use std::{collections::HashMap, path::PathBuf, time::Duration};
use thiserror::Error;
use tokio::{fs::create_dir_all, sync::mpsc::Sender, time::sleep};
use tracing::*;
use xcap::{Window, XCapError};

pub mod config;
pub mod items;
pub mod log_watcher;
pub mod ocr;
pub mod relic_screen_parser;
pub mod state;

pub struct Engine {
    items: HashMap<String, Item>,
}

#[derive(Error, Debug)]
pub enum EngineCreateError {
    #[error("create cache path error")]
    CreateCachePathError(#[from] std::io::Error),
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
    pub async fn new(cache_path: PathBuf) -> Result<Self, EngineCreateError> {
        create_dir_all(&cache_path).await?;
        let item_identifiers = cached_get_item_identifiers(&cache_path).await?;
        let (items, _sets) = cached_items_and_sets(&cache_path, &item_identifiers).await?;
        Ok(Self { items })
    }

    pub async fn run(self, sender: Sender<State>) -> Result<(), EngineRunError> {
        let windows = Window::all()?;
        let warframe = windows
            .into_iter()
            .find(|x| x.title().unwrap_or_default() == "Warframe")
            .ok_or(EngineRunError::WarframeNotRunning)?;
        {
            let image = warframe.capture_image().unwrap();
            let image = DynamicImage::ImageRgba8(image);
            debug_write_image(&image, "initial");
        }

        let mut squad_size = 4;

        let mut reciever = watcher().await;

        let relic_screen_enabler = {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<usize>(100);

            tokio::spawn(async move {
                while let Some(amount) = rx.recv().await {
                    event!(Level::INFO, "relic screen parser activated");
                    let mut total_results = (0..amount)
                        .map(|_| None)
                        .collect::<Vec<Option<ItemOrForma>>>();
                    for i in 0..10 {
                        event!(Level::INFO, "relic screen run {i}");
                        sleep(Duration::from_millis(1000)).await;
                        let image = warframe.capture_image().unwrap();
                        let image = DynamicImage::ImageRgba8(image);
                        debug_write_image(&image, &format!("reward_capture_{i}"));
                        let results = parse_relic_screen(
                            &image,
                            &total_results
                                .iter()
                                .enumerate()
                                .filter(|(_, x)| x.is_none())
                                .map(|(i, _)| i)
                                .collect(),
                            &self.items,
                        )
                        .await;
                        total_results = total_results
                            .into_iter()
                            .zip(results.into_iter())
                            .map(|(a, b)| match (a, b) {
                                (Some(x), _) => Some(x),
                                (_, Some(x)) => Some(x),
                                _ => None,
                            })
                            .collect();
                        let finished =
                            total_results.iter().filter(|x| x.is_some()).count() == amount;
                        let _ = sender
                            .send(State {
                                relic_rewards: total_results
                                    .iter()
                                    .map(|x| async move {
                                        let x = match x.as_ref()? {
                                            ItemOrForma::Item(x) => Some(x),
                                            ItemOrForma::Forma1X => None,
                                            ItemOrForma::Forma2X => None,
                                        }?;
                                        Some((x.clone(), x.price().await.ok()?))
                                    })
                                    .collect::<FuturesUnordered<_>>()
                                    .collect::<Vec<_>>()
                                    .await,
                            })
                            .await;

                        if finished {
                            event!(Level::INFO, "relic screen run found all, finishing early");
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
                            event!(Level::INFO, "Running relic screen parser");
                            let _ = relic_screen_enabler.send(squad_size).await;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                LogEntry::NetInfo(x) if x == "Num session players: 1" => squad_size = 1,
                LogEntry::NetInfo(x) if x == "Num session players: 2" => squad_size = 2,
                LogEntry::NetInfo(x) if x == "Num session players: 3" => squad_size = 3,
                LogEntry::NetInfo(x) if x == "Num session players: 4" => squad_size = 4,
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(debug_assertions)]
pub(crate) fn debug_write_image(img: &DynamicImage, name: &str) {
    std::fs::create_dir_all("debug_img_out").unwrap();
    img.save(format!("debug_img_out/{name}.png")).unwrap();
}

#[cfg(not(debug_assertions))]
pub(crate) fn debug_write_image(_img: &DynamicImage, _name: &str) {}
