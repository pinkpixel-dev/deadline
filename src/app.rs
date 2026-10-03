use std::collections::VecDeque;
use std::rc::Rc;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::content::Content;
use crate::content::effect::{ChoiceOpt, Effect, PasswordPrompt};
use crate::game::chat::ChatRun;
use crate::game::flow::{OverlayRun, SeqRun};
use crate::game::state::{GameState, MetaState};
use crate::game::text::{self, Ctx};
use crate::input::Input;
use crate::output::{Output, Speed};
use crate::systems::rng::Rng;
use crate::systems::save::Store;
use crate::ui::theme;

/// A choice the player is currently being asked.
pub struct ActiveChoice {
    pub prompt: String,
    pub opts: Vec<ChoiceOpt>,
    /// Conversation choices let you keep typing commands. Inline ones don't.
    pub from_chat: bool,
    pub sel: usize,
}

pub struct App {
    pub content: Rc<Content>,
    pub st: GameState,
    pub meta: MetaState,
    pub store: Option<Store>,
    pub out: Output,
    pub log: VecDeque<Line<'static>>,
    pub input: Input,
    pub choice: Option<ActiveChoice>,
    pub chat: Option<ChatRun>,
    pub overlay: Option<OverlayRun>,
    pub seq: Option<SeqRun>,
    pub password: Option<PasswordPrompt>,
    /// Blocking effects waiting for the screen to be free.
    pub queue: VecDeque<Effect>,
    /// App time in seconds since launch.
    pub t: f64,
    pub glitch_until: f64,
    pub rng: Rng,
    pub quit: bool,
    pub next_ambient: f64,
    /// User shown in the sidebar portrait when nobody is chatting.
    pub portrait: Option<String>,
    pub ctrl_c_at: f64,
    pub speed: Speed,
    /// Clickable choice rows from the last frame.
    pub hits: Vec<(Rect, usize)>,
    /// Pane title for the main terminal.
    pub context: String,
    next_event_check: f64,
}

impl App {
    pub fn new(content: Content, st: GameState, meta: MetaState, store: Option<Store>) -> Self {
        Self {
            content: Rc::new(content),
            st,
            meta,
            store,
            out: Output::default(),
            log: VecDeque::new(),
            input: Input::default(),
            choice: None,
            chat: None,
            overlay: None,
            seq: None,
            password: None,
            queue: VecDeque::new(),
            t: 0.0,
            glitch_until: 0.0,
            rng: Rng::from_time(),
            quit: false,
            next_ambient: 40.0,
            portrait: None,
            ctrl_c_at: -10.0,
            speed: Speed::Normal,
            hits: Vec::new(),
            context: "MAIN".into(),
            next_event_check: 0.0,
        }
    }

