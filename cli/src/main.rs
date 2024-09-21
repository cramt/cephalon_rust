pub mod config;

use std::{collections::HashMap, path::Path};

use cephalon_rust_core::{state::State, Engine};
use config::settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let setting = settings().await;
    let engine = Engine::new(
        Path::new(&setting.tesseract_path).to_path_buf(),
        Path::new(&setting.cache_path).to_path_buf(),
        true,
    )
    .await?;
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
