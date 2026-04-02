use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::{App, AppScreen, AssignMode, CatMode, ColMode, FKeyMod, MenuState, Mode, SectionMode, SortState, ViewMgrMode};

/// Action labels for F1–F10 (index 0 = F1, index 9 = F10).
pub struct FKeyLabels {
    pub normal: [&'static str; 10],
    pub shift:  [&'static str; 10],
    pub ctrl:   [&'static str; 10],
    pub alt:    [&'static str; 10],
}

pub static VIEW_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "Choices", "Done", "Note", "Props", "Mark", "Vw Mgr", "Cat Mgr", "Menu"],
    shift:  ["",     "",     "",  "",  "",     "",      "",      "",       "",        ""    ],
    ctrl:   ["",     "",     "",  "",  "",     "",      "",      "",       "",        ""    ],
    alt:    ["",     "",     "",  "",  "",     "",      "",      "",       "",        ""    ],
};

pub static CATMGR_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "",  "",  "Note", "Props", "Prm (\u{2190})", "Dem (\u{2192})", "To View", "Menu"],
    shift:  ["",     "",     "",  "",  "",     "",      "",               "",               "",         ""   ],
    ctrl:   ["",     "",     "",  "",  "",     "",      "",               "",               "",         ""   ],
    alt:    ["",     "",     "",  "",  "",     "",      "",               "",               "",         ""   ],
};

static COL_FORM_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "Choices", "", "", "Props", "", "Default", "", ""],
    shift:  ["",     "",     "",        "", "", "",      "", "",         "", ""],
    ctrl:   ["",     "",     "",        "", "", "",      "", "",         "", ""],
    alt:    ["",     "",     "",        "", "", "",      "", "",         "", ""],
};

static CALENDAR_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "", "", "", "", "SetTime", "", "", "", ""],
    shift:  ["",     "", "", "", "", "",        "", "", "", ""],
    ctrl:   ["",     "", "", "", "", "",        "", "", "", ""],
    alt:    ["",     "", "", "", "", "",        "", "", "", ""],
};

static SET_TIME_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "", "", "", "", "", "", "", "", ""],
    shift:  ["",     "", "", "", "", "", "", "", "", ""],
    ctrl:   ["",     "", "", "", "", "", "", "", "", ""],
    alt:    ["",     "", "", "", "", "", "", "", "", ""],
};

static SEARCH_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "", "", "", "", "", "PrevSel", "NextSel", "", ""],
    shift:  ["",     "", "", "", "", "", "",        "",        "", ""],
    ctrl:   ["",     "", "", "", "", "", "",        "",        "", ""],
    alt:    ["",     "", "", "", "", "", "",        "",        "", ""],
};

static ASSIGN_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "", "", "", "Props", "InclCat", "", "", ""],
    shift:  ["",     "",     "", "", "", "",      "",        "", "", ""],
    ctrl:   ["",     "",     "", "", "", "",      "",        "", "", ""],
    alt:    ["",     "",     "", "", "", "",      "",        "", "", ""],
};

static QUICK_ADD_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "", "", "", "Props", "", "", "", ""],
    shift:  ["",     "",     "", "", "", "",      "", "", "", ""],
    ctrl:   ["",     "",     "", "", "", "",      "", "", "", ""],
    alt:    ["",     "",     "", "", "", "",      "", "", "", ""],
};

static ITEM_PROPS_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "Choices", "", "", "Props", "", "Default", "", ""],
    shift:  ["",     "",     "",        "", "", "",      "", "",         "", ""],
    ctrl:   ["",     "",     "",        "", "", "",      "", "",         "", ""],
    alt:    ["",     "",     "",        "", "", "",      "", "",         "", ""],
};

static MENU_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "", "", "", "", "", "", "", "", ""],
    shift:  ["",     "", "", "", "", "", "", "", "", ""],
    ctrl:   ["",     "", "", "", "", "", "", "", "", ""],
    alt:    ["",     "", "", "", "", "", "", "", "", ""],
};

static CATPROPS_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "Choices", "", "", "Props", "", "Default", "", ""],
    shift:  ["",     "",     "",        "", "", "",      "", "",         "", ""],
    ctrl:   ["",     "",     "",        "", "", "",      "", "",         "", ""],
    alt:    ["",     "",     "",        "", "", "",      "", "",         "", ""],
};

static EDIT_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Paste", "Copy", "Cut",    "Note", "Marker", "Mark", "", "", ""],
    shift:  ["",     "",      "",     "",        "",     "",       "",     "", "", ""],
    ctrl:   ["",     "",      "",     "",        "",     "",       "",     "", "", ""],
    alt:    ["Compose", "MakeCat", "", "Delete", "",     "",       "Split","", "", ""],
};

