//! Keyboard and mouse handling.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::app::App;

impl App {
    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if let Some(seq) = self.seq.as_mut() {
            if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ')) {
                seq.fast = true;
            }
            if ctrl && key.code == KeyCode::Char('c') {
                self.quit = true;
            }
            return;
        }
        if self.overlay.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ')) {
                self.dismiss_overlay();
            }
            return;
        }
        if ctrl {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('d') => self.ctrl_c(),
                KeyCode::Char('l') => self.out.clear(),
                KeyCode::Char('w') => self.input.delete_word(),
                KeyCode::Char('u') => {
                    self.input.take();
                }
                KeyCode::Char('a') => self.input.home(),
                KeyCode::Char('e') => self.input.end(),
                _ => {}
            }
            return;
        }

        // Inline Y/N choices answer on a single keypress.
        if let Some(choice) = self.choice.as_ref().filter(|c| !c.from_chat) {
            match key.code {
                KeyCode::Char(c) => {
                    let s = c.to_string();
                    let idx = choice.opts.iter().position(|o| o.key.as_deref().is_some_and(|k| k.eq_ignore_ascii_case(&s)));
                    let idx = idx.or_else(|| c.to_digit(10).map(|d| d as usize).filter(|d| *d >= 1 && *d <= choice.opts.len()).map(|d| d - 1));
                    if let Some(i) = idx {
                        self.select_choice(i);
                    }
                }
                KeyCode::Up | KeyCode::Left => self.move_sel(-1),
                KeyCode::Down | KeyCode::Right | KeyCode::Tab => self.move_sel(1),
                KeyCode::Enter => {
                    let sel = choice.sel;
                    self.select_choice(sel);
                }
                KeyCode::Esc => {
                    let n = choice.opts.iter().position(|o| o.key.as_deref() == Some("n"));
                    if let Some(i) = n {
                        self.select_choice(i);
                    }
                }
                _ => {}
            }
            return;
        }

        // Conversation choices: digits and arrows work while the line is empty.
        let chat_choice = self.choice.as_ref().filter(|c| c.from_chat).map(|c| (c.sel, c.opts.len()));
        if let Some((sel, len)) = chat_choice {
            if self.input.is_empty() {
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let d = c.to_digit(10).unwrap_or(0) as usize;
                        if d >= 1 && d <= len {
                            self.select_choice(d - 1);
                        }
                        return;
                    }
                    KeyCode::Up => return self.move_sel(-1),
                    KeyCode::Down | KeyCode::Tab => return self.move_sel(1),
                    KeyCode::Enter => return self.select_choice(sel),
                    _ => {}
                }
            }
        }

        match key.code {
            KeyCode::Char(c) => self.input.insert(c),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Left => self.input.left(),
            KeyCode::Right => self.input.right(),
            KeyCode::Home => self.input.home(),
            KeyCode::End => self.input.end(),
            KeyCode::Up if self.password.is_none() => {
                let h = self.st.history.clone();
                self.input.history_up(&h);
            }
            KeyCode::Down if self.password.is_none() => {
                let h = self.st.history.clone();
                self.input.history_down(&h);
            }
            KeyCode::PageUp => self.out.scroll_by(10),
            KeyCode::PageDown => self.out.scroll_by(-10),
            KeyCode::Tab if self.password.is_none() => self.complete(),
            KeyCode::Esc => {
                if self.password.is_some() {
                    self.cancel_password();
                } else if self.out.scroll > 0 {
                    self.out.scroll = 0;
                } else {
                    self.input.take();
                }
            }
            KeyCode::Enter => {
                let line = self.input.take();
                self.submit(&line);
            }
            _ => {}
        }
    }

    fn move_sel(&mut self, d: isize) {
        if let Some(c) = self.choice.as_mut() {
            let n = c.opts.len().max(1) as isize;
            c.sel = ((c.sel as isize + d).rem_euclid(n)) as usize;
        }
    }

    fn ctrl_c(&mut self) {
        if self.t - self.ctrl_c_at < 2.0 {
            self.logout();
        } else {
            self.ctrl_c_at = self.t;
            self.input.take();
            self.dim("^C  {dim}(again to log off){/}");
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::ScrollUp => self.out.scroll_by(3),
            MouseEventKind::ScrollDown => self.out.scroll_by(-3),
            MouseEventKind::Down(_) => {
                if self.seq.is_some() {
                    if let Some(s) = self.seq.as_mut() {
                        s.fast = true;
                    }
                    return;
                }
                if self.overlay.is_some() {
                    self.dismiss_overlay();
                    return;
                }
                let pos = Position::new(m.column, m.row);
                let hit = self.hits.iter().find(|(r, _)| r.contains(pos)).map(|(_, i)| *i);
                if let Some(i) = hit {
                    self.select_choice(i);
                }
            }
            _ => {}
        }
    }
}
