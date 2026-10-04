use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Everything that belongs to one timeline. Snapshots copy this.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    pub player: String,
    pub flags: BTreeSet<String>,
    pub vars: BTreeMap<String, i32>,
    /// Delivered mail ids, newest last.
    pub inbox: Vec<String>,
    pub history: Vec<String>,
    pub commands: u32,
    /// Seconds of play in this timeline.
    pub elapsed: f64,
    pub fired: BTreeSet<String>,
    /// Events whose condition holds, waiting on their delay: (commands, seconds).
    pub armed: BTreeMap<String, (u32, f64)>,
    /// Conversations waiting for a `reply`.
    pub pages: Vec<String>,
    pub act: u8,
    /// The act whose opening has already run. Acts open on the next login
    /// after the previous one ends.
    pub act_started: u8,
    /// Board the player is currently in, for `n` and bare `read`.
    pub board: Option<String>,
    /// Current directory in the file area.
    pub cwd: String,
    /// Journal entry ids, in the order they were discovered.
    pub journal: Vec<String>,
    /// The player's own notes: (BBS clock time, text).
    pub notes: Vec<(String, String)>,
    /// Set by the `Clock` effect: (elapsed when set, seconds past midnight).
    pub clock: Option<(f64, u32)>,
    /// Set by the `Mark` effect. `Rewind` returns here.
    pub mark: Option<Box<GameState>>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new("guest")
    }
}

impl GameState {
    pub fn new(player: &str) -> Self {
        Self {
            player: player.to_string(),
            flags: BTreeSet::new(),
            vars: BTreeMap::new(),
            inbox: Vec::new(),
            history: Vec::new(),
            commands: 0,
            elapsed: 0.0,
            fired: BTreeSet::new(),
            armed: BTreeMap::new(),
            pages: Vec::new(),
            act: 1,
            act_started: 1,
            board: None,
            cwd: "/".to_string(),
            journal: Vec::new(),
            notes: Vec::new(),
            clock: None,
            mark: None,
        }
    }

    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn set(&mut self, flag: &str) -> bool {
        self.flags.insert(flag.to_string())
    }

    pub fn unset(&mut self, flag: &str) {
        self.flags.remove(flag);
    }

    pub fn var(&self, name: &str) -> i32 {
        self.vars.get(name).copied().unwrap_or(0)
    }

    pub fn add(&mut self, name: &str, delta: i32) {
        *self.vars.entry(name.to_string()).or_insert(0) += delta;
    }

    pub fn set_var(&mut self, name: &str, value: i32) {
        self.vars.insert(name.to_string(), value);
    }

    /// Return to the marked moment, keeping what belongs to the player rather
    /// than the timeline: vars, history, notes, the journal and `keep` flags.
    pub fn rewind(&mut self, keep: &[String]) -> bool {
        let Some(mark) = self.mark.clone() else { return false };
        let mut back = (*mark).clone();
        back.mark = Some(mark);
        back.player = self.player.clone();
        back.vars = std::mem::take(&mut self.vars);
        back.history = std::mem::take(&mut self.history);
        back.notes = std::mem::take(&mut self.notes);
        back.journal = std::mem::take(&mut self.journal);
        for f in keep.iter().filter(|f| self.has(f)) {
            back.set(f);
        }
        *self = back;
        true
    }

    /// Called on login. If an act ended last session, open the next one by
    /// setting `act:<n>`. Returns true when a new act begins.
    pub fn begin_act(&mut self) -> bool {
        if self.act <= self.act_started {
            return false;
        }
        self.act_started = self.act;
        self.set(&format!("act:{}", self.act));
        true
    }
}

/// State that lives outside every timeline. Restores never touch it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaState {
    pub total_restores: u32,
    pub sessions: u32,
    pub endings: Vec<String>,
    pub discoveries: BTreeSet<String>,
    /// Runs that reached an ending, counting repeats.
    pub runs: u32,
}

impl MetaState {
    /// Finished runs. Saves from before `runs` existed count their endings.
    pub fn runs_done(&self) -> u32 {
        let endings = self.endings.iter().filter(|e| !e.starts_with("act")).count() as u32;
        self.runs.max(endings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vars_default_to_zero_and_accumulate() {
        let mut st = GameState::new("x");
        assert_eq!(st.var("trust_ghost"), 0);
        st.add("trust_ghost", 2);
        st.add("trust_ghost", -3);
        assert_eq!(st.var("trust_ghost"), -1);
        assert!(st.set("a"));
        assert!(!st.set("a"));
    }

    #[test]
    fn acts_open_on_the_next_login() {
        let mut st = GameState::new("x");
        assert!(!st.begin_act());
        st.act = 2;
        assert!(st.begin_act());
        assert!(st.has("act:2"));
        assert!(!st.begin_act());
    }

    #[test]
    fn rewind_returns_to_the_mark_but_keeps_the_player_s_side() {
        let mut st = GameState::new("x");
        st.set("night");
        let mut marked = st.clone();
        marked.mark = None;
        st.mark = Some(Box::new(marked));
        st.set("warned");
        st.set("posted");
        st.add("truth", 2);
        st.history.push("chat eli".into());
        st.notes.push(("09:10 PM".into(), "eli is tired".into()));
        assert!(st.rewind(&["warned".to_string()]));
        assert!(st.has("night") && st.has("warned"));
        assert!(!st.has("posted"), "the rest of the night is undone");
        assert_eq!(st.var("truth"), 2);
        assert_eq!(st.history, vec!["chat eli".to_string()]);
        assert_eq!(st.notes.len(), 1);
        assert!(st.mark.is_some(), "the mark stays for the next rewind");
        st.set("posted");
        assert!(st.rewind(&[]));
        assert!(!st.has("posted") && !st.has("warned"));
    }

    #[test]
    fn state_roundtrips_through_json() {
        let mut st = GameState::new("x");
        st.set("seen");
        st.history.push("whoami".into());
        let json = serde_json::to_string(&st).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert!(back.has("seen"));
        assert_eq!(back.history, vec!["whoami".to_string()]);
    }
}
