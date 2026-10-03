//! Story markup.
//!
//! Inline: `{cyan}text{/}`, `{b}`, `{dim}`, `{rev}`, `{u}`, `{player}`,
//! `{date}`, `{today}`, `{time}`, `{var:name}`, `{word:name}`.
//!
//! Line directives: `@sprite name [dim|mono|glitch]`, `@banner color TEXT`,
//! `@art name`, `@rule`. A line like `ghost_17:` becomes a colored speaker
//! label.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::content::Content;
use crate::game::state::{GameState, MetaState};
use crate::systems::clock;
use crate::ui::font;
use crate::ui::pixel::Tint;
use crate::ui::theme;

pub struct Ctx<'a> {
    pub content: &'a Content,
    pub st: &'a GameState,
    pub meta: &'a MetaState,
}

const WORDS: [&str; 21] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
    "nineteen", "twenty",
];

pub fn number_word(n: i32) -> String {
    if (0..=20).contains(&n) {
        WORDS[n as usize].to_string()
    } else {
        n.to_string()
    }
}

/// Replace `{player}` style variables. Style tags are left in place.
pub fn subst(s: &str, cx: &Ctx) -> String {
    if !s.contains('{') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(j) = tail.find('}') else {
            out.push_str(tail);
            rest = "";
            break;
        };
        let key = &tail[1..j];
        let rep = match key {
            "player" => Some(cx.st.player.clone()),
            "PLAYER" => Some(cx.st.player.to_uppercase()),
            "date" => Some(clock::date_string(cx.st)),
            "today" => Some(chrono::Local::now().format("%m/%d/%Y").to_string()),
            "year" => Some(chrono::Local::now().format("%Y").to_string()),
            "time" => Some(clock::time_string(cx.st)),
            "restores" => Some(cx.meta.total_restores.to_string()),
            _ => {
                if let Some(v) = key.strip_prefix("var:") {
                    Some(cx.st.var(v).to_string())
                } else {
                    key.strip_prefix("word:").map(|v| number_word(cx.st.var(v)))
                }
            }
        };
        match rep {
            Some(r) => out.push_str(&r),
            None => out.push_str(&tail[..=j]),
        }
        rest = &tail[j + 1..];
    }
    out.push_str(rest);
    out
}

fn tag_style(tag: &str, cur: Style) -> Option<Style> {
    Some(match tag {
        "b" => cur.add_modifier(Modifier::BOLD),
        "dim" => cur.fg(theme::DIM),
        "rev" => cur.add_modifier(Modifier::REVERSED),
        "u" => cur.add_modifier(Modifier::UNDERLINED),
        "i" => cur.add_modifier(Modifier::ITALIC),
        _ => cur.fg(theme::named(tag)?),
    })
}

/// Parse inline style tags into spans.
pub fn spans(s: &str, base: Style) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut stack = vec![base];
    let mut buf = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        buf.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(j) = tail.find('}') else {
            buf.push_str(tail);
            rest = "";
            break;
        };
        let tag = &tail[1..j];
        let cur = *stack.last().unwrap_or(&base);
        if tag == "/" {
            if !buf.is_empty() {
                out.push(Span::styled(std::mem::take(&mut buf), cur));
            }
            if stack.len() > 1 {
                stack.pop();
            }
        } else if let Some(st) = tag_style(tag, cur) {
            if !buf.is_empty() {
                out.push(Span::styled(std::mem::take(&mut buf), cur));
            }
            stack.push(st);
        } else {
            buf.push_str(&tail[..=j]);
        }
        rest = &tail[j + 1..];
    }
    buf.push_str(rest);
    if !buf.is_empty() {
        out.push(Span::styled(buf, *stack.last().unwrap_or(&base)));
    }
    out
}

/// Color for a user id, including the player.
pub fn user_color(name: &str, cx: &Ctx) -> ratatui::style::Color {
    if name.eq_ignore_ascii_case(&cx.st.player) {
        return theme::WHITE;
    }
    cx.content
        .user(name)
        .map(|u| theme::color(&u.color))
        .unwrap_or(theme::TEXT)
}

fn is_speaker(line: &str, cx: &Ctx) -> Option<String> {
    let name = line.strip_suffix(':')?;
    if name.is_empty() || name.contains(' ') {
        return None;
    }
    let known = name.eq_ignore_ascii_case(&cx.st.player)
        || cx.content.user(name).is_some()
        || name == "ROOT";
    known.then(|| name.to_string())
}

/// Render a whole markup block into terminal lines.
pub fn block(text: &str, cx: &Ctx) -> Vec<Line<'static>> {
    let text = text.strip_prefix('\n').unwrap_or(text);
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = subst(raw, cx);
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("@sprite ") {
            let mut it = rest.split_whitespace();
            let name = it.next().unwrap_or("");
            let tint = match it.next() {
                Some("dim") => Tint::Dim(0.45),
                Some("mono") => Tint::Mono((230, 228, 222)),
                Some("glitch") => Tint::Glitch,
                _ => Tint::None,
            };
            if let Some(s) = cx.content.sprites.get(name) {
                let indent = leading(&line);
                for l in s.lines(0, tint) {
                    out.push(indented(l, indent));
                }
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("@banner ") {
            let (color, words) = rest.split_once(' ').unwrap_or(("white", rest));
            let indent = leading(&line);
            for l in font::banner(words, theme::to_rgb(theme::color(color)), Tint::None) {
                out.push(indented(l, indent));
            }
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("@art ") {
            if let Some(art) = cx.content.art.get(name.trim()) {
                for l in art.lines() {
                    out.push(Line::from(spans(&subst(l, cx), theme::base())));
                }
            }
            continue;
        }
        if trimmed == "@rule" {
            out.push(Line::styled("─".repeat(60), Style::default().fg(theme::BORDER)));
            continue;
        }
        if let Some(name) = is_speaker(trimmed, cx) {
            out.push(Line::from(Span::styled(
                format!("{name}:"),
                theme::bold(user_color(&name, cx)),
            )));
            continue;
        }
        out.push(Line::from(spans(&line, Style::default().fg(theme::TEXT))));
    }
    out
}

fn leading(s: &str) -> usize {
    s.len() - s.trim_start().len()
}

fn indented(mut l: Line<'static>, n: usize) -> Line<'static> {
    if n > 0 {
        l.spans.insert(0, Span::raw(" ".repeat(n)));
    }
    l
}

/// Plain text of a line, for tests.
#[cfg(test)]
pub fn plain(l: &Line) -> String {
    l.spans.iter().map(|s| s.content.as_ref()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_nest_and_unknown_braces_stay() {
        let s = spans("a {red}b {b}c{/}{/} {nope} d", Style::default());
        let text: String = s.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "a b c {nope} d");
        assert_eq!(s[1].style.fg, Some(theme::RED));
        assert!(s[2].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn substitutes_player_and_words() {
        let content = Content::default();
        let mut st = GameState::new("neo");
        st.add("doors", 3);
        let meta = MetaState::default();
        let cx = Ctx { content: &content, st: &st, meta: &meta };
        assert_eq!(subst("hi {player}, {word:doors} {red}", &cx), "hi neo, three {red}");
    }
}
