pub mod config;

use std::{fs::OpenOptions, path::Path};

use cephalon_rust_core::{
    event::{Event, RewardSlot},
    Engine,
};
use config::settings;
use tracing_subscriber::{fmt, prelude::*, Registry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("cephalon.log")
        .unwrap();
    let subscriber = Registry::default().with(fmt::layer().with_writer(log_file));
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let setting = settings().await;
    let engine = Engine::new(Path::new(&setting.cache_path).to_path_buf()).await?;
    println!("engine inited");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                Event::RewardScreenOpened { count, window } => {
                    println!("reward screen opened ({count} cards) window: {window:?}")
                }
                Event::RewardsResolved(slots) => {
                    let summary = slots
                        .iter()
                        .map(|s| match s {
                            RewardSlot::Pending => "…".to_string(),
                            RewardSlot::Forma => "forma".to_string(),
                            RewardSlot::Item {
                                item,
                                price: Some(p),
                            } => format!("{} {p}p", item.name),
                            RewardSlot::Item { item, price: None } => format!("{} ?p", item.name),
                        })
                        .collect::<Vec<_>>()
                        .join(" | ");
                    println!("{summary}");
                }
                Event::RewardScreenClosed => println!("reward screen closed"),
            }
        }
    });
    engine.run(tx).await;
    Ok(())
}
