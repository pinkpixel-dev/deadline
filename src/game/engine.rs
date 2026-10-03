//! Applying story effects, firing scheduled events and ambient chatter.

use crate::app::App;
use crate::content::effect::Effect;

impl App {
    pub fn apply(&mut self, effects: &[Effect]) {
        for e in effects {
            self.apply_one(e);
        }
    }

    pub fn apply_one(&mut self, e: &Effect) {
        match e {
            Effect::Set(f) => {
                self.st.set(f);
            }
            Effect::Unset(f) => self.st.unset(f),
            Effect::Add(v, n) => self.st.add(v, *n),
            Effect::SetVar(v, n) => self.st.set_var(v, *n),
            Effect::Meta(f) => {
                if self.meta.discoveries.insert(f.clone()) {
                    self.save_meta();
                }
            }
            Effect::Print(s) => self.print(s),
            Effect::Notice(s) => self.notice(s),
            Effect::Log(s) => self.log(s),
            Effect::Mail(id) => self.deliver_mail(id),
            Effect::Page(id) => self.page(id),
            Effect::Glitch(ms) => {
                self.glitch_until = self.glitch_until.max(self.t + *ms as f64 / 1000.0);
            }
            Effect::History(s) => self.st.history.push(s.clone()),
            Effect::Forget(s) => self.st.history.retain(|h| h != s),
            Effect::If(c, a, b) => {
                let branch = if c.eval(&self.st, &self.meta) { a } else { b };
                let branch = branch.clone();
                self.apply(&branch);
            }
            Effect::EndAct(n) => self.end_act(*n),
            Effect::Chat(_)
            | Effect::Overlay(_)
            | Effect::Sequence(_)
            | Effect::Choose(_)
            | Effect::Password(_) => self.start_or_queue(e.clone()),
        }
    }

    fn deliver_mail(&mut self, id: &str) {
        if self.st.inbox.iter().any(|m| m == id) {
            return;
        }
        let Some(mail) = self.content.mail.get(id) else { return };
        let from = mail.from.clone();
        self.st.inbox.push(id.to_string());
        self.st.set(&format!("mail:{id}"));
        let n = self.st.inbox.len();
        self.notice_link(&format!("NEW MAIL FROM {from}"), &format!("mail {n}"));
        self.log(&format!("mail queued from {from}"));
    }

    fn page(&mut self, id: &str) {
        let Some(d) = self.content.dialogues.get(id) else { return };
        if self.st.pages.iter().any(|p| p == id) || self.st.has(&format!("done:{id}")) {
            return;
        }
        let with = d.with.clone();
        self.st.pages.push(id.to_string());
        self.st.set(&format!("page:{id}"));
        self.blank();
        self.notice_link(&format!("PRIVATE MESSAGE FROM {with}"), "reply");
        self.log(&format!("page from {with}"));
    }

    fn end_act(&mut self, n: u8) {
        self.st.act = self.st.act.max(n + 1);
        self.st.set(&format!("actend:{n}"));
        let tag = format!("act{n}");
        if !self.meta.endings.contains(&tag) {
            self.meta.endings.push(tag);
        }
        self.meta.discoveries.insert(format!("actend:{n}"));
        self.save_meta();
        let seq = format!("act{n}_end");
        if self.content.sequences.contains_key(&seq) {
            self.start_or_queue(Effect::Sequence(seq));
        }
    }

    /// Fire every event whose condition holds and whose delay has passed.
    pub fn check_events(&mut self) {
        if self.modal() {
            return;
        }
        let content = self.content.clone();
        for ev in &content.events {
            if !ev.repeat && self.st.fired.contains(&ev.id) {
                continue;
            }
            if !ev.when.eval(&self.st, &self.meta) {
                self.st.armed.remove(&ev.id);
                continue;
            }
            let now = (self.st.commands, self.st.elapsed);
            let armed = *self.st.armed.entry(ev.id.clone()).or_insert(now);
            let ready = now.0.saturating_sub(armed.0) >= ev.after_cmds
                && now.1 - armed.1 >= ev.after_secs as f64;
            if !ready {
                continue;
            }
            self.st.armed.remove(&ev.id);
            self.st.fired.insert(ev.id.clone());
            if ev.repeat {
                self.st.armed.insert(ev.id.clone(), now);
            }
            self.apply(&ev.effects);
            if self.modal() {
                break;
            }
        }
    }

    /// Background noise that makes the board feel inhabited.
    pub fn ambient(&mut self) {
        if self.t < self.next_ambient || self.chat.is_some() || self.modal() {
            return;
        }
        self.next_ambient = self.t + self.rng.range(35.0, 85.0);
        let content = self.content.clone();
        let pool: Vec<_> = content
            .ambient
            .iter()
            .filter(|a| a.cond.eval(&self.st, &self.meta))
            .collect();
        if pool.is_empty() {
            return;
        }
        let pick = pool[self.rng.below(pool.len())];
        self.log(&pick.text);
        if pick.loud {
            self.dim(&pick.text);
        }
    }
}
