use serde::{Deserialize, Serialize};

use crate::{config::client, items::Data};

use super::ReqwestSerdeError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    #[serde(alias = "buy", alias = "BUY")]
    Buy,
    #[serde(alias = "sell", alias = "SELL")]
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    #[serde(alias = "pc")]
    Pc,
    #[serde(alias = "xbox")]
    Xbox,
    #[serde(alias = "ps4")]
    Ps4,
    #[serde(alias = "switch")]
    Switch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserStatus {
    #[serde(alias = "ingame")]
    Ingame,
    #[serde(alias = "online")]
    Online,
    #[serde(alias = "offline")]
    Offline,
}

/// In v2 the platform, locale (v1's "region") and status all live on the user
/// rather than on the order itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub platform: Platform,
    pub locale: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub platinum: u32,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub user: User,
}

pub async fn fetch_orders(id_name: &str) -> Result<Vec<Order>, ReqwestSerdeError> {
    Ok(client()
        .await
        .get(format!(
            "https://api.warframe.market/v2/orders/item/{id_name}"
        ))
        .send()
        .await?
        .json::<Data<Vec<Order>>>()
        .await?
        .data)
}

#[cfg(test)]
mod tests {
    use crate::items::orders::fetch_orders;

    #[tokio::test]
    async fn it_works() {
        let result = fetch_orders("saryn_prime_systems_blueprint").await;
        assert!(result.is_ok(), "{result:?}");
    }
}