    pub fn ctx(&self) -> Ctx<'_> {
        Ctx { content: &self.content, st: &self.st, meta: &self.meta }
    }

    // ---- output helpers -------------------------------------------------

    pub fn print(&mut self, markup: &str) {
        let lines = text::block(markup, &self.ctx());
        self.out.push(lines);
        self.out.scroll = 0;
    }

    pub fn print_lines(&mut self, lines: Vec<Line<'static>>) {
        self.out.push(lines);
        self.out.scroll = 0;
    }

    pub fn blank(&mut self) {
        self.out.push_line(Line::default());
    }

    /// A highlighted `***` notice in the terminal.
    pub fn notice(&mut self, markup: &str) {
        let s = text::subst(markup, &self.ctx());
        let mut spans = vec![Span::styled("*** ", theme::bold(theme::AMBER))];
        spans.extend(text::spans(&s, theme::bold(theme::AMBER)));
        self.out.push_line(Line::from(spans));
        self.out.scroll = 0;
    }

    /// A dim error-style line.
    pub fn err(&mut self, markup: &str) {
        let s = text::subst(markup, &self.ctx());
        self.out.push_line(Line::from(text::spans(&s, Style::default().fg(theme::RED))));
    }

    pub fn dim(&mut self, markup: &str) {
        let s = text::subst(markup, &self.ctx());
        self.out.push_line(Line::from(text::spans(&s, theme::dim())));
    }

    pub fn log(&mut self, markup: &str) {
        let s = text::subst(markup, &self.ctx());
        let stamp = crate::systems::clock::time_string(&self.st);
        let mut spans = vec![Span::styled(format!("{} ", &stamp[..5]), Style::default().fg(theme::FAINT))];
        spans.extend(text::spans(&s, theme::dim()));
        self.log.push_back(Line::from(spans));
        while self.log.len() > 200 {
            self.log.pop_front();
        }
    }

    // ---- blocking state -------------------------------------------------

    /// True when a modal (overlay, cinematic, password, inline choice) owns input.
    pub fn modal(&self) -> bool {
        self.overlay.is_some()
            || self.seq.is_some()
            || self.password.is_some()
            || self.choice.as_ref().is_some_and(|c| !c.from_chat)
    }

    fn can_start(&self, e: &Effect) -> bool {
        match e {
            Effect::Chat(_) => !self.modal() && self.chat.is_none(),
            Effect::Overlay(_) | Effect::Sequence(_) => !self.modal(),
            Effect::Choose(_) | Effect::Password(_) => !self.modal() && self.choice.is_none(),
            _ => true,
        }
    }

    /// Start a blocking effect now, or queue it until the screen is free.
    pub fn start_or_queue(&mut self, e: Effect) {
        if self.queue.is_empty() && self.can_start(&e) {
            self.start_blocking(e);
        } else {
            self.queue.push_back(e);
        }
    }

    fn pump_queue(&mut self) {
        while let Some(front) = self.queue.front() {
            if !self.can_start(front) {
                break;
            }
            let e = self.queue.pop_front().expect("front exists");
            self.start_blocking(e);
        }
    }

    fn start_blocking(&mut self, e: Effect) {
        match e {
            Effect::Chat(id) => self.start_chat(&id),
            Effect::Overlay(id) => self.open_overlay(&id),
            Effect::Sequence(id) => self.start_sequence(&id),
            Effect::Choose(c) => {
                let opts: Vec<ChoiceOpt> = c
                    .options
                    .into_iter()
                    .filter(|o| o.cond.eval(&self.st, &self.meta))
                    .collect();
                if !c.prompt.is_empty() {
                    self.print(&format!("{{amber}}{}{{/}}", c.prompt));
                }
                self.choice = Some(ActiveChoice { prompt: c.prompt, opts, from_chat: false, sel: 0 });
            }
            Effect::Password(p) => {
                let prompt = if p.prompt.is_empty() { "PASSWORD:".to_string() } else { p.prompt.clone() };
                self.print(&format!("{{amber}}{prompt}{{/}}"));
                self.password = Some(p);
            }
            other => self.apply_one(&other),
        }
    }

    // ---- frame tick -----------------------------------------------------

    pub fn tick(&mut self, dt: f64) {
        self.t += dt;
        if let Some(seq) = self.seq.as_mut() {
            if seq.tick(dt) {
                self.finish_sequence();
            }
            return;
        }
        self.st.elapsed += dt;
        self.out.tick(dt, self.speed);
        if self.overlay.is_none() {
            self.tick_chat(dt);
        }
        self.pump_queue();
        if self.t >= self.next_event_check {
            self.next_event_check = self.t + 0.5;
            self.check_events();
            self.ambient();
        }
    }

    pub fn glitching(&self) -> bool {
        self.t < self.glitch_until
    }

    /// Persist the live timeline. Errors are logged, never fatal.
    pub fn save_session(&mut self) {
        if let Some(store) = &self.store {
            if let Err(e) = store.save_session(&self.st) {
                let msg = format!("{{red}}SESSION WRITE FAILED: {e}{{/}}");
                self.log(&msg);
            }
        }
    }

    pub fn save_meta(&mut self) {
        if let Some(store) = &self.store {
            let _ = store.save_meta(&self.meta);
        }
    }
}
