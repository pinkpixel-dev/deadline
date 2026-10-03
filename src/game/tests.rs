//! Tests that drive the real game with the real story content.

use crate::app::App;
use crate::content::{Content, validate};
use crate::game::state::{GameState, MetaState};
use crate::game::text::plain;
use crate::output::Speed;
use crate::systems::save::Store;

pub(super) fn app() -> App {
    let content = Content::load().expect("story content loads");
    let mut a = App::new(content, GameState::new("tester"), MetaState::default(), None);
    a.speed = Speed::Instant;
    a
}

pub(super) fn run(a: &mut App, secs: f64) {
    let mut t = 0.0;
    while t < secs {
        a.tick(0.05);
        t += 0.05;
    }
}

pub(super) fn boot(a: &mut App) {
    a.start_sequence("boot");
    if let Some(s) = a.seq.as_mut() {
        s.fast = true;
    }
    run(a, 1.0);
    assert!(a.seq.is_none(), "boot sequence should finish");
    run(a, 0.5);
}

pub(super) fn cmd(a: &mut App, s: &str) {
    a.submit(s);
    run(a, 0.3);
}

pub(super) fn screen(a: &App) -> String {
    a.out.lines.iter().map(plain).collect::<Vec<_>>().join("\n")
}

/// Wait for a choice to appear and pick the option whose text contains `needle`.
pub(super) fn pick(a: &mut App, needle: &str) {
    for _ in 0..400 {
        if a.choice.is_some() {
            break;
        }
        a.tick(0.05);
    }
    let choice = a.choice.as_ref().unwrap_or_else(|| panic!("no choice for '{needle}'\n{}", screen(a)));
    let idx = choice
        .opts
        .iter()
        .position(|o| o.text.contains(needle))
        .unwrap_or_else(|| panic!("no option '{needle}' in {:?}", choice.opts.iter().map(|o| &o.text).collect::<Vec<_>>()));
    a.select_choice(idx);
    run(a, 0.2);
}

/// Let a conversation play out until it closes.
pub(super) fn finish_chat(a: &mut App) {
    for _ in 0..600 {
        if a.chat.is_none() {
            return;
        }
        a.tick(0.05);
    }
    panic!("chat never ended\n{}", screen(a));
}

/// Press Esc on overlays until they close.
pub(super) fn dismiss(a: &mut App) {
    for _ in 0..10 {
        if a.overlay.is_none() {
            return;
        }
        run(a, 0.4);
        a.dismiss_overlay();
    }
    assert!(a.overlay.is_none(), "overlay should close");
}

pub(super) fn opening(a: &mut App) {
    boot(a);
    cmd(a, "boards");
    cmd(a, "whoami");
    assert!(a.st.has("page:ghost_intro"), "ghost should page after a couple of commands");
    cmd(a, "reply");
    pick(a, "long enough for what");
    pick(a, "ok.");
    finish_chat(a);
    run(a, 0.5);
    cmd(a, "users");
    cmd(a, "reply");
    pick(a, "LAST LOGIN");
    pick(a, "i'm not");
    pick(a, "ok.");
    finish_chat(a);
    assert!(a.st.has("quest_nodelist"));
}

/// Play Act I on the close route (ghost_17 stays) through the end card.
pub(super) fn act1_close(a: &mut App) {
    opening(a);
    a.st.set("null_hint_mail");
    cmd(a, "mail -u ghost_17");
    a.submit("y");
    run(a, 0.5);
    cmd(a, "open 08");
    a.submit("LANTERN");
    run(a, 0.3);
    cmd(a, "open 08");
    cmd(a, "read 804");
    while a.chat.is_some() {
        a.end_chat(false);
        run(a, 0.2);
    }
    run(a, 8.0);
    pick(a, "i'm still here");
    finish_chat(a);
    run(a, 1.0);
    if let Some(s) = a.seq.as_mut() {
        s.fast = true;
    }
    run(a, 2.0);
    assert!(a.st.has("act1_complete"));
}

#[test]
fn story_content_is_valid() {
    let content = Content::load().expect("story content loads");
    let errors = validate::check(&content);
    assert!(errors.is_empty(), "content problems:\n{}", errors.join("\n"));
}

#[test]
fn boot_welcomes_and_plants_history() {
    let mut a = app();
    boot(&mut a);
    assert!(a.st.has("welcomed"));
    assert!(a.st.history.contains(&"logout".to_string()), "history should hold commands you never typed");
    assert!(a.st.inbox.contains(&"m_welcome".to_string()));
    assert!(screen(&a).contains("DON'T SHUT IT DOWN."));
}

