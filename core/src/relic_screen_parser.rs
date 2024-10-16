use std::collections::{HashMap, HashSet};

use ctreg::regex;
use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;
use tracing::*;

regex! { CapitalFinder = r#"[^$\s](?<capital>[A-Z])"# }

use crate::{debug_write_image, items::items::Item, ocr};

pub async fn parse_relic_screen<'a>(
    img: &DynamicImage,
    amount: &HashSet<usize>,
    items: &HashMap<String, Item>,
) -> Vec<Option<Item>> {
    #[instrument]
    fn match_item(items: &HashMap<String, Item>, name: &str) -> Option<Item> {
        let mut items = items
            .iter()
            .filter(|(_, v)| name.contains(&v.name))
            .map(|(_, v)| v)
            .take(2);
        let first = items.next();
        let second = items.next();
        let item = match (first, second) {
            (Some(item), None) => Some(item),
            _ => None,
        };
        item.cloned()
    }
    #[instrument]
    fn clean_ocr_output(mut buffer: String) -> String {
        let finder = CapitalFinder::new();
        while let Some(res) = finder.captures(buffer.as_str()) {
            buffer.insert(res.capital.start, ' ')
        }
        buffer
    }
    let width = img.width();
    let height = img.height();
    let middle = width / 2;
    let frame_width = (width * 243) / 1920;
    let frame_bottom = (height * 460) / 1080;
    let text_height = (height * 24) / 1080;
    let start_points = match amount.len() {
        4 => vec![
            (middle - frame_width * 2),
            (middle - frame_width),
            (middle),
            (middle + frame_width),
        ],
        2 => vec![(middle - frame_width), (middle)],
        3 => vec![
            middle - ((3 * frame_width) / 2),
            middle + (frame_width / 2),
            middle - (frame_width / 2),
        ],
        1 => vec![(middle - (frame_width / 2))],
        _ => Vec::new(),
    };
    start_points
        .into_iter()
        .enumerate()
        .map(|(i, x)| if amount.contains(&i) { Some(x) } else { None })
        .map(|p| {
            let mut img = img.clone();
            async move {
                let p = p?;
                // naive cropping
                {
                    for i in (2..=3).rev() {
                        event!(Level::INFO, "trying naive cropping: {i} lines");
                        let new = img.crop(
                            p,
                            frame_bottom - (text_height * i),
                            frame_width,
                            text_height * i,
                        );
                        debug_write_image(&new, &format!("naive_crop_{p}_{i}"));
                        let result = ocr::ocr(new).await.ok()?;
                        let res = result.trim().replace("\n", " ");
                        let buffer = clean_ocr_output(res);
                        if let Some(result) = match_item(items, &buffer) {
                            event!(Level::INFO, "match: {result:?}");
                            return Some(result);
                        }
                    }
                }
                // pessimistic cropping
                {
                    let mut buffer = String::new();
                    for i in 1.. {
                        event!(Level::INFO, "trying pessimistic cropping: {i} lines");
                        let new = img.crop(
                            p,
                            frame_bottom - (text_height * i),
                            frame_width,
                            text_height,
                        );
                        debug_write_image(&new, &format!("pessimistic_crop_{p}_{i}"));
                        let result = ocr::ocr(new).await.ok()?;
                        let res = result.trim();
                        if res.is_empty() {
                            break;
                        } else {
                            buffer = format!("{res}{buffer}");
                        }
                    }
                    let buffer = clean_ocr_output(buffer);
                    if let Some(result) = match_item(items, &buffer) {
                        event!(Level::INFO, "match: {result:?}");
                        return Some(result);
                    }
                }
                None
            }
        })
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
        .await
}

#[cfg(test)]
mod tests {
    use std::{env, path::Path};

    use image::ImageReader;

    use crate::items::{cached_get_item_identifiers, cached_items_and_sets};

    use super::*;

    async fn assert(img: &DynamicImage, rhs: Vec<Option<String>>) {
        let cache_path = env::var("CACHE_PATH").unwrap();
        let cache_path = Path::new(&cache_path);
        let item_identifiers = cached_get_item_identifiers(cache_path).await.unwrap();
        let (items, _) = cached_items_and_sets(cache_path, &item_identifiers)
            .await
            .unwrap();
        let result = parse_relic_screen(img, &(0..4).collect(), &items)
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

    #[tokio::test]
    async fn _3() {
        let img = ImageReader::open("test_rewards_screens/3.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Grendel Prime Neuroptics Blueprint".to_string()),
                Some("Burston Prime Receiver".to_string()),
                None,
                None,
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn _4() {
        let img = ImageReader::open("test_rewards_screens/4.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Larkspur Prime Blueprint".to_string()),
                None,
                Some("Paris Prime Blueprint".to_string()),
                Some("Braton Prime Blueprint".to_string()),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn _5() {
        let img = ImageReader::open("test_rewards_screens/5.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Baruuk Prime Systems Blueprint".to_string()),
                None,
                Some("Shade Prime Blueprint".to_string()),
                None,
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn _6() {
        let img = ImageReader::open("test_rewards_screens/6.png")
            .unwrap()
            .decode()
            .unwrap();
        assert(
            &img,
            vec![
                Some("Lex Prime Receiver".to_string()),
                Some("Khora Prime Systems Blueprint".to_string()),
                Some("Equinox Prime Chassis Blueprint".to_string()),
                Some("Braton Prime Blueprint".to_string()),
            ],
        )
        .await;
    }
}
