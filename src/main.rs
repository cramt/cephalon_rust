use items::fetch_all;

pub mod config;
pub mod items;

#[tokio::main]
async fn main() {
    fetch_all().await.unwrap()
}
