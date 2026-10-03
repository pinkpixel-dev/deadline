//! Header bar, main terminal and sidebar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::pixel::Tint;
use super::{theme, wrap};
use crate::app::App;
use crate::systems::clock;

pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { theme::BORDER_HI } else { theme::BORDER }))
        .style(Style::default().bg(theme::BG));
    if title.is_empty() {
        return block;
    }
    block.title(Span::styled(format!(" {title} "), theme::bold(if focused { theme::WHITE } else { theme::DIM })))
}

pub fn header(f: &mut Frame, app: &App, area: Rect) {
    let name = if app.glitching() && app.rng.clone().chance(0.5) { " DEADL_NE BB$ " } else { " DEADLINE BBS " };
    let wide = area.width >= 84;
    let mut left = vec![
        Span::styled(name, theme::bold(theme::BG).bg(theme::AMBER)),
        Span::styled("  NODE 02", Style::default().fg(theme::TEXT)),
    ];
    if wide {
        left.extend([
            Span::styled("  ·  ", Style::default().fg(theme::FAINT)),
            Span::styled("14.4K", Style::default().fg(theme::TEXT)),
            Span::styled("  ·  ", Style::default().fg(theme::FAINT)),
            Span::styled("● ", Style::default().fg(theme::GREEN)),
            Span::styled("CONNECTED", Style::default().fg(theme::TEXT)),
        ]);
    }
    let right = if wide {
        format!("{}  {} ", clock::date_string(&app.st), clock::time_string(&app.st))
    } else {
        format!("{} ", clock::time_string(&app.st))
    };
    let [l, r] = Layout::horizontal([Constraint::Min(10), Constraint::Length(right.len() as u16)]).areas(area);
    let bg = Style::default().bg(theme::PANEL);
    f.render_widget(Paragraph::new(Line::from(left)).style(bg), l);
    f.render_widget(Paragraph::new(Line::styled(right, Style::default().fg(theme::DIM))).style(bg), r);
}

pub fn terminal(f: &mut Frame, app: &App, area: Rect) {
    let scrolled = app.out.scroll > 0;
    let mut title = app.context.clone();
    if scrolled {
        title.push_str(&format!("  ↑{}", app.out.scroll));
    }
    let block = panel(&title, true);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Faint scanline texture across the terminal.
    let buf = f.buffer_mut();
    for y in inner.top()..inner.bottom() {
        if y % 2 == 0 {
            for x in inner.left()..inner.right() {
                buf[(x, y)].set_bg(theme::PANEL);
            }
        }
    }

    let width = inner.width.saturating_sub(1) as usize;
    let mut visual: Vec<Line<'static>> = Vec::new();
    for l in &app.out.lines {
        visual.extend(wrap::wrap(l, width));
    }
    let h = inner.height as usize;
    let max_scroll = visual.len().saturating_sub(h);
    let scroll = app.out.scroll.min(max_scroll);
    let end = visual.len() - scroll;
    let start = end.saturating_sub(h);
    let text_area = Rect { x: inner.x + 1, width: inner.width.saturating_sub(1), ..inner };
    f.render_widget(Paragraph::new(visual[start..end].to_vec()), text_area);
}

pub fn sidebar(f: &mut Frame, app: &App, area: Rect) {
    let users = online_lines(app);
    let portrait_h = 13u16.min(area.height / 2);
    let users_h = (users.len() as u16 + 2).min(area.height.saturating_sub(portrait_h + 4));
    let [p, u, l] = Layout::vertical([
        Constraint::Length(portrait_h),
        Constraint::Length(users_h),
        Constraint::Min(3),
    ])
    .areas(area);
    portrait(f, app, p);
    f.render_widget(Paragraph::new(users).block(panel("USERS", false)), u);

    let block = panel("SYSTEM LOG", false);
    let inner = block.inner(l);
    f.render_widget(block, l);
    let width = inner.width as usize;
    let mut lines: Vec<Line<'static>> = Vec::new();
    for line in &app.log {
        lines.extend(wrap::wrap(line, width));
    }
    let start = lines.len().saturating_sub(inner.height as usize);
    f.render_widget(Paragraph::new(lines[start..].to_vec()), inner);
}

fn online_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled(" ● ", Style::default().fg(theme::GREEN)),
        Span::styled(app.st.player.clone(), theme::bold(theme::WHITE)),
    ])];
    for u in &app.content.users {
        let Some(p) = app.presence(&u.id) else { continue };
        if p.status == "hidden" {
            continue;
        }
        let dot = if p.status == "online" { theme::GREEN } else { theme::AMBER };
        let mut spans = vec![
            Span::styled(" ● ", Style::default().fg(dot)),
            Span::styled(u.id.clone(), Style::default().fg(theme::color(&u.color))),
        ];
        if p.status != "online" {
            spans.push(Span::styled(format!(" {}", p.status), theme::dim()));
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn portrait(f: &mut Frame, app: &App, area: Rect) {
    let who = app
        .chat
        .as_ref()
        .map(|c| c.with.clone())
        .or_else(|| app.portrait.clone());
    let user = who.as_deref().and_then(|w| app.content.user(w));
    let (title, sprite, tint, status) = match user {
        Some(u) => {
            let online = app.presence(&u.id).is_some();
            let tint = if online { Tint::None } else { Tint::Dim(0.4) };
            let status = if online { "online" } else { "offline" };
            (u.id.clone(), u.sprite.clone().unwrap_or_else(|| "modem".into()), tint, status)
        }
        None => ("LINE".to_string(), "modem".to_string(), Tint::None, "carrier ok"),
    };
    let block = panel(&title, app.chat.is_some());
    let inner = block.inner(area);
    f.render_widget(block, area);
    let Some(s) = app.content.sprites.get(&sprite) else { return };
    let frame = s.frame_at(app.t);
    let tint = if app.glitching() { Tint::Glitch } else { tint };
    let mut lines = s.lines(frame, tint);
    let pad = (inner.width as usize).saturating_sub(s.width) / 2;
    for l in lines.iter_mut() {
        l.spans.insert(0, Span::raw(" ".repeat(pad)));
    }
    let top = (inner.height as usize).saturating_sub(lines.len() + 1) / 2;
    let mut all: Vec<Line<'static>> = vec![Line::default(); top];
    all.extend(lines);
    all.push(Line::styled(format!("{status:^w$}", w = inner.width as usize), theme::dim()));
    f.render_widget(Paragraph::new(all), inner);
}
