//! Overlays, full-screen cinematics and the glitch post-process.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use super::pixel::Tint;
use super::{font, theme, wrap};
use crate::app::App;
use crate::game::flow::SeqLine;
use crate::game::text;
use crate::systems::rng::Rng;

pub fn overlay(f: &mut Frame, app: &App) {
    let Some(run) = &app.overlay else { return };
    let Some(def) = app.content.overlays.get(&run.id) else { return };
    let (accent, body_color) = match def.style.as_str() {
        "root" => (theme::WHITE, theme::WHITE),
        "janus" => (theme::VIOLET, theme::TEXT),
        "danger" => (theme::RED, theme::TEXT),
        _ => (theme::AMBER, theme::TEXT),
    };
    let area = f.area();
    let w = area.width.saturating_sub(4).min(60);
    let cx = app.ctx();
    let mut lines = vec![Line::default()];
    for l in text::block(&def.body, &cx) {
        lines.extend(wrap::wrap(&l, w.saturating_sub(6) as usize));
    }
    for extra in &run.extra {
        lines.push(Line::default());
        let s = text::subst(extra, &cx);
        lines.push(Line::from(text::spans(&s, theme::bold(theme::RED))));
    }
    lines.push(Line::default());
    let h = (lines.len() as u16 + 2).min(area.height);
    let rect = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    };
    let lines: Vec<Line> = lines
        .into_iter()
        .map(|mut l| {
            l.spans.insert(0, Span::raw("  "));
            l.style = Style::default().fg(body_color);
            l
        })
        .collect();
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(format!(" {} ", def.title), theme::bold(accent)))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme::PANEL));
    f.render_widget(Paragraph::new(lines).block(block), rect);
}

/// Truncate styled spans to `n` visible characters.
fn truncate(spans: Vec<Span<'static>>, mut n: usize) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for s in spans {
        if n == 0 {
            break;
        }
        let len = s.content.chars().count();
        if len <= n {
            n -= len;
            out.push(s);
        } else {
            let cut: String = s.content.chars().take(n).collect();
            out.push(Span::styled(cut, s.style));
            n = 0;
        }
    }
    out
}

pub fn sequence(f: &mut Frame, app: &App) {
    let Some(seq) = &app.seq else { return };
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);
    let cx = app.ctx();
    let mut rows: Vec<(Line<'static>, bool)> = Vec::new();
    for item in &seq.lines {
        match item {
            SeqLine::Text { markup, shown, total } => {
                let s = text::subst(markup, &cx);
                let mut spans = text::spans(&s, Style::default().fg(theme::TEXT));
                if shown < total {
                    spans = truncate(spans, *shown);
                    spans.push(Span::styled("█", Style::default().fg(theme::TEXT)));
                }
                rows.push((Line::from(spans), false));
            }
            SeqLine::Sprite(name) => {
                if let Some(sp) = app.content.sprites.get(name) {
                    for l in sp.lines(sp.frame_at(seq.t), Tint::None) {
                        rows.push((l, true));
                    }
                }
            }
            SeqLine::Banner(color, words) => {
                let rgb = theme::to_rgb(theme::color(color));
                for l in font::banner(&text::subst(words, &cx), rgb, Tint::None) {
                    rows.push((l, true));
                }
            }
        }
    }
    let h = area.height.saturating_sub(2) as usize;
    let start = rows.len().saturating_sub(h);
    let content_h = rows.len() - start;
    let top = area.y + 1 + (h.saturating_sub(content_h) / 3) as u16;
    // Text shares a left edge with the centered art column.
    let margin = (area.width.saturating_sub(72) / 2).max(2);
    for (i, (line, center)) in rows[start..].iter().enumerate() {
        let y = top + i as u16;
        if y >= area.bottom() {
            break;
        }
        let w = line.width() as u16;
        let x = if *center { area.x + area.width.saturating_sub(w) / 2 } else { area.x + margin };
        let r = Rect { x, y, width: area.right().saturating_sub(x), height: 1 };
        f.render_widget(Paragraph::new(line.clone()), r);
    }
}

const JUNK: &[&str] = &["▓", "▒", "░", "█", "▄", "▀", "#", "$", "%", "&", "@", "?", "_", "/"];

/// Corrupt the rendered frame. Intensity 0.0..1.0.
pub fn glitch(f: &mut Frame, seed: u64, intensity: f64) {
    let area = f.area();
    let mut rng = Rng::new(seed);
    let buf = f.buffer_mut();
    let p = 0.01 + 0.09 * intensity;
    for y in area.top()..area.bottom() {
        // Occasional horizontal tear.
        if rng.chance(0.04 * intensity) {
            let shift = 1 + rng.below(6) as u16;
            for x in (area.left()..area.right().saturating_sub(shift)).rev() {
                let src = buf[(x, y)].clone();
                buf[(x + shift, y)] = src;
            }
        }
        for x in area.left()..area.right() {
            if rng.chance(p) {
                let cell = &mut buf[(x, y)];
                cell.set_symbol(JUNK[rng.below(JUNK.len())]);
                if rng.chance(0.4) {
                    cell.set_fg(if rng.chance(0.5) { theme::MAGENTA } else { theme::CYAN });
                }
            }
        }
    }
}
