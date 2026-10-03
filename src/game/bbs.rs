//! Boards, posts, mail, users and finger.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::content::{model::Board, pick};
use crate::game::menu::{Row, action};
use crate::game::text;
use crate::ui::theme;

impl App {
    fn board_open(&self, b: &Board) -> bool {
        b.lock.as_ref().is_none_or(|l| l.open.eval(&self.st, &self.meta))
    }

    fn visible_board(&self, id: &str) -> Option<Board> {
        self.content
            .board(id)
            .filter(|b| b.visible.eval(&self.st, &self.meta))
            .cloned()
    }

    fn post_visible(&self, p: &crate::content::model::Post) -> bool {
        p.visible.eval(&self.st, &self.meta)
            && self
                .visible_board(&p.board)
                .is_some_and(|b| self.board_open(&b))
    }

    pub fn cmd_boards(&mut self) {
        let content = self.content.clone();
        self.print_lines(vec![Line::from(Span::styled("MESSAGE BOARDS", theme::bold(theme::WHITE)))]);
        let mut rows = Vec::new();
        for b in content.boards.iter().filter(|b| b.visible.eval(&self.st, &self.meta)) {
            let posts: Vec<_> = content
                .posts
                .iter()
                .filter(|p| p.board == b.id && p.visible.eval(&self.st, &self.meta))
                .collect();
            let unread = posts.iter().filter(|p| !self.st.has(&format!("read:{}", p.id))).count();
            let count = b.count_label.clone().unwrap_or_else(|| format!("{} msgs", posts.len()));
            let locked = !self.board_open(b);
            let mut spans = vec![
                Span::styled(format!("  [{}] ", b.id), Style::default().fg(theme::DIM)),
                Span::styled(format!("{:<20}", b.name), theme::bold(if locked { theme::DIM } else { theme::CYAN })),
                Span::styled(format!("{count:>12}"), Style::default().fg(theme::DIM)),
            ];
            if locked {
                spans.push(Span::styled("   LOCKED", Style::default().fg(theme::RED)));
            } else if unread > 0 {
                spans.push(Span::styled(format!("   {unread} new"), Style::default().fg(theme::AMBER)));
            }
            rows.push(Row::new(Line::from(spans), format!("open {}", b.id), b.id.clone()));
        }
        let example = rows.first().map(|r| r.cmd.clone()).unwrap_or_default();
        self.show_menu(rows);
        self.menu_hint(&example);
    }

    pub fn cmd_open(&mut self, arg: &str) {
        if arg.is_empty() {
            self.cmd_boards();
            return;
        }
        let Some(b) = self.visible_board(arg) else {
            self.err("ERROR: BOARD DOES NOT EXIST");
            self.dim("  {dim}type{/} boards {dim}to see them all{/}");
            return;
        };
        if !self.board_open(&b) {
            let denied = b.lock.as_ref().map(|l| l.denied.clone()).unwrap_or_default();
            if denied.is_empty() {
                self.err("ACCESS DENIED");
            } else {
                self.apply(&denied);
            }
            return;
        }
        self.st.set(&format!("board:{}", b.id));
        self.st.board = Some(b.id.clone());
        self.context = format!("[{}] {}", b.id, b.name);
        self.out.flush();
        if !b.header.is_empty() {
            self.print(&b.header);
        }
        self.list_posts(&b);
        self.apply(&b.on_open);
    }

    fn list_posts(&mut self, b: &Board) {
        let content = self.content.clone();
        let mut rows = Vec::new();
        let posts: Vec<_> = content
            .posts
            .iter()
            .filter(|p| p.board == b.id && p.visible.eval(&self.st, &self.meta))
            .collect();
        if posts.is_empty() {
            self.dim("  (no messages)");
            return;
        }
        for p in &posts {
            let read = self.st.has(&format!("read:{}", p.id));
            let author = text::subst(&p.author, &self.ctx());
            let color = text::user_color(&author, &self.ctx());
            let line = Line::from(vec![
                Span::styled(if read { "   " } else { " ● " }, Style::default().fg(theme::AMBER)),
                Span::styled(format!("{:<5}", p.id), Style::default().fg(theme::DIM)),
                Span::styled(format!("{:<12}", p.date), Style::default().fg(theme::FAINT)),
                Span::styled(format!("{:<12}", author), Style::default().fg(color)),
                Span::styled(
                    text::subst(&p.subject, &self.ctx()),
                    Style::default().fg(if read { theme::DIM } else { theme::TEXT }),
                ),
            ]);
            rows.push(Row::new(line, format!("read {}", p.id), p.id.to_string()));
        }
        let example = rows.first().map(|r| r.cmd.clone()).unwrap_or_default();
        self.show_menu(rows);
        self.menu_hint(&example);
    }

