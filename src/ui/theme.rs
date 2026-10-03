use ratatui::style::{Color, Modifier, Style};

/// Design tokens. Every color in the UI comes from here.
pub const BG: Color = Color::Rgb(22, 23, 27);
pub const PANEL: Color = Color::Rgb(27, 28, 33);
pub const BORDER: Color = Color::Rgb(56, 58, 67);
pub const BORDER_HI: Color = Color::Rgb(96, 99, 112);
pub const TEXT: Color = Color::Rgb(233, 231, 226);
pub const DIM: Color = Color::Rgb(128, 130, 142);
pub const FAINT: Color = Color::Rgb(78, 80, 90);
pub const WHITE: Color = Color::Rgb(246, 244, 239);
pub const CYAN: Color = Color::Rgb(94, 214, 214);
pub const MAGENTA: Color = Color::Rgb(214, 108, 200);
pub const AMBER: Color = Color::Rgb(255, 184, 82);
pub const RED: Color = Color::Rgb(240, 92, 88);
pub const GREEN: Color = Color::Rgb(132, 214, 128);
pub const BLUE: Color = Color::Rgb(108, 142, 250);
pub const VIOLET: Color = Color::Rgb(170, 142, 240);
pub const YELLOW: Color = Color::Rgb(240, 222, 112);

/// Look up a palette color by the name used in story markup.
pub fn named(name: &str) -> Option<Color> {
    Some(match name {
        "text" => TEXT,
        "dim" => DIM,
        "faint" => FAINT,
        "white" => WHITE,
        "cyan" => CYAN,
        "magenta" => MAGENTA,
        "amber" => AMBER,
        "red" => RED,
        "green" => GREEN,
        "blue" => BLUE,
        "violet" => VIOLET,
        "yellow" => YELLOW,
        "border" => BORDER_HI,
        _ => return None,
    })
}

pub fn color(name: &str) -> Color {
    named(name).unwrap_or(TEXT)
}

pub fn base() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn dim() -> Style {
    Style::default().fg(DIM)
}

pub fn bold(c: Color) -> Style {
    Style::default().fg(c).add_modifier(Modifier::BOLD)
}

/// Darken an RGB color, used for shadows and dimmed sprites.
pub fn shade(c: (u8, u8, u8), f: f32) -> (u8, u8, u8) {
    let m = |v: u8| ((v as f32) * f).clamp(0.0, 255.0) as u8;
    (m(c.0), m(c.1), m(c.2))
}

pub fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

pub fn to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (233, 231, 226),
    }
}
