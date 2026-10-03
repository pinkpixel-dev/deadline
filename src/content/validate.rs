//! Story content validation. Catches typos in flags, ids and references.

use std::collections::{BTreeSet, HashSet};

use super::Content;
use super::cond::Cond;
use super::effect::Effect;
use super::model::*;

/// Flags the engine sets on its own, by prefix.
const ENGINE_PREFIXES: &[&str] = &[
    "ran:", "board:", "read:", "file:", "dl:", "deleted:", "inspected:", "recovered:", "changed:",
    "mail:", "mailread:", "page:", "chat:", "done:", "left:", "finger:", "overlay:", "seq:",
    "actend:", "sum:",
];
const ENGINE_FLAGS: &[&str] = &["restored"];

#[derive(Default)]
struct Refs {
    effects: Vec<Effect>,
    conds: Vec<Cond>,
    texts: Vec<String>,
}

impl Refs {
    fn fx(&mut self, e: &[Effect]) {
        self.effects.extend(e.iter().cloned());
    }
    fn alts(&mut self, a: &[Alt]) {
        for alt in a {
            self.conds.push(alt.cond.clone());
            self.texts.push(alt.body.clone());
        }
    }
    fn lock(&mut self, l: &Option<Lock>) {
        if let Some(l) = l {
            self.conds.push(l.open.clone());
            self.fx(&l.denied);
        }
    }
    fn steps(&mut self, steps: &[Step]) {
        for s in steps {
            match s {
                Step::When(c, inner) => {
                    self.conds.push(c.clone());
                    self.steps(inner);
                }
                Step::Line(t) | Step::Type(t) => self.texts.push(t.clone()),
                Step::TypeAfter(a, b) => self.texts.push(format!("{a}{b}")),
                Step::Sprite(n) => self.texts.push(format!("@sprite {n}")),
                _ => {}
            }
        }
    }
}

fn gather(c: &Content) -> Refs {
    let mut r = Refs::default();
    for b in &c.boards {
        r.conds.push(b.visible.clone());
        r.lock(&b.lock);
        r.fx(&b.on_open);
        r.texts.push(b.header.clone());
    }
    for p in &c.posts {
        r.conds.push(p.visible.clone());
        r.fx(&p.on_read);
        r.texts.push(p.body.clone());
        r.alts(&p.alt);
    }
    for m in c.mail.values() {
        r.fx(&m.on_read);
        r.texts.push(m.body.clone());
        r.alts(&m.alt);
    }
    for f in &c.files {
        r.conds.push(f.visible.clone());
        r.lock(&f.lock);
        for fx in [&f.on_open, &f.on_download, &f.on_delete, &f.on_inspect] {
            r.fx(fx);
        }
        r.texts.push(f.body.clone());
        r.alts(&f.alt);
    }
    for d in &c.dirs {
        r.conds.push(d.visible.clone());
        r.lock(&d.lock);
    }
    for u in &c.users {
        r.conds.push(u.listed.clone());
        u.presence.iter().for_each(|p| r.conds.push(p.cond.clone()));
        u.chats.iter().for_each(|ch| r.conds.push(ch.cond.clone()));
        r.alts(&u.finger);
        r.alts(&u.idle);
    }
    for d in c.dialogues.values() {
        r.fx(&d.on_leave);
        r.fx(&d.on_end);
        for n in &d.nodes {
            r.fx(&n.effects);
            n.branch.iter().for_each(|b| r.conds.push(b.cond.clone()));
            for o in &n.choices {
                r.conds.push(o.cond.clone());
                r.fx(&o.effects);
            }
        }
    }
    for e in &c.events {
        r.conds.push(e.when.clone());
        r.fx(&e.effects);
    }
    for h in &c.hooks {
        r.conds.push(h.cond.clone());
        r.fx(&h.effects);
    }
    for o in c.overlays.values() {
        r.fx(&o.on_close);
        r.texts.push(o.body.clone());
    }
    for s in c.sequences.values() {
        r.fx(&s.on_end);
        r.steps(&s.steps);
    }
    c.ambient.iter().for_each(|a| r.conds.push(a.cond.clone()));
    c.help.iter().for_each(|h| r.conds.push(h.cond.clone()));

    // Expand nested effects and the conditions/texts they carry.
    let mut all = Vec::new();
    for e in &r.effects {
        e.walk(&mut |x| all.push(x.clone()));
    }
    for e in &all {
        match e {
            Effect::If(c, ..) => r.conds.push(c.clone()),
            Effect::Choose(ch) => ch.options.iter().for_each(|o| r.conds.push(o.cond.clone())),
            Effect::Print(t) => r.texts.push(t.clone()),
            _ => {}
        }
    }
    r.effects = all;
    r
}

