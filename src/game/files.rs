//! The file area: listing, viewing, inspecting, downloading and recovering.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::content::effect::{Choice, ChoiceOpt, Effect};
use crate::content::model::{FileEntry, Lock};
use crate::content::pick;
use crate::game::menu::{Row, action};
use crate::game::text;
use crate::ui::theme;

/// FNV-1a, shown as the file "checksum". It changes when the file changes.
pub fn checksum(s: &str) -> String {
    let mut h: u32 = 0x811C_9DC5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:08X}")
}

pub fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) | None => "/",
        Some(i) => &path[..i],
    }
}

pub fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn join(cwd: &str, rel: &str) -> String {
    let base = if rel.starts_with('/') { "/" } else { cwd };
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Glitch text for corrupt files, stable per character position.
fn scramble(s: &str) -> String {
    const JUNK: &[char] = &['▓', '▒', '░', '#', '%', '&', '?', '¤', '§', '■'];
    s.chars()
        .enumerate()
        .map(|(i, c)| {
            let h = (i as u32).wrapping_mul(2_654_435_761) >> 27;
            if !c.is_whitespace() && (h % 3 == 0 || h % 5 == 0) {
                JUNK[(h as usize) % JUNK.len()]
            } else {
                c
            }
        })
        .collect()
}

impl App {
    fn lock_open(&self, lock: &Option<Lock>) -> bool {
        lock.as_ref().is_none_or(|l| l.open.eval(&self.st, &self.meta))
    }

    fn deny(&mut self, lock: &Option<Lock>) {
        let denied = lock.as_ref().map(|l| l.denied.clone()).unwrap_or_default();
        if denied.is_empty() {
            self.err("ACCESS DENIED");
        } else {
            self.apply(&denied);
        }
    }

    fn dir_visible(&self, path: &str) -> bool {
        path == "/" || self.content.dir(path).is_some_and(|d| d.visible.eval(&self.st, &self.meta))
    }

    fn file_present(&self, f: &FileEntry) -> bool {
        f.visible.eval(&self.st, &self.meta) && !self.st.has(&format!("deleted:{}", f.path))
    }

    /// Resolve a file argument: absolute, relative to cwd, or a unique basename.
    pub fn resolve_file(&self, arg: &str) -> Option<FileEntry> {
        let arg = arg.trim();
        if arg.is_empty() {
            return None;
        }
        let path = join(&self.st.cwd, arg);
        let present = |f: &&FileEntry| self.file_present(f) && self.dir_visible(parent(&f.path));
        if let Some(f) = self.content.files.iter().filter(present).find(|f| f.path == path) {
            return Some(f.clone());
        }
        let matches: Vec<_> = self
            .content
            .files
            .iter()
            .filter(present)
            .filter(|f| basename(&f.path).eq_ignore_ascii_case(arg))
            .collect();
        (matches.len() == 1).then(|| matches[0].clone())
    }

    fn file_body(&self, f: &FileEntry) -> String {
        pick(&f.body, &f.alt, &self.st, &self.meta).to_string()
    }

    /// Check every directory lock on the way to a file.
    fn path_open(&mut self, path: &str) -> bool {
        let mut p = parent(path).to_string();
        loop {
            if let Some(d) = self.content.dir(&p).cloned() {
                if !self.lock_open(&d.lock) {
                    self.deny(&d.lock);
                    return false;
                }
            }
            if p == "/" {
                return true;
            }
            p = parent(&p).to_string();
        }
    }

    pub fn cmd_files(&mut self, arg: &str) {
        let path = if arg.is_empty() { self.st.cwd.clone() } else { join(&self.st.cwd, arg) };
        if !self.dir_visible(&path) {
            self.err(&format!("files: {path}: no such directory"));
            return;
        }
        if let Some(d) = self.content.dir(&path).cloned() {
            if !self.lock_open(&d.lock) {
                self.deny(&d.lock);
                return;
            }
        }
        let content = self.content.clone();
        self.print_lines(vec![Line::from(vec![
            Span::styled("FILES ", theme::bold(theme::WHITE)),
            Span::styled(path.clone(), Style::default().fg(theme::CYAN)),
        ])]);
        let mut rows = Vec::new();
        if path != "/" {
            let up = parent(&path).to_string();
            let line = Line::from(Span::styled(format!("  {:<24}", "../"), theme::bold(theme::BLUE)));
            rows.push(Row::new(line, format!("cd {up}"), ".."));
        }
        for d in content.dirs.iter().filter(|d| parent(&d.path) == path && d.path != path) {
            if !d.visible.eval(&self.st, &self.meta) {
                continue;
            }
            let locked = !self.lock_open(&d.lock);
            let name = basename(&d.path).to_string();
            let line = Line::from(vec![
                Span::styled(format!("  {:<24}", format!("{name}/")), theme::bold(theme::BLUE)),
                Span::styled(if locked { "<LOCKED>" } else { "<DIR>" }, Style::default().fg(if locked { theme::RED } else { theme::DIM })),
            ]);
            rows.push(Row::new(line, format!("cd {}", d.path), name));
        }
        let mut example = String::new();
        for f in content.files.iter().filter(|f| parent(&f.path) == path && self.file_present(f)) {
            let dl = self.st.has(&format!("dl:{}", f.path));
            let name = basename(&f.path).to_string();
            let line = Line::from(vec![
                Span::styled(format!("  {:<24}", name), Style::default().fg(if f.corrupt && !self.st.has(&format!("recovered:{}", f.path)) { theme::MAGENTA } else { theme::TEXT })),
                Span::styled(format!("{:>8}  ", f.size), Style::default().fg(theme::DIM)),
                Span::styled(format!("{:<11}", f.date), Style::default().fg(theme::FAINT)),
                Span::styled(text::subst(&f.desc, &self.ctx()), theme::dim()),
                Span::styled(if dl { "  ↓" } else { "" }, Style::default().fg(theme::GREEN)),
            ]);
            if example.is_empty() {
                example = format!("view {name}");
            }
            rows.push(Row::new(line, format!("view {}", f.path), name));
        }
        if rows.is_empty() {
            self.dim("  (empty)");
            return;
        }
        if example.is_empty() {
            example = rows.last().map(|r| r.cmd.clone()).unwrap_or_default();
        }
        self.show_menu(rows);
        self.menu_hint(&example);
    }

    /// Things you can do with a file you're looking at.
    fn file_actions(&mut self, f: &FileEntry, with_view: bool) {
        let p = f.path.clone();
        let mut rows = Vec::new();
        if with_view {
            rows.push(Row::new(action("›", "view", ""), format!("view {p}"), "v"));
        }
        if !self.st.has(&format!("dl:{p}")) {
            rows.push(Row::new(action("↓", "download", &format!("{} bytes", f.size)), format!("download {p}"), "d"));
        }
        rows.push(Row::new(action("·", "inspect", ""), format!("inspect {p}"), "i"));
        if f.corrupt && self.st.has("cmd:recover") && !self.st.has(&format!("recovered:{p}")) {
            rows.push(Row::new(action("+", "recover", ""), format!("recover {p}"), "r"));
        }
        if f.can_delete {
            rows.push(Row::new(action("×", "delete", ""), format!("delete {p}"), "x"));
        }
        let dir = parent(&p).to_string();
        rows.push(Row::new(action("‹", &format!("back to {dir}"), ""), format!("cd {dir}"), "b"));
        self.blank();
        self.show_menu(rows);
    }

    pub fn cmd_cd(&mut self, arg: &str) {
        let path = if arg.is_empty() { "/".to_string() } else { join(&self.st.cwd, arg) };
        if !self.dir_visible(&path) {
            self.err(&format!("cd: {path}: no such directory"));
            return;
        }
        if let Some(d) = self.content.dir(&path).cloned() {
            if !self.lock_open(&d.lock) {
                self.deny(&d.lock);
                return;
            }
        }
        self.st.cwd = path;
        self.cmd_files("");
    }

    fn need_file(&mut self, verb: &str, arg: &str) -> Option<FileEntry> {
        if arg.is_empty() {
            self.dim(&format!("{verb} which file?"));
            self.cmd_files("");
            return None;
        }
        let Some(f) = self.resolve_file(arg) else {
            self.err(&format!("{verb}: {arg}: file not found"));
            self.dim(&format!("  {{dim}}you're in{{/}} {}  {{dim}}type{{/}} files {{dim}}to look around{{/}}", self.st.cwd));
            return None;
        };
        if !self.path_open(&f.path) {
            return None;
        }
        if !self.lock_open(&f.lock) {
            self.deny(&f.lock);
            return None;
        }
        Some(f)
    }

    pub fn cmd_view(&mut self, arg: &str) {
        let Some(f) = self.need_file("view", arg) else { return };
        let body = self.file_body(&f);
        self.out.flush();
        self.blank();
        self.print_lines(vec![Line::from(vec![
            Span::styled(format!(" {} ", basename(&f.path)), theme::bold(theme::BG).bg(theme::AMBER)),
            Span::styled(format!(" {}", f.path), Style::default().fg(theme::DIM)),
        ])]);
        self.blank();
        if f.corrupt && !self.st.has(&format!("recovered:{}", f.path)) {
            let junk = scramble(&body);
            let lines = junk.lines().map(|l| Line::styled(format!("  {l}"), Style::default().fg(theme::MAGENTA))).collect();
            self.print_lines(lines);
            self.err("  ERR: CRC MISMATCH. FILE DAMAGED.");
        } else {
            self.print(&body);
        }
        self.st.set(&format!("file:{}", f.path));
        self.file_actions(&f, false);
        self.apply(&f.on_open);
    }

    pub fn cmd_inspect(&mut self, arg: &str) {
        let Some(f) = self.need_file("inspect", arg) else { return };
        let body = self.file_body(&f);
        let sum = checksum(&body);
        let seen_key = format!("sum:{}", f.path);
        let changed = self.st.has(&format!("inspected:{}", f.path)) && !self.st.has(&format!("{seen_key}={sum}"));
        let mut text = format!(
            "{{b}}{}{{/}}\n  SIZE      {} bytes\n  UPLOADED  {}\n  BY        {}\n  CRC32     {{cyan}}{sum}{{/}}",
            f.path, f.size, f.date, f.uploader
        );
        if f.corrupt && !self.st.has(&format!("recovered:{}", f.path)) {
            text.push_str("\n  STATUS    {magenta}DAMAGED{/}");
        }
        if changed {
            text.push_str("\n  {amber}NOTE      checksum differs from last inspection{/}");
        }
        self.print(&text);
        self.st.flags.retain(|k| !k.starts_with(&format!("{seen_key}=")));
        self.st.set(&format!("{seen_key}={sum}"));
        self.st.set(&format!("inspected:{}", f.path));
        if changed {
            self.st.set(&format!("changed:{}", f.path));
        }
        self.file_actions(&f, true);
        self.apply(&f.on_inspect);
    }

    pub fn cmd_download(&mut self, arg: &str) {
        let Some(f) = self.need_file("download", arg) else { return };
        let key = format!("dl:{}", f.path);
        if self.st.has(&key) {
            self.dim(&format!("{} is already in local storage.", basename(&f.path)));
            return;
        }
        let name = basename(&f.path).to_string();
        let mut lines = vec![Line::styled(format!("  ZMODEM  receiving {name}  ({} bytes)", f.size), theme::dim())];
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("━".repeat(24), Style::default().fg(theme::GREEN)),
            Span::styled(" 100%", theme::dim()),
        ]));
        lines.push(Line::styled("  TRANSFER COMPLETE", theme::bold(theme::GREEN)));
        self.print_lines(lines);
        self.st.set(&key);
        self.log(&format!("dl {name}"));
        self.file_actions(&f, true);
        self.apply(&f.on_download);
    }

    pub fn cmd_delete(&mut self, arg: &str) {
        let Some(f) = self.need_file("delete", arg) else { return };
        if !f.can_delete {
            self.err(&format!("delete: {}: permission denied", basename(&f.path)));
            return;
        }
        let mut yes = vec![Effect::Set(format!("deleted:{}", f.path)), Effect::Print(format!("{{red}}{} DELETED.{{/}}", f.path))];
        yes.extend(f.on_delete.clone());
        self.start_or_queue(Effect::Choose(Choice {
            prompt: format!("DELETE {}? [Y/N]", f.path),
            options: vec![
                ChoiceOpt { key: Some("y".into()), text: "Delete it".into(), effects: yes, ..Default::default() },
                ChoiceOpt { key: Some("n".into()), text: "Leave it".into(), effects: vec![Effect::Print("{dim}Nothing deleted.{/}".into())], ..Default::default() },
            ],
        }));
    }

    pub fn cmd_recover(&mut self, arg: &str) {
        if !self.st.has("cmd:recover") {
            self.err("BAD COMMAND OR FILE NAME: recover");
            return;
        }
        if arg.is_empty() {
            self.dim("recover <file>");
            return;
        }
        // Deleted files can come back too.
        let path = join(&self.st.cwd, arg);
        let deleted = self.content.files.iter().find(|f| {
            (f.path == path || basename(&f.path).eq_ignore_ascii_case(arg)) && self.st.has(&format!("deleted:{}", f.path))
        });
        if let Some(f) = deleted.cloned() {
            self.st.unset(&format!("deleted:{}", f.path));
            self.st.set(&format!("recovered:{}", f.path));
            self.print(&format!("{{green}}RECOVERED{{/}} {}  {{dim}}(from slack space){{/}}", f.path));
            return;
        }
        let Some(f) = self.need_file("recover", arg) else { return };
        let key = format!("recovered:{}", f.path);
        if !f.corrupt || self.st.has(&key) {
            self.dim(&format!("{}: nothing to recover.", basename(&f.path)));
            return;
        }
        self.print(&format!(
            "  {{dim}}rebuilding sectors...{{/}}\n  {{dim}}sector 0x00A1 ok{{/}}\n  {{dim}}sector 0x00A2 {{amber}}weak{{/}}{{/}}\n  {{dim}}sector 0x00A3 ok{{/}}\n  {{green}}RECOVERED{{/}} {}",
            f.path
        ));
        self.st.set(&key);
        self.log(&format!("recovered {}", basename(&f.path)));
        self.file_actions(&f, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_join_and_split() {
        assert_eq!(join("/", "uploads"), "/uploads");
        assert_eq!(join("/uploads", "../archive/x.txt"), "/archive/x.txt");
        assert_eq!(join("/uploads", "/sysop/"), "/sysop");
        assert_eq!(join("/uploads", ".."), "/");
        assert_eq!(parent("/uploads/a.txt"), "/uploads");
        assert_eq!(parent("/uploads"), "/");
        assert_eq!(basename("/uploads/a.txt"), "a.txt");
    }

    #[test]
    fn checksum_changes_with_content() {
        assert_ne!(checksum("ELI VOSS"), checksum("ELI VOSS\n1978 - 1998"));
        assert_eq!(checksum("x").len(), 8);
    }
}
