use ratatui::style::{Color, Modifier, Style};
use crate::config::CustomTheme;

// ── Solarized palette ─────────────────────────────────────────────────────────
const S_BASE03:  Color = Color::Rgb(0x00, 0x2b, 0x36);  // darkest bg
const S_BASE02:  Color = Color::Rgb(0x07, 0x36, 0x42);  // dark bg highlights
const S_BASE01:  Color = Color::Rgb(0x58, 0x6e, 0x75);  // optional emphasis (dark)
const S_BASE00:  Color = Color::Rgb(0x65, 0x7b, 0x83);  // body text (dark)
const S_BASE0:   Color = Color::Rgb(0x83, 0x94, 0x96);  // body text (light)
const S_BASE1:   Color = Color::Rgb(0x93, 0xa1, 0xa1);  // optional emphasis (light)
const S_BASE2:   Color = Color::Rgb(0xee, 0xe8, 0xd5);  // light bg highlights
const S_BASE3:   Color = Color::Rgb(0xfd, 0xf6, 0xe3);  // lightest bg
const S_BLUE:    Color = Color::Rgb(0x26, 0x8b, 0xd2);
const S_CYAN:    Color = Color::Rgb(0x2a, 0xa1, 0x98);

// ── Gruvbox palette ───────────────────────────────────────────────────────────
// dark backgrounds
const G_BG:      Color = Color::Rgb(0x28, 0x28, 0x28);
const G_BG1:     Color = Color::Rgb(0x3c, 0x38, 0x36);
const G_BG2:     Color = Color::Rgb(0x50, 0x49, 0x45);
const G_FG:      Color = Color::Rgb(0xeb, 0xdb, 0xb2);
const G_FG4:     Color = Color::Rgb(0xa8, 0x99, 0x84);  // muted fg (dark)
const G_GRAY:    Color = Color::Rgb(0x92, 0x83, 0x74);
// light backgrounds
const G_BG_L:    Color = Color::Rgb(0xfb, 0xf1, 0xc7);
const G_BG2_L:   Color = Color::Rgb(0xd5, 0xc4, 0xa1);
const G_BG4_L:   Color = Color::Rgb(0xa8, 0x99, 0x84);  // muted fg (light bar)
const G_FG_L:    Color = Color::Rgb(0x3c, 0x38, 0x36);
const G_FG4_L:   Color = Color::Rgb(0x7c, 0x6f, 0x64);  // dim text (light)
// shared accents
const G_YELLOW:  Color = Color::Rgb(0xfa, 0xbd, 0x2f);  // bright yellow — selection
const G_BLUE_D:  Color = Color::Rgb(0x83, 0xa5, 0x98);  // bright blue (dark theme)
const G_BLUE_L:  Color = Color::Rgb(0x07, 0x66, 0x78);  // dark blue (light theme)

// ── Dracula palette ───────────────────────────────────────────────────────────
const D_BG:      Color = Color::Rgb(0x28, 0x2a, 0x36);
const D_CUR:     Color = Color::Rgb(0x44, 0x47, 0x5a);  // current line / bar bg
const D_FG:      Color = Color::Rgb(0xf8, 0xf8, 0xf2);
const D_COMMENT: Color = Color::Rgb(0x62, 0x72, 0xa4);
const D_PURPLE:  Color = Color::Rgb(0xbd, 0x93, 0xf9);
const D_CYAN:    Color = Color::Rgb(0x8b, 0xe9, 0xfd);

// ── Built-in line-selection backgrounds ──────────────────────────────────────
const AGENDA_COLOR_LINE_BG: Color = Color::Rgb(0x99, 0x00, 0x00);  // dark red (toned-down Red)
const AGENDA_MONO_LINE_BG:  Color = Color::Rgb(0x80, 0x80, 0x80);  // mid-gray (toned-down White)

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Default,
    AgendaColor,
    AgendaMono,
    SolarizedDark,
    SolarizedLight,
    GruvboxDark,
    GruvboxLight,
    Dracula,
    Custom,
}

impl ColorScheme {
    pub fn from_str(s: &str) -> Self {
        match s {
            "AgendaColor"    => ColorScheme::AgendaColor,
            "AgendaMono"     => ColorScheme::AgendaMono,
            "SolarizedDark"  => ColorScheme::SolarizedDark,
            "SolarizedLight" => ColorScheme::SolarizedLight,
            "GruvboxDark"    => ColorScheme::GruvboxDark,
            "GruvboxLight"   => ColorScheme::GruvboxLight,
            "Dracula"        => ColorScheme::Dracula,
            "Custom"         => ColorScheme::Custom,
            _                => ColorScheme::Default,
        }
    }

}

