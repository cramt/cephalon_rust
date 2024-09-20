use std::collections::HashMap;
use std::path::Path;

use ctreg::regex;
use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;

regex! { CapitalFinder = r#"[^$\s](?<capital>[A-Z])"# }

use crate::{items::items::Item, ocr};

pub async fn parse_relic_screen<'a>(
    img: &DynamicImage,
    amount: u8,
    tesseract_path: &Path,
    items: &HashMap<String, Item>,
) -> Vec<Option<Item>> {
    let width = img.width();
    let height = img.height();
    let middle = width / 2;
    let frame_width = (width * 243) / 1920;
    let frame_bottom = (height * 460) / 1080;
    let text_height = (height * 24) / 1080;
    let start_points = match amount {
        4 => [
            Some(middle - frame_width * 2),
            Some(middle - frame_width),
            Some(middle),
            Some(middle + frame_width),
        ],
        2 => [None, None, Some(middle - frame_width), Some(middle)],
        3 => [
            None,
            Some(middle - ((3 * frame_width) / 2)),
            Some(middle + (frame_width / 2)),
            Some(middle - (frame_width / 2)),
        ],
        1 => [None, None, None, Some(middle - (frame_width / 2))],
        _ => [None, None, None, None],
    };
    start_points
        .into_iter()
        .flatten()
        .map(|p| {
            let mut img = img.clone();
            async move {
                let mut buffer = String::new();
                for i in 1.. {
                    let new = img.crop(
                        p,
                        frame_bottom - (text_height * i),
                        frame_width,
                        text_height,
                    );
                    new.save(format!("debug_img_out/{p}_{i}.png")).unwrap();
                    let result = ocr::ocr(new, tesseract_path).await.ok()?;
                    let res = result.trim();
                    if res.is_empty() {
                        break;
                    } else {
                        buffer = format!("{res}{buffer}");
                    }
                }
                let finder = CapitalFinder::new();
                let mut buffer = buffer
                    .replace("Primie", "Prime")
                    .replace("Bursten", "Burston")
                    .replace("Recelver", "Receiver"); //TODO: bad solution, get
                                                      //tesseract to act better
                while let Some(res) = finder.captures(buffer.as_str()) {
                    buffer.insert(res.capital.start, ' ')
                }

                let mut items = items
                    .iter()
                    .filter(|(_, v)| buffer.contains(&v.name))
                    .map(|(_, v)| v)
                    .take(2);
                let first = items.next();
                let second = items.next();
                let item = match (first, second) {
                    (Some(item), None) => Some(item),
                    _ => None,
                };
                println!("{buffer:?} = {item:?}");
                item.cloned()
            }
        })
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
        .await
}

#[cfg(test)]
mod tests {
    use std::env;

    use image::ImageReader;

    use crate::items::{cached_get_item_identifiers, cached_items_and_sets};

    use super::*;

    async fn assert(img: &DynamicImage, rhs: Vec<Option<String>>) {
        let cache_path = env::var("CACHE_PATH").unwrap();
        let cache_path = Path::new(&cache_path);
        let tes = env::var("TESSERACT_PATH").unwrap();
        let tes = Path::new(&tes);
        let item_identifiers = cached_get_item_identifiers(cache_path).await.unwrap();
        let (items, _) = cached_items_and_sets(cache_path, &item_identifiers)
            .await
            .unwrap();
        let result = parse_relic_screen(img, 4, tes, &items)
            .await
            .into_iter()
            .map(|x| x.map(|y| y.name.to_string()))
            .collect::<Vec<_>>();
        assert_eq!(result, rhs);
    }

    #[tokio::test]
    async fn _1() {
        let img = ImageReader::open("test_rewards_screens/1.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                None,
                Some("Okina Prime Handle".to_string()),
                Some("Baruuk Prime Chassis Blueprint".to_string()),
                Some("Shade Prime Systems".to_string()),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn _2a() {
        let img = ImageReader::open("test_rewards_screens/2a.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Sybaris Prime Blueprint".to_string()),
                Some("Oberon Prime Blueprint".to_string()),
                Some("Burston Prime Receiver".to_string()),
                Some("Lex Prime Receiver".to_string()),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn _2b() {
        let img = ImageReader::open("test_rewards_screens/2b.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Burston Prime Receiver".to_string()),
                Some("Oberon Prime Blueprint".to_string()),
                Some("Sybaris Prime Blueprint".to_string()),
                Some("Lex Prime Receiver".to_string()),
            ],
        )
        .await;
    }
}
