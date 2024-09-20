pub mod item_identifiers;
pub mod items;
pub mod relics;

use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug, Display},
    fs::{File, OpenOptions},
    future::Future,
    io::{BufReader, BufWriter},
    path::Path,
};

use item_identifiers::{get_item_identifiers, ItemIdentifier};
use items::{fetch_items_and_sets, Item, ItemSet};
use relics::{fetch_relics, Relic};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ReqwestSerdeError {
    #[error("serde error")]
    SerdeError(#[from] serde_json::Error),
    #[error("reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("reqwest middleware error")]
    ReqwestMiddlewareError(#[from] reqwest_middleware::Error),
}

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

#[derive(Debug)]
pub enum CacheError<T> {
    CreateFileError(std::io::Error),
    SerdeError(serde_json::Error),
    InnerError(T),
}

impl<T: Display> Display for CacheError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::CreateFileError(io) => write!(f, "io error: {io}"),
            CacheError::InnerError(inner) => inner.fmt(f),
            CacheError::SerdeError(e) => write!(f, "serde error: {e}"),
        }
    }
}

impl<T: Display + Error> Error for CacheError<T> {}

async fn cache_in_file<
    E: Error + Display,
    P: AsRef<Path>,
    T: DeserializeOwned + Serialize,
    Fut: Future<Output = Result<T, E>>,
    F: FnOnce() -> Fut,
>(
    path: P,
    create: F,
) -> Result<T, CacheError<E>> {
    println!("{:?}", std::path::absolute(path.as_ref()));
    //TODO: make the file IO async in this file
    fn cache_read<P: AsRef<Path>, T: DeserializeOwned + Serialize>(path: P) -> Option<T> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }
    match cache_read(&path) {
        Some(x) => Ok(x),
        None => {
            let result = create().await.map_err(|x| CacheError::InnerError(x))?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)
                .map_err(|x| CacheError::CreateFileError(x))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, &result).map_err(|x| CacheError::SerdeError(x))?;
            Ok(result)
        }
    }
}

pub async fn cached_get_item_identifiers(
    cache_path: &Path,
) -> Result<Vec<ItemIdentifier>, CacheError<ReqwestSerdeError>> {
    cache_in_file(cache_path.join("item_identifiers.json"), || async {
        get_item_identifiers().await
    })
    .await
}

pub async fn cached_fetch_relics(
    cache_path: &Path,
    item_identifiers: &Vec<ItemIdentifier>,
) -> Result<Vec<Relic>, CacheError<ReqwestSerdeError>> {
    cache_in_file(cache_path.join("relics.json"), || async {
        fetch_relics(item_identifiers).await
    })
    .await
}

pub async fn cached_items_and_sets(
    cache_path: &Path,
    item_identifiers: &Vec<ItemIdentifier>,
) -> Result<(HashMap<String, Item>, HashMap<String, ItemSet>), CacheError<ReqwestSerdeError>> {
    cache_in_file(cache_path.join("items_and_sets.json"), || async {
        fetch_items_and_sets(item_identifiers).await
    })
    .await
}
