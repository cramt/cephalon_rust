use std::{collections::HashMap, time::Duration};

use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Instant},
};
use tracing::*;

use crate::{
    debug_write_image,
    event::{Event, RewardSlot},
    geometry::WindowRect,
    items::items::Item,
    relic_screen_parser::{parse_relic_screen, ItemOrForma},
};

/// how long the in-game reward pick window stays on screen
pub const REWARD_PICK_WINDOW: Duration = Duration::from_secs(15);

const MAX_ATTEMPTS: usize = 10;

pub trait CaptureSource: Send + Sync + 'static {
    fn capture(&self) -> anyhow::Result<DynamicImage>;
}

pub async fn run_reward_session(
    capture: &dyn CaptureSource,
    items: &HashMap<String, Item>,
    sender: &Sender<Event>,
    count: usize,
    window_rect: Option<WindowRect>,
    session_duration: Duration,
) {
    let started = Instant::now();
    let _ = sender
        .send(Event::RewardScreenOpened {
            count,
            window: window_rect,
        })
        .await;

    let mut total_results: Vec<Option<ItemOrForma>> = (0..count).map(|_| None).collect();
    for attempt in 0..MAX_ATTEMPTS {
        event!(Level::INFO, "relic screen run {attempt}");
        sleep(Duration::from_millis(1000)).await;
        let image = match capture.capture() {
            Ok(img) => img,
            Err(e) => {
                event!(Level::WARN, "capture failed mid-session: {e}");
                break;
            }
        };
        debug_write_image(&image, &format!("reward_capture_{attempt}"));
        let results = parse_relic_screen(
            &image,
            &total_results
                .iter()
                .enumerate()
                .filter(|(_, x)| x.is_none())
                .map(|(i, _)| i)
                .collect(),
            items,
        )
        .await;
        total_results = total_results
            .into_iter()
            .zip(results)
            .map(|(a, b)| a.or(b))
            .collect();
        let finished = total_results.iter().all(|x| x.is_some());

        let slots = total_results
            .iter()
            .map(|x| async move {
                match x {
                    None => RewardSlot::Pending,
                    Some(ItemOrForma::Forma1X) | Some(ItemOrForma::Forma2X) => RewardSlot::Forma,
                    Some(ItemOrForma::Item(item)) => RewardSlot::Item {
                        item: item.clone(),
                        price: item.price().await.ok(),
                    },
                }
            })
            .collect::<FuturesOrdered<_>>()
            .collect::<Vec<_>>()
            .await;
        let _ = sender.send(Event::RewardsResolved(slots)).await;

        if finished {
            event!(Level::INFO, "relic screen run found all, finishing early");
            break;
        }
    }

    // keep the overlay up for the whole pick window even if OCR finished early
    if let Some(rest) = session_duration.checked_sub(started.elapsed()) {
        sleep(rest).await;
    }
    let _ = sender.send(Event::RewardScreenClosed).await;
}
