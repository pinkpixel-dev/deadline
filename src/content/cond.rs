use serde::{Deserialize, Serialize};

use crate::game::state::{GameState, MetaState};

/// A condition evaluated against the hidden game state.
///
/// Story files use these everywhere: post visibility, file versions,
/// dialogue branches, event triggers and choice availability.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum Cond {
    #[default]
    Always,
    Never,
    /// Session flag is set.
    Flag(String),
    /// Session flag is not set.
    NoFlag(String),
    /// Variable is greater than or equal to the value.
    Gte(String, i32),
    /// Variable is less than or equal to the value.
    Lte(String, i32),
    All(Vec<Cond>),
    Any(Vec<Cond>),
    Not(Box<Cond>),
    /// Persistent discovery that survives restores and new runs.
    Meta(String),
    /// Total restores across every timeline is at least this many.
    Restores(u32),
    /// Commands entered this session is at least this many.
    Commands(u32),
    /// Current act is at least this number.
    Act(u8),
}

impl Cond {
    pub fn eval(&self, st: &GameState, meta: &MetaState) -> bool {
        match self {
            Cond::Always => true,
            Cond::Never => false,
            Cond::Flag(f) => st.has(f),
            Cond::NoFlag(f) => !st.has(f),
            Cond::Gte(v, n) => st.var(v) >= *n,
            Cond::Lte(v, n) => st.var(v) <= *n,
            Cond::All(cs) => cs.iter().all(|c| c.eval(st, meta)),
            Cond::Any(cs) => cs.iter().any(|c| c.eval(st, meta)),
            Cond::Not(c) => !c.eval(st, meta),
            Cond::Meta(f) => meta.discoveries.contains(f),
            Cond::Restores(n) => meta.total_restores >= *n,
            Cond::Commands(n) => st.commands >= *n,
            Cond::Act(n) => st.act >= *n,
        }
    }

    /// Every flag name this condition reads. Used by content validation.
    pub fn flags(&self, out: &mut Vec<String>) {
        match self {
            Cond::Flag(f) | Cond::NoFlag(f) => out.push(f.clone()),
            Cond::All(cs) | Cond::Any(cs) => cs.iter().for_each(|c| c.flags(out)),
            Cond::Not(c) => c.flags(out),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_conditions_evaluate() {
        let mut st = GameState::new("tester");
        let meta = MetaState::default();
        st.set("a");
        st.add("trust", 3);
        let c = Cond::All(vec![
            Cond::Flag("a".into()),
            Cond::Gte("trust".into(), 2),
            Cond::Not(Box::new(Cond::Flag("b".into()))),
        ]);
        assert!(c.eval(&st, &meta));
        st.set("b");
        assert!(!c.eval(&st, &meta));
        assert!(Cond::Any(vec![Cond::Never, Cond::Lte("trust".into(), 3)]).eval(&st, &meta));
    }
}
