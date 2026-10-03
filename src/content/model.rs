use serde::{Deserialize, Serialize};

use super::cond::Cond;
use super::effect::{ChoiceOpt, Effect};

/// One story file. Every field is optional so any `.ron` file can hold any
/// mix of content. All files are merged at startup.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ContentFile {
    /// Events in this file only fire during this act's sessions (0 = any).
    pub act: u8,
    pub boards: Vec<Board>,
    pub posts: Vec<Post>,
    pub mail: Vec<Mail>,
    pub files: Vec<FileEntry>,
    pub dirs: Vec<Dir>,
    pub users: Vec<User>,
    pub dialogues: Vec<Dialogue>,
    pub events: Vec<Event>,
    pub hooks: Vec<Hook>,
    pub overlays: Vec<Overlay>,
    pub sequences: Vec<Sequence>,
    pub ambient: Vec<Ambient>,
    pub help: Vec<HelpEntry>,
}

/// A body that changes based on state. The first matching alt wins,
/// otherwise the base body is used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Alt {
    pub cond: Cond,
    pub body: String,
}

/// Something that blocks access until its condition holds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Lock {
    /// Access is granted while this holds.
    pub open: Cond,
    /// Effects run when access is denied (messages, password prompts).
    pub denied: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub visible: Cond,
    pub lock: Option<Lock>,
    /// Markup shown above the post list.
    pub header: String,
    pub on_open: Vec<Effect>,
    /// Shown in the board list instead of the real post count.
    pub count_label: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Post {
    pub id: u32,
    pub board: String,
    pub subject: String,
    pub date: String,
    pub author: String,
    pub visible: Cond,
    pub body: String,
    pub alt: Vec<Alt>,
    pub on_read: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Mail {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub body: String,
    pub alt: Vec<Alt>,
    pub on_read: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FileEntry {
    /// Full path such as `/uploads/eli_portrait.ans`.
    pub path: String,
    pub size: u32,
    pub date: String,
    pub uploader: String,
    pub desc: String,
    pub visible: Cond,
    pub lock: Option<Lock>,
    pub body: String,
    pub alt: Vec<Alt>,
    /// Shown scrambled until the `recovered:<path>` flag is set.
    pub corrupt: bool,
    pub can_delete: bool,
    pub on_open: Vec<Effect>,
    pub on_download: Vec<Effect>,
    pub on_delete: Vec<Effect>,
    pub on_inspect: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Dir {
    pub path: String,
    pub visible: Cond,
    pub lock: Option<Lock>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct User {
    pub id: String,
    /// Palette color name used for this user's text.
    pub color: String,
    /// Pixel portrait sprite id.
    pub sprite: Option<String>,
    /// First matching presence wins. No match means offline.
    pub presence: Vec<Presence>,
    /// `finger` output. First matching alt wins.
    pub finger: Vec<Alt>,
    /// Conversations this user can start when you `chat` them.
    pub chats: Vec<ChatRef>,
    /// What happens when there is nothing to talk about.
    pub idle: Vec<Alt>,
    /// Shown in the `users` directory listing.
    pub listed: Cond,
    pub last_login: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Presence {
    pub cond: Cond,
    /// `online`, `idle`, `away` or `hidden`.
    pub status: String,
    pub node: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ChatRef {
    pub cond: Cond,
    pub dialogue: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Dialogue {
    pub id: String,
    /// Default speaker for lines.
    pub with: String,
    pub nodes: Vec<Node>,
    /// Effects when the player leaves before the conversation ends.
    pub on_leave: Vec<Effect>,
    /// Effects when the conversation reaches an end node.
    pub on_end: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Node {
    pub id: String,
    /// Lines in order. `@name: text` changes speaker, `#text` is a system line.
    pub lines: Vec<String>,
    pub effects: Vec<Effect>,
    pub choices: Vec<ChoiceOpt>,
    /// Automatic branches, checked before `next`.
    pub branch: Vec<Branch>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Branch {
    pub cond: Cond,
    pub goto: String,
}

/// Fires when `when` holds, after the optional delays have passed.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Event {
    pub id: String,
    /// Only fire while this act is the one in play (0 = any). Defaults to the file's `act`.
    pub act: u8,
    pub when: Cond,
    pub after_cmds: u32,
    pub after_secs: u32,
    pub repeat: bool,
    pub effects: Vec<Effect>,
}

/// A scripted command. Checked before built-in commands.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Hook {
    /// Patterns compared against the normalized input. A trailing `*`
    /// matches any remainder.
    pub input: Vec<String>,
    pub cond: Cond,
    pub effects: Vec<Effect>,
    /// When false the built-in command still runs afterwards.
    pub consume: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Overlay {
    pub id: String,
    pub title: String,
    pub body: String,
    /// `root`, `system` or `janus`.
    pub style: String,
    /// How many Esc presses are ignored before it closes.
    pub hold: u32,
    /// Lines shown, in order, each time Esc is ignored.
    pub hold_text: Vec<String>,
    pub on_close: Vec<Effect>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Sequence {
    pub id: String,
    pub steps: Vec<Step>,
    pub on_end: Vec<Effect>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Step {
    /// Print a markup line instantly.
    Line(String),
    /// Type a line out character by character.
    Type(String),
    /// Show a prompt instantly, pause, then type the rest: (prompt, typed).
    TypeAfter(String, String),
    Pause(u32),
    Clear,
    Glitch(u32),
    /// Show a pixel sprite centered.
    Sprite(String),
    /// Big pixel-font text: (color, text).
    Banner(String, String),
    /// Only run the nested steps when the condition holds.
    When(Cond, Vec<Step>),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Ambient {
    pub cond: Cond,
    pub text: String,
    /// Also print into the main terminal, not just the log.
    pub loud: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HelpEntry {
    pub cmd: String,
    pub desc: String,
    pub cond: Cond,
}
