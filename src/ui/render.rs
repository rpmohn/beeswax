use ratatui::Frame;
use crate::app::{App, AppScreen};
use super::{catmgr, view, viewmgr};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        AppScreen::View    => view::render(frame, app),
        AppScreen::CatMgr  => catmgr::render(frame, app),
        AppScreen::ViewMgr => viewmgr::render(frame, app),
    }
    // These dialogs float above all screens.
    let area = frame.area();
    view::render_ask_save_dialog(frame, app, area);
    view::render_password_entry_dialog(frame, app, area);
    view::render_view_add_dialog(frame, app, area);
    view::render_sec_props_dialog(frame, app, area);
}