/// Return a list of human-readable problems. Empty means valid.
pub fn check(c: &Content) -> Vec<String> {
    let mut errs = Vec::new();
    let r = gather(c);

    // Flags: everything read must be set somewhere (or by the engine).
    let mut set: HashSet<String> = HashSet::new();
    let mut vars_set: HashSet<String> = HashSet::new();
    for e in &r.effects {
        match e {
            Effect::Set(f) => {
                set.insert(f.clone());
            }
            Effect::Add(v, _) | Effect::SetVar(v, _) => {
                vars_set.insert(v.clone());
            }
            _ => {}
        }
    }
    let mut read = Vec::new();
    for cond in &r.conds {
        cond.flags(&mut read);
    }
    let unknown: BTreeSet<_> = read
        .into_iter()
        .filter(|f| !set.contains(f))
        .filter(|f| !ENGINE_FLAGS.contains(&f.as_str()))
        .filter(|f| !ENGINE_PREFIXES.iter().any(|p| f.starts_with(p)))
        .collect();
    for f in unknown {
        errs.push(format!("flag read but never set: {f}"));
    }
    let mut vars_read = BTreeSet::new();
    fn vars(c: &Cond, out: &mut BTreeSet<String>) {
        match c {
            Cond::Gte(v, _) | Cond::Lte(v, _) => {
                out.insert(v.clone());
            }
            Cond::All(cs) | Cond::Any(cs) => cs.iter().for_each(|c| vars(c, out)),
            Cond::Not(c) => vars(c, out),
            _ => {}
        }
    }
    r.conds.iter().for_each(|c| vars(c, &mut vars_read));
    for v in vars_read.iter().filter(|v| !vars_set.contains(*v)) {
        errs.push(format!("variable read but never changed: {v}"));
    }

    // References.
    for e in &r.effects {
        match e {
            Effect::Page(id) | Effect::Chat(id) if !c.dialogues.contains_key(id) => {
                errs.push(format!("unknown dialogue: {id}"))
            }
            Effect::Overlay(id) if !c.overlays.contains_key(id) => errs.push(format!("unknown overlay: {id}")),
            Effect::Sequence(id) if !c.sequences.contains_key(id) => errs.push(format!("unknown sequence: {id}")),
            Effect::Mail(id) if !c.mail.contains_key(id) => errs.push(format!("unknown mail: {id}")),
            _ => {}
        }
    }
    for u in &c.users {
        for ch in &u.chats {
            if !c.dialogues.contains_key(&ch.dialogue) {
                errs.push(format!("user {} references unknown dialogue {}", u.id, ch.dialogue));
            }
        }
        if let Some(s) = &u.sprite {
            if !c.sprites.contains_key(s) {
                errs.push(format!("user {} has unknown sprite {s}", u.id));
            }
        }
    }
    for d in c.dialogues.values() {
        if c.user(&d.with).is_none() {
            errs.push(format!("dialogue {} is with unknown user {}", d.id, d.with));
        }
        let ids: HashSet<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
        if ids.len() != d.nodes.len() {
            errs.push(format!("dialogue {} has duplicate node ids", d.id));
        }
        for n in &d.nodes {
            if !n.choices.is_empty() && !n.branch.is_empty() {
                errs.push(format!("dialogue {} node {} has choices and branches (branches never run)", d.id, n.id));
            }
            let targets = n
                .branch
                .iter()
                .map(|b| b.goto.as_str())
                .chain(n.next.as_deref())
                .chain(n.choices.iter().filter_map(|o| o.goto.as_deref()));
            for t in targets {
                if !ids.contains(t) {
                    errs.push(format!("dialogue {} node {} jumps to missing node {t}", d.id, n.id));
                }
            }
        }
    }
    let mut post_ids = HashSet::new();
    for p in &c.posts {
        if !post_ids.insert(p.id) {
            errs.push(format!("duplicate post id {}", p.id));
        }
        if c.board(&p.board).is_none() {
            errs.push(format!("post {} on unknown board {}", p.id, p.board));
        }
    }
    for t in &r.texts {
        for line in t.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("@sprite ") {
                let name = rest.split_whitespace().next().unwrap_or("");
                if !c.sprites.contains_key(name) {
                    errs.push(format!("unknown sprite in text: {name}"));
                }
            }
            if let Some(name) = line.strip_prefix("@art ") {
                if !c.art.contains_key(name.trim()) {
                    errs.push(format!("unknown art in text: {name}"));
                }
            }
        }
    }
    for id in ["boot", "reconnect", "logout"] {
        if !c.sequences.contains_key(id) {
            errs.push(format!("missing required sequence: {id}"));
        }
    }
    errs
}