#[test]
fn ghost_opening_tracks_trust_and_truth() {
    let mut a = app();
    opening(&mut a);
    assert_eq!(a.st.var("trust_ghost"), 4);
    assert_eq!(a.st.var("truth"), 2);
    assert!(screen(&a).contains("i don't remember making yours."));
}

#[test]
fn reading_posts_and_files_sets_flags() {
    let mut a = app();
    boot(&mut a);
    cmd(&mut a, "open 02");
    cmd(&mut a, "read 206");
    assert!(a.st.has("cmd:recover"), "eli's post teaches recover");
    cmd(&mut a, "cd uploads");
    cmd(&mut a, "view manual.txt");
    assert!(screen(&a).contains("CRC MISMATCH"));
    cmd(&mut a, "recover manual.txt");
    cmd(&mut a, "view manual.txt");
    assert!(a.st.has("knows_listen"));
    cmd(&mut a, "listen");
    assert!(screen(&a).contains("PASSIVE NODE CAPTURE ENABLED"));
}

#[test]
fn portrait_changes_and_checksum_notices() {
    let mut a = app();
    boot(&mut a);
    cmd(&mut a, "inspect /uploads/eli_portrait.ans");
    cmd(&mut a, "view /uploads/eli_portrait.ans");
    assert!(!screen(&a).contains("1978 - 1998"));
    a.st.set("eli_seen");
    cmd(&mut a, "clear");
    cmd(&mut a, "view /uploads/eli_portrait.ans");
    assert!(screen(&a).contains("1978 - 1998"), "the file changed");
    cmd(&mut a, "inspect /uploads/eli_portrait.ans");
    assert!(screen(&a).contains("checksum differs"));
}

#[test]
fn board_nine_vanishes_and_history_remembers() {
    let mut a = app();
    boot(&mut a);
    a.st.set("null_hint_09");
    cmd(&mut a, "open 09");
    assert!(a.overlay.is_some(), "ROOT objects");
    dismiss(&mut a);
    assert!(a.st.has("board09_gone"));
    cmd(&mut a, "open 09");
    assert!(screen(&a).contains("BOARD DOES NOT EXIST"));
    run(&mut a, 1.0);
    cmd(&mut a, "users");
    assert!(a.st.has("page:ghost_board9"), "ghost asks how you knew");
}

#[test]
fn betraying_ghost_to_root_opens_the_archive_route() {
    let mut a = app();
    opening(&mut a);
    cmd(&mut a, "download /uploads/nodelist.txt");
    assert!(a.st.has("got_nodelist"));
    dismiss(&mut a); // THAT FILE WAS NOT YOURS TO TAKE.
    run(&mut a, 0.5);
    cmd(&mut a, "users");
    dismiss(&mut a); // THIS SESSION IS BEING OBSERVED.
    run(&mut a, 0.3);
    pick(&mut a, "ghost_17 told me");
    pick(&mut a, "I'll stop");
    finish_chat(&mut a);
    assert!(a.st.has("told_root_ghost"));
    assert!(a.st.var("trust_root") >= 3);
    cmd(&mut a, "open 07");
    for _ in 0..5 {
        cmd(&mut a, "boards");
        if a.chat.is_some() {
            break;
        }
    }
    assert!(a.chat.as_ref().is_some_and(|c| c.id == "root_archive"), "ROOT offers the archive");
    pick(&mut a, "Why help me now");
    finish_chat(&mut a);
    assert!(a.st.has("archive_unlocked") && a.st.has("root_opened_archive"));
}

