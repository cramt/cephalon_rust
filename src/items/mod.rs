pub mod item_identifiers;

use futures::stream::{FuturesUnordered, StreamExt};
use item_identifiers::{get_item_identifiers, ItemIdentifier};
use serde::{Deserialize, Serialize};

use crate::config::client;

pub struct Item {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub vaulted: bool,
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

pub async fn fetch_all() -> Result<(), anyhow::Error> {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum MessageInner1 {
        Relic { url_name: String, vaulted: bool },
        Item { url_name: String },
    }
    let json = reqwest::get("https://api.warframe.market/v1/items")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let all_elements = get_item_identifiers().await?;
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInnerInnerInner2 {
        item_name: String,
        description: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInnerInner2 {
        vaulted: bool,
        trading_tax: u64,
        id: String,
        url_name: String,
        tags: Vec<String>,
        subtypes: Vec<String>,
        en: MessageInnerInnerInner2,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInner2 {
        id: String,
        items_in_set: Vec<MessageInnerInner2>,
    }
    let relics: Vec<_> = all_elements
        .iter()
        .flat_map(|x| match x {
            ItemIdentifier::Relic { id_name } => Some(id_name),
            ItemIdentifier::Item { id_name: _ } => None,
        })
        .map(|name| async move {
            let text = client()
                .await
                .get(format!("https://api.warframe.market/v1/items/{name}"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            println!("{text}");
            text
        })
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;
    println!("{relics:?}");
    Ok(())
}

pub async fn db_reset() {}
pub async fn db_init() {}
