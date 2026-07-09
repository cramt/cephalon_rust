use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{config::client, items::Data};

use super::{item_identifiers::ItemIdentifier, I18n, ReqwestSerdeError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Relic {
    pub id: String,
    pub id_name: String,
    pub name: String,
    pub vaulted: bool,
    pub era: String,
    pub trading_tax: u32,
}

pub async fn fetch_relics(identifiers: &[ItemIdentifier]) -> Result<Vec<Relic>, ReqwestSerdeError> {
    #[derive(Debug, Serialize, Deserialize)]
    struct Message {
        id: String,
        slug: String,
        tags: Vec<String>,
        // Some relics omit `vaulted` when unvaulted.
        #[serde(default)]
        vaulted: bool,
        #[serde(rename = "tradingTax")]
        trading_tax: u32,
        i18n: I18n,
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
                .get(format!("https://api.warframe.market/v2/item/{name}"))
                .send()
                .await?
                .json::<Data<Message>>()
                .await?
                .data)
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, ReqwestSerdeError>>()?;
    Ok(relics
        .into_iter()
        .flat_map(|x| {
            // Relic tags look like ["relic", "lith"]; the era is the non-"relic" tag.
            let era = x.tags.into_iter().find(|t| t != "relic")?;
            Some(Relic {
                id: x.id,
                id_name: x.slug,
                vaulted: x.vaulted,
                era,
                trading_tax: x.trading_tax,
                name: x.i18n.en.name,
            })
        })
        .collect::<Vec<_>>())
}
