#![allow(clippy::single_match)]

use event::Event;
use geometry::WindowRect;
use image::DynamicImage;
use items::{
    cached_get_item_identifiers, cached_items_and_sets, items::Item, CacheError, ReqwestSerdeError,
};
use log_watcher::{watcher, LogEntry};
use reward_session::{run_reward_session, CaptureSource, REWARD_PICK_WINDOW};
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use tokio::{fs::create_dir_all, sync::mpsc::Sender};
use tracing::*;
use xcap::{Window, XCapError};

pub mod config;
pub mod geometry;
pub mod items;
pub mod log_watcher;
pub mod event;
pub mod ocr;
pub mod relic_screen_parser;
pub mod reward_session;

pub struct WindowCapture(pub Window);

impl CaptureSource for WindowCapture {
    fn capture(&self) -> anyhow::Result<DynamicImage> {
        Ok(DynamicImage::ImageRgba8(self.0.capture_image()?))
    }
}

/// build a [`WindowRect`] from the live window geometry; `None` if any of the
/// position/size queries fail so frontends fall back to monitor-sized layout
fn window_rect(window: &Window) -> Option<WindowRect> {
    Some(WindowRect {
        x: window.x().ok()?,
        y: window.y().ok()?,
        width: window.width().ok()?,
        height: window.height().ok()?,
    })
}

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

    pub async fn run(self, sender: Sender<Event>) -> Result<(), EngineRunError> {
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
                let capture = WindowCapture(warframe);
                while let Some(amount) = rx.recv().await {
                    event!(Level::INFO, "relic screen parser activated");
                    run_reward_session(
                        &capture,
                        &self.items,
                        &sender,
                        amount,
                        window_rect(&capture.0),
                        REWARD_PICK_WINDOW,
                    )
                    .await;
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
