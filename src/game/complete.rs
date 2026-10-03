//! Tab completion for verbs and their targets.

use crate::app::App;
use crate::game::files::{basename, parent};

const VERBS: &[&str] = &[
    "boards", "open", "read", "next", "mail", "users", "finger", "whoami", "chat", "reply",
    "leave", "files", "cd", "view", "inspect", "download", "delete", "history", "clear",
    "snapshot", "restore", "snapshots", "speed", "status", "help", "logout", "journal", "note",
];

impl App {
    fn verb_candidates(&self) -> Vec<String> {
        let mut v: Vec<String> = VERBS.iter().map(|s| s.to_string()).collect();
        for h in self.content.help.iter().filter(|h| h.cond.eval(&self.st, &self.meta)) {
            if let Some(first) = h.cmd.split_whitespace().next() {
                v.push(first.to_string());
            }
        }
        v.sort();
        v.dedup();
        v
    }

    fn target_candidates(&self, verb: &str) -> Vec<String> {
        let mut out = Vec::new();
        match verb {
            "open" | "board" | "join" => {
                for b in self.content.boards.iter().filter(|b| b.visible.eval(&self.st, &self.meta)) {
                    out.push(b.id.clone());
                    out.push(b.name.to_lowercase());
                }
            }
            "read" | "r" => {
                if let Some(board) = &self.st.board {
                    for p in self.content.posts.iter().filter(|p| &p.board == board && p.visible.eval(&self.st, &self.meta)) {
                        out.push(p.id.to_string());
                    }
                }
            }
            "chat" | "finger" | "trace" | "page" | "talk" => {
                for u in &self.content.users {
                    if u.id.starts_with('$') {
                        continue;
                    }
                    if self.presence(&u.id).is_some() || u.listed.eval(&self.st, &self.meta) {
                        out.push(u.id.clone());
                    }
                }
                out.push(self.st.player.clone());
            }
            "view" | "cat" | "inspect" | "download" | "dl" | "delete" | "rm" | "recover" | "cd" | "files" | "ls" => {
                let cwd = self.st.cwd.clone();
                for d in self.content.dirs.iter().filter(|d| parent(&d.path) == cwd && d.visible.eval(&self.st, &self.meta)) {
                    out.push(basename(&d.path).to_string());
                }
                if verb != "cd" {
                    for f in self.content.files.iter().filter(|f| {
                        parent(&f.path) == cwd
                            && f.visible.eval(&self.st, &self.meta)
                            && !self.st.has(&format!("deleted:{}", f.path))
                    }) {
                        out.push(basename(&f.path).to_string());
                    }
                }
            }
            "restore" => {
                if let Some(s) = &self.store {
                    out.extend(s.snapshots());
                }
            }
            "speed" => out.extend(["slow", "normal", "fast", "instant"].map(String::from)),
            _ => {}
        }
        out
    }

    /// Complete the word under the cursor. Multiple matches are listed.
    pub fn complete(&mut self) {
        let buf = self.input.buf.clone();
        let (head, word, candidates) = match buf.rsplit_once(' ') {
            None => (String::new(), buf.clone(), self.verb_candidates()),
            Some((head, word)) => {
                let verb = head.split_whitespace().next().unwrap_or("").to_lowercase();
                (format!("{head} "), word.to_string(), self.target_candidates(&verb))
            }
        };
        let lw = word.to_lowercase();
        let mut matches: Vec<String> = candidates
            .into_iter()
            .filter(|c| c.to_lowercase().starts_with(&lw))
            .collect();
        matches.sort();
        matches.dedup();
        match matches.len() {
            0 => {}
            1 => {
                let done = format!("{head}{} ", matches[0]);
                self.input.set(&done);
            }
            _ => {
                let common = common_prefix(&matches);
                if common.len() > word.len() {
                    self.input.set(&format!("{head}{common}"));
                } else {
                    let list = matches.join("  ");
                    self.dim(&list);
                    self.out.flush();
                }
            }
        }
    }
}

fn common_prefix(items: &[String]) -> String {
    let first = &items[0];
    let mut end = first.len();
    for s in &items[1..] {
        end = end.min(
            first
                .chars()
                .zip(s.chars())
                .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
                .map(|(a, _)| a.len_utf8())
                .sum(),
        );
    }
    first[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_prefix() {
        let v = vec!["snapshot".to_string(), "snapshots".to_string()];
        assert_eq!(common_prefix(&v), "snapshot");
        let v = vec!["read".to_string(), "reply".to_string(), "restore".to_string()];
        assert_eq!(common_prefix(&v), "re");
    }
}