    pub fn cmd_read(&mut self, arg: &str) {
        let content = self.content.clone();
        let post = if arg.is_empty() {
            let Some(board) = self.st.board.clone() else {
                self.dim("Pick a board first:");
                self.cmd_boards();
                return;
            };
            let next = content.posts.iter().find(|p| {
                p.board == board && self.post_visible(p) && !self.st.has(&format!("read:{}", p.id))
            });
            match next {
                Some(p) => p,
                None => {
                    self.dim("No unread messages here.");
                    if let Some(b) = self.visible_board(&board) {
                        self.list_posts(&b);
                    }
                    return;
                }
            }
        } else {
            let id: u32 = arg.trim_start_matches('#').parse().unwrap_or(0);
            match content.post(id).filter(|p| self.post_visible(p)) {
                Some(p) => p,
                None => {
                    self.err(&format!("MESSAGE {arg} NOT FOUND"));
                    match self.st.board.clone().and_then(|b| self.visible_board(&b)) {
                        Some(b) => self.list_posts(&b),
                        None => self.dim("  {dim}open a board first:{/} boards"),
                    }
                    return;
                }
            }
        };
        let board_name = content.board(&post.board).map(|b| b.name.clone()).unwrap_or_default();
        let author = text::subst(&post.author, &self.ctx());
        let color = text::user_color(&author, &self.ctx());
        let label = |s: &str| Span::styled(format!("  {s:<6}"), Style::default().fg(theme::FAINT));
        let mut lines = vec![
            Line::default(),
            Line::from(vec![
                Span::styled(format!(" #{} ", post.id), theme::bold(theme::BG).bg(theme::CYAN)),
                Span::styled(format!(" {board_name}"), Style::default().fg(theme::DIM)),
            ]),
            Line::from(vec![label("SUBJ"), Span::styled(text::subst(&post.subject, &self.ctx()), theme::bold(theme::WHITE))]),
            Line::from(vec![label("FROM"), Span::styled(author, Style::default().fg(color))]),
            Line::from(vec![label("DATE"), Span::styled(text::subst(&post.date, &self.ctx()), Style::default().fg(theme::DIM))]),
            Line::styled("  ".to_string() + &"─".repeat(56), Style::default().fg(theme::BORDER)),
        ];
        let body = pick(&post.body, &post.alt, &self.st, &self.meta).to_string();
        lines.extend(text::block(&body, &self.ctx()).into_iter().map(|l| pad(l, 2)));
        self.out.flush();
        self.print_lines(lines);
        self.st.board = Some(post.board.clone());
        self.st.set(&format!("read:{}", post.id));
        self.post_actions(post.id, &post.board, &board_name);
        self.apply(&post.on_read);
    }

    /// "next" and "back" rows under a post.
    fn post_actions(&mut self, id: u32, board: &str, board_name: &str) {
        let content = self.content.clone();
        let next = content
            .posts
            .iter()
            .filter(|p| p.board == board && p.id > id && self.post_visible(p))
            .min_by_key(|p| p.id);
        let mut rows = Vec::new();
        if let Some(n) = next {
            let subject = text::subst(&n.subject, &self.ctx());
            rows.push(Row::new(action("›", "next", &format!("#{} {subject}", n.id)), format!("read {}", n.id), "n"));
        }
        rows.push(Row::new(action("‹", &format!("back to {board_name}"), ""), format!("open {board}"), "b"));
        self.blank();
        self.show_menu(rows);
    }

