//! Half-block pixel art. Each terminal cell shows two vertical pixels by
//! drawing `▀` with the top pixel as foreground and the bottom pixel as
//! background.

use std::collections::HashMap;

use anyhow::{Result, bail};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::theme;

pub type Rgb = (u8, u8, u8);
pub type Grid = Vec<Vec<Option<Rgb>>>;

#[derive(Debug, Clone)]
pub struct Sprite {
    pub frames: Vec<Grid>,
    /// Frames per second for animated sprites.
    pub fps: f32,
    pub width: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub height: usize,
}

/// Recolor applied while rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tint {
    None,
    /// Darken everything (faded memory, offline users).
    Dim(f32),
    /// Collapse to one hue (ROOT-style monochrome).
    Mono(Rgb),
    /// Channel-swapped glitch colors.
    Glitch,
}

fn hex(s: &str) -> Result<Rgb> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        bail!("bad color {s}");
    }
    let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16);
    Ok((p(0)?, p(2)?, p(4)?))
}

impl Sprite {
    /// Parse the text sprite format:
    ///
    /// ```text
    /// fps 2
    /// palette
    /// . none
    /// k 1a1b1f
    /// frame
    /// ..kk..
    /// ```
    pub fn parse(text: &str) -> Result<Self> {
        let mut palette: HashMap<char, Option<Rgb>> = HashMap::new();
        palette.insert('.', None);
        palette.insert(' ', None);
        let mut frames: Vec<Vec<&str>> = Vec::new();
        // How many ticks each frame is held for.
        let mut holds: Vec<usize> = Vec::new();
        let mut fps = 0.0;
        let mut section = "";
        for raw in text.lines() {
            let line = raw.trim_end();
            if line.starts_with("//") {
                continue;
            }
            let t = line.trim();
            if t == "palette" {
                section = "palette";
                continue;
            }
            if t == "frame" || t.starts_with("frame ") {
                section = "frame";
                frames.push(Vec::new());
                holds.push(t[5..].trim().parse().unwrap_or(1).max(1));
                continue;
            }
            match section {
                "palette" if !line.trim().is_empty() => {
                    let mut it = line.split_whitespace();
                    let key = it.next().and_then(|k| k.chars().next());
                    let val = it.next().unwrap_or("none");
                    let Some(key) = key else { continue };
                    let color = if val == "none" { None } else { Some(hex(val)?) };
                    palette.insert(key, color);
                }
                "frame" => {
                    if let Some(f) = frames.last_mut() {
                        if !line.is_empty() {
                            f.push(line);
                        }
                    }
                }
                _ => {
                    if let Some(v) = line.trim().strip_prefix("fps ") {
                        fps = v.trim().parse().unwrap_or(0.0);
                    }
                }
            }
        }
        if frames.is_empty() {
            bail!("sprite has no frames");
        }
        let width = frames.iter().flatten().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut height = frames.iter().map(|f| f.len()).max().unwrap_or(0);
        height += height % 2;
        let mut grids = Vec::new();
        for (f, hold) in frames.into_iter().zip(holds) {
            let mut grid: Grid = Vec::with_capacity(height);
            for y in 0..height {
                let row = f.get(y).copied().unwrap_or("");
                let mut out = Vec::with_capacity(width);
                let chars: Vec<char> = row.chars().collect();
                for x in 0..width {
                    let ch = chars.get(x).copied().unwrap_or('.');
                    match palette.get(&ch) {
                        Some(c) => out.push(*c),
                        None => bail!("unknown palette key '{ch}'"),
                    }
                }
                grid.push(out);
            }
            for _ in 1..hold {
                grids.push(grid.clone());
            }
            grids.push(grid);
        }
        Ok(Self { frames: grids, fps, width, height })
    }

    /// Which frame to show at a given time in seconds.
    pub fn frame_at(&self, t: f64) -> usize {
        if self.fps <= 0.0 || self.frames.len() < 2 {
            return 0;
        }
        ((t * self.fps as f64) as usize) % self.frames.len()
    }

    pub fn lines(&self, frame: usize, tint: Tint) -> Vec<Line<'static>> {
        grid_lines(&self.frames[frame % self.frames.len()], tint)
    }
}

fn apply(c: Rgb, tint: Tint) -> Rgb {
    match tint {
        Tint::None => c,
        Tint::Dim(f) => theme::shade(c, f),
        Tint::Mono(m) => {
            let l = (c.0 as f32 * 0.3 + c.1 as f32 * 0.59 + c.2 as f32 * 0.11) / 255.0;
            theme::shade(m, l * 1.2)
        }
        Tint::Glitch => (c.2, c.0, c.1),
    }
}

/// Turn a pixel grid into half-block lines.
pub fn grid_lines(grid: &Grid, tint: Tint) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(grid.len() / 2 + 1);
    let empty = Vec::new();
    for y in (0..grid.len()).step_by(2) {
        let top = &grid[y];
        let bot = grid.get(y + 1).unwrap_or(&empty);
        let w = top.len().max(bot.len());
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(w);
        for x in 0..w {
            let t = top.get(x).copied().flatten().map(|c| apply(c, tint));
            let b = bot.get(x).copied().flatten().map(|c| apply(c, tint));
            let span = match (t, b) {
                (None, None) => Span::raw(" "),
                (Some(t), None) => Span::styled("▀", Style::default().fg(theme::rgb(t))),
                (None, Some(b)) => Span::styled("▄", Style::default().fg(theme::rgb(b))),
                (Some(t), Some(b)) if t == b => {
                    Span::styled("█", Style::default().fg(theme::rgb(t)))
                }
                (Some(t), Some(b)) => Span::styled(
                    "▀",
                    Style::default().fg(theme::rgb(t)).bg(theme::rgb(b)),
                ),
            };
            lines_push(&mut spans, span);
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Merge adjacent spans with identical styles to keep lines small.
fn lines_push(spans: &mut Vec<Span<'static>>, span: Span<'static>) {
    if let Some(last) = spans.last_mut() {
        if last.style == span.style {
            last.content.to_mut().push_str(&span.content);
            return;
        }
    }
    spans.push(span);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "fps 2\npalette\n. none\na ff0000\nb 00ff00\nframe\naa\nab\nab\n..\nframe\nbb\nbb\n";

    #[test]
    fn parses_frames_and_pads_height() {
        let s = Sprite::parse(SRC).unwrap();
        assert_eq!(s.frames.len(), 2);
        assert_eq!(s.width, 2);
        assert_eq!(s.height, 4);
        assert_eq!(s.frame_at(0.6), 1);
    }

    #[test]
    fn half_blocks_pair_rows() {
        let s = Sprite::parse(SRC).unwrap();
        let lines = s.lines(0, Tint::None);
        assert_eq!(lines.len(), 2);
        let text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "█▀");
        let text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "▀▀");
    }

    #[test]
    fn rejects_unknown_palette_keys() {
        assert!(Sprite::parse("palette\na ffffff\nframe\naz\n").is_err());
    }
}