// ─────────────────────────────────────────────────────────────────────────────

/// All pre-built styles for a color scheme. Used throughout rendering.
#[derive(Clone)]
pub struct Theme {
    /// Title bar, f-key bar, and menu bar background.
    pub bar:              Style,
    /// Selected item within a bar (appears un-highlighted against the bar).
    pub bar_cursor:       Style,
    /// Highlighted (selected) field / edit cursor within the selected row.
    pub item_selected_field: Style,
    /// Background highlight for the entire selected item row (toned-down).
    pub item_selected_line:  Style,
    /// Unselected section head.
    pub section:          Style,
    /// Edit/create text cursor character.
    pub cursor:           Style,
    /// Modal dialog content area.
    pub dialog:           Style,
    /// Modal dialog border/frame.
    pub dialog_border:    Style,
    /// Field label text in dialogs (unselected).
    pub dialog_label:     Style,
    /// Field label text in dialogs when that field is selected.
    pub dialog_label_sel: Style,
    /// Dimmed hint / autocomplete text.
    pub dim:              Style,

    // ── View-specific ─────────────────────────────────────────────────────────
    /// View body background + default fg (paragraph base style).
    pub view_bg:       Style,
    /// Foreground for unselected item text.
    pub view_item:     Style,
    /// Foreground for column value entries.
    pub view_col:      Style,
    /// Foreground for column header labels.
    pub view_col_head: Style,
    /// Foreground for section header names (rendering code adds BOLD).
    pub view_sec_head: Style,
    /// Background applied to the entire section/column header line.
    pub view_head_bg:  Style,
}

