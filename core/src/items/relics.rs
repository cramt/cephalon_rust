use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    config::client,
    items::{ItemWrapper, Payload},
};

use super::item_identifiers::ItemIdentifier;

#[derive(Debug, Serialize, Deserialize)]
pub struct Relic {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub vaulted: bool,
    pub era: String,
    pub trading_tax: u32,
}

pub async fn fetch_relics(identifiers: &Vec<ItemIdentifier>) -> Result<Vec<Relic>, anyhow::Error> {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInner2 {
        item_name: String,
        description: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInner {
        vaulted: bool,
        trading_tax: u32,
        id: String,
        url_name: String,
        tags: Vec<String>,
        en: MessageInner2,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Message {
        items_in_set: Vec<MessageInner>,
    }
    let relics = identifiers
        .iter()
        .flat_map(|x| match x {
            ItemIdentifier::Relic { id_name } => Some(id_name),
            ItemIdentifier::Item { id_name: _ } => None,
        })
        .map(|name| async move {
            Ok(client()
                .await
                .get(format!("https://api.warframe.market/v1/items/{name}"))
                .send()
                .await?
                .json::<Payload<ItemWrapper<Message>>>()
                .await?
                .payload
                .item)
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    Ok(relics
        .into_iter()
        .flat_map(|mut x| {
            let x = x.items_in_set.pop()?;
            Some(Relic {
                id: x.id,
                id_name: x.url_name,
                vaulted: x.vaulted,
                era: x.tags.into_iter().filter(|x| x != "relic").next()?,
                trading_tax: x.trading_tax,
                name: x.en.item_name,
            })
        })
        .collect::<Vec<_>>())
}
