use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::sync::OnceLock;
use tokio::task::spawn_blocking;

use image::DynamicImage;

pub async fn ocr(img: DynamicImage) -> anyhow::Result<String> {
    spawn_blocking(move || ocr_blocking(img)).await?
}

fn ocr_blocking(img: DynamicImage) -> anyhow::Result<String> {
    static ENGINE: OnceLock<OcrEngine> = OnceLock::new();

    let engine = ENGINE.get_or_init(|| {
        OcrEngine::new(OcrEngineParams {
            detection_model: Some(
                Model::load_static_slice(include_bytes!(env!("DETECTION_MODEL"))).unwrap(),
            ),
            recognition_model: Some(
                Model::load_static_slice(include_bytes!(env!("RECOGNITION_MODEL"))).unwrap(),
            ),
            ..Default::default()
        })
        .unwrap()
    });
    let img = img.into_rgb8();
    let source = ImageSource::from_bytes(img.as_raw(), img.dimensions())?;

    let ocr_input = engine.prepare_input(source)?;
    let result = engine.get_text(&ocr_input);
    println!("{result:?}");
    result
}
