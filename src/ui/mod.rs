pub mod font;
pub mod panes;
pub mod pixel;
pub mod prompt;
pub mod screens;
pub mod theme;
pub mod wrap;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::widgets::Block;

use crate::app::App;

/// Sidebar appears once the terminal is wide enough to spare it.
const SIDEBAR_MIN_WIDTH: u16 = 96;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

    if app.seq.is_some() {
        screens::sequence(f, app);
        let g = app.seq.as_ref().map(|s| s.glitch).unwrap_or(0.0);
        if g > 0.0 || app.glitching() {
            screens::glitch(f, (app.t * 30.0) as u64 + 7, 0.8);
        }
        return;
    }

    let footer_h = if area.height >= 24 { 1 } else { 0 };
    let choice_h = prompt::choice_height(app).min(area.height / 3);
    let [header, body, choice, input, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(choice_h),
        Constraint::Length(3),
        Constraint::Length(footer_h),
    ])
    .areas(area);

    panes::header(f, app, header);
    if area.width >= SIDEBAR_MIN_WIDTH {
        let side_w = if area.width >= 130 { 36 } else { 30 };
        let [main, side] =
            Layout::horizontal([Constraint::Min(40), Constraint::Length(side_w)]).areas(body);
        app.link_hits = panes::terminal(f, app, main);
        panes::sidebar(f, app, side);
    } else {
        app.link_hits = panes::terminal(f, app, body);
    }
    prompt::choices(f, app, choice);
    prompt::input(f, app, input);
    if footer_h > 0 {
        prompt::footer(f, app, footer);
    }
    screens::overlay(f, app);

    if app.glitching() {
        let left = (app.glitch_until - app.t).clamp(0.0, 1.0);
        screens::glitch(f, (app.t * 30.0) as u64 + 1, 0.4 + left * 0.6);
    }
}