#[test]
fn reading_ghost_mail_unlocks_the_key_and_the_finale() {
    let mut a = app();
    opening(&mut a);
    a.st.set("null_hint_mail");
    cmd(&mut a, "mail -u ghost_17");
    assert!(a.choice.as_ref().is_some_and(|c| !c.from_chat));
    a.submit("y");
    run(&mut a, 0.5);
    assert!(a.st.has("read_ghost_mail"));
    assert!(a.st.has("archive_revealed"), "knowing the key reveals board 08");
    // ghost confronts you a few commands later
    for _ in 0..6 {
        cmd(&mut a, "boards");
    }
    assert!(a.st.has("page:ghost_mailread"));
    cmd(&mut a, "open 08");
    assert!(a.password.is_some());
    a.submit("LANTERN");
    run(&mut a, 0.3);
    assert!(a.st.has("archive_unlocked"));
    cmd(&mut a, "open 08");
    cmd(&mut a, "read 804");
    assert!(a.st.has("read_reveal"));
    assert!(screen(&a).contains("there will be."));
    while a.chat.is_some() {
        a.end_chat(false);
        run(&mut a, 0.2);
    }
    run(&mut a, 8.0);
    assert!(a.chat.as_ref().is_some_and(|c| c.id == "ghost_finale_close"), "high trust gets the warm finale");
    pick(&mut a, "i'm still here");
    finish_chat(&mut a);
    run(&mut a, 1.0);
    assert!(a.st.has("actend:1"), "act one ends");
    if let Some(s) = a.seq.as_mut() {
        s.fast = true;
    }
    run(&mut a, 2.0);
    assert!(a.st.has("act1_complete"));
    cmd(&mut a, "whoami");
    assert!(screen(&a).contains("07/22/1998"), "the account now has a creation date");
}

#[test]
fn restore_keeps_history_and_counts_in_meta() {
    let dir = std::env::temp_dir().join(format!("deadline-play-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::at(dir).unwrap();
    let content = Content::load().unwrap();
    let mut a = App::new(content, GameState::new("tester"), MetaState::default(), Some(store));
    a.speed = Speed::Instant;
    boot(&mut a);
    cmd(&mut a, "snapshot before");
    cmd(&mut a, "open 02");
    cmd(&mut a, "read 206");
    assert!(a.st.has("cmd:recover"));
    cmd(&mut a, "restore before");
    assert!(!a.st.has("cmd:recover"), "the timeline rolled back");
    assert!(a.st.history.contains(&"read 206".to_string()), "the terminal remembers");
    assert_eq!(a.meta.total_restores, 1);
    assert!(a.st.has("restored"));
}

#[test]
fn lists_are_selectable_by_keyboard_number_and_link() {
    let mut a = app();
    boot(&mut a);
    cmd(&mut a, "boards");
    assert!(a.menu.is_some(), "board list is a menu");
    assert!(a.out.links.iter().flatten().any(|l| l.cmd == "open 03"), "rows carry commands");
    // ↓ then Enter opens the first board
    assert!(a.menu_step(true));
    assert!(a.menu_activate());
    run(&mut a, 0.3);
    assert_eq!(a.st.board.as_deref(), Some("01"));
    // a bare post number reads it, and the post offers next/back
    cmd(&mut a, "102");
    assert!(a.st.has("read:102"));
    assert!(a.out.links.iter().flatten().any(|l| l.cmd == "read 103"), "next row");
    cmd(&mut a, "b");
    assert!(screen(&a).contains("GENERAL"));
    // ↑ from the top of a list hands the arrow back to history
    a.menu_step(true);
    assert!(a.menu_step(false));
    assert!(!a.menu_step(false));
}

#[test]
fn commands_forgive_missing_arguments() {
    let mut a = app();
    boot(&mut a);
    cmd(&mut a, "read");
    assert!(screen(&a).contains("MESSAGE BOARDS"), "read with no board shows boards");
    cmd(&mut a, "open trading");
    assert_eq!(a.st.board.as_deref(), Some("05"));
    cmd(&mut a, "open off");
    assert_eq!(a.st.board.as_deref(), Some("06"));
    cmd(&mut a, "view");
    assert!(screen(&a).contains("FILES"), "view with no file lists files");
    cmd(&mut a, "cd uploads");
    cmd(&mut a, "nodelist.txt");
    assert!(a.st.has("file:/uploads/nodelist.txt"), "a bare filename from the list opens it");
    assert!(a.out.links.iter().flatten().any(|l| l.cmd == "download /uploads/nodelist.txt"));
}

#[test]
fn a_wrong_archive_key_reminds_you_and_ghost_nudges() {
    let mut a = app();
    boot(&mut a);
    a.st.set("archive_revealed");
    a.st.set("knows_lantern");
    a.st.set_var("trust_ghost", 2);
    cmd(&mut a, "open 08");
    a.submit("lighthouse");
    run(&mut a, 0.3);
    assert!(screen(&a).contains("lantern"), "the prompt reminds you of the key you learned");
    for _ in 0..16 {
        cmd(&mut a, "boards");
    }
    assert!(a.st.has("page:ghost_nudge_key"), "ghost nudges if you stall");
    cmd(&mut a, "open 08");
    a.submit("lantern");
    run(&mut a, 0.3);
    assert!(a.st.has("archive_unlocked"));
}
