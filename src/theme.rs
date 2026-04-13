use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, PartialEq)]
pub enum ColorScheme {
    Default,
    AgendaColor,
    AgendaMono,
}

impl ColorScheme {
    pub fn from_str(s: &str) -> Self {
        match s {
            "AgendaColor" => ColorScheme::AgendaColor,
            "AgendaMono"  => ColorScheme::AgendaMono,
            _             => ColorScheme::Default,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ColorScheme::Default     => "Default",
            ColorScheme::AgendaColor => "AgendaColor",
            ColorScheme::AgendaMono  => "AgendaMono",
        }
    }
}

/// All pre-built styles for a color scheme. Used throughout rendering.
#[derive(Clone)]
pub struct Theme {
    /// Title bar, f-key bar, and menu bar background.
    pub bar:              Style,
    /// Selected item within a bar (appears un-highlighted against the bar).
    pub bar_cursor:       Style,
    /// Body area background — fills empty space and sets default text color.
    pub body:             Style,
    /// Normal (unselected) item text.
    pub item:             Style,
    /// Highlighted (selected) item or section head.
    pub item_selected:    Style,
    /// Unselected section head.
    pub section:          Style,
    /// Selected section head.
    pub section_selected: Style,
    /// Edit/create text cursor character.
    pub cursor:           Style,
    /// Modal dialog content area.
    pub dialog:           Style,
    /// Modal dialog border/frame.
    pub dialog_border:    Style,
    /// Dimmed hint / autocomplete text.
    pub dim:              Style,
}

impl Theme {
    pub fn for_scheme(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Default     => Self::default_theme(),
            ColorScheme::AgendaColor => Self::agenda_color(),
            ColorScheme::AgendaMono  => Self::agenda_mono(),
        }
    }

    fn default_theme() -> Self {
        let rev  = Style::default().add_modifier(Modifier::REVERSED);
        let bold = Style::default().add_modifier(Modifier::BOLD);
        Theme {
            bar:              rev,
            bar_cursor:       Style::default().remove_modifier(Modifier::REVERSED),
            body:             Style::default(),
            item:             Style::default(),
            item_selected:    rev,
            section:          bold,
            section_selected: Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
            cursor:           rev,
            dialog:           Style::default(),
            dialog_border:    Style::default(),
            dim:              Style::default().add_modifier(Modifier::DIM),
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
            bar:              Style::default().fg(bar_fg).bg(bar_bg),
            bar_cursor:       Style::default().fg(body_fg).bg(body_bg),
            body:             Style::default().fg(body_fg).bg(body_bg),
            item:             Style::default().fg(body_fg).bg(body_bg),
            item_selected:    Style::default().fg(sel_fg).bg(sel_bg),
            section:          Style::default().fg(Color::Blue).bg(body_bg).add_modifier(Modifier::BOLD),
            section_selected: Style::default().fg(sel_fg).bg(sel_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(Color::Blue).bg(body_bg),
            dim:              Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::DIM),
        }
    }

    fn agenda_mono() -> Self {
        let body_fg = Color::White;
        let body_bg = Color::Black;
        let sel_fg  = Color::Black;
        let sel_bg  = Color::White;
        Theme {
            bar:              Style::default().fg(sel_fg).bg(sel_bg),
            bar_cursor:       Style::default().fg(body_fg).bg(body_bg),
            body:             Style::default().fg(body_fg).bg(body_bg),
            item:             Style::default().fg(body_fg).bg(body_bg),
            item_selected:    Style::default().fg(sel_fg).bg(sel_bg),
            section:          Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::BOLD),
            section_selected: Style::default().fg(sel_fg).bg(sel_bg).add_modifier(Modifier::BOLD),
            cursor:           Style::default().fg(sel_fg).bg(sel_bg),
            dialog:           Style::default().fg(body_fg).bg(body_bg),
            dialog_border:    Style::default().fg(body_fg).bg(body_bg),
            dim:              Style::default().fg(body_fg).bg(body_bg).add_modifier(Modifier::DIM),
        }
    }
}
