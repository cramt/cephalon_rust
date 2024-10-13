pub mod config;

use std::{collections::HashMap, fs::OpenOptions, path::Path};

use cephalon_rust_core::{state::State, Engine};
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

    let (tx, mut rx) = tokio::sync::mpsc::channel::<State>(100);

    tokio::spawn(async move {
        while let Some(state) = rx.recv().await {
            let a = state
                .relic_rewards
                .into_iter()
                .flatten()
                .map(|(i, v)| (i.name, v))
                .collect::<HashMap<_, _>>();
            println!("{a:?}");
        }
    });
    engine.run(tx).await?;
    Ok(())
}
