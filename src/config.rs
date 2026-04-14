use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Color scheme name: "AgendaColor", "AgendaMono", "SolarizedDark",
    /// "SolarizedLight", "Custom", or "" for the default.
    #[serde(default)]
    pub colorscheme: String,

    /// Navigation mode: "Agenda" (default) or "vi".
    #[serde(default)]
    pub nav_mode: String,

    /// Custom color overrides — only used when colorscheme = "Custom".
    /// Each field is an optional hex color string, e.g. "#268bd2".
    /// Omitted fields fall back to the Default (terminal REVERSED) theme.
    #[serde(default)]
    pub custom_theme: CustomTheme,
}

/// Per-element color overrides for the "Custom" color scheme.
/// All fields are optional hex color strings ("#rrggbb").
/// Modifiers (bold on section heads, dim on hint text) are applied automatically.
#[derive(Serialize, Deserialize, Default)]
pub struct CustomTheme {
    // ── Bars (title / fkey / menu) ───────────────────────────────────────────
    #[serde(default)] pub bar_fg:              Option<String>,
    #[serde(default)] pub bar_bg:              Option<String>,
    /// Bar cursor: the currently-selected top-level menu item.
    #[serde(default)] pub bar_cursor_fg:       Option<String>,
    #[serde(default)] pub bar_cursor_bg:       Option<String>,

    // ── Body ─────────────────────────────────────────────────────────────────
    #[serde(default)] pub body_fg:             Option<String>,
    #[serde(default)] pub body_bg:             Option<String>,

    // ── Selected item / edit cursor ──────────────────────────────────────────
    #[serde(default)] pub selected_fg:         Option<String>,
    #[serde(default)] pub selected_bg:         Option<String>,

    // ── Section heads ────────────────────────────────────────────────────────
    /// Unselected section head foreground (background = body_bg).
    #[serde(default)] pub section_fg:          Option<String>,
    // ── Modal dialogs ────────────────────────────────────────────────────────
    /// Dialog content area — defaults to body_fg / body_bg.
    #[serde(default)] pub dialog_fg:           Option<String>,
    #[serde(default)] pub dialog_bg:           Option<String>,
    /// Dialog border foreground and background.
    #[serde(default)] pub dialog_border_fg:    Option<String>,
    #[serde(default)] pub dialog_border_bg:    Option<String>,

    // ── View body ────────────────────────────────────────────────────────────────
    /// View body background color.
    #[serde(default)] pub view_bg:             Option<String>,
    /// Foreground color for item text.
    #[serde(default)] pub view_item:           Option<String>,
    /// Foreground color for column value entries.
    #[serde(default)] pub view_col:            Option<String>,
    /// Foreground color for column header labels.
    #[serde(default)] pub view_col_head:       Option<String>,
    /// Foreground color for section header names.
    #[serde(default)] pub view_sec_head:       Option<String>,
    /// Background color for the entire section/column header line.
    #[serde(default)] pub view_head_bg:        Option<String>,
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
