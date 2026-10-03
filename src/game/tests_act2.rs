//! Act II playthroughs, starting from a finished Act I.

use super::tests::{act1_close, app, cmd, dismiss, finish_chat, pick, run, screen};
use crate::app::App;

/// Skip a running cinematic.
pub(super) fn skip(a: &mut App) {
    for _ in 0..20 {
        match a.seq.as_mut() {
            Some(s) => s.fast = true,
            None => return,
        }
        run(a, 0.5);
    }
}

/// Open a waiting private message by dialogue id.
pub(super) fn answer(a: &mut App, id: &str) {
    for _ in 0..40 {
        if a.st.pages.iter().any(|p| p == id) {
            break;
        }
        cmd(a, "users");
    }
    assert!(a.st.pages.iter().any(|p| p == id), "expected a page for {id}, have {:?}", a.st.pages);
    if a.chat.is_some() {
        a.end_chat(false);
    }
    a.start_chat(id);
    run(a, 0.2);
}

/// Log off and back on, the way a real next session starts.
pub(super) fn next_login(a: &mut App) {
    skip(a);
    a.st.begin_act();
    a.start_sequence("reconnect");
    skip(a);
    run(a, 1.0);
    skip(a);
}

#[test]
fn act_two_waits_for_the_next_login_and_root_kicks_you_off() {
    let mut a = app();
    act1_close(&mut a);
    assert_eq!(a.st.act, 2);
    assert!(!a.st.has("act:2"), "act two doesn't start in the same session");
    run(&mut a, 245.0);
    assert!(screen(&a).contains("SESSION LIMIT"), "ROOT warns before hanging up");
    run(&mut a, 60.0);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "kick_1"), "ROOT hangs up after five minutes");
    skip(&mut a);
    assert!(a.quit, "the kick disconnects");
}

#[test]
fn act_two_close_route_plays_to_the_1998_cliffhanger() {
    let mut a = app();
    act1_close(&mut a);
    a.quit = false;
    next_login(&mut a);
    assert!(a.st.has("act:2"));
    assert!(a.st.has("route:close"), "ghost stayed, so ghost guides act two");
    assert!(screen(&a).contains("THE ARCHIVE IS MOUNTED"));

    cmd(&mut a, "boards");
    answer(&mut a, "a2_ghost_close");
    pick(&mut a, "i'm here");
    pick(&mut a, "i'll talk to him");
    finish_chat(&mut a);

    answer(&mut a, "eli_a2_hello");
    pick(&mut a, "reading the archive");
    finish_chat(&mut a);

    cmd(&mut a, "open 08");
    cmd(&mut a, "read 806");
    answer(&mut a, "eli_a2_drive");
    pick(&mut a, "maybe it's not a bug");
    finish_chat(&mut a);

    cmd(&mut a, "read 808");
    assert!(a.st.has("found_consent"));
    answer(&mut a, "parallax_call");
    pick(&mut a, "How do I know");
    pick(&mut a, "I'll read it first");
    finish_chat(&mut a);
    assert!(a.st.has("quest_consent"));

    cmd(&mut a, "read 810");
    cmd(&mut a, "read 812");
    assert!(a.st.has("janus_named"));
    answer(&mut a, "eli_a2_doubt");
    pick(&mut a, "isn't 1998");
    pick(&mut a, "it's");
    finish_chat(&mut a);
    assert!(a.st.has("eli_told_year"));
    assert!(a.st.has("eli_dropped"), "eli logs off to think");

    cmd(&mut a, "download /archive/consent.log");
    for _ in 0..40 {
        if a.chat.as_ref().is_some_and(|c| c.id == "root_consent") {
            break;
        }
        run(&mut a, 0.2);
        if a.chat.is_some() && a.chat.as_ref().is_some_and(|c| c.id != "root_consent") {
            a.end_chat(false);
        }
    }
    assert!(
        a.chat.as_ref().is_some_and(|c| c.id == "root_consent"),
        "ROOT notices the copy. chat={:?} overlay={:?} queue={:?} locked={} alert={} has_log={}\n{}",
        a.chat.as_ref().map(|c| c.id.clone()),
        a.overlay.as_ref().map(|o| o.id.clone()),
        a.queue,
        a.st.has("locked"),
        a.st.var("root_alert"),
        a.st.has("has_consent_log"),
        screen(&a).lines().rev().take(12).collect::<Vec<_>>().join("\n")
    );
    pick(&mut a, "Parallax.");
    pick(&mut a, "haven't decided");
    finish_chat(&mut a);
    cmd(&mut a, "send parallax consent.log");
    assert!(a.st.has("gave_parallax_log"));

    while a.chat.is_some() {
        a.end_chat(false);
        run(&mut a, 0.2);
    }
    cmd(&mut a, "open 08");
    cmd(&mut a, "read 815");
    assert!(a.st.has("read_fragment"), "the recovered fragment is readable");
    while a.chat.is_some() {
        a.end_chat(false);
        run(&mut a, 0.2);
    }
    run(&mut a, 7.0);
    dismiss(&mut a);
    run(&mut a, 1.0);
    assert!(
        a.chat.as_ref().is_some_and(|c| c.id == "a2_finale_ghost"),
        "chat={:?} overlay={:?} queue={:?}",
        a.chat.as_ref().map(|c| c.id.clone()),
        a.overlay.as_ref().map(|o| o.id.clone()),
        a.queue
    );
    pick(&mut a, "maybe i was");
    finish_chat(&mut a);
    run(&mut a, 1.0);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "act2_end"));
    skip(&mut a);
    assert!(a.st.has("act2_complete"));
    assert_eq!(a.st.act, 3);
}

