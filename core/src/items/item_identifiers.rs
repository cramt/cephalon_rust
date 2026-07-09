use serde::{Deserialize, Serialize};

use crate::{config::client, items::Data};

use super::ReqwestSerdeError;

#[derive(Debug, Serialize, Deserialize)]
pub enum ItemIdentifier {
    Relic { id_name: String },
    Item { id_name: String },
}

pub async fn get_item_identifiers() -> Result<Vec<ItemIdentifier>, ReqwestSerdeError> {
    #[derive(Debug, Serialize, Deserialize)]
    struct Message {
        slug: String,
        tags: Vec<String>,
    }
    Ok(client()
        .await
        .get("https://api.warframe.market/v2/items")
        .send()
        .await?
        .json::<Data<Vec<Message>>>()
        .await?
        .data
        .into_iter()
        .map(|x| {
            // v1 distinguished relics by the presence of a `vaulted` field; v2 tags
            // every relic with "relic", which is a cleaner signal.
            if x.tags.iter().any(|t| t == "relic") {
                ItemIdentifier::Relic { id_name: x.slug }
            } else {
                ItemIdentifier::Item { id_name: x.slug }
            }
        })
        .collect())
}
