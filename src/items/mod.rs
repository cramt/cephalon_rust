pub mod item_identifiers;
pub mod relics;

use std::{
    fs::{File, OpenOptions},
    future::Future,
    io::{BufReader, BufWriter},
    path::Path,
};

use item_identifiers::{get_item_identifiers, ItemIdentifier};
use relics::{fetch_relics, Relic};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{io::AsyncReadExt, sync::OnceCell};

use crate::config::settings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<T> {
    payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemsWrapper<T> {
    items: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemWrapper<T> {
    item: T,
}

async fn cache_in_file<
    P: AsRef<Path>,
    T: DeserializeOwned + Serialize,
    Fut: Future<Output = Option<T>>,
    F: FnOnce() -> Fut,
>(
    path: P,
    create: F,
) -> Option<T> {
    //TODO: make the file IO async in this file
    fn cache_read<P: AsRef<Path>, T: DeserializeOwned + Serialize>(path: P) -> Option<T> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }
    match cache_read(&path) {
        Some(x) => Some(x),
        None => {
            let result = create().await?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .ok()?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, &result).ok()?;
            Some(result)
        }
    }
}

pub async fn item_identifiers() -> &'static Vec<ItemIdentifier> {
    static ONCE: OnceCell<Vec<ItemIdentifier>> = OnceCell::const_new();
    let static_ref = ONCE
        .get_or_init(|| async {
            cache_in_file(
                settings().await.cache_path.join("item_identifiers.json"),
                || async { get_item_identifiers().await.ok() },
            )
            .await
            .unwrap()
        })
        .await;
    static_ref
}

pub async fn relics() -> &'static Vec<Relic> {
    static ONCE: OnceCell<Vec<Relic>> = OnceCell::const_new();
    let static_ref = ONCE
        .get_or_init(|| async {
            cache_in_file(settings().await.cache_path.join("relics.json"), || async {
                fetch_relics(&get_item_identifiers().await.ok()?).await.ok()
            })
            .await
            .unwrap()
        })
        .await;
    static_ref
}
