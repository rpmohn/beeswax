use ratatui::Frame;
use crate::app::{App, AppScreen};
use super::{catmgr, view};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        AppScreen::View   => view::render(frame, app),
        AppScreen::CatMgr => catmgr::render(frame, app),
    }
}
