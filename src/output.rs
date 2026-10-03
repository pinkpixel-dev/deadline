//! The main terminal scrollback, with modem-speed line reveal.

use std::collections::VecDeque;

use ratatui::text::Line;

const MAX_LINES: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Speed {
    Slow,
    Normal,
    Fast,
    Instant,
}

impl Speed {
    /// Seconds between revealed lines.
    pub fn line_delay(self) -> f64 {
        match self {
            Speed::Slow => 0.045,
            Speed::Normal => 0.016,
            Speed::Fast => 0.004,
            Speed::Instant => 0.0,
        }
    }

    /// Multiplier for chat typing delays.
    pub fn chat_factor(self) -> f64 {
        match self {
            Speed::Slow => 1.5,
            Speed::Normal => 1.0,
            Speed::Fast => 0.45,
            Speed::Instant => 0.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Speed::Slow => "slow",
            Speed::Normal => "normal",
            Speed::Fast => "fast",
            Speed::Instant => "instant",
        }
    }
}

/// A clickable line. `menu` 0 means click-only (no keyboard selection).
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub menu: u32,
    pub item: usize,
    pub cmd: String,
}

#[derive(Default)]
pub struct Output {
    pub lines: Vec<Line<'static>>,
    /// Parallel to `lines`.
    pub links: Vec<Option<Link>>,
    pending: VecDeque<(Line<'static>, Option<Link>)>,
    wait: f64,
    /// Visual lines scrolled up from the bottom.
    pub scroll: usize,
}

impl Output {
    /// Queue lines to be revealed one at a time.
    pub fn push(&mut self, lines: impl IntoIterator<Item = Line<'static>>) {
        self.pending.extend(lines.into_iter().map(|l| (l, None)));
    }

    pub fn push_line(&mut self, line: Line<'static>) {
        self.pending.push_back((line, None));
    }

    pub fn push_link(&mut self, line: Line<'static>, link: Link) {
        self.pending.push_back((line, Some(link)));
    }

    /// Show everything that is still queued right now.
    pub fn flush(&mut self) {
        while let Some((l, link)) = self.pending.pop_front() {
            self.commit(l, link);
        }
    }

    #[cfg(test)]
    pub fn busy(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.links.clear();
        self.pending.clear();
        self.scroll = 0;
    }

    fn commit(&mut self, l: Line<'static>, link: Option<Link>) {
        self.lines.push(l);
        self.links.push(link);
        if self.lines.len() > MAX_LINES {
            let extra = self.lines.len() - MAX_LINES;
            self.lines.drain(..extra);
            self.links.drain(..extra);
        }
    }

    pub fn tick(&mut self, dt: f64, speed: Speed) {
        let delay = speed.line_delay();
        if delay <= 0.0 {
            self.flush();
            return;
        }
        self.wait -= dt;
        while self.wait <= 0.0 {
            match self.pending.pop_front() {
                Some((l, link)) => {
                    // Blank lines cost nothing so paragraphs feel snappy.
                    let blank = l.width() == 0;
                    self.commit(l, link);
                    if !blank {
                        self.wait += delay;
                    }
                }
                None => {
                    self.wait = 0.0;
                    break;
                }
            }
        }
    }

    pub fn scroll_by(&mut self, delta: isize) {
        self.scroll = (self.scroll as isize + delta).max(0) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveals_over_time_and_flushes() {
        let mut o = Output::default();
        o.push((0..10).map(|i| Line::from(format!("line {i}"))));
        o.tick(0.0, Speed::Normal);
        assert!(o.lines.len() < 10);
        o.tick(0.05, Speed::Normal);
        let partial = o.lines.len();
        assert!(partial > 0 && partial < 10);
        o.flush();
        assert_eq!(o.lines.len(), 10);
        assert!(!o.busy());
    }
}
