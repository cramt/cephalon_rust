#![allow(clippy::single_match)]

use event::Event;
use geometry::WindowRect;
use image::DynamicImage;
use items::{
    cached_get_item_identifiers, cached_items_and_sets, items::Item, CacheError, ReqwestSerdeError,
};
use log_watcher::{watcher, LogEntry};
use reward_session::{run_reward_session, CaptureSource, REWARD_PICK_WINDOW};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::{fs::create_dir_all, sync::mpsc::Sender};
use tracing::*;
use xcap::Window;

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

pub struct MonitorCapture(xcap::Monitor);

impl CaptureSource for MonitorCapture {
    fn capture(&self) -> anyhow::Result<DynamicImage> {
        Ok(DynamicImage::ImageRgba8(self.0.capture_image()?))
    }
}

fn find_warframe_window() -> Option<Window> {
    Window::all()
        .ok()?
        .into_iter()
        .find(|x| x.title().unwrap_or_default() == "Warframe")
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

fn primary_monitor_capture() -> Option<MonitorCapture> {
    xcap::Monitor::all()
        .ok()?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .map(MonitorCapture)
}

pub struct Engine {
    items: Arc<HashMap<String, Item>>,
}

#[derive(Error, Debug)]
pub enum EngineCreateError {
    #[error("create cache path error")]
    CreateCachePathError(#[from] std::io::Error),
    #[error("create cache path error")]
    FetchError(#[from] CacheError<ReqwestSerdeError>),
}

impl Engine {
    pub async fn new(cache_path: PathBuf) -> Result<Self, EngineCreateError> {
        create_dir_all(&cache_path).await?;
        let item_identifiers = cached_get_item_identifiers(&cache_path).await?;
        let (items, _sets) = cached_items_and_sets(&cache_path, &item_identifiers).await?;
        Ok(Self {
            items: Arc::new(items),
        })
    }

    pub async fn run(self, sender: Sender<Event>) {
        let mut squad_size = 4;
        let mut receiver = watcher().await;

        while let Some(entry) = receiver.recv().await {
            match entry {
                LogEntry::ScriptInfo { script, content }
                    if script == "ProjectionRewardChoice"
                        && content == "Relic rewards initialized" =>
                {
                    event!(Level::INFO, "relic reward screen detected");
                    match find_warframe_window() {
                        Some(window) => {
                            // window rect in screen coords so frontends can position UI
                            // relative to the game window (half-ultrawide, second monitor)
                            let rect = window_rect(&window);
                            let capture = WindowCapture(window);
                            let items = self.items.clone();
                            let sender = sender.clone();
                            let count = squad_size;
                            tokio::spawn(async move {
                                run_reward_session(
                                    &capture,
                                    &items,
                                    &sender,
                                    count,
                                    rect,
                                    REWARD_PICK_WINDOW,
                                )
                                .await;
                            });
                        }
                        None => {
                            // no X11 window: warframe may be running as a native wayland
                            // client (PROTON_ENABLE_WAYLAND) where xcap can't enumerate
                            // windows. fall back to capturing the primary monitor —
                            // borderless game means the frame still contains the cards,
                            // and window=None tells frontends to assume monitor-sized.
                            match primary_monitor_capture() {
                                Some(capture) => {
                                    event!(
                                        Level::INFO,
                                        "no warframe window found, falling back to primary monitor capture"
                                    );
                                    let items = self.items.clone();
                                    let sender = sender.clone();
                                    let count = squad_size;
                                    tokio::spawn(async move {
                                        run_reward_session(
                                            &capture,
                                            &items,
                                            &sender,
                                            count,
                                            None,
                                            REWARD_PICK_WINDOW,
                                        )
                                        .await;
                                    });
                                }
                                None => event!(
                                    Level::WARN,
                                    "reward screen detected but no warframe window and no monitor to capture"
                                ),
                            }
                        }
                    }
                }
                LogEntry::NetInfo(x) if x == "Num session players: 1" => squad_size = 1,
                LogEntry::NetInfo(x) if x == "Num session players: 2" => squad_size = 2,
                LogEntry::NetInfo(x) if x == "Num session players: 3" => squad_size = 3,
                LogEntry::NetInfo(x) if x == "Num session players: 4" => squad_size = 4,
                _ => {}
            }
        }
    }
}

#[cfg(debug_assertions)]
pub(crate) fn debug_write_image(img: &DynamicImage, name: &str) {
    std::fs::create_dir_all("debug_img_out").unwrap();
    img.save(format!("debug_img_out/{name}.png")).unwrap();
}

#[cfg(not(debug_assertions))]
pub(crate) fn debug_write_image(_img: &DynamicImage, _name: &str) {}