#[test]
fn act_one_endings_pick_the_act_two_route() {
    let content = crate::content::Content::load().unwrap();
    let open = content.events.iter().find(|e| e.id == "a2_open").unwrap().effects.clone();
    let cases: &[(&[&str], i32, &str)] = &[
        (&[], 3, "route:close"),
        (&["told_root_ghost", "root_opened_archive"], 3, "route:root"),
        (&["forced_archive"], 0, "route:null"),
        (&[], 0, "route:cold"),
    ];
    for (flags, trust, want) in cases {
        let mut a = app();
        for f in *flags {
            a.st.set(f);
        }
        a.st.set_var("trust_ghost", *trust);
        a.apply(&open);
        assert!(a.st.has(want), "{flags:?} trust {trust} should give {want}");
    }
}

#[test]
fn both_root_lockouts_lift_after_a_while() {
    let mut a = app();
    a.st.act = 2;
    a.st.act_started = 2;
    a.st.set("act:2");
    a.st.fired.insert("a2_open".into());
    for (alert, waits) in [(7, 6), (12, 10)] {
        a.st.set_var("root_alert", alert);
        cmd(&mut a, "users");
        cmd(&mut a, "users");
        dismiss(&mut a);
        assert!(a.st.has("locked"), "ROOT locks you at root_alert {alert}");
        cmd(&mut a, "files");
        assert!(screen(&a).contains("try again later"), "the lockout says it is temporary");
        for _ in 0..waits {
            cmd(&mut a, "users");
        }
        assert!(!a.st.has("locked"), "the lockout at root_alert {alert} lifts");
    }
    assert!(a.st.has("lock2_done"));
}

#[test]
fn null_errands_unlock_in_order() {
    let mut a = app();
    a.st.set("act:2");
    a.st.fired.insert("a2_open".into());
    a.st.set("janus_src_visible");
    a.st.set("cmd:recover");
    cmd(&mut a, "open 07 --node");
    assert!(screen(&a).contains("KEY REQUIRED"), "screen:\n{}", screen(&a));
    cmd(&mut a, "recover /janus/src/seal.dat");
    cmd(&mut a, "open 07 --node");
    assert!(a.st.has("node07_open"));
    assert!(a.presence("null").is_some_and(|p| p.status == "online"), "null shows up on node 07");
    a.st.set("knows_shadow");
    cmd(&mut a, "shadow eli");
    assert!(a.st.has("shadowed_eli"));
    assert!(a.st.var("privacy") < 0);
}
