use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::format,
    process::Stdio,
};

use config::settings;
use items::db_reset;
use log_watcher::watcher;
use sysinfo::{Pid, System};
use tokio::process::Command;

pub mod config;
pub mod items;
pub mod log_watcher;

#[tokio::main]
async fn main() {
    let sys = System::new_all();
    let warframe = sys
        .processes_by_exact_name(OsStr::new("Warframe.x64.ex"))
        .next()
        .unwrap();
    let envs = warframe
        .environ()
        .iter()
        .flat_map(|x| {
            let a = x.as_encoded_bytes();
            let mut split = a.splitn(2, |b| *b == b'=');
            let split1 = split.next()?;
            let split2 = split.next()?;
            let split1 = unsafe { OsStr::from_encoded_bytes_unchecked(split1) };
            let split2 = unsafe { OsStr::from_encoded_bytes_unchecked(split2) };
            Some((split1, split2))
        })
        .collect::<HashMap<_, _>>();
    let wine_envs = envs
        .into_iter()
        .filter(|(k, _)| {
            k.to_str()
                .map(|x| x.starts_with("WINE") && x != "WINESERVERSOCKET")
                .unwrap_or(false)
        })
        .collect::<HashMap<_, _>>();
    let wine_exec = wine_envs
        .get(OsStr::new("WINELOADER"))
        .unwrap()
        .to_str()
        .unwrap();
    println!("{:?}", wine_envs);
    let result = Command::new(wine_exec)
        .arg(&settings().await.overlay_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .envs(wine_envs)
        .spawn()
        .unwrap()
        .wait()
        .await
        .unwrap();
    println!("{:?}", result);
}
