//! The journal: a notes file on the player's side of the modem. The story
//! fills in leads, keys and people; files and commands fill themselves in;
//! the player adds their own notes with `note <text>`.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::content::cond::Cond;
use crate::content::pick;
use crate::game::files::basename;
use crate::game::menu::Row;
use crate::game::text;
use crate::systems::clock;
use crate::ui::theme;

const MAX_NOTES: usize = 60;
const MAX_NOTE_LEN: usize = 200;
/// Resolved leads shown before the rest are summarized.
const RESOLVED_SHOWN: usize = 5;

/// "03:52:25 AM" -> "03:52 AM"
fn short_time(t: &str) -> String {
    match (t.get(..5), t.get(t.len().saturating_sub(2)..)) {
        (Some(hm), Some(ampm)) if t.len() > 8 => format!("{hm} {ampm}"),
        _ => t.to_string(),
    }
}

fn header(title: &str) -> Row {
    Row::new(Line::from(Span::styled(format!("  {title}"), theme::bold(theme::WHITE))), "", "")
}

impl App {
    /// Add any journal entries whose conditions now hold.
    pub fn check_journal(&mut self) {
        let content = self.content.clone();
        let mut added = 0;
        for rule in &content.journal {
            if self.st.journal.iter().any(|id| id == &rule.id) {
                continue;
            }
            if rule.when.eval(&self.st, &self.meta) {
                self.st.journal.push(rule.id.clone());
                added += 1;
            }
        }
        if added > 0 {
            self.notice_link(&format!("JOURNAL UPDATED (+{added})"), "journal");
            self.log(&format!("journal: {added} new"));
        }
    }

    fn entry_line(&self, marker: &str, marker_style: Style, body: &str, body_style: Style) -> Line<'static> {
        let body = text::subst(body, &self.ctx());
        let mut spans = vec![Span::styled(format!("    {marker} "), marker_style)];
        spans.extend(text::spans(&body, body_style));
        Line::from(spans)
    }

    pub fn cmd_journal(&mut self) {
        let content = self.content.clone();
        self.out.flush();
        self.blank();
        self.print_lines(vec![Line::from(vec![
            Span::styled(" JOURNAL ", theme::bold(theme::BG).bg(theme::AMBER)),
            Span::styled(format!("  {}'s notes · local file, not on the BBS", self.st.player), theme::dim()),
        ])]);
        let mut rows = Vec::new();

        let rule = |id: &String| content.journal.iter().find(|r| &r.id == id);
        let found: Vec<_> = self.st.journal.iter().filter_map(rule).collect();
        let body = |r: &crate::content::model::JournalRule| pick(&r.text, &r.alt, &self.st, &self.meta).to_string();

        // Leads: open first, then the most recent resolved ones.
        let leads: Vec<&crate::content::model::JournalRule> = found.iter().copied().filter(|r| r.section == "leads").collect();
        let (done, open): (Vec<_>, Vec<_>) = leads.iter().copied().partition(|r| r.done.eval(&self.st, &self.meta));
        if !leads.is_empty() {
            rows.push(header("LEADS"));
            for r in &open {
                let line = self.entry_line("○", theme::bold(theme::AMBER), &body(r), Style::default().fg(theme::TEXT));
                rows.push(Row::new(line, r.cmd.clone().unwrap_or_default(), ""));
            }
            if open.is_empty() {
                rows.push(Row::new(Line::styled("    nothing open right now.", theme::dim()), "", ""));
            }
            for r in done.iter().rev().take(RESOLVED_SHOWN) {
                rows.push(Row::new(self.entry_line("✓", Style::default().fg(theme::GREEN), &body(r), theme::dim()), "", ""));
            }
            if done.len() > RESOLVED_SHOWN {
                let more = done.len() - RESOLVED_SHOWN;
                rows.push(Row::new(Line::styled(format!("    + {more} older, resolved"), Style::default().fg(theme::FAINT)), "", ""));
            }
        }

        for (section, title) in [("keys", "KEYS & CODES"), ("people", "PEOPLE")] {
            let items: Vec<_> = found.iter().filter(|r| r.section == section).collect();
            if items.is_empty() {
                continue;
            }
            rows.push(header(title));
            for r in items {
                let line = self.entry_line("·", Style::default().fg(theme::CYAN), &body(r), Style::default().fg(theme::TEXT));
                rows.push(Row::new(line, r.cmd.clone().unwrap_or_default(), ""));
            }
        }

        let files: Vec<_> = content.files.iter().filter(|f| self.st.has(&format!("dl:{}", f.path))).collect();
        if !files.is_empty() {
            rows.push(header("FILES  (local storage)"));
            for f in files {
                let line = Line::from(vec![
                    Span::styled("    ↓ ", Style::default().fg(theme::GREEN)),
                    Span::styled(format!("{:<22}", basename(&f.path)), Style::default().fg(theme::TEXT)),
                    Span::styled(f.path.clone(), theme::dim()),
                ]);
                rows.push(Row::new(line, format!("view {}", f.path), ""));
            }
        }

        let commands: Vec<_> = content
            .help
            .iter()
            .filter(|h| !matches!(h.cond, Cond::Always) && h.cond.eval(&self.st, &self.meta))
            .collect();
        if !commands.is_empty() {
            rows.push(header("COMMANDS YOU'VE FOUND"));
            for h in commands {
                let cmd = text::subst(&h.cmd, &self.ctx());
                let line = Line::from(vec![
                    Span::styled("    › ", theme::bold(theme::AMBER)),
                    Span::styled(format!("{cmd:<22}"), theme::bold(theme::CYAN)),
                    Span::styled(h.desc.clone(), theme::dim()),
                ]);
                let run = if cmd.contains('<') { String::new() } else { cmd.clone() };
                rows.push(Row::new(line, run, ""));
            }
        }

        rows.push(header("YOUR NOTES"));
        if self.st.notes.is_empty() {
            rows.push(Row::new(Line::styled("    none yet. note <text> to write one.", theme::dim()), "", ""));
        }
        for (i, (time, note)) in self.st.notes.iter().enumerate() {
            let line = Line::from(vec![
                Span::styled(format!("    {:>2}  ", i + 1), Style::default().fg(theme::FAINT)),
                Span::styled(format!("{}  ", short_time(time)), theme::dim()),
                Span::styled(note.clone(), Style::default().fg(theme::WHITE)),
            ]);
            rows.push(Row::new(line, "", ""));
        }
        self.show_menu(rows);
        self.dim("  note <text> adds a note · note rm <#> removes one");
    }

    /// `note <text>` with the player's original casing.
    pub fn cmd_note(&mut self, raw: &str) {
        let raw = raw.trim();
        if raw.is_empty() {
            self.cmd_journal();
            return;
        }
        let mut words = raw.split_whitespace();
        if matches!(words.next(), Some("rm" | "del" | "delete")) {
            let n: usize = words.next().and_then(|w| w.parse().ok()).unwrap_or(0);
            if n == 0 || n > self.st.notes.len() {
                self.err("note rm <#>  (the number from your journal)");
                return;
            }
            self.st.notes.remove(n - 1);
            self.dim(&format!("note {n} removed."));
            return;
        }
        if self.st.notes.len() >= MAX_NOTES {
            self.err("journal full. note rm <#> to make room.");
            return;
        }
        let text: String = raw.chars().take(MAX_NOTE_LEN).collect();
        let time = clock::time_string(&self.st);
        self.st.notes.push((time, text));
        let n = self.st.notes.len();
        self.print(&format!("{{green}}noted.{{/}} {{dim}}journal entry {n}{{/}}"));
    }
}
