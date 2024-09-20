#![allow(clippy::single_match)]

use image::DynamicImage;
use log_watcher::{watcher, LogEntry};
use relic_screen_parser::parse_relic_screen;
use state::State;
use std::{path::PathBuf, time::Duration};
use thiserror::Error;
use tokio::{
    fs::create_dir_all,
    sync::{
        mpsc::Sender,
        oneshot::{self, error::TryRecvError},
    },
    time::sleep,
};
use xcap::{Window, XCapError};

pub mod config;
pub mod items;
pub mod log_watcher;
pub mod ocr;
pub mod relic_screen_parser;
pub mod state;

pub struct Engine {
    tesseract_path: PathBuf,
    cache_path: PathBuf,
    debug: bool,
}

#[derive(Error, Debug)]
pub enum EngineCreateError {
    #[error("create cache path error")]
    CreateCachePathError(#[from] std::io::Error),
    #[error("invalid tesseract")]
    InvalidTesseract,
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
        Ok(Self {
            tesseract_path,
            cache_path,
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
            let (tx, mut rx) = tokio::sync::mpsc::channel::<oneshot::Receiver<()>>(100);

            tokio::spawn(async move {
                while let Some(mut stopper) = rx.recv().await {
                    while matches!(stopper.try_recv(), Err(TryRecvError::Empty)) {
                        sleep(Duration::from_millis(300)).await;
                        let image = warframe.capture_image().unwrap();
                        let image = DynamicImage::ImageRgba8(image);
                        if self.debug {
                            image.save("reward_capture.png").unwrap();
                        }
                        let results = parse_relic_screen(&image, 4).await;
                        let finished = results.iter().filter(|x| x.is_some()).count() == 4;
                        let _ = sender
                            .send(State {
                                relic_rewards: results
                                    .into_iter()
                                    .map(|x| x.map(|y| (y.clone(), 1)))
                                    .collect(),
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

        let mut relic_screen_disabler = None;

        while let Some(entry) = reciever.recv().await {
            match entry {
                LogEntry::ScriptInfo { script, content } => match script.as_str() {
                    "ProjectionRewardChoice" => match content.as_str() {
                        "Relic rewards initialized" => {
                            let (oneshot_tx, oneshot_rx) = oneshot::channel::<()>();
                            let _ = relic_screen_enabler.send(oneshot_rx).await;
                            relic_screen_disabler = Some(oneshot_tx);
                        }
                        "Got rewards" => {
                            if let Some(disabler) = relic_screen_disabler {
                                let _ = disabler.send(());
                                relic_screen_disabler = None;
                            }
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
}
