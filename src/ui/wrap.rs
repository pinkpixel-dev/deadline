//! Word wrapping that keeps span styles, and never breaks pixel art.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

fn is_art(line: &Line) -> bool {
    line.spans
        .iter()
        .any(|s| s.content.contains(['▀', '▄', '█', '▓', '▒', '░']))
}

/// Continuation indent: past the `name │ ` gutter for chat lines,
/// otherwise the line's own leading whitespace.
fn hang(line: &Line) -> usize {
    match line.spans.get(1) {
        Some(s) if s.content == " │ " => line.spans[0].width() + 3,
        _ => {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            text.len() - text.trim_start_matches(' ').len()
        }
    }
}

pub fn wrap(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width == 0 || line.width() <= width || is_art(line) {
        return vec![line.clone()];
    }
    let indent = hang(line).min(width / 2);
    let cells: Vec<(char, Style)> = line
        .spans
        .iter()
        .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
        .collect();

    let mut rows: Vec<Vec<(char, Style)>> = Vec::new();
    let mut row: Vec<(char, Style)> = Vec::new();
    let mut row_w = 0;
    let mut last_space: Option<usize> = None;
    for &(c, st) in &cells {
        let cw = c.width().unwrap_or(0);
        let limit = if rows.is_empty() { width } else { width - indent };
        if row_w + cw > limit {
            let carry = match last_space {
                Some(i) if i > 0 => {
                    let rest = row.split_off(i + 1);
                    row.pop();
                    rest
                }
                _ => Vec::new(),
            };
            rows.push(std::mem::take(&mut row));
            row = carry;
            row_w = row.iter().map(|(c, _)| c.width().unwrap_or(0)).sum();
            last_space = None;
            if row.is_empty() && c == ' ' {
                continue;
            }
        }
        if c == ' ' {
            last_space = Some(row.len());
        }
        row.push((c, st));
        row_w += cw;
    }
    if !row.is_empty() {
        rows.push(row);
    }

    rows.into_iter()
        .enumerate()
        .map(|(i, cells)| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            if i > 0 && indent > 0 {
                spans.push(Span::raw(" ".repeat(indent)));
            }
            for (c, st) in cells {
                match spans.last_mut() {
                    Some(last) if last.style == st => last.content.to_mut().push(c),
                    _ => spans.push(Span::styled(c.to_string(), st)),
                }
            }
            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::text::plain;

    #[test]
    fn wraps_on_words_and_keeps_art() {
        let l = Line::from("the quick brown fox jumps over the lazy dog");
        let rows = wrap(&l, 16);
        assert!(rows.iter().all(|r| r.width() <= 16));
        assert_eq!(plain(&rows[0]), "the quick brown");
        let indented = Line::from("  the quick brown fox jumps over");
        let rows = wrap(&indented, 16);
        assert!(plain(&rows[1]).starts_with("  ") && !plain(&rows[1]).starts_with("   "));
        let art = Line::from("▀".repeat(40));
        assert_eq!(wrap(&art, 10).len(), 1);
    }

    #[test]
    fn chat_lines_hang_indent() {
        let l = Line::from(vec![
            Span::raw("  ghost_17"),
            Span::raw(" │ "),
            Span::raw("this message is long enough to wrap around"),
        ]);
        let rows = wrap(&l, 30);
        assert!(rows.len() > 1);
        assert!(plain(&rows[1]).starts_with(&" ".repeat(13)));
    }
}
