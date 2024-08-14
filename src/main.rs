use items::db_reset;
use xcap::Window;

pub mod config;
pub mod items;

#[tokio::main]
async fn main() {
    let a = Window::all()
        .unwrap()
        .into_iter()
        .map(|x| x.app_name().to_owned())
        .collect::<Vec<_>>();
    println!("{a:?}");
}