impl Theme {
    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Default      => Self::default_theme(),
            ColorScheme::AgendaColor  => Self::agenda_color(),
            ColorScheme::AgendaMono   => Self::agenda_mono(),
            ColorScheme::SolarizedDark  => Self::solarized_dark(),
            ColorScheme::SolarizedLight => Self::solarized_light(),
            ColorScheme::GruvboxDark  => Self::gruvbox_dark(),
            ColorScheme::GruvboxLight => Self::gruvbox_light(),
            ColorScheme::Dracula      => Self::dracula(),
            ColorScheme::Custom       => Self::default_theme(), // replaced by from_custom()
        }
    }

    /// Build a theme from the [custom_theme] config table.
    /// Any field not supplied falls back to the Default (REVERSED) theme.
    pub fn from_custom(c: &CustomTheme) -> Self {
        let def = Self::default_theme();

        // Helper: parse an optional hex color, fall back to `fallback`.
        let color = |opt: &Option<String>, fallback: Option<Color>| -> Option<Color> {
            if let Some(s) = opt { parse_hex(s).or(fallback) } else { fallback }
        };

        let body_fg   = color(&c.body_fg,   None);
        let body_bg   = color(&c.body_bg,   None);
        let bar_fg    = color(&c.bar_fg,    None);
        let bar_bg    = color(&c.bar_bg,    None);
        let barcur_fg = color(&c.bar_cursor_fg, body_fg);
        let barcur_bg = color(&c.bar_cursor_bg, body_bg);
        let sel_fg    = color(&c.selected_fg, None);
        let sel_bg    = color(&c.selected_bg, None);
        let sline_fg  = color(&c.selected_line_fg, sel_fg);
        let sline_bg  = color(&c.selected_line_bg, None);
        let sec_fg    = color(&c.section_fg,  None);

        let dlg_fg      = color(&c.dialog_fg,           body_fg);
        let dlg_bg      = color(&c.dialog_bg,           body_bg);
        let dlgbrd_fg   = color(&c.dialog_border_fg,    None);
        let dlgbrd_bg   = color(&c.dialog_border_bg,    None);
        let dlglbl_fg   = color(&c.dialog_label_fg,     dlg_fg);
        let dlglblsel_fg = color(&c.dialog_label_sel_fg, sel_fg);
        let vbg_bg    = color(&c.view_bg,            body_bg);
        let vitem_fg  = color(&c.view_item,          body_fg);
        let vcol_fg   = color(&c.view_col,           body_fg);
        let vcolh_fg  = color(&c.view_col_head,      sec_fg.or(body_fg));
        let vsech_fg  = color(&c.view_sec_head,      sec_fg);
        let vhbg_bg   = color(&c.view_head_bg,       vbg_bg);

        let apply = |s: Style, fg: Option<Color>, bg: Option<Color>| -> Style {
            let s = if let Some(f) = fg { s.fg(f) } else { s };
            if let Some(b) = bg { s.bg(b) } else { s }
        };

        // If no custom colors supplied for an element, fall back to the
        // Default theme's modifier-based style.
        let bar = if bar_fg.is_some() || bar_bg.is_some() {
            apply(Style::default(), bar_fg, bar_bg)
        } else { def.bar };

        let bar_cursor = if barcur_fg.is_some() || barcur_bg.is_some() {
            apply(Style::default(), barcur_fg, barcur_bg)
        } else { def.bar_cursor };

        let item_selected_field = if sel_fg.is_some() || sel_bg.is_some() {
            apply(Style::default(), sel_fg, sel_bg)
        } else { def.item_selected_field };

        let item_selected_line = if sline_fg.is_some() || sline_bg.is_some() {
            apply(Style::default(), sline_fg, sline_bg)
        } else { def.item_selected_line };

        let section = if sec_fg.is_some() || body_bg.is_some() {
            apply(Style::default(), sec_fg.or(body_fg), body_bg)
                .add_modifier(Modifier::BOLD)
        } else { def.section };

        let cursor = if sel_fg.is_some() || sel_bg.is_some() {
            apply(Style::default(), sel_fg, sel_bg)
        } else { def.cursor };

        let dialog = apply(Style::default(), dlg_fg, dlg_bg);
        let dialog_border    = apply(Style::default(), dlgbrd_fg.or(dlg_fg), dlgbrd_bg.or(dlg_bg));
        let dialog_label     = apply(Style::default(), dlglbl_fg, None);
        let dialog_label_sel = apply(Style::default(), dlglblsel_fg, None);
        let dim = apply(Style::default(), body_fg, body_bg).add_modifier(Modifier::DIM);

        let view_bg       = apply(Style::default(), body_fg, vbg_bg);
        let view_item     = apply(Style::default(), vitem_fg, None);
        let view_col      = apply(Style::default(), vcol_fg,  None);
        let view_col_head = apply(Style::default(), vcolh_fg, None);
        let view_sec_head = apply(Style::default(), vsech_fg, None);
        let view_head_bg  = apply(Style::default(), None, vhbg_bg);

        Theme { bar, bar_cursor, item_selected_field, item_selected_line,
                section, cursor, dialog, dialog_border, dialog_label, dialog_label_sel, dim,
                view_bg, view_item, view_col, view_col_head, view_sec_head, view_head_bg }
    }

    fn default_theme() -> Self {
        let rev  = Style::default().add_modifier(Modifier::REVERSED);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        Theme {
            bar:                  rev,
            bar_cursor:           Style::default().remove_modifier(Modifier::REVERSED),
            item_selected_field:  rev,
            item_selected_line:   rev,
            section:              bold,
            cursor:           rev,
            dialog:           Style::default(),
            dialog_border:    Style::default(),
            dialog_label:     Style::default().add_modifier(Modifier::DIM),
            dialog_label_sel: bold,
            dim:              Style::default().add_modifier(Modifier::DIM),
            view_bg:          Style::default(),
            view_item:        Style::default(),
            view_col:         Style::default(),
            view_col_head:    Style::default(),
            view_sec_head:    Style::default(),
            view_head_bg:     Style::default(),
        }
    }

    fn agenda_color() -> Self {
        let body_fg = Color::Black;
        let body_bg = Color::Cyan;
        let sel_fg  = Color::White;
        let sel_bg  = Color::Red;
        let bar_fg  = Color::White;
        let bar_bg  = Color::Blue;
        Theme {
            bar:                  Style::default().fg(bar_fg).bg(bar_bg),
            bar_cursor:           Style::default().fg(body_fg).bg(body_bg),
            item_selected_field:  Style::default().fg(sel_fg).bg(sel_bg),
            item_selected_line:   Style::default().fg(sel_fg).bg(AGENDA_COLOR_LINE_BG),
            section:          Style::default().fg(Color::Blue).bg(body_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(Color::Red).bg(body_bg),
            dialog_label:     Style::default().fg(Color::Blue),
            dialog_label_sel: Style::default().fg(Color::Red),
            dim:              Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(body_fg).bg(body_bg),
            view_item:        Style::default().fg(body_fg),
            view_col:         Style::default().fg(body_fg),
            view_col_head:    Style::default().fg(Color::Blue),
            view_sec_head:    Style::default().fg(Color::Blue),
            view_head_bg:     Style::default().bg(body_bg),
        }
    }

    fn agenda_mono() -> Self {
        let body_fg = Color::White;
        let body_bg = Color::Black;
        let sel_fg  = Color::Black;
        let sel_bg  = Color::White;
        Theme {
            bar:                  Style::default().fg(sel_fg).bg(sel_bg),
            bar_cursor:           Style::default().fg(body_fg).bg(body_bg),
            item_selected_field:  Style::default().fg(sel_fg).bg(sel_bg),
            item_selected_line:   Style::default().fg(sel_fg).bg(AGENDA_MONO_LINE_BG),
            section:          Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(body_fg).bg(body_bg),
            dialog_label:     Style::default().fg(body_fg).add_modifier(Modifier::DIM),
            dialog_label_sel: Style::default().fg(body_fg).add_modifier(Modifier::BOLD),
            dim:              Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(body_fg).bg(body_bg),
            view_item:        Style::default().fg(body_fg),
            view_col:         Style::default().fg(body_fg),
            view_col_head:    Style::default().fg(body_fg),
            view_sec_head:    Style::default().fg(body_fg),
            view_head_bg:     Style::default().bg(body_bg),
        }
    }

    fn solarized_dark() -> Self {
        // body text on darkest background
        let body_fg  = S_BASE0;   // #839496
        let body_bg  = S_BASE03;  // #002b36
        // bars on slightly lighter dark bg
        let bar_fg   = S_BASE1;   // #93a1a1
        let bar_bg   = S_BASE02;  // #073642
        // selection / cursor: bright text on blue accent
        let sel_fg   = S_BASE3;   // #fdf6e3
        let sel_bg   = S_BLUE;    // #268bd2
        Theme {
            bar:                  Style::default().fg(bar_fg).bg(bar_bg),
            bar_cursor:           Style::default().fg(body_fg).bg(body_bg),
            item_selected_field:  Style::default().fg(sel_fg).bg(sel_bg),
            item_selected_line:   Style::default().fg(sel_fg).bg(S_BASE01),
            section:              Style::default().fg(S_CYAN).bg(body_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(S_CYAN).bg(body_bg),
            dialog_label:     Style::default().fg(S_BASE01),
            dialog_label_sel: Style::default().fg(S_CYAN),
            dim:              Style::default().fg(S_BASE01).bg(body_bg).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(body_fg).bg(body_bg),
            view_item:        Style::default().fg(body_fg),
            view_col:         Style::default().fg(body_fg),
            view_col_head:    Style::default().fg(S_CYAN),
            view_sec_head:    Style::default().fg(S_CYAN),
            view_head_bg:     Style::default().bg(body_bg),
        }
    }

    fn solarized_light() -> Self {
        // body text on lightest background
        let body_fg  = S_BASE00;  // #657b83
        let body_bg  = S_BASE3;   // #fdf6e3
        // bars on slightly darker light bg
        let bar_fg   = S_BASE01;  // #586e75
        let bar_bg   = S_BASE2;   // #eee8d5
        // selection / cursor: bright text on blue accent
        let sel_fg   = S_BASE3;   // #fdf6e3
        let sel_bg   = S_BLUE;    // #268bd2
        Theme {
            bar:                  Style::default().fg(bar_fg).bg(bar_bg),
            bar_cursor:           Style::default().fg(body_fg).bg(body_bg),
            item_selected_field:  Style::default().fg(sel_fg).bg(sel_bg),
            item_selected_line:   Style::default().fg(sel_fg).bg(S_BASE2),
            section:              Style::default().fg(S_BLUE).bg(body_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(S_BLUE).bg(body_bg),
            dialog_label:     Style::default().fg(S_BASE1),
            dialog_label_sel: Style::default().fg(S_BLUE),
            dim:              Style::default().fg(S_BASE1).bg(body_bg).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(body_fg).bg(body_bg),
            view_item:        Style::default().fg(body_fg),
            view_col:         Style::default().fg(body_fg),
            view_col_head:    Style::default().fg(S_BLUE),
            view_sec_head:    Style::default().fg(S_BLUE),
            view_head_bg:     Style::default().bg(body_bg),
        }
    }

    fn gruvbox_dark() -> Self {
        // warm cream text on charcoal; bright yellow selection; muted blue section heads
        Theme {
            bar:                  Style::default().fg(G_FG4).bg(G_BG1),
            bar_cursor:           Style::default().fg(G_FG).bg(G_BG),
            item_selected_field:  Style::default().fg(G_BG).bg(G_YELLOW),
            item_selected_line:   Style::default().fg(G_BG).bg(G_BG2),
            section:          Style::default().fg(G_BLUE_D).bg(G_BG).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(G_BG).bg(G_YELLOW),
            dialog:           Style::default().fg(G_FG).bg(G_BG),
            dialog_border:    Style::default().fg(G_YELLOW).bg(G_BG),
            dialog_label:     Style::default().fg(G_FG4),
            dialog_label_sel: Style::default().fg(G_YELLOW),
            dim:              Style::default().fg(G_GRAY).bg(G_BG).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(G_FG).bg(G_BG),
            view_item:        Style::default().fg(G_FG),
            view_col:         Style::default().fg(G_FG),
            view_col_head:    Style::default().fg(G_BLUE_D),
            view_sec_head:    Style::default().fg(G_BLUE_D),
            view_head_bg:     Style::default().bg(G_BG),
        }
    }

    fn gruvbox_light() -> Self {
        // dark warm text on cream; bright yellow selection; dark blue section heads
        Theme {
            bar:                  Style::default().fg(G_BG4_L).bg(G_BG2_L),
            bar_cursor:           Style::default().fg(G_FG_L).bg(G_BG_L),
            item_selected_field:  Style::default().fg(G_BG_L).bg(G_YELLOW),
            item_selected_line:   Style::default().fg(G_BG_L).bg(G_BG2_L),
            section:          Style::default().fg(G_BLUE_L).bg(G_BG_L).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(G_BG_L).bg(G_YELLOW),
            dialog:           Style::default().fg(G_FG_L).bg(G_BG_L),
            dialog_border:    Style::default().fg(G_BLUE_L).bg(G_BG_L),
            dialog_label:     Style::default().fg(G_FG4_L),
            dialog_label_sel: Style::default().fg(G_BLUE_L),
            dim:              Style::default().fg(G_FG4_L).bg(G_BG_L).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(G_FG_L).bg(G_BG_L),
            view_item:        Style::default().fg(G_FG_L),
            view_col:         Style::default().fg(G_FG_L),
            view_col_head:    Style::default().fg(G_BLUE_L),
            view_sec_head:    Style::default().fg(G_BLUE_L),
            view_head_bg:     Style::default().bg(G_BG_L),
        }
    }

    fn dracula() -> Self {
        // light fg on dark purple-grey bg; purple selection; cyan section heads
        Theme {
            bar:                  Style::default().fg(D_FG).bg(D_CUR),
            bar_cursor:           Style::default().fg(D_FG).bg(D_BG),
            item_selected_field:  Style::default().fg(D_BG).bg(D_PURPLE),
            item_selected_line:   Style::default().fg(D_BG).bg(D_CUR),
            section:          Style::default().fg(D_CYAN).bg(D_BG).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(D_BG).bg(D_PURPLE),
            dialog:           Style::default().fg(D_FG).bg(D_BG),
            dialog_border:    Style::default().fg(D_PURPLE).bg(D_BG),
            dialog_label:     Style::default().fg(D_COMMENT),
            dialog_label_sel: Style::default().fg(D_PURPLE),
            dim:              Style::default().fg(D_COMMENT).bg(D_BG).add_modifier(Modifier::DIM),
            view_bg:          Style::default().fg(D_FG).bg(D_BG),
            view_item:        Style::default().fg(D_FG),
            view_col:         Style::default().fg(D_FG),
            view_col_head:    Style::default().fg(D_CYAN),
            view_sec_head:    Style::default().fg(D_CYAN),
            view_head_bg:     Style::default().bg(D_BG),
        }
    }
}  // impl Theme

// ── Hex color parser ──────────────────────────────────────────────────────────

/// Parse a "#rrggbb" hex string into a ratatui Color.  Returns None if malformed.
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 { return None; }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
