use serde::{Deserialize, Serialize};

use super::cond::Cond;

/// Something the story does to the world.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Effect {
    /// Set a session flag.
    Set(String),
    /// Clear a session flag.
    Unset(String),
    /// Add to a hidden variable (negative values subtract).
    Add(String, i32),
    /// Set a hidden variable outright.
    SetVar(String, i32),
    /// Record a persistent discovery that survives restores.
    Meta(String),
    /// Print a markup block into the main terminal.
    Print(String),
    /// Print a `***` style system notice into the main terminal.
    Notice(String),
    /// Write a line to the system log pane.
    Log(String),
    /// Deliver a mail message into the player's inbox.
    Mail(String),
    /// Queue a private message conversation the player can answer.
    Page(String),
    /// Open a conversation immediately.
    Chat(String),
    /// Show a modal overlay window.
    Overlay(String),
    /// Play a full-screen cinematic sequence.
    Sequence(String),
    /// Corrupt the screen for this many milliseconds.
    Glitch(u32),
    /// Inject a line into the command history the player never typed.
    History(String),
    /// Remove every history line matching this text.
    Forget(String),
    /// Ask the player something inline (Y/N style prompts).
    Choose(Choice),
    /// Ask for a password.
    Password(PasswordPrompt),
    /// Conditional effects.
    If(Cond, Vec<Effect>, Vec<Effect>),
    /// Finish an act.
    EndAct(u8),
    /// Save the session and hang up.
    Disconnect,
}

/// A choice offered to the player, inline or inside a conversation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Choice {
    pub prompt: String,
    pub options: Vec<ChoiceOpt>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ChoiceOpt {
    /// Optional hotkey such as "y" or "n". Numbers always work too.
    pub key: Option<String>,
    /// What the option says in the choice panel.
    pub text: String,
    /// What the player "types" into the chat when chosen. Defaults to `text`.
    pub say: Option<String>,
    pub cond: Cond,
    pub effects: Vec<Effect>,
    /// Conversation node to jump to.
    pub goto: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PasswordPrompt {
    pub prompt: String,
    /// Accepted answers, compared case-insensitively.
    pub answers: Vec<String>,
    pub ok: Vec<Effect>,
    pub fail: Vec<Effect>,
}

impl Effect {
    /// Visit this effect and every nested effect.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Effect)) {
        f(self);
        match self {
            Effect::If(_, a, b) => {
                a.iter().for_each(|e| e.walk(f));
                b.iter().for_each(|e| e.walk(f));
            }
            Effect::Choose(c) => c
                .options
                .iter()
                .flat_map(|o| o.effects.iter())
                .for_each(|e| e.walk(f)),
            Effect::Password(p) => {
                p.ok.iter().for_each(|e| e.walk(f));
                p.fail.iter().for_each(|e| e.walk(f));
            }
            _ => {}
        }
    }
}
