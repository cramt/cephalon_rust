use std::convert::identity;

use ctreg::regex;
use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;

regex! { CapitalFinder = r#"[^$\s](?<capital>[A-Z])"# }

use crate::ocr;

pub async fn parse_relic_screen(img: &DynamicImage, amount: u8) -> Vec<Option<String>> {
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
        .filter_map(identity)
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
                    let result = ocr(new).await.ok()?;
                    let res = result.trim();
                    if res.is_empty() {
                        break;
                    } else {
                        buffer = format!("{res}{buffer}");
                    }
                }
                let finder = CapitalFinder::new();
                let mut buffer = buffer.replace("Primie", "Prime"); //TODO: bad solution, get
                                                                    //tesseract to act better
                loop {
                    if let Some(res) = finder.captures(buffer.as_str()) {
                        buffer.insert_str(res.capital.start, " ")
                    } else {
                        break;
                    }
                }
                Some(buffer)
            }
        })
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
        .await
}

#[cfg(test)]
mod tests {
    use image::ImageReader;

    use super::*;

    #[tokio::test]
    async fn _1() {
        let img = ImageReader::open("test_rewards_screens/1.png")
            .unwrap()
            .decode()
            .unwrap();
        let result = parse_relic_screen(&img, 4).await;
        assert_eq!(
            result,
            vec![
                Some("2 X Forma Blueprint".to_string()),
                Some("Okina Prime Handle".to_string()),
                Some("Baruuk Prime Chassis Blueprint".to_string()),
                Some("Shade Prime Systems".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn _2() {
        async fn inner_test(img: &DynamicImage) {
            let result = parse_relic_screen(&img, 4).await;
            assert_eq!(
                result,
                vec![
                    Some("Burston Prime Receiver".to_string()),
                    Some("Oberon Prime Blueprint".to_string()),
                    Some("Sybarus Prime Blueprint".to_string()),
                    Some("Lex Prime Receiver".to_string())
                ]
            );
        }
        let img = ImageReader::open("test_rewards_screens/2a.png")
            .unwrap()
            .decode()
            .unwrap();
        inner_test(&img).await;
        let img = ImageReader::open("test_rewards_screens/2b.png")
            .unwrap()
            .decode()
            .unwrap();
        inner_test(&img).await;
    }
}
