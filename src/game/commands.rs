//! Command entry and dispatch.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::output::Speed;
use crate::systems::save;
use crate::ui::theme;

/// Commands between session autosaves.
const AUTOSAVE_EVERY: u32 = 10;

/// Lowercase and collapse whitespace.
pub fn normalize(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Does a hook pattern match the normalized input?
pub fn pattern_matches(pattern: &str, input: &str) -> bool {
    let pattern = normalize(pattern);
    match pattern.strip_suffix('*') {
        Some(prefix) => {
            let prefix = prefix.trim_end();
            input == prefix || input.starts_with(&format!("{prefix} "))
        }
        None => pattern == input,
    }
}

impl App {
    /// Enter pressed on the command line.
    pub fn submit(&mut self, raw: &str) {
        if self.password.is_some() {
            self.submit_password(raw);
            return;
        }
        if let Some(choice) = &self.choice {
            if let Some(idx) = choice_index(raw, choice) {
                self.select_choice(idx);
                return;
            }
            if !choice.from_chat {
                let keys: Vec<String> = choice
                    .opts
                    .iter()
                    .enumerate()
                    .map(|(i, o)| o.key.clone().unwrap_or_else(|| (i + 1).to_string()).to_uppercase())
                    .collect();
                self.dim(&format!("[{}]", keys.join("/")));
                return;
            }
        }
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            self.out.flush();
            return;
        }
        // A bare row key or number picks from the list on screen.
        if !trimmed.contains(' ') {
            if let Some(cmd) = self.menu_match(trimmed) {
                self.submit(&cmd);
                return;
            }
        }
        self.menu = None;
        self.echo(trimmed);
        if self.st.history.last().map(String::as_str) != Some(trimmed) {
            self.st.history.push(trimmed.to_string());
            if self.st.history.len() > 500 {
                self.st.history.remove(0);
            }
        }
        self.st.commands += 1;
        let input = normalize(trimmed);
        let verb = input.split(' ').next().unwrap_or("").to_string();
        if verb == "note" {
            // Notes keep the player's own casing.
            let raw = trimmed.split_once(char::is_whitespace).map(|(_, r)| r).unwrap_or("");
            self.cmd_note(raw);
        } else if !self.run_hooks(&input) {
            let args = input[verb.len()..].trim().to_string();
            self.builtin(&verb, &args);
        }
        self.st.set(&format!("ran:{verb}"));
        self.check_events();
        // Closing the window skips logout, so don't wait for it.
        if self.st.commands % AUTOSAVE_EVERY == 0 {
            self.save_session();
        }
    }

    fn echo(&mut self, s: &str) {
        let prompt = format!("{}@deadline", self.st.player);
        self.out.push_line(Line::default());
        self.out.push_line(Line::from(vec![
            Span::styled(prompt, Style::default().fg(theme::DIM)),
            Span::styled(" > ", Style::default().fg(theme::FAINT)),
            Span::styled(s.to_string(), theme::bold(theme::WHITE)),
        ]));
        self.out.scroll = 0;
    }

    /// Run the first matching story hook. Returns true if it consumed the command.
    fn run_hooks(&mut self, input: &str) -> bool {
        let content = self.content.clone();
        // The act in play gets first say, so a later act can override an
        // earlier act's catch-all for the same command.
        let act = self.st.act_started;
        let current = content.hooks.iter().filter(|h| h.act == act);
        let rest = content.hooks.iter().filter(|h| h.act != act);
        for hook in current.chain(rest) {
            let player = self.st.player.clone();
            if !hook.input.iter().any(|p| pattern_matches(&p.replace("{player}", &player), input)) {
                continue;
            }
            if !hook.cond.eval(&self.st, &self.meta) {
                continue;
            }
            self.apply(&hook.effects);
            return hook.consume.unwrap_or(true);
        }
        false
    }

    fn builtin(&mut self, verb: &str, args: &str) {
        match verb {
            "help" | "?" | "h" => self.cmd_help(),
            "boards" | "b" | "list" => self.cmd_boards(),
            "open" | "board" | "join" | "j" | "go" => {
                if looks_like_file(args) {
                    self.cmd_view(args)
                } else {
                    self.cmd_open(args)
                }
            }
            "read" | "r" => {
                if looks_like_file(args) {
                    self.cmd_view(args)
                } else {
                    self.cmd_read(args)
                }
            }
            "n" | "next" => self.cmd_read(""),
            "mail" | "m" => self.cmd_mail(args),
            "journal" | "notes" | "jo" => self.cmd_journal(),
            "users" | "who" | "w" | "online" => self.cmd_users(),
            "finger" | "f" => self.cmd_finger(args),
            "whoami" => self.cmd_whoami(),
            "chat" | "page" | "talk" | "msg" => {
                if args.is_empty() {
                    self.dim("chat <user>");
                } else {
                    self.chat_with(args)
                }
            }
            "reply" | "re" => {
                if args.is_empty() {
                    self.reply()
                } else {
                    self.chat_with(args)
                }
            }
            "send" => self.cmd_send(args),
            "leave" | "close" | "part" => self.leave_chat(),
            "bye" if self.chat.is_some() => self.leave_chat(),
            "files" | "ls" | "dir" => self.cmd_files(args),
            "cd" => self.cmd_cd(args),
            "view" | "cat" | "type" | "more" | "less" => self.cmd_view(args),
            "inspect" | "stat" => self.cmd_inspect(args),
            "download" | "dl" | "get" => self.cmd_download(args),
            "delete" | "rm" | "del" => self.cmd_delete(args),
            "recover" => self.cmd_recover(args),
            "history" => self.cmd_history(),
            "clear" | "cls" => self.out.clear(),
            "snapshot" | "save" => self.cmd_snapshot(args),
            "restore" | "load" => self.cmd_restore(args),
            "snapshots" => self.cmd_snapshots(),
            "speed" => self.cmd_speed(args),
            "date" | "time" => self.print("{dim}SYSTEM CLOCK:{/} {date}  {time}"),
            "status" | "stats" => self.cmd_status(),
            "trace" => self.err(&format!("trace: {args}: no route to host")),
            "logout" | "logoff" | "quit" | "exit" | "bye" | "g" | "goodbye" => self.logout(),
            _ if verb.parse::<u32>().is_ok() => self.bare_number(verb),
            _ => {
                self.err(&format!("BAD COMMAND OR FILE NAME: {verb}"));
                self.dim("  {dim}type{/} help {dim}for commands{/}");
            }
        }
    }

    /// A number on its own: a post on this board, or a board.
    fn bare_number(&mut self, n: &str) {
        let id: u32 = n.parse().unwrap_or(0);
        let on_board = self.st.board.as_ref().is_some_and(|b| {
            self.content.post(id).is_some_and(|p| &p.board == b)
        });
        if on_board || self.content.post(id).is_some() {
            self.cmd_read(n);
        } else {
            self.cmd_open(n);
        }
    }

    fn cmd_help(&mut self) {
        let content = self.content.clone();
        self.print("{b}COMMANDS{/}");
        let mut rows = Vec::new();
        for h in content.help.iter().filter(|h| h.cond.eval(&self.st, &self.meta)) {
            let cmd = crate::game::text::subst(&h.cmd, &self.ctx());
            let line = Line::from(vec![
                Span::styled(format!("  {cmd:<22}"), theme::bold(theme::CYAN)),
                Span::styled(h.desc.clone(), theme::dim()),
            ]);
            // Rows that work without an argument run when tapped.
            let first = cmd.split('/').next().unwrap_or("").trim();
            let verb = first.split_whitespace().next().unwrap_or("");
            let runnable = !first.contains('<') && !matches!(verb, "logout" | "restore" | "clear");
            let cmd = if runnable { verb.to_string() } else { String::new() };
            rows.push(crate::game::menu::Row::new(line, cmd, String::new()));
        }
        self.show_menu(rows);
    }

    fn cmd_history(&mut self) {
        let hist = self.st.history.clone();
        let start = hist.len().saturating_sub(40);
        let lines: Vec<Line<'static>> = hist[start..]
            .iter()
            .enumerate()
            .map(|(i, h)| {
                Line::from(vec![
                    Span::styled(format!("{:>5}  ", start + i + 1), Style::default().fg(theme::FAINT)),
                    Span::styled(h.clone(), Style::default().fg(theme::TEXT)),
                ])
            })
            .collect();
        self.print_lines(lines);
    }

    fn cmd_speed(&mut self, args: &str) {
        self.speed = match args {
            "slow" => Speed::Slow,
            "normal" => Speed::Normal,
            "fast" => Speed::Fast,
            "instant" => Speed::Instant,
            _ => {
                let cur = self.speed.name();
                self.dim(&format!("speed: {cur}  {{dim}}(slow | normal | fast | instant){{/}}"));
                return;
            }
        };
        let name = self.speed.name();
        self.dim(&format!("LINE SPEED SET: {name}"));
    }

    fn cmd_status(&mut self) {
        let noise = 2 + self.rng.below(9);
        let retries = self.st.commands / 7;
        self.print(&format!(
            "{{b}}LINK STATUS{{/}}\n  CARRIER     {{green}}14400 BPS V.32bis{{/}}\n  NODE        02 of 08\n  PROTOCOL    ZMODEM / ANSI-BBS\n  LINE NOISE  {noise}%\n  RETRIES     {retries}\n  SESSION     {{time}}"
        ));
    }

    fn cmd_snapshot(&mut self, args: &str) {
        if self.chat.is_some() || self.choice.is_some() {
            self.err("SNAPSHOT REFUSED: CHANNEL OPEN");
            return;
        }
        let name = save::clean_name(args).unwrap_or_else(|| "quick".into());
        let result = self.store.as_ref().map(|s| s.save_snapshot(&name, &self.st));
        match result {
            Some(Ok(())) => {
                self.print(&format!("{{green}}SNAPSHOT WRITTEN:{{/}} {name}  {{dim}}{{date}} {{time}}{{/}}"));
                self.log(&format!("snapshot {name}"));
            }
            Some(Err(e)) => self.err(&format!("SNAPSHOT FAILED: {e}")),
            None => self.err("SNAPSHOT FAILED: NO STORAGE"),
        }
    }

    fn cmd_restore(&mut self, args: &str) {
        if self.chat.is_some() || self.choice.is_some() {
            self.err("RESTORE REFUSED: CHANNEL OPEN");
            return;
        }
        let name = save::clean_name(args).unwrap_or_else(|| "quick".into());
        let Some(mut snap) = self.store.as_ref().and_then(|s| s.snapshot(&name)) else {
            self.err(&format!("restore: {name}: no such snapshot  {{dim}}snapshots{{/}}"));
            return;
        };
        if snap.has("planted") {
            // JANUS wrote this one. It's rebuilt from who you are now, and it
            // remembers where you were so the story can send you back.
            // Restoring it again from inside keeps the original way back,
            // or the replay would become its own return point.
            let here = match &self.st.mark {
                Some(mark) if self.st.has("planted") => mark.clone(),
                _ => {
                    let mut here = self.st.clone();
                    here.mark = None;
                    Box::new(here)
                }
            };
            snap.vars = self.st.vars.clone();
            snap.journal = self.st.journal.clone();
            snap.mark = Some(here);
        }
        // The command history and your own notes belong to your side of the
        // modem, not the timeline.
        snap.history = std::mem::take(&mut self.st.history);
        snap.notes = std::mem::take(&mut self.st.notes);
        snap.player = self.st.player.clone();
        self.st = snap;
        self.st.set("restored");
        self.meta.total_restores += 1;
        self.save_meta();
        self.queue.clear();
        self.out.clear();
        self.glitch_until = self.t + 0.6;
        self.context = "MAIN".into();
        self.print(&format!(
            "{{green}}SESSION RESTORED:{{/}} {name}\n{{dim}}restore count: {{restores}} (persistent){{/}}"
        ));
        self.log(&format!("restore {name}"));
        self.check_events();
        self.save_session();
    }

    fn cmd_snapshots(&mut self) {
        let names = self.store.as_ref().map(|s| s.snapshots()).unwrap_or_default();
        if names.is_empty() {
            self.dim("No snapshots. {dim}snapshot <name>{/}");
            return;
        }
        self.print("{b}SNAPSHOTS{/}");
        for n in names {
            self.dim(&format!("  {n}"));
        }
    }
}

fn looks_like_file(args: &str) -> bool {
    args.contains('/') || args.contains('.')
}

fn choice_index(raw: &str, c: &crate::app::ActiveChoice) -> Option<usize> {
    let r = raw.trim().to_lowercase();
    if r.is_empty() {
        return None;
    }
    if let Ok(n) = r.parse::<usize>() {
        return (n >= 1 && n <= c.opts.len()).then(|| n - 1);
    }
    c.opts.iter().position(|o| {
        o.key.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(&r))
            || (r == "yes" && o.key.as_deref() == Some("y"))
            || (r == "no" && o.key.as_deref() == Some("n"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_match_exact_and_wildcard() {
        assert!(pattern_matches("open 09", "open 09"));
        assert!(pattern_matches("Open  09", "open 09"));
        assert!(!pattern_matches("open 09", "open 08"));
        assert!(pattern_matches("trace *", "trace ghost_17"));
        assert!(pattern_matches("trace *", "trace"));
        assert!(!pattern_matches("trace *", "tracer"));
        assert_eq!(normalize("  READ   101 "), "read 101");
    }
}
