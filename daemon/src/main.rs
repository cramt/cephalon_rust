use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::format,
};

use items::db_reset;
use log_watcher::watcher;
use sysinfo::{Pid, System};

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
    println!("{:?}", envs)
}
