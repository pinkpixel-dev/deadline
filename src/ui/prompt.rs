//! Choice panel, typing indicator, command line and key hints.

use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use unicode_width::UnicodeWidthStr;

use super::panes::panel;
use super::theme;
use crate::app::App;
use crate::game::text;

/// Rows the choice panel needs this frame.
pub fn choice_height(app: &App) -> u16 {
    match &app.choice {
        Some(c) => c.opts.len() as u16 + 2,
        None if app.chat.as_ref().and_then(|c| c.typing()).is_some() => 1,
        None => 0,
    }
}

pub fn choices(f: &mut Frame, app: &mut App, area: Rect) {
    app.hits.clear();
    if area.height == 0 {
        return;
    }
    let Some(choice) = &app.choice else {
        if let Some(who) = app.chat.as_ref().and_then(|c| c.typing()) {
            let dots = ["   ", ".  ", ".. ", "..."][((app.t * 3.0) as usize) % 4];
            let color = text::user_color(who, &app.ctx());
            let line = Line::from(vec![
                Span::styled(format!("  {who}"), Style::default().fg(color)),
                Span::styled(format!(" is typing{dots}"), theme::dim()),
            ]);
            f.render_widget(Paragraph::new(line), area);
        }
        return;
    };
    let title = if choice.from_chat { "REPLY".to_string() } else { strip_tags(&choice.prompt) };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::AMBER))
        .title(Span::styled(format!(" {title} "), theme::bold(theme::AMBER)))
        .style(Style::default().bg(theme::BG));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let cx = app.ctx();
    let mut lines = Vec::new();
    let mut hits = Vec::new();
    for (i, o) in choice.opts.iter().enumerate() {
        let key = o.key.clone().map(|k| k.to_uppercase()).unwrap_or_else(|| (i + 1).to_string());
        let selected = i == choice.sel;
        let body = text::subst(&o.text, &cx);
        let style = if selected { theme::bold(theme::WHITE) } else { Style::default().fg(theme::TEXT) };
        let mut spans = vec![
            Span::styled(if selected { " › " } else { "   " }, Style::default().fg(theme::AMBER)),
            Span::styled(format!("[{key}] "), theme::bold(theme::AMBER)),
        ];
        spans.extend(text::spans(&body, style));
        lines.push(Line::from(spans));
        if (i as u16) < inner.height {
            hits.push((Rect { y: inner.y + i as u16, height: 1, ..inner }, i));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
    app.hits = hits;
}

fn strip_tags(s: &str) -> String {
    text::spans(s, Style::default()).iter().map(|s| s.content.as_ref()).collect()
}

pub fn input(f: &mut Frame, app: &App, area: Rect) {
    let block = panel("", false);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let (prompt, body) = if app.password.is_some() {
        ("PASSWORD: ".to_string(), "*".repeat(app.input.len()))
    } else {
        (format!("{}@deadline:{} > ", app.st.player, app.st.cwd), app.input.buf.clone())
    };
    let prompt_w = prompt.width();
    let avail = (inner.width as usize).saturating_sub(prompt_w + 1);
    let chars: Vec<char> = body.chars().collect();
    let start = app.input.cursor.saturating_sub(avail);
    let visible: String = chars.iter().skip(start).take(avail).collect();
    let line = Line::from(vec![
        Span::styled(prompt, Style::default().fg(if app.password.is_some() { theme::AMBER } else { theme::DIM })),
        Span::styled(visible, theme::bold(theme::WHITE)),
    ]);
    f.render_widget(Paragraph::new(line), inner);
    if !app.modal() || app.password.is_some() {
        let before: String = chars[start..app.input.cursor.min(chars.len())].iter().collect();
        let x = inner.x + (prompt_w + before.width()) as u16;
        f.set_cursor_position(Position::new(x.min(inner.right().saturating_sub(1)), inner.y));
    }
}

pub fn footer(f: &mut Frame, app: &App, area: Rect) {
    let hints: &[(&str, &str)] = if app.choice.as_ref().is_some_and(|c| !c.from_chat) {
        &[("KEY", "answer"), ("↑↓", "select"), ("ENTER", "confirm")]
    } else if app.choice.is_some() {
        &[("1-9", "reply"), ("↑↓", "select"), ("TYPE", "command"), ("leave", "close")]
    } else if app.password.is_some() {
        &[("ENTER", "submit"), ("ESC", "cancel")]
    } else {
        &[("TAB", "complete"), ("↑↓", "history"), ("PGUP/PGDN", "scroll"), ("ENTER", "skip"), ("help", "commands")]
    };
    let mut spans = Vec::new();
    let mut used = 0;
    for (k, v) in hints {
        let w = k.chars().count() + v.chars().count() + 6;
        if used + w > area.width as usize {
            break;
        }
        used += w;
        spans.push(Span::styled(format!(" {k} "), Style::default().fg(theme::TEXT).bg(theme::BORDER)));
        spans.push(Span::styled(format!(" {v}   "), Style::default().fg(theme::DIM)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::BG)), area);
}
