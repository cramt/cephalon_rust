use items::db_reset;
use xcap::Window;

pub mod config;
pub mod items;

#[tokio::main]
async fn main() {
    let windows = Window::all().unwrap();

    println!(
        "{:?}",
        windows.iter().map(|x| x.title()).collect::<Vec<_>>()
    );
    let warframe_window = windows
        .into_iter()
        .find(|x| x.title() == "Warframe")
        .unwrap();
    let image = warframe_window.capture_image().unwrap();
    image.save("a.png").unwrap();
}
