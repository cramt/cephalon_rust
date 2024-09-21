use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{config::client, items::Payload};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub reputation: u32,
    pub locale: String,
    pub avatar: Option<String>,
    pub ingame_name: String,
    pub last_seen: DateTime<Utc>,
    pub id: String,
    pub region: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub quantity: u32,
    pub platinum: u32,
    pub order_type: OrderType,
    pub visible: bool,
    pub platform: Platform,
    pub creation_date: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub id: String,
    pub region: String,
    pub user: User,
}

#[derive(Serialize, Deserialize)]
struct OrdersWrapper {
    orders: Vec<Order>,
}

pub async fn fetch_orders(id_name: &str) -> Result<Vec<Order>, ReqwestSerdeError> {
    Ok(client()
        .await
        .get(format!(
            "https://api.warframe.market/v1/items/{id_name}/orders"
        ))
        .send()
        .await?
        .json::<Payload<OrdersWrapper>>()
        .await?
        .payload
        .orders)
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
