//! The launch splash: about five seconds of full-screen pixel animation
//! before the board picks up. A CRT warms up, the modem handshake plays out
//! as carrier waves on a scope, the waves lock into one line, and the line
//! comes apart into the DEADLINE title. Any key or click skips it.
//!
//! `frame` is a pure function of screen size and time, so the whole thing
//! can be tested without a terminal.

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::pixel::{Grid, Rgb, Tint, grid_lines};
use super::{font, screens, theme};
use crate::systems::rng::Rng;

pub const LENGTH: f64 = 5.2;
const WARM: f64 = 0.7;
const LOCK: f64 = 2.6;
const TITLE: f64 = 3.1;
const TEAR: f64 = 4.85;

/// Play the splash. Returns early on any key or click.
pub fn play(terminal: &mut DefaultTerminal) -> Result<()> {
    let start = Instant::now();
    let seed = Rng::from_time().below(1 << 30) as u64;
    loop {
        let t = start.elapsed().as_secs_f64();
        if t >= LENGTH {
            return Ok(());
        }
        terminal.draw(|f| {
            let area = f.area();
            draw(f, area, t, seed);
        })?;
        if event::poll(Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(k) if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                Event::Key(_) => return Ok(()),
                Event::Mouse(m) if matches!(m.kind, MouseEventKind::Down(_)) => return Ok(()),
                _ => {}
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, area: Rect, t: f64, seed: u64) {
    f.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);
    let grid = frame(area.width as usize, area.height as usize * 2, t, seed);
    f.render_widget(Paragraph::new(grid_lines(&grid, Tint::None)), area);

    // The modem talks along the bottom while the scope runs.
    if (WARM..TITLE).contains(&t) && area.height > 4 {
        let lines = [
            (WARM, "ATDT 1-555-0142", theme::DIM),
            (1.6, "CARRIER DETECTED", theme::AMBER),
            (2.3, "CONNECT 14400/ARQ/V32BIS", theme::GREEN),
        ];
        let shown: Vec<Line> = lines
            .iter()
            .filter(|(at, ..)| t >= *at)
            .map(|(at, s, c)| {
                let n = (((t - at) * 40.0) as usize).min(s.len());
                Line::from(Span::styled(format!("  {}", &s[..n]), Style::default().fg(*c).bg(theme::BG)))
            })
            .collect();
        let h = shown.len() as u16;
        let r = Rect { x: area.x, y: area.bottom().saturating_sub(h + 1), width: area.width.min(40), height: h };
        f.render_widget(Paragraph::new(shown), r);
    }

    if t >= 4.0 && area.height > 6 {
        let s = "made by pink pixel";
        let w = s.len() as u16;
        let r = Rect {
            x: area.x + area.width.saturating_sub(w) / 2,
            y: area.bottom().saturating_sub(2),
            width: w.min(area.width),
            height: 1,
        };
        f.render_widget(Paragraph::new(Line::from(Span::styled(s, Style::default().fg(theme::DIM)))), r);
    }

    if t >= TEAR {
        let k = ((t - TEAR) / (LENGTH - TEAR)).clamp(0.0, 1.0);
        screens::glitch(f, seed + (t * 30.0) as u64, 0.3 + 0.7 * k);
    }
}

fn rgb(c: ratatui::style::Color) -> Rgb {
    theme::to_rgb(c)
}

fn ease_out(x: f64) -> f64 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

/// Cheap per-pixel noise in 0..1, stable for a given (x, y, step, seed).
fn hash(x: usize, y: usize, step: u64, seed: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ step.wrapping_mul(0x1656_67B1_9E37_79F9)
        ^ seed;
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    (h % 10_000) as f64 / 10_000.0
}

/// One frame of the splash as a pixel grid (`h` is in pixels, two per row).
pub fn frame(w: usize, h: usize, t: f64, seed: u64) -> Grid {
    let mut g: Grid = vec![vec![None; w]; h];
    if w == 0 || h == 0 {
        return g;
    }
    let mid = h as f64 / 2.0;
    let step = (t * 24.0) as u64;

    // CRT warm-up: a band of static opens out from the center line.
    if t < WARM {
        let open = ease_out(t / WARM) * mid;
        for (y, row) in g.iter_mut().enumerate() {
            let d = (y as f64 + 0.5 - mid).abs();
            if d > open.max(0.6) {
                continue;
            }
            for (x, px) in row.iter_mut().enumerate() {
                let n = hash(x, y, step, seed);
                let v = (40.0 + 140.0 * n * (1.0 - t / WARM * 0.5)) as u8;
                *px = Some((v, v, v.saturating_add(6)));
            }
        }
        scanlines(&mut g);
        return g;
    }

    if t < TITLE {
        scope(&mut g, t, seed, step);
    } else {
        title(&mut g, t, seed);
    }
    scanlines(&mut g);
    g
}

/// The oscilloscope: a faint graticule, fading static, and three carriers
/// that settle into one flat line.
fn scope(g: &mut Grid, t: f64, seed: u64, step: u64) {
    let (w, h) = (g[0].len(), g.len());
    let mid = h as f64 / 2.0;
    graticule(g, 0.55);
    let static_left = 1.0 - ((t - WARM) / 1.0).clamp(0.0, 1.0);
    for (y, row) in g.iter_mut().enumerate() {
        for (x, px) in row.iter_mut().enumerate() {
            if hash(x, y, step, seed) < 0.06 * static_left {
                *px = Some(theme::shade(rgb(theme::DIM), 0.8));
            }
        }
    }

    // Amplitude rises with the handshake, then collapses at the lock.
    let rise = ease_out((t - WARM) / 0.6);
    let lock = ease_out((t - LOCK) / (TITLE - LOCK));
    let amp = mid * 0.55 * rise * (1.0 - lock);
    let waves = [
        (rgb(theme::AMBER), 2.0, 3.1, 0.0),
        (rgb(theme::CYAN), 3.3, -2.2, 1.7),
        (rgb(theme::VIOLET), 5.1, 4.4, 3.9),
    ];
    for (i, (color, freq, speed, phase)) in waves.iter().enumerate() {
        let a = amp * (1.0 - i as f64 * 0.25);
        let mut last: Option<usize> = None;
        for x in 0..w {
            let u = x as f64 / w as f64 * std::f64::consts::TAU;
            let wob = (u * 0.5 + t * 1.3 + phase).sin() * 0.35 + 0.65;
            let y = mid + a * wob * (u * freq + t * speed + phase).sin();
            let yi = (y.round() as usize).min(h - 1);
            // Join steep segments so the trace stays continuous.
            let (lo, hi) = match last {
                Some(p) => (p.min(yi), p.max(yi)),
                None => (yi, yi),
            };
            for row in g.iter_mut().take(hi + 1).skip(lo) {
                row[x] = Some(*color);
            }
            last = Some(yi);
        }
    }
    if lock > 0.0 {
        let y = (mid.floor() as usize).min(h - 1);
        let c = theme::shade(rgb(theme::AMBER), (0.5 + 0.5 * lock) as f32);
        g[y].iter_mut().for_each(|px| *px = Some(c));
    }
}

/// The scope's faint grid, at some brightness.
fn graticule(g: &mut Grid, level: f32) {
    let (w, h) = (g[0].len(), g.len());
    let mid = h as f64 / 2.0;
    let faint = theme::shade(rgb(theme::FAINT), level);
    let cell = (w / 12).max(6);
    for (y, row) in g.iter_mut().enumerate() {
        for (x, px) in row.iter_mut().enumerate() {
            let on_x = x % cell == 0 && y % 2 == 0;
            let on_y = (y as f64 - mid).abs() < 0.5 || (y % (cell / 2).max(3) == 0 && x % 3 == 0);
            if on_x || on_y {
                *px = Some(faint);
            }
        }
    }
}

fn mix(a: Rgb, b: Rgb, k: f64) -> Rgb {
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * k.clamp(0.0, 1.0)) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// The title: every pixel lifts off the carrier line and flies into place.
fn title(g: &mut Grid, t: f64, seed: u64) {
    let (w, h) = (g[0].len(), g.len());
    let mid = h as f64 / 2.0;
    let text = "DEADLINE";
    let src = font::grid(text, rgb(theme::AMBER));
    let (sw, sh) = (src[0].len(), src.len());
    // As big as fits: most of the width, at most half the height.
    let scale = ((w as f64 * 0.86) / sw as f64).min(h as f64 * 0.5 / sh as f64).max(0.5);
    let (tw, th) = ((sw as f64 * scale) as usize, (sh as f64 * scale) as usize);
    let ox = w.saturating_sub(tw) / 2;
    let oy = (h.saturating_sub(th) as f64 * 0.42) as usize;
    graticule(g, 0.4);
    // One light sweep across the letters once they've landed.
    let sweep = (t - 4.05) / 0.7 * (tw + th) as f64;

    // What's left of the carrier, fading as its pixels leave.
    let left = 1.0 - ((t - TITLE) / 0.7).clamp(0.0, 1.0);
    if left > 0.0 {
        let y = (mid.floor() as usize).min(h - 1);
        for (x, px) in g[y].iter_mut().enumerate() {
            if hash(x, 0, 0, seed) < left {
                *px = Some(rgb(theme::AMBER));
            }
        }
    }

    for py in 0..th {
        for px in 0..tw {
            let (sx, sy) = (((px as f64 / scale) as usize).min(sw - 1), ((py as f64 / scale) as usize).min(sh - 1));
            let Some(color) = src[sy][sx] else { continue };
            let (tx, ty) = (ox + px, oy + py);
            let start = TITLE + hash(tx, ty, 1, seed) * 0.8;
            if tx >= w || ty >= h || t < start {
                continue;
            }
            // Fly in from a random spot on the carrier line.
            let k = ease_out((t - start) / 0.65);
            let fx = hash(tx, ty, 2, seed) * w as f64;
            let (xi, yi) = ((fx + (tx as f64 - fx) * k).round() as usize, (mid + (ty as f64 - mid) * k).round() as usize);
            if xi >= w || yi >= h {
                continue;
            }
            let d = sweep - (px + py / 2) as f64;
            let color = if (0.0..4.0).contains(&d) { mix(color, rgb(theme::WHITE), 0.7) } else { color };
            g[yi][xi] = Some(if k < 1.0 { theme::shade(color, (0.6 + 0.4 * k) as f32) } else { color });
        }
    }
}

/// Darken every other pixel row a touch, like a CRT.
fn scanlines(g: &mut Grid) {
    for row in g.iter_mut().skip(1).step_by(2) {
        for px in row.iter_mut().flatten() {
            *px = theme::shade(*px, 0.82);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(g: &Grid) -> usize {
        g.iter().flatten().filter(|p| p.is_some()).count()
    }

    #[test]
    fn every_stage_draws_something_at_any_size() {
        for (w, h) in [(1, 2), (20, 12), (80, 48), (240, 120)] {
            for t in [0.0, 0.3, 1.0, 2.0, 2.9, 3.4, 4.5, 5.1] {
                let g = frame(w, h, t, 7);
                assert_eq!((g.len(), g[0].len()), (h, w));
                // A 1x2 screen only has to survive.
                assert!(w < 20 || lit(&g) > 0, "{w}x{h} at {t}s is blank");
            }
        }
    }

    #[test]
    fn the_title_settles_into_place() {
        // Once every pixel has landed and the sweep has passed, frames hold still.
        let a = frame(120, 60, 4.95, 3);
        let b = frame(120, 60, 5.1, 3);
        assert_eq!(a, b);
        let amber = theme::to_rgb(theme::AMBER);
        assert!(a.iter().flatten().filter(|p| **p == Some(amber)).count() > 200, "DEADLINE is drawn big");
    }
}
