//! Private-channel conversations: typed messages, choices and branches.

use std::collections::VecDeque;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::{ActiveChoice, App};
use crate::content::effect::Effect;
use crate::content::model::Presence;
use crate::game::text;
use crate::ui::theme;

enum Item {
    Msg(String, String),
    Sys(String),
    Effects(Vec<Effect>),
    Resolve,
}

pub struct ChatRun {
    pub id: String,
    pub with: String,
    node: String,
    queue: VecDeque<Item>,
    wait: f64,
}

impl ChatRun {
    /// Who is "typing" right now, for the indicator.
    pub fn typing(&self) -> Option<&str> {
        match self.queue.front() {
            Some(Item::Msg(who, _)) if self.wait > 0.0 => Some(who),
            _ => None,
        }
    }
}

const NAME_W: usize = 10;

impl App {
    pub fn presence(&self, user: &str) -> Option<&Presence> {
        let u = self.content.user(user)?;
        u.presence
            .iter()
            .find(|p| p.cond.eval(&self.st, &self.meta))
            .filter(|p| p.status != "offline")
    }

    pub fn chat_line(&self, who: &str, body: &str) -> Line<'static> {
        let cx = self.ctx();
        let name: String = who.chars().take(NAME_W).collect();
        let color = text::user_color(who, &cx);
        let mut spans = vec![
            Span::styled(format!("{name:>NAME_W$}"), theme::bold(color)),
            Span::styled(" │ ", Style::default().fg(theme::FAINT)),
        ];
        let body = text::subst(body, &cx);
        spans.extend(text::spans(&body, Style::default().fg(theme::TEXT)));
        Line::from(spans)
    }

    fn delay_for(&self, msg: &str) -> f64 {
        let len = msg.chars().count() as f64;
        (0.45 + 0.032 * len).min(3.0) * self.speed.chat_factor()
    }

    pub fn start_chat(&mut self, id: &str) {
        let Some(d) = self.content.dialogues.get(id) else { return };
        let with = d.with.clone();
        let first = d.nodes.first().map(|n| n.id.clone()).unwrap_or_default();
        self.st.pages.retain(|p| p != id);
        self.st.set(&format!("chat:{id}"));
        self.out.flush();
        self.blank();
        let bar = format!("── PRIVATE CHANNEL :: {with} ");
        let pad = "─".repeat(52usize.saturating_sub(bar.chars().count()));
        self.print_lines(vec![Line::styled(
            format!("{bar}{pad}"),
            Style::default().fg(text::user_color(&with, &self.ctx())),
        )]);
        self.context = format!("CHANNEL :: {with}");
        self.chat = Some(ChatRun {
            id: id.to_string(),
            with,
            node: String::new(),
            queue: VecDeque::new(),
            wait: 0.0,
        });
        self.enter_node(&first);
    }

    fn enter_node(&mut self, node_id: &str) {
        let Some(chat) = self.chat.as_ref() else { return };
        let Some(d) = self.content.dialogues.get(&chat.id) else { return };
        let Some(node) = d.nodes.iter().find(|n| n.id == node_id) else {
            self.end_chat(true);
            return;
        };
        let with = chat.with.clone();
        let mut items = VecDeque::new();
        for raw in &node.lines {
            if let Some(sys) = raw.strip_prefix('#') {
                items.push_back(Item::Sys(sys.to_string()));
            } else if let Some(rest) = raw.strip_prefix('@') {
                let (who, msg) = rest.split_once(':').unwrap_or((rest, ""));
                items.push_back(Item::Msg(who.trim().to_string(), msg.trim_start().to_string()));
            } else {
                items.push_back(Item::Msg(with.clone(), raw.clone()));
            }
        }
        if !node.effects.is_empty() {
            items.push_back(Item::Effects(node.effects.clone()));
        }
        items.push_back(Item::Resolve);
        let first_wait = match items.front() {
            Some(Item::Msg(_, m)) => self.delay_for(m),
            _ => 0.0,
        };
        if let Some(chat) = self.chat.as_mut() {
            chat.node = node_id.to_string();
            chat.queue = items;
            chat.wait = first_wait;
        }
    }

    pub fn tick_chat(&mut self, dt: f64) {
        if self.choice.is_some() {
            return;
        }
        for _ in 0..32 {
            let Some(chat) = self.chat.as_mut() else { return };
            chat.wait -= dt.min(0.25);
            if chat.wait > 0.0 {
                return;
            }
            let Some(item) = chat.queue.pop_front() else { return };
            match item {
                Item::Msg(who, msg) => {
                    let line = self.chat_line(&who, &msg);
                    self.out.flush();
                    self.print_lines(vec![line]);
                }
                Item::Sys(s) => {
                    let s = text::subst(&s, &self.ctx());
                    self.out.flush();
                    self.print_lines(vec![Line::from(text::spans(&s, Style::default().fg(theme::AMBER)))]);
                }
                Item::Effects(fx) => self.apply(&fx),
                Item::Resolve => {
                    self.resolve_node();
                    return;
                }
            }
            let next_wait = match self.chat.as_ref().and_then(|c| c.queue.front()) {
                Some(Item::Msg(_, m)) => self.delay_for(m),
                Some(Item::Sys(_)) => 0.5 * self.speed.chat_factor(),
                _ => 0.0,
            };
            if let Some(chat) = self.chat.as_mut() {
                chat.wait = next_wait;
            }
            if next_wait > 0.0 {
                return;
            }
        }
    }

    fn resolve_node(&mut self) {
        let Some(chat) = self.chat.as_ref() else { return };
        let Some(d) = self.content.dialogues.get(&chat.id) else { return };
        let Some(node) = d.nodes.iter().find(|n| n.id == chat.node).cloned() else {
            self.end_chat(true);
            return;
        };
        let opts: Vec<_> = node
            .choices
            .iter()
            .filter(|o| o.cond.eval(&self.st, &self.meta))
            .cloned()
            .collect();
        if !opts.is_empty() {
            self.choice = Some(ActiveChoice { prompt: "REPLY".into(), opts, from_chat: true, sel: 0 });
            return;
        }
        self.follow(&node.branch, node.next.as_deref());
    }

    fn follow(&mut self, branch: &[crate::content::model::Branch], next: Option<&str>) {
        if let Some(b) = branch.iter().find(|b| b.cond.eval(&self.st, &self.meta)) {
            let goto = b.goto.clone();
            self.enter_node(&goto);
        } else if let Some(n) = next {
            let n = n.to_string();
            self.enter_node(&n);
        } else {
            self.end_chat(true);
        }
    }

    /// Pick option `idx` of the current choice.
    pub fn select_choice(&mut self, idx: usize) {
        let Some(choice) = self.choice.take() else { return };
        let Some(opt) = choice.opts.get(idx).cloned() else {
            self.choice = Some(choice);
            return;
        };
        if choice.from_chat {
            let said = opt.say.clone().unwrap_or_else(|| opt.text.clone());
            if !said.is_empty() {
                let player = self.st.player.clone();
                let line = self.chat_line(&player, &said);
                self.print_lines(vec![line]);
            }
            self.apply(&opt.effects);
            if let Some(goto) = &opt.goto {
                self.enter_node(goto);
            } else if let Some(chat) = self.chat.as_ref() {
                let node = self
                    .content
                    .dialogues
                    .get(&chat.id)
                    .and_then(|d| d.nodes.iter().find(|n| n.id == chat.node))
                    .cloned();
                match node {
                    Some(n) => self.follow(&n.branch, n.next.as_deref()),
                    None => self.end_chat(true),
                }
            }
        } else {
            let label = opt.key.clone().map(|k| k.to_uppercase()).unwrap_or_else(|| opt.text.clone());
            self.dim(&format!("> {label}"));
            self.apply(&opt.effects);
        }
    }

    pub fn end_chat(&mut self, finished: bool) {
        let Some(chat) = self.chat.take() else { return };
        if self.choice.as_ref().is_some_and(|c| c.from_chat) {
            self.choice = None;
        }
        let id = chat.id.clone();
        let fx = self.content.dialogues.get(&id).map(|d| {
            if finished { d.on_end.clone() } else { d.on_leave.clone() }
        });
        if finished {
            self.st.set(&format!("done:{id}"));
        } else {
            self.st.set(&format!("left:{id}"));
        }
        self.out.flush();
        self.print_lines(vec![Line::styled(
            format!("── channel closed {}", "─".repeat(34)),
            Style::default().fg(theme::FAINT),
        )]);
        self.blank();
        self.context = "MAIN".into();
        self.portrait = Some(chat.with);
        if let Some(fx) = fx {
            self.apply(&fx);
        }
    }

    pub fn reply(&mut self) {
        if self.chat.is_some() {
            self.dim("You're already in a channel. {dim}leave{/} to close it.");
            return;
        }
        match self.st.pages.first().cloned() {
            Some(id) => self.start_chat(&id),
            None => self.dim("No messages waiting."),
        }
    }

    pub fn chat_with(&mut self, who: &str) {
        let Some(user) = self.content.user(who).cloned() else {
            self.err(&format!("chat: {who}: no such user"));
            return;
        };
        if self.chat.as_ref().is_some_and(|c| c.with == user.id) {
            self.dim("You're already talking.");
            return;
        }
        if self.chat.is_some() {
            self.dim("Close this channel first. {dim}leave{/}");
            return;
        }
        let paged = self.st.pages.iter().find(|p| {
            self.content.dialogues.get(*p).is_some_and(|d| d.with == user.id)
        });
        if let Some(id) = paged.cloned() {
            self.start_chat(&id);
            return;
        }
        if self.presence(&user.id).is_none() {
            self.err(&format!("{} is not online.", user.id));
            return;
        }
        let next = user.chats.iter().find(|c| {
            c.cond.eval(&self.st, &self.meta)
                && !self.st.has(&format!("done:{}", c.dialogue))
                && !self.st.has(&format!("chat:{}", c.dialogue))
        });
        if let Some(c) = next {
            let id = c.dialogue.clone();
            self.start_chat(&id);
            return;
        }
        let idle = crate::content::pick("", &user.idle, &self.st, &self.meta).to_string();
        let msg = if idle.is_empty() { format!("{} doesn't answer.", user.id) } else { idle };
        self.dim(&msg);
    }

    pub fn leave_chat(&mut self) {
        if self.chat.is_none() {
            self.dim("You're not in a channel.");
            return;
        }
        let msg = format!("*** you closed the channel");
        self.dim(&msg);
        self.end_chat(false);
    }
}
