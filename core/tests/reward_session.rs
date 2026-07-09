use std::{collections::HashMap, env, path::Path, time::Duration};

use cephalon_rust_core::{
    event::{Event, RewardSlot},
    geometry::WindowRect,
    items::{cached_get_item_identifiers, cached_items_and_sets, items::Item},
    reward_session::{run_reward_session, CaptureSource},
};
use image::{DynamicImage, ImageReader};

struct StaticCapture(DynamicImage);

impl CaptureSource for StaticCapture {
    fn capture(&self) -> anyhow::Result<DynamicImage> {
        Ok(self.0.clone())
    }
}

async fn items() -> HashMap<String, Item> {
    let cache_path = env::var("CACHE_PATH").unwrap();
    let cache_path = Path::new(&cache_path);
    let identifiers = cached_get_item_identifiers(cache_path).await.unwrap();
    let (items, _) = cached_items_and_sets(cache_path, &identifiers).await.unwrap();
    items
}

#[tokio::test]
async fn full_session_event_sequence() {
    let img = ImageReader::open("test_rewards_screens/1.png")
        .unwrap()
        .decode()
        .unwrap();
    let window = WindowRect {
        x: 0,
        y: 0,
        width: img.width(),
        height: img.height(),
    };
    let capture = StaticCapture(img);
    let items = items().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

    run_reward_session(&capture, &items, &tx, 4, Some(window), Duration::from_secs(2)).await;
    drop(tx);

    let mut events = Vec::new();
    while let Some(e) = rx.recv().await {
        events.push(e);
    }

    assert_eq!(
        events.first(),
        Some(&Event::RewardScreenOpened {
            count: 4,
            window: Some(window),
        })
    );
    assert_eq!(events.last(), Some(&Event::RewardScreenClosed));

    let resolved = events
        .iter()
        .filter_map(|e| match e {
            Event::RewardsResolved(slots) => Some(slots),
            _ => None,
        })
        .last()
        .expect("at least one RewardsResolved event");

    let names = resolved
        .iter()
        .map(|s| match s {
            RewardSlot::Pending => "PENDING".to_string(),
            RewardSlot::Forma => "FORMA".to_string(),
            RewardSlot::Item { item, .. } => item.name.clone(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "FORMA".to_string(),
            "Okina Prime Handle".to_string(),
            "Baruuk Prime Chassis Blueprint".to_string(),
            "Shade Prime Systems".to_string(),
        ]
    );
}
