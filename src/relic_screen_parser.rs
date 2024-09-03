use std::convert::identity;

use ctreg::regex;
use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;

regex! { CapitalFinder = r#"[^$\s](?<capital>[A-Z])[a-z]"# }

use crate::ocr;

pub async fn parse_relic_screen(img: &DynamicImage, amount: u8) -> Vec<Option<String>> {
    let width = 1920u32;
    let middle = width / 2;
    let frame_width = 243u32;
    let frame_bottom = 460u32;
    let text_height = 24u32;
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
                    let result = ocr(new).await.ok()?;
                    let res = result.trim();
                    if res.is_empty() {
                        break;
                    } else {
                        buffer = format!("{res}{buffer}");
                    }
                }
                let finder = CapitalFinder::new();
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