pub static VIEWMGR_FKEYS: FKeyLabels = FKeyLabels {
    normal: ["Help", "Edit", "", "Delete", "", "Props", "", "To View", "Cat Mgr", "Menu"],
    shift:  ["",     "",     "", "",       "", "",      "", "",        "",         ""   ],
    ctrl:   ["",     "",     "", "",       "", "",      "", "",        "",         ""   ],
    alt:    ["",     "",     "", "",       "", "",      "", "",        "",         ""   ],
};

/// Render the two-row, 10-section F-key bar into `area` (must be 2 rows tall).
pub fn render_fkey_bar(frame: &mut Frame, area: Rect, app: &App) {
    let def = if !matches!(app.menu, MenuState::Closed) {
        &MENU_FKEYS
    } else if app.cat_search.is_some() {
        &SEARCH_FKEYS
    } else if matches!(app.assign_mode, AssignMode::Profile { .. }) {
        &ASSIGN_FKEYS
    } else if matches!(app.sec_mode, SectionMode::Add { .. } | SectionMode::Choices { .. } | SectionMode::ConfirmRemove { .. }) {
        &MENU_FKEYS   // section dialogs are self-describing
    } else if matches!(app.sec_mode, SectionMode::Props { sort_state: SortState::Dialog { .. }, .. }) {
        &MENU_FKEYS   // sort dialog is self-describing
    } else if matches!(app.sec_mode, SectionMode::Props { .. }) {
        &MENU_FKEYS   // section props is self-describing
    } else if matches!(app.mode, Mode::ItemProps { .. }) {
        &ITEM_PROPS_FKEYS
    } else if matches!(app.mode, Mode::ConfirmDeleteItem { .. }) {
        &MENU_FKEYS   // item delete dialog is self-describing
    } else if matches!(app.col_mode, ColMode::QuickAdd { .. }) {
        &QUICK_ADD_FKEYS
    } else if matches!(app.col_mode, ColMode::ConfirmRemove { .. }) {
        &MENU_FKEYS
    } else if matches!(app.col_mode, ColMode::Calendar { .. }) {
        &CALENDAR_FKEYS
    } else if matches!(app.col_mode, ColMode::SetTime { .. }) {
        &SET_TIME_FKEYS
    } else if matches!(app.col_mode, ColMode::Form { .. } | ColMode::Choices { .. } | ColMode::Props { .. }) {
        &COL_FORM_FKEYS
    } else if matches!(app.cat_state.mode, CatMode::Props { .. }) {
        &CATPROPS_FKEYS
    } else if matches!(app.mode, Mode::Edit { .. } | Mode::Create { .. }) {
        &EDIT_FKEYS
    } else if matches!(app.vmgr_state.mode, ViewMgrMode::Rename { .. } | ViewMgrMode::Props { .. }) {
        &EDIT_FKEYS
    } else if matches!(app.vmgr_state.mode, ViewMgrMode::ConfirmDelete { .. }) {
        &MENU_FKEYS
    } else {
        match app.screen {
            AppScreen::View    => &VIEW_FKEYS,
            AppScreen::CatMgr  => &CATMGR_FKEYS,
            AppScreen::ViewMgr => &VIEWMGR_FKEYS,
        }
    };
    let actions = match app.fkey_mod {
        FKeyMod::Normal => &def.normal,
        FKeyMod::Shift  => &def.shift,
        FKeyMod::Ctrl   => &def.ctrl,
        FKeyMod::Alt    => &def.alt,
    };
    let prefix = match app.fkey_mod {
        FKeyMod::Normal => "",
        FKeyMod::Shift  => "Shf",
        FKeyMod::Ctrl   => "Ctl",
        FKeyMod::Alt    => "Alt",
    };

    // Layout: sec0|sec1|...|sec9  — 9 pipe chars + 10 section widths = area.width
    let w      = area.width as usize;
    let avail  = w.saturating_sub(9);
    let sec_w  = avail / 10;
    let extra  = avail % 10;  // first `extra` sections get sec_w+1

    let key_names: Vec<String> = (1..=10).map(|n| format!("{}F{}", prefix, n)).collect();
    let action_strs: Vec<String> = actions.iter().map(|s| s.to_string()).collect();

    let row1 = build_row(&key_names, sec_w, extra);
    let row2 = build_row(&action_strs, sec_w, extra);

    frame.render_widget(
        Paragraph::new(vec![row1, row2])
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
    );
}

fn build_row(labels: &[String], sec_w: usize, extra: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        if i > 0 { spans.push(Span::raw("|")); }
        let w = if i < extra { sec_w + 1 } else { sec_w };
        spans.push(Span::raw(center_in(label, w)));
    }
    Line::from(spans)
}

fn center_in(s: &str, w: usize) -> String {
    if w == 0 { return String::new(); }
    let len = s.chars().count();
    if len == 0 { return " ".repeat(w); }
    if len >= w  { return s.chars().take(w).collect(); }
    let pad = w - len;
    let l = pad / 2;
    let r = pad - l;
    format!("{}{}{}", " ".repeat(l), s, " ".repeat(r))
}
