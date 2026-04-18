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
#[derive(Serialize, Deserialize, Default)]
pub struct CustomTheme {
    // ── Bars (title / fkey / menu) ───────────────────────────────────────────
    #[serde(default)] pub bar_fg:              Option<String>,
    #[serde(default)] pub bar_bg:              Option<String>,
    /// Currently-selected top-level menu item.
    #[serde(default)] pub bar_selected_fg:     Option<String>,
    #[serde(default)] pub bar_selected_bg:     Option<String>,

    // ── Selected field / line ────────────────────────────────────────────────
    #[serde(default)] pub selected_fg:         Option<String>,
    #[serde(default)] pub selected_bg:         Option<String>,
    #[serde(default)] pub selected_line_fg:    Option<String>,
    #[serde(default)] pub selected_line_bg:    Option<String>,

    // ── Modal dialogs ─────────────────────────────────────────────────────────
    #[serde(default)] pub dialog_fg:           Option<String>,
    #[serde(default)] pub dialog_bg:           Option<String>,
    #[serde(default)] pub dialog_border_fg:    Option<String>,
    #[serde(default)] pub dialog_border_bg:    Option<String>,
    /// Field label foreground in dialogs (unselected).
    #[serde(default)] pub dialog_label_fg:     Option<String>,
    /// Field label foreground in dialogs when that field is selected.
    #[serde(default)] pub dialog_label_sel_fg: Option<String>,

    // ── View body ─────────────────────────────────────────────────────────────
    /// View body background color.
    #[serde(default)] pub view_bg:             Option<String>,
    /// Foreground color for item text.
    #[serde(default)] pub view_item:           Option<String>,
    /// Foreground color for column value entries.
    #[serde(default)] pub view_col_entry:      Option<String>,
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

/// Save config to the platform config path, creating directories as needed.
/// If the file already exists, only the values managed by beeswax are touched;
/// unknown keys, blank lines, comments (including trailing comments), and all
/// other formatting are left exactly as they were.
pub fn save(cfg: &Config) -> std::io::Result<()> {
    let Some(path) = config_path() else { return Ok(()); };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Parse into a toml_edit document.  toml_edit preserves the raw source
    // text for every node it doesn't touch, so comments, blank lines, and
    // spacing survive the round-trip unchanged.
    let mut doc: toml_edit::DocumentMut = if path.exists() {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        text.parse().unwrap_or_default()
    } else {
        toml_edit::DocumentMut::new()
    };

    // Update a string value inside a toml_edit Table in-place.
    //
    // Captures the existing suffix decor (trailing whitespace + comment) before
    // replacing, then restores it on the new item.  This means a line like
    // `key = "old"  # comment` becomes `key = "new"  # comment`.
    //
    // When the key is absent or has an unexpected type it is simply inserted.
    fn set_str(table: &mut toml_edit::Table, key: &str, val: &str) {
        // Capture existing trailing comment (stored as value suffix decor).
        let existing_suffix: Option<String> =
            if let Some(toml_edit::Item::Value(toml_edit::Value::String(s))) = table.get(key) {
                s.decor().suffix().and_then(|rs| rs.as_str()).map(|s| s.to_owned())
            } else {
                None
            };

        table[key] = toml_edit::value(val);

        // Restore trailing comment on the newly-inserted item.
        if let Some(suffix) = existing_suffix {
            if let Some(toml_edit::Item::Value(toml_edit::Value::String(s))) =
                table.get_mut(key)
            {
                s.decor_mut().set_suffix(suffix);
            }
        }
    }

    let root = doc.as_table_mut();
    set_str(root, "nav_mode",    &cfg.nav_mode);
    set_str(root, "colorscheme", &cfg.colorscheme);

    // The 20 field names beeswax manages inside [custom_theme].
    const KNOWN: &[&str] = &[
        "bar_fg", "bar_bg", "bar_selected_fg", "bar_selected_bg",
        "selected_fg", "selected_bg", "selected_line_fg", "selected_line_bg",
        "dialog_fg", "dialog_bg", "dialog_border_fg", "dialog_border_bg",
        "dialog_label_fg", "dialog_label_sel_fg",
        "view_bg", "view_item", "view_col_entry", "view_col_head",
        "view_sec_head", "view_head_bg",
    ];

    // Collect the non-None custom_theme values.
    let ct_toml = toml::Value::try_from(&cfg.custom_theme)
        .unwrap_or(toml::Value::Table(toml::map::Map::new()));
    let new_ct = if let toml::Value::Table(m) = ct_toml { m } else { Default::default() };

    // Ensure [custom_theme] is a block table (create if absent or wrong type).
    if root.get("custom_theme").and_then(|i| i.as_table()).is_none() {
        root.insert("custom_theme", toml_edit::table());
    }

    if let Some(ct_item) = root.get_mut("custom_theme") {
        if let Some(ct) = ct_item.as_table_mut() {
            for &k in KNOWN {
                match new_ct.get(k) {
                    // Value is set: update in-place (preserves trailing comment) or insert.
                    Some(toml::Value::String(s)) => set_str(ct, k, s),
                    // Value was cleared (None): remove the key entirely.
                    _ => { ct.remove(k); }
                }
            }
            // Drop the section only when it is completely empty (no unknown keys left).
            if ct.is_empty() {
                root.remove("custom_theme");
            }
        }
    }

    std::fs::write(&path, doc.to_string())
}
