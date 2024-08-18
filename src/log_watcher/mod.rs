use std::{ffi::OsStr, io::SeekFrom, path::PathBuf, time::Duration};

use async_watcher::{notify::RecursiveMode, AsyncDebouncer};
use tokio::{fs::File, io::{AsyncBufReadExt, AsyncSeekExt, BufReader}, time::sleep};

#[cfg(target_os = "linux")]
static DEFAULT_PATH: [&str; 14] = [".local","share","Steam","steamapps","compatdata","230410","pfx","drive_c","users","steamuser","AppData","Local","Warframe","EE.log"];
#[cfg(target_os = "windows")]
static DEFAULT_PATH: [&str; 3] = ["AppData","Local","Warframe"];

pub fn get_default_path() -> PathBuf {
    dirs::home_dir().unwrap().into_iter().chain(DEFAULT_PATH.iter().map(OsStr::new)).collect()
}

async fn raw_watcher() -> Result<tokio::sync::mpsc::Receiver<Result<Vec<async_watcher::DebouncedEvent>, Vec<async_watcher::notify::Error>>>, anyhow::Error> {

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let mut debouncer = AsyncDebouncer::new(Duration::from_millis(100), Some(Duration::from_millis(100)), tx).await?; 
    debouncer.watcher().watch(&get_default_path(), RecursiveMode::Recursive)?;
    Ok(rx)
}

pub async fn watcher() {
    let mut file = BufReader::new(File::open(get_default_path()).await.unwrap());
    file.seek(SeekFrom::End(0)).await.unwrap();
    let mut buffer = Vec::with_capacity(50);
    tokio::spawn(async move {
        loop {
            loop {
                file.read_until(b'\n', &mut buffer).await.unwrap();
                if buffer.last() != Some(&b'\n') {
                    break;
                }
                let str = String::from_utf8(buffer).unwrap();
                buffer = Vec::with_capacity(50);
                println!("{:?}", str.trim());
            }
            sleep(Duration::from_millis(100)).await;
        }
    });

    
}

