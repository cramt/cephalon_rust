use std::collections::{HashMap, HashSet};

use crate::items::ItemIdentifier;
use crate::{
    config::client,
    items::{ItemWrapper, Payload},
};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSet {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub part_ids: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub trading_tax: u32,
    pub set_id: String,
    pub ducats: u32,
    pub quantity_for_set: u32,
}

pub async fn fetch_items_and_sets(
    identifiers: &Vec<ItemIdentifier>,
) -> Result<(HashMap<String, Item>, HashMap<String, ItemSet>), anyhow::Error> {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInner2 {
        item_name: String,
        description: String,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct MessageInner {
        trading_tax: u32,
        id: String,
        ducats: u32,
        #[serde(default)]
        quantity_for_set: u32,
        set_root: bool,
        url_name: String,
        tags: Vec<String>,
        en: MessageInner2,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Message {
        items_in_set: Vec<MessageInner>,
    }
    let items = identifiers
        .iter()
        .flat_map(|x| match x {
            ItemIdentifier::Relic { id_name: _ } => None,
            ItemIdentifier::Item { id_name } => Some(id_name),
        })
        .filter(|x| x.contains("prime"))
        .filter(|x| !x.contains("primed"))
        .filter(|x| "gotva_prime" != x.as_str())
        .map(|name| async move {
            let item = client()
                .await
                .get(format!("https://api.warframe.market/v1/items/{name}"))
                .send()
                .await?
                .json::<Payload<ItemWrapper<Message>>>()
                .await?
                .payload
                .item;
            let (roots, parts): (Vec<_>, Vec<_>) =
                item.items_in_set.into_iter().partition(|n| n.set_root);
            let root = roots.into_iter().next().unwrap();
            let set = ItemSet {
                id: root.id,
                id_name: root.url_name,
                name: root.en.item_name,
                part_ids: parts.iter().map(|x| x.id.clone()).collect(),
            };
            let parts = parts
                .into_iter()
                .map(|x| Item {
                    id: x.id,
                    id_name: x.url_name,
                    name: x.en.item_name,
                    trading_tax: x.trading_tax,
                    set_id: set.id.clone(),
                    ducats: x.ducats,
                    quantity_for_set: x.quantity_for_set,
                })
                .collect::<Vec<_>>();
            Ok((set, parts))
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    Ok(items.into_iter().fold(
        (HashMap::new(), HashMap::new()),
        |(mut parts, mut sets), (set, part)| {
            sets.insert(set.id.clone(), set);
            for p in part {
                parts.insert(p.id.clone(), p);
            }
            (parts, sets)
        },
    ))
}
