use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Color scheme name: "AgendaColor", "AgendaMono", or "" for the default.
    #[serde(default)]
    pub colorscheme: String,
}

/// Platform-specific path to the beeswax config file.
pub fn config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = std::env::var("APPDATA").ok().map(PathBuf::from)?;
    #[cfg(not(windows))]
    let base = std::env::var("HOME").ok()
        .map(|h| PathBuf::from(h).join(".config"))?;
    Some(base.join("beeswax").join("config.toml"))
}

/// Load config from the platform config path. Returns default if missing or invalid.
pub fn load() -> Config {
    let Some(path) = config_path() else { return Config::default(); };
    let Ok(text) = std::fs::read_to_string(&path) else { return Config::default(); };
    toml::from_str(&text).unwrap_or_default()
}
