use std::{io::Cursor, process::Stdio};
use tokio::io::AsyncWriteExt;

use image::DynamicImage;
use tokio::process::Command;

use crate::config::settings;

pub async fn ocr(img: DynamicImage) -> anyhow::Result<String> {
    let mut process = Command::new(settings().await.tesseract_path.as_str())
        .arg("stdin")
        .arg("stdout")
        .arg("-c")
        .arg("tessedit_char_whitelist=0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut stdin = process.stdin.take().unwrap();
        let mut pnm = Vec::new();
        img.write_to(&mut Cursor::new(&mut pnm), image::ImageFormat::Pnm)?;
        stdin.write_all(pnm.as_ref()).await?;
    }
    let output = process.wait_with_output().await?;
    Ok(String::from_utf8(output.stdout)?)
}
