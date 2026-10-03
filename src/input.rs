//! The command line: editing, cursor and history browsing.

#[derive(Default)]
pub struct Input {
    pub buf: String,
    /// Cursor position in chars.
    pub cursor: usize,
    /// Index into history while browsing with Up/Down.
    browse: Option<usize>,
    /// What the player had typed before browsing.
    draft: String,
}

impl Input {
    fn byte_at(&self, char_idx: usize) -> usize {
        self.buf
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.buf.len())
    }

    pub fn len(&self) -> usize {
        self.buf.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn insert(&mut self, c: char) {
        if self.len() >= 200 {
            return;
        }
        let i = self.byte_at(self.cursor);
        self.buf.insert(i, c);
        self.cursor += 1;
        self.browse = None;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let i = self.byte_at(self.cursor);
        self.buf.remove(i);
    }

    pub fn delete(&mut self) {
        if self.cursor < self.len() {
            let i = self.byte_at(self.cursor);
            self.buf.remove(i);
        }
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.len();
    }

    pub fn set(&mut self, s: &str) {
        self.buf = s.to_string();
        self.cursor = self.len();
    }

    /// Delete the word before the cursor (Ctrl+W).
    pub fn delete_word(&mut self) {
        while self.cursor > 0 && self.buf[..self.byte_at(self.cursor)].ends_with(' ') {
            self.backspace();
        }
        while self.cursor > 0 && !self.buf[..self.byte_at(self.cursor)].ends_with(' ') {
            self.backspace();
        }
    }

    pub fn take(&mut self) -> String {
        self.cursor = 0;
        self.browse = None;
        std::mem::take(&mut self.buf)
    }

    pub fn history_up(&mut self, history: &[String]) {
        if history.is_empty() {
            return;
        }
        let idx = match self.browse {
            None => {
                self.draft = self.buf.clone();
                history.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.browse = Some(idx);
        self.set(&history[idx]);
    }

    pub fn history_down(&mut self, history: &[String]) {
        match self.browse {
            None => {}
            Some(i) if i + 1 < history.len() => {
                self.browse = Some(i + 1);
                self.set(&history[i + 1]);
            }
            Some(_) => {
                self.browse = None;
                let d = std::mem::take(&mut self.draft);
                self.set(&d);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_and_history() {
        let mut i = Input::default();
        for c in "helo".chars() {
            i.insert(c);
        }
        i.left();
        i.insert('l');
        assert_eq!(i.buf, "hello");
        let hist = vec!["open 09".to_string(), "whoami".to_string()];
        i.history_up(&hist);
        assert_eq!(i.buf, "whoami");
        i.history_up(&hist);
        assert_eq!(i.buf, "open 09");
        i.history_down(&hist);
        i.history_down(&hist);
        assert_eq!(i.buf, "hello");
        i.delete_word();
        assert_eq!(i.buf, "");
    }
}
