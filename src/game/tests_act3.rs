//! Act III playthroughs, starting from a finished Act II.

use super::tests::{app, cmd, finish_chat, pick, run, screen};
use super::tests_act2::{answer, next_login};
use crate::app::App;
use crate::systems::clock;

/// A session that just finished Act II on the close route, then logs back in.
fn act3(a: &mut App) {
    for f in ["actend:1", "act:2", "route:close", "archive_revealed", "archive_unlocked", "archive_mounted", "actend:2", "act2_complete", "locked"] {
        a.st.set(f);
    }
    a.st.act = 3;
    a.st.act_started = 2;
    next_login(a);
}

#[test]
fn act_three_opens_on_a_live_1998_night() {
    let mut a = app();
    act3(&mut a);
    assert!(a.st.has("y1998"));
    assert!(!a.st.has("locked"), "a ROOT lockout from act two doesn't follow you into 1998");
    assert_eq!(clock::date_string(&a.st), "08/14/1998");
    assert!(clock::time_string(&a.st).starts_with("09:0"), "the night starts around 9pm: {}", clock::time_string(&a.st));
    assert!(screen(&a).contains("MEETUP TONIGHT"));

    cmd(&mut a, "users");
    let s = screen(&a);
    for who in ["mara", "eli", "ghost_17", "kestrel", "parallax", "crankshaft"] {
        assert!(s.contains(who), "{who} should be online in 1998");
    }
    cmd(&mut a, "boards");
    assert!(!screen(&a).contains("ARCHIVE"), "the archive board doesn't exist yet in 1998");
    cmd(&mut a, "read 607");
    assert!(!screen(&a).contains("save me a seat"), "present-day replies are hidden");
    cmd(&mut a, "files /archive");
    assert!(screen(&a).contains("no such directory"));
}

#[test]
fn the_current_acts_hooks_win() {
    let mut a = app();
    act3(&mut a);
    cmd(&mut a, "trace eli");
    assert!(screen(&a).contains("eli's apartment"), "act three's trace runs before act one's:\n{}", screen(&a));
    cmd(&mut a, "trace crankshaft");
    assert!(screen(&a).contains("no such node"));
}

#[test]
fn warning_eli_makes_the_night_slip() {
    let mut a = app();
    act3(&mut a);
    answer(&mut a, "ghost_98_hello");
    pick(&mut a, "it's me");
    pick(&mut a, "why would you ask");
    finish_chat(&mut a);
    answer(&mut a, "eli_98_hello");
    pick(&mut a, "how do you run it");
    finish_chat(&mut a);
    assert!(a.st.has("eli_plans_run") && a.st.has("knows_door"));

    cmd(&mut a, "chat eli");
    pick(&mut a, "just saying hi");
    finish_chat(&mut a);
    cmd(&mut a, "chat eli");
    assert!(a.chat.as_ref().is_some_and(|c| c.id == "eli_98_more"), "eli stays chatty until you warn him");
    pick(&mut a, "don't run the archiver");
    pick(&mut a, "you haven't slept");
    finish_chat(&mut a);
    assert!(a.st.has("warned_eli"));

    cmd(&mut a, "users");
    cmd(&mut a, "users");
    assert!(a.st.has("night_slips"));
    assert_eq!(a.st.var("build"), 2);
    assert!(a.st.has("show_build"), "the header shows the build number");
    run(&mut a, 6.0);
    assert!(!a.st.has("show_build"), "only for a moment");
}

#[test]
fn the_diner_skips_and_keeping_eli_talking_gets_him_to_bed() {
    let mut a = app();
    act3(&mut a);
    a.st.set("diner_go");
    cmd(&mut a, "users");
    cmd(&mut a, "users");
    assert!(screen(&a).contains("diner. board goes into maintenance"));
    run(&mut a, 46.0);
    assert!(a.st.has("at_diner"));
    assert!(a.presence("mara").is_none() && a.presence("eli").is_none(), "everyone leaves for the diner");
    assert!(clock::time_string(&a.st).starts_with("10:0"));

    run(&mut a, 111.0);
    assert!(a.st.has("back_from_diner"));
    assert!(clock::time_string(&a.st).starts_with("11:4"), "the board skips the diner: {}", clock::time_string(&a.st));
    assert!(screen(&a).contains("no records 22:00 - 23:41"));

    answer(&mut a, "eli_98_last");
    pick(&mut a, "talk to me");
    pick(&mut a, "it's quiet");
    pick(&mut a, "comics");
    pick(&mut a, "keep going");
    pick(&mut a, "skip the run");
    finish_chat(&mut a);
    assert!(a.st.has("stalled_eli"));
    assert!(a.presence("eli").is_none(), "eli logs off to sleep");
    assert!(clock::time_string(&a.st).starts_with("03:5"), "four hours went by: {}", clock::time_string(&a.st));
}

#[test]
fn eli_s_back_door_can_kill_the_indexer() {
    let mut a = app();
    act3(&mut a);
    cmd(&mut a, "open 04 --node");
    assert!(screen(&a).contains("ARCHIVER CONSOLE"));
    cmd(&mut a, "y");
    assert!(a.st.has("sabotaged_node4"));
    cmd(&mut a, "open 04 --node");
    assert!(screen(&a).contains("console is just gone"));
}

#[test]
fn reply_by_name_and_send_a_downloaded_file() {
    let mut a = app();
    act3(&mut a);
    cmd(&mut a, "send eli modem_faq.txt");
    assert!(screen(&a).contains("not in local storage"));
    cmd(&mut a, "download /uploads/modem_faq.txt");
    cmd(&mut a, "send eli modem_faq.txt");
    assert!(a.st.has("sent:eli:/uploads/modem_faq.txt"), "{}", screen(&a));
    cmd(&mut a, "send nobody modem_faq.txt");
    assert!(screen(&a).contains("no such user"));
    cmd(&mut a, "send eli");
    assert!(screen(&a).contains("send <user> <file>"));

    cmd(&mut a, "reply kestrel");
    assert!(a.chat.as_ref().is_some_and(|c| c.id == "kestrel_98"), "reply <user> opens a channel with them");
}
