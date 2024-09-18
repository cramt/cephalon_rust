use crate::items::ItemIdentifier;
use crate::{
    config::client,
    items::{ItemWrapper, Payload},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub vaulted: bool,
    pub era: String,
    pub trading_tax: u32,
}

pub async fn fetch_items(identifiers: &Vec<ItemIdentifier>) -> Result<Vec<Item>, anyhow::Error> {
    let items = identifiers
        .iter()
        .flat_map(|x| match x {
            ItemIdentifier::Relic { id_name: _ } => None,
            ItemIdentifier::Item { id_name } => Some(id_name),
        })
        .next()
        .unwrap();
    let result = client()
        .await
        .get(format!("https://api.warframe.market/v1/items/{items}"))
        .send()
        .await?
        .text()
        .await?;
    println!("{result}");
    todo!()
}
