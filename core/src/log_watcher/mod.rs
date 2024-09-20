use std::{ffi::OsStr, io::SeekFrom, path::PathBuf, str::FromStr, time::Duration};

use ctreg::regex;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncSeekExt, BufReader},
    time::sleep,
};

#[cfg(target_os = "linux")]
static DEFAULT_PATH: [&str; 14] = [
    ".local",
    "share",
    "Steam",
    "steamapps",
    "compatdata",
    "230410",
    "pfx",
    "drive_c",
    "users",
    "steamuser",
    "AppData",
    "Local",
    "Warframe",
    "EE.log",
];
#[cfg(target_os = "windows")]
static DEFAULT_PATH: [&str; 3] = ["AppData", "Local", "Warframe"];

pub fn get_default_path() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .into_iter()
        .chain(DEFAULT_PATH.iter().map(OsStr::new))
        .collect()
}

regex! { LogEntryParser = r#"(?:\d+\.\d+ )?(?<system>[A-z]+) \[(?<level>[A-z]+)\]: (?<rest>.*)"# }
regex! { LogScriptEntryParser = r#"(?<script>[A-z]+)\.lua: (?<rest>.*)"# }

#[derive(Debug)]
pub enum LogEntry {
    SysInfo(String),
    SysWarning(String),
    SysError(String),
    NetInfo(String),
    NetError(String),
    PhysInfo(String),
    PhysWarning(String),
    PhysError(String),
    SndInfo(String),
    GfxInfo(String),
    InputInfo(String),
    AIInfo(String),
    GameInfo(String),
    GameWarning(String),
    AnimInfo(String),
    ScriptInfo { script: String, content: String },
}

impl FromStr for LogEntry {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let captures = LogEntryParser::new().captures(s).ok_or(())?;
        let system = captures.system.content.trim();
        let level = captures.level.content.trim();
        let rest = captures.rest.content.trim();
        match (system, level) {
            ("Sys", "Info") => Ok(Self::SysInfo(rest.to_string())),
            ("Sys", "Warning") => Ok(Self::SysWarning(rest.to_string())),
            ("Sys", "Error") => Ok(Self::SysError(rest.to_string())),
            ("Net", "Info") => Ok(Self::NetInfo(rest.to_string())),
            ("Net", "Error") => Ok(Self::NetError(rest.to_string())),
            ("Phys", "Info") => Ok(Self::PhysInfo(rest.to_string())),
            ("Phys", "Warning") => Ok(Self::PhysWarning(rest.to_string())),
            ("Phys", "Error") => Ok(Self::PhysError(rest.to_string())),
            ("Snd", "Info") => Ok(Self::SndInfo(rest.to_string())),
            ("Gfx", "Info") => Ok(Self::GfxInfo(rest.to_string())),
            ("Input", "Info") => Ok(Self::InputInfo(rest.to_string())),
            ("AI", "Info") => Ok(Self::AIInfo(rest.to_string())),
            ("Game", "Info") => Ok(Self::GameInfo(rest.to_string())),
            ("Game", "Warning") => Ok(Self::GameWarning(rest.to_string())),
            ("Anim", "Info") => Ok(Self::AnimInfo(rest.to_string())),
            ("Script", "Info") => {
                let captures = LogScriptEntryParser::new().captures(rest).ok_or(())?;
                Ok(Self::ScriptInfo {
                    script: captures.script.content.trim().to_string(),
                    content: captures.rest.content.trim().to_string(),
                })
            }
            _ => Err(()),
        }
    }
}

pub async fn watcher() -> tokio::sync::mpsc::Receiver<LogEntry> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
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
                if let Ok(entry) = str.parse() {
                    tx.send(entry).await.unwrap();
                } else {
                    println!("failed to parse log entry: {str}");
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    });
    rx
}
