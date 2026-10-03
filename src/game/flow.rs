//! Overlays, cinematics, password prompts and logging off.

use std::collections::VecDeque;

use crate::app::App;
use crate::content::model::Step;

pub struct OverlayRun {
    pub id: String,
    pub presses: u32,
    /// Extra lines that appeared when Esc was ignored.
    pub extra: Vec<String>,
    pub opened_at: f64,
}

/// One rendered row of a cinematic.
#[derive(Clone)]
pub enum SeqLine {
    /// Markup text, revealed up to `shown` visible characters.
    Text { markup: String, shown: usize, total: usize },
    Sprite(String),
    Banner(String, String),
}

pub struct SeqRun {
    pub id: String,
    steps: VecDeque<Step>,
    pub lines: Vec<SeqLine>,
    wait: f64,
    typing: bool,
    pub fast: bool,
    /// Seconds since the sequence started, for animated sprites.
    pub t: f64,
    pub glitch: f64,
}

impl SeqRun {
    /// Advance. Returns true when the sequence is finished.
    pub fn tick(&mut self, dt: f64) -> bool {
        self.t += dt;
        self.glitch = (self.glitch - dt).max(0.0);
        let rate = if self.fast { 4000.0 } else { 42.0 };
        if self.typing && self.wait > 0.0 {
            self.wait -= dt;
            return false;
        }
        if self.typing {
            if let Some(SeqLine::Text { shown, total, .. }) = self.lines.last_mut() {
                let step = ((rate * dt).ceil() as usize).max(1);
                *shown = (*shown + step).min(*total);
                if *shown < *total {
                    return false;
                }
            }
            self.typing = false;
        }
        self.wait -= dt;
        while self.wait <= 0.0 {
            let Some(step) = self.steps.pop_front() else {
                return self.wait <= -0.6 || self.fast;
            };
            match step {
                Step::Line(s) => {
                    let total = visible_len(&s);
                    self.lines.push(SeqLine::Text { markup: s, shown: total, total });
                }
                Step::Type(s) => {
                    let total = visible_len(&s);
                    self.lines.push(SeqLine::Text { markup: s, shown: 0, total });
                    self.typing = true;
                    return false;
                }
                Step::TypeAfter(prompt, typed) => {
                    let shown = visible_len(&prompt);
                    let markup = format!("{prompt}{typed}");
                    let total = visible_len(&markup);
                    self.lines.push(SeqLine::Text { markup, shown, total });
                    self.typing = true;
                    self.wait = if self.fast { 0.0 } else { 0.9 };
                    return false;
                }
                Step::Pause(ms) => {
                    if !self.fast {
                        self.wait += ms as f64 / 1000.0;
                    }
                }
                Step::Clear => self.lines.clear(),
                Step::Glitch(ms) => {
                    if !self.fast {
                        self.glitch = ms as f64 / 1000.0;
                    }
                }
                Step::Sprite(name) => self.lines.push(SeqLine::Sprite(name)),
                Step::Banner(color, text) => self.lines.push(SeqLine::Banner(color, text)),
                Step::When(..) => {}
            }
        }
        false
    }
}

/// Count visible characters in markup (tags excluded).
pub fn visible_len(s: &str) -> usize {
    crate::game::text::spans(s, Default::default())
        .iter()
        .map(|sp| sp.content.chars().count())
        .sum()
}

impl App {
    pub fn open_overlay(&mut self, id: &str) {
        if !self.content.overlays.contains_key(id) {
            return;
        }
        self.st.set(&format!("overlay:{id}"));
        self.glitch_until = self.glitch_until.max(self.t + 0.25);
        self.overlay = Some(OverlayRun { id: id.to_string(), presses: 0, extra: Vec::new(), opened_at: self.t });
    }

    /// Esc or Enter on an overlay. Some overlays refuse a few times.
    pub fn dismiss_overlay(&mut self) {
        let Some(run) = self.overlay.as_mut() else { return };
        if self.t - run.opened_at < 0.35 {
            return;
        }
        let Some(def) = self.content.overlays.get(&run.id) else {
            self.overlay = None;
            return;
        };
        if run.presses < def.hold {
            if let Some(line) = def.hold_text.get(run.presses as usize) {
                run.extra.push(line.clone());
            }
            run.presses += 1;
            run.opened_at = self.t;
            self.glitch_until = self.t + 0.18;
            return;
        }
        let fx = def.on_close.clone();
        self.overlay = None;
        self.apply(&fx);
    }

    pub fn start_sequence(&mut self, id: &str) {
        let Some(def) = self.content.sequences.get(id) else { return };
        let mut steps = VecDeque::new();
        flatten(&def.steps, self, &mut steps);
        self.st.set(&format!("seq:{id}"));
        self.seq = Some(SeqRun {
            id: id.to_string(),
            steps,
            lines: Vec::new(),
            wait: 0.0,
            typing: false,
            fast: false,
            t: 0.0,
            glitch: 0.0,
        });
    }

    pub fn finish_sequence(&mut self) {
        let Some(run) = self.seq.take() else { return };
        let fx = self
            .content
            .sequences
            .get(&run.id)
            .map(|s| s.on_end.clone())
            .unwrap_or_default();
        self.apply(&fx);
        if run.id == "logout" {
            self.quit = true;
        }
    }

    /// Check a typed password.
    pub fn submit_password(&mut self, raw: &str) {
        let Some(p) = self.password.take() else { return };
        self.dim(&"*".repeat(raw.chars().count().clamp(4, 12)));
        let ok = p.answers.iter().any(|a| a.eq_ignore_ascii_case(raw.trim()));
        if ok {
            self.apply(&p.ok);
        } else {
            self.apply(&p.fail);
        }
    }

    pub fn cancel_password(&mut self) {
        if self.password.take().is_some() {
            self.dim("ABORTED.");
        }
    }

    /// Log off: save the timeline, then play the hang-up cinematic.
    pub fn logout(&mut self) {
        self.st.pages.clear();
        if self.chat.is_some() {
            self.end_chat(false);
        }
        self.save_session();
        if self.content.sequences.contains_key("logout") {
            self.start_sequence("logout");
        } else {
            self.quit = true;
        }
    }
}

fn flatten(steps: &[Step], app: &App, out: &mut VecDeque<Step>) {
    for s in steps {
        match s {
            Step::When(c, inner) => {
                if c.eval(&app.st, &app.meta) {
                    flatten(inner, app, out);
                }
            }
            other => out.push_back(other.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_types_then_finishes() {
        let mut run = SeqRun {
            id: "t".into(),
            steps: VecDeque::from(vec![
                Step::Type("{cyan}CONNECTING{/}".into()),
                Step::Pause(100),
                Step::Line("OK".into()),
            ]),
            lines: Vec::new(),
            wait: 0.0,
            typing: false,
            fast: false,
            t: 0.0,
            glitch: 0.0,
        };
        assert!(!run.tick(0.01));
        assert_eq!(visible_len("{cyan}CONNECTING{/}"), 10);
        let mut done = false;
        for _ in 0..200 {
            if run.tick(0.03) {
                done = true;
                break;
            }
        }
        assert!(done);
        assert_eq!(run.lines.len(), 2);
    }
}
