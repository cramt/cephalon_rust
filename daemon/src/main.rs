use image::ImageReader;
use ocr::ocr;
use relic_screen_parser::parse_relic_screen;

pub mod config;
pub mod items;
pub mod log_watcher;
pub mod ocr;
pub mod relic_screen_parser;

#[tokio::main]
async fn main() {
    let img = ImageReader::open("image.png").unwrap().decode().unwrap();
    let results = parse_relic_screen(&img, 4).await;
    println!("{results:?}");
}
