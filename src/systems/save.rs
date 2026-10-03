//! Session, snapshot and meta persistence.
//!
//! - `session.json`: the live timeline, written on logout and autosaved.
//! - `snapshots/<name>.json`: player snapshots (`snapshot` / `restore`).
//! - `meta.json`: what JANUS remembers across every timeline.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::game::state::{GameState, MetaState};

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Uses `DEADLINE_HOME` when set, otherwise the platform data dir.
    pub fn open() -> Result<Self> {
        let root = match std::env::var_os("DEADLINE_HOME") {
            Some(p) => PathBuf::from(p),
            None => directories::ProjectDirs::from("dev", "pinkpixel", "deadline")
                .context("no home directory")?
                .data_dir()
                .to_path_buf(),
        };
        Self::at(root)
    }

    pub fn at(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("snapshots"))?;
        Ok(Self { root })
    }

    fn read<T: serde::de::DeserializeOwned>(&self, rel: &str) -> Option<T> {
        let text = fs::read_to_string(self.root.join(rel)).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn write<T: serde::Serialize>(&self, rel: &str, v: &T) -> Result<()> {
        let path = self.root.join(rel);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn meta(&self) -> MetaState {
        self.read("meta.json").unwrap_or_default()
    }

    pub fn save_meta(&self, m: &MetaState) -> Result<()> {
        self.write("meta.json", m)
    }

    pub fn session(&self) -> Option<GameState> {
        self.read("session.json")
    }

    pub fn save_session(&self, st: &GameState) -> Result<()> {
        self.write("session.json", st)
    }

    pub fn clear_session(&self) -> Result<()> {
        let p = self.root.join("session.json");
        if p.exists() {
            fs::remove_file(p)?;
        }
        Ok(())
    }

    pub fn save_snapshot(&self, name: &str, st: &GameState) -> Result<()> {
        self.write(&format!("snapshots/{name}.json"), st)
    }

    pub fn snapshot(&self, name: &str) -> Option<GameState> {
        self.read(&format!("snapshots/{name}.json"))
    }

    /// Delete the snapshots JANUS planted, so a new act can't restore one
    /// from an earlier run before the story writes it again.
    pub fn clear_planted(&self) -> Result<()> {
        for name in self.snapshots() {
            if self.snapshot(&name).is_some_and(|s| s.has("planted")) {
                fs::remove_file(self.root.join(format!("snapshots/{name}.json")))?;
            }
        }
        Ok(())
    }

    /// Snapshot names, newest first.
    pub fn snapshots(&self) -> Vec<String> {
        let Ok(rd) = fs::read_dir(self.root.join("snapshots")) else {
            return Vec::new();
        };
        let mut items: Vec<(std::time::SystemTime, String)> = rd
            .flatten()
            .filter_map(|e| {
                let name = e.path().file_stem()?.to_str()?.to_string();
                if e.path().extension()?.to_str()? != "json" {
                    return None;
                }
                let t = e.metadata().ok()?.modified().ok()?;
                Some((t, name))
            })
            .collect();
        items.sort_by(|a, b| b.0.cmp(&a.0));
        items.into_iter().map(|(_, n)| n).collect()
    }
}

/// Snapshot names are short, lowercase and filesystem safe.
pub fn clean_name(raw: &str) -> Option<String> {
    let s: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(16)
        .collect();
    (!s.is_empty()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("deadline-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store::at(dir).unwrap()
    }

    #[test]
    fn snapshots_and_meta_persist_separately() {
        let store = temp_store("snap");
        let mut st = GameState::new("p");
        st.set("found_it");
        store.save_snapshot("one", &st).unwrap();
        let mut meta = store.meta();
        meta.total_restores += 1;
        store.save_meta(&meta).unwrap();

        assert!(store.snapshot("one").unwrap().has("found_it"));
        assert_eq!(store.meta().total_restores, 1);
        assert_eq!(store.snapshots(), vec!["one".to_string()]);
        assert!(store.session().is_none());
        store.save_session(&st).unwrap();
        assert!(store.session().is_some());
        store.clear_session().unwrap();
        assert!(store.session().is_none());
    }

    #[test]
    fn a_new_act_clears_only_planted_snapshots() {
        let store = temp_store("planted");
        let mut st = GameState::new("p");
        store.save_snapshot("mine", &st).unwrap();
        st.set("planted");
        store.save_snapshot("before_you", &st).unwrap();
        store.clear_planted().unwrap();
        assert_eq!(store.snapshots(), vec!["mine".to_string()]);
    }

    #[test]
    fn names_are_sanitized() {
        assert_eq!(clean_name(" Before Board/09 ").as_deref(), Some("beforeboard09"));
        assert_eq!(clean_name("!!!"), None);
    }
}