    pub fn cmd_mail(&mut self, args: &str) {
        let content = self.content.clone();
        if args.is_empty() {
            if self.st.inbox.is_empty() {
                self.dim("Your mailbox is empty.");
                return;
            }
            self.print_lines(vec![Line::from(Span::styled("MAILBOX", theme::bold(theme::WHITE)))]);
            let mut rows = Vec::new();
            for (i, id) in self.st.inbox.clone().iter().enumerate() {
                let Some(m) = content.mail.get(id) else { continue };
                let read = self.st.has(&format!("mailread:{id}"));
                let from = text::subst(&m.from, &self.ctx());
                let color = text::user_color(&from, &self.ctx());
                let line = Line::from(vec![
                    Span::styled(if read { "   " } else { " ● " }, Style::default().fg(theme::AMBER)),
                    Span::styled(format!("{:<4}", i + 1), Style::default().fg(theme::DIM)),
                    Span::styled(format!("{from:<12}"), Style::default().fg(color)),
                    Span::styled(format!("{:<12}", text::subst(&m.date, &self.ctx())), Style::default().fg(theme::FAINT)),
                    Span::styled(m.subject.clone(), Style::default().fg(if read { theme::DIM } else { theme::TEXT })),
                ]);
                rows.push(Row::new(line, format!("mail {}", i + 1), String::new()));
            }
            self.show_menu(rows);
            self.menu_hint("mail 1");
            return;
        }
        let Ok(n) = args.parse::<usize>() else {
            self.err(&format!("mail: {args}: mailbox access denied"));
            return;
        };
        let Some(id) = self.st.inbox.get(n.wrapping_sub(1)).cloned() else {
            self.err(&format!("mail: no message {n}"));
            self.cmd_mail("");
            return;
        };
        let Some(m) = content.mail.get(&id) else { return };
        let from = text::subst(&m.from, &self.ctx());
        let color = text::user_color(&from, &self.ctx());
        let mut lines = vec![
            Line::default(),
            Line::from(vec![Span::styled("  FROM  ", Style::default().fg(theme::FAINT)), Span::styled(from, theme::bold(color))]),
            Line::from(vec![Span::styled("  SUBJ  ", Style::default().fg(theme::FAINT)), Span::styled(m.subject.clone(), theme::bold(theme::WHITE))]),
            Line::from(vec![Span::styled("  DATE  ", Style::default().fg(theme::FAINT)), Span::styled(text::subst(&m.date, &self.ctx()), theme::dim())]),
            Line::styled("  ".to_string() + &"─".repeat(56), Style::default().fg(theme::BORDER)),
        ];
        let body = pick(&m.body, &m.alt, &self.st, &self.meta).to_string();
        lines.extend(text::block(&body, &self.ctx()).into_iter().map(|l| pad(l, 2)));
        self.out.flush();
        self.print_lines(lines);
        self.st.set(&format!("mailread:{id}"));
        let mut rows = Vec::new();
        if n < self.st.inbox.len() {
            rows.push(Row::new(action("›", "next message", ""), format!("mail {}", n + 1), "n"));
        }
        rows.push(Row::new(action("‹", "back to mailbox", ""), "mail", "b"));
        self.blank();
        self.show_menu(rows);
        self.apply(&m.on_read);
    }

    pub fn cmd_users(&mut self) {
        let content = self.content.clone();
        self.print_lines(vec![Line::from(Span::styled("USERS ONLINE", theme::bold(theme::WHITE)))]);
        let player = self.st.player.clone();
        let mut rows = vec![Row::new(
            Line::from(vec![
                Span::styled("  NODE 02  ", Style::default().fg(theme::DIM)),
                Span::styled(format!("{:<14}", player), theme::bold(theme::WHITE)),
                Span::styled("you", theme::dim()),
            ]),
            "whoami",
            player.clone(),
        )];
        for u in &content.users {
            let Some(p) = self.presence(&u.id) else { continue };
            if p.status == "hidden" {
                continue;
            }
            let node = if p.node.is_empty() { "??".to_string() } else { p.node.clone() };
            let line = Line::from(vec![
                Span::styled(format!("  NODE {node:<3} "), Style::default().fg(theme::DIM)),
                Span::styled(format!("{:<14}", u.id), theme::bold(theme::color(&u.color))),
                Span::styled(p.status.clone(), theme::dim()),
            ]);
            rows.push(Row::new(line, format!("chat {}", u.id), u.id.clone()));
        }
        let example = rows.get(1).map(|r| r.cmd.clone()).unwrap_or_else(|| "whoami".into());
        self.show_menu(rows);
        self.menu_hint(&example);
    }

    pub fn cmd_finger(&mut self, arg: &str) {
        if arg.is_empty() {
            self.dim("finger <user>");
            return;
        }
        let is_player = arg.eq_ignore_ascii_case(&self.st.player);
        let key = if is_player { "$player" } else { arg };
        let Some(u) = self.content.user(key).cloned() else {
            self.err(&format!("finger: {arg}: no such user"));
            return;
        };
        let body = pick("", &u.finger, &self.st, &self.meta).to_string();
        self.out.flush();
        self.blank();
        if let Some(sprite) = u.sprite.as_ref().filter(|_| !body.contains("@sprite")) {
            let lines = self
                .content
                .sprites
                .get(sprite)
                .map(|s| s.lines(0, crate::ui::pixel::Tint::None))
                .unwrap_or_default();
            self.print_lines(lines.into_iter().map(|l| pad(l, 2)).collect());
        }
        self.print(&body);
        self.st.set(&format!("finger:{}", u.id.trim_start_matches('$')));
        if !is_player {
            self.portrait = Some(u.id.clone());
        }
    }

    pub fn cmd_whoami(&mut self) {
        self.print("USER: {player}\nACCESS: 4");
    }
}

fn pad(mut l: Line<'static>, n: usize) -> Line<'static> {
    l.spans.insert(0, Span::raw(" ".repeat(n)));
    l
}
