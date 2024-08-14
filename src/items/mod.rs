pub mod item_identifiers;
pub mod relics;

use item_identifiers::get_item_identifiers;
use relics::fetch_relics;
use sea_query::{Iden, Query, QueryStatementWriter, SqliteQueryBuilder};
use serde::{Deserialize, Serialize};

use crate::config::db_conn;

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

#[derive(Iden)]
enum Relics {
    Table,
    Id,
    IdName,
    Name,
    Vaulted,
    Era,
    TradingTax,
}

pub async fn db_reset() -> Result<(), anyhow::Error> {
    let db = db_conn().await;
    let items = get_item_identifiers().await?;
    let relics = fetch_relics(&items).await?;
    let query = relics
        .into_iter()
        .fold(
            Query::insert().into_table(Relics::Table).columns([
                Relics::Id,
                Relics::IdName,
                Relics::Name,
                Relics::Vaulted,
                Relics::Era,
                Relics::TradingTax,
            ]),
            |builder, x| {
                builder.values_panic([
                    x.id.into(),
                    x.id_name.into(),
                    x.name.into(),
                    x.vaulted.into(),
                    x.era.into(),
                    x.trading_tax.into(),
                ])
            },
        )
        .to_owned()
        .to_string(SqliteQueryBuilder);
    sqlx::query(&query).execute(db).await?;

    Ok(())
}
pub async fn db_init() {}
