use std::collections::{HashMap, HashSet};

use crate::items::orders::{Order, OrderType, Platform};
use crate::items::ItemIdentifier;
use crate::{
    config::client,
    items::{Data, I18n},
};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

use super::orders::{fetch_orders, UserStatus};
use super::ReqwestSerdeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSet {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub part_ids: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub trading_tax: u32,
    pub set_id: String,
    pub ducats: u32,
    pub quantity_for_set: u32,
}

impl Item {
    pub async fn price(&self) -> Result<u32, ReqwestSerdeError> {
        fn not_offlines(order: &&Order) -> bool {
            order.user.status != UserStatus::Offline
        }
        fn any(_: &&Order) -> bool {
            true
        }
        let orders = fetch_orders(&self.id_name)
            .await?
            .into_iter()
            .filter(|x| {
                x.user.platform == Platform::Pc
                    && x.user.locale == "en"
                    && x.order_type == OrderType::Buy
            })
            .collect::<Vec<_>>();
        let filter = if orders.iter().filter(not_offlines).count() > 3 {
            not_offlines
        } else {
            any
        };
        let mut orders = orders
            .iter()
            .filter(filter)
            .map(|x| x.platinum)
            .collect::<Vec<_>>();
        orders.sort();
        if orders.is_empty() {
            return Ok(0);
        }
        Ok(orders[orders.len() / 2])
    }
}

pub async fn fetch_items_and_sets(
    identifiers: &[ItemIdentifier],
) -> Result<(HashMap<String, Item>, HashMap<String, ItemSet>), ReqwestSerdeError> {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SetItem {
        id: String,
        slug: String,
        #[serde(rename = "setRoot")]
        set_root: bool,
        #[serde(rename = "tradingTax")]
        trading_tax: u32,
        ducats: u32,
        // The set root omits `quantityInSet`; parts always carry it.
        #[serde(rename = "quantityInSet", default)]
        quantity_for_set: u32,
        i18n: I18n,
    }
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SetResponse {
        items: Vec<SetItem>,
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
            let set = client()
                .await
                .get(format!(
                    "https://api.warframe.market/v2/item/{name}/set"
                ))
                .send()
                .await?
                .json::<Data<SetResponse>>()
                .await?
                .data;
            let (roots, parts): (Vec<_>, Vec<_>) =
                set.items.into_iter().partition(|n| n.set_root);
            let root = roots.into_iter().next().unwrap();
            let set = ItemSet {
                id: root.id,
                id_name: root.slug,
                name: root.i18n.en.name,
                part_ids: parts.iter().map(|x| x.id.clone()).collect(),
            };
            let parts = parts
                .into_iter()
                .map(|x| Item {
                    id: x.id,
                    id_name: x.slug,
                    name: x.i18n.en.name,
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
        .collect::<Result<Vec<_>, ReqwestSerdeError>>()?;

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
