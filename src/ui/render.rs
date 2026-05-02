use ratatui::Frame;
use crate::app::{App, AppScreen};
use super::{catmgr, customize, view, viewmgr};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        AppScreen::View    => view::render(frame, app),
        AppScreen::CatMgr  => catmgr::render(frame, app),
        AppScreen::ViewMgr => viewmgr::render(frame, app),
    }
    // These dialogs float above all screens.
    let area = frame.area();
    view::render_ask_save_dialog(frame, app, area);
    view::render_file_props_dialog(frame, app, area);
    view::render_view_add_dialog(frame, app, area);
    view::render_sec_props_dialog(frame, app, area);
    catmgr::render_cat_props_modal(frame, app, area);
    viewmgr::render_view_props_overlay(frame, app, area);
    customize::render_overlay(frame, app, area);
}
