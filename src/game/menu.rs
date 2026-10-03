//! Clickable lists. Rows printed through here can be tapped, clicked, or
//! picked with the keyboard (↓ on an empty prompt, then ↑↓ and Enter).

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::output::Link;
use crate::ui::theme;

/// One selectable row: what it runs, and a short key you can type instead.
pub struct Row {
    pub line: Line<'static>,
    pub cmd: String,
    pub key: String,
}

impl Row {
    pub fn new(line: Line<'static>, cmd: impl Into<String>, key: impl Into<String>) -> Self {
        Self { line, cmd: cmd.into(), key: key.into() }
    }
}

pub struct Menu {
    pub id: u32,
    pub cmds: Vec<String>,
    pub keys: Vec<String>,
    /// None until the player steps into the list.
    pub sel: Option<usize>,
}

/// An action row such as `  › next  #102 ...`.
pub fn action(marker: &str, label: &str, detail: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {marker} "), theme::bold(theme::AMBER)),
        Span::styled(label.to_string(), theme::bold(theme::CYAN)),
        Span::styled(if detail.is_empty() { String::new() } else { format!("  {detail}") }, theme::dim()),
    ])
}

impl App {
    /// Print rows as a new selectable list. Rows with an empty command are
    /// printed in place but can't be selected.
    pub fn show_menu(&mut self, rows: Vec<Row>) {
        if rows.is_empty() {
            return;
        }
        let id = self.next_menu;
        self.next_menu += 1;
        let mut cmds = Vec::with_capacity(rows.len());
        let mut keys = Vec::with_capacity(rows.len());
        for r in rows {
            if r.cmd.is_empty() {
                self.out.push_line(r.line);
                continue;
            }
            self.out.push_link(r.line, Link { menu: id, item: cmds.len(), cmd: r.cmd.clone() });
            cmds.push(r.cmd);
            keys.push(r.key.to_lowercase());
        }
        self.out.scroll = 0;
        if !cmds.is_empty() {
            self.menu = Some(Menu { id, cmds, keys, sel: None });
        }
    }

    /// A clickable line that isn't part of the keyboard list (notices).
    pub fn print_link(&mut self, line: Line<'static>, cmd: &str) {
        self.out.push_link(line, Link { menu: 0, item: 0, cmd: cmd.to_string() });
        self.out.scroll = 0;
    }

    /// A short dim hint under a list.
    pub fn menu_hint(&mut self, typed: &str) {
        let line = Line::from(vec![
            Span::styled("  tap a row · ↓ to select · or type ", Style::default().fg(theme::FAINT)),
            Span::styled(typed.to_string(), theme::dim()),
        ]);
        self.out.push_line(line);
    }

    /// Bare input that names a row in the current list (`3`, `101`, `nodelist.txt`).
    pub fn menu_match(&self, input: &str) -> Option<String> {
        let m = self.menu.as_ref()?;
        let input = input.trim().to_lowercase();
        if let Some(i) = m.keys.iter().position(|k| !k.is_empty() && *k == input) {
            return Some(m.cmds[i].clone());
        }
        let n: usize = input.parse().ok()?;
        (n >= 1 && n <= m.cmds.len()).then(|| m.cmds[n - 1].clone())
    }

    /// Move the keyboard selection. Returns false when the key wasn't used.
    pub fn menu_step(&mut self, down: bool) -> bool {
        let Some(m) = self.menu.as_mut() else { return false };
        let last = m.cmds.len().saturating_sub(1);
        m.sel = match (m.sel, down) {
            (None, true) => Some(0),
            (None, false) => return false,
            (Some(0), false) => None,
            (Some(s), false) => Some(s - 1),
            (Some(s), true) => Some((s + 1).min(last)),
        };
        self.out.scroll = 0;
        true
    }

    /// Run the selected row, if any.
    pub fn menu_activate(&mut self) -> bool {
        let cmd = self.menu.as_ref().and_then(|m| m.sel.map(|s| m.cmds[s].clone()));
        match cmd {
            Some(c) => {
                self.submit(&c);
                true
            }
            None => false,
        }
    }
}
