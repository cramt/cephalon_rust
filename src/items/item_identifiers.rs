use serde::{Deserialize, Serialize};

use crate::{
    config::client,
    items::{ItemsWrapper, Payload},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum ItemIdentifier {
    Relic { id_name: String },
    Item { id_name: String },
}

pub async fn get_item_identifiers() -> Result<Vec<ItemIdentifier>, anyhow::Error> {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Message {
        Relic { url_name: String, vaulted: bool },
        Item { url_name: String },
    }
    Ok(client()
        .await
        .get("https://api.warframe.market/v1/items")
        .send()
        .await?
        .json::<Payload<ItemsWrapper<Message>>>()
        .await?
        .payload
        .items
        .into_iter()
        .map(|x| match x {
            Message::Relic {
                url_name,
                vaulted: _,
            } => ItemIdentifier::Relic { id_name: url_name },
            Message::Item { url_name } => ItemIdentifier::Item { id_name: url_name },
        })
        .collect())
}
