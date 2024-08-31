use log_watcher::watcher;

pub mod config;
pub mod items;
pub mod log_watcher;

#[tokio::main]
async fn main() {
    let mut rx = watcher().await;

    while let Some(i) = rx.recv().await {
        println!("{:?}", i);
    }
}
