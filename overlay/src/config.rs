use std::path::PathBuf;

use tokio::sync::OnceCell;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub cache_path: PathBuf,
    /// index into display_info::DisplayInfo::all(); primary display when unset
    #[serde(default)]
    pub monitor: Option<usize>,
}

pub async fn settings() -> &'static Settings {
    static ONCE: OnceCell<Settings> = OnceCell::const_new();

    (ONCE
        .get_or_init(|| async {
            config::Config::builder()
                .add_source(config::Environment::default())
                .build()
                .unwrap()
                .try_deserialize()
                .unwrap()
        })
        .await) as _
}
