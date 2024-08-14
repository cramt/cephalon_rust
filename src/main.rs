use items::db_reset;

pub mod config;
pub mod items;

#[tokio::main]
async fn main() {
    db_reset().await.unwrap()
}
