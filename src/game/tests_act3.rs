//! Act III playthroughs, starting from a finished Act II.

use super::tests::{app, cmd, finish_chat, pick, run, screen};
use super::tests_act2::{answer, next_login, skip};
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

    rebuilt(&mut a);
    assert_eq!(a.st.var("build"), 2);
    assert!(a.st.has("night_slips") && a.st.has("warned_eli"), "what you did carries into the next build");
    assert!(a.st.has("show_build"), "the header shows the build number");
    run(&mut a, 6.0);
    assert!(!a.st.has("show_build"), "only for a moment");
}

/// Keep typing until the night rebuilds, then let the cinematic play out.
fn rebuilt(a: &mut App) {
    for _ in 0..5 {
        if a.seq.is_some() {
            break;
        }
        cmd(a, "users");
    }
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "a3_rebuild"), "the night rebuilds
{}", screen(a));
    skip(a);
    run(a, 0.5);
}

/// Wait for dawn and eli's question, and read it.
fn dawn(a: &mut App) {
    run(a, 26.0);
    assert!(a.st.has("dawn"), "dawn comes\n{}", screen(a));
    assert_eq!(clock::date_string(&a.st), "08/15/1998");
    run(a, 9.0);
    answer(a, "eli_98_news");
    finish_chat(a);
}

#[test]
fn three_builds_then_the_newspaper_then_the_collapse() {
    let mut a = app();
    act3(&mut a);
    a.st.set("heard_wipe");
    cmd(&mut a, "open 04 --node");
    cmd(&mut a, "y");
    rebuilt(&mut a);

    // BUILD 1998.2: back to 9:04, and the cracks show.
    assert_eq!(a.st.var("build"), 2);
    assert!(clock::time_string(&a.st).starts_with("09:04"), "{}", clock::time_string(&a.st));
    assert!(a.st.has("sabotaged_node4") && !a.st.has("changed_night"));
    assert!(screen(&a).contains("(AGAIN.)"));
    answer(&mut a, "ghost_98_again");
    pick(&mut a, "because it is");
    finish_chat(&mut a);
    assert!(a.st.has("heard_there_will_be"));
    cmd(&mut a, "scan");
    assert!(screen(&a).contains("JANUS"), "node 04 shows up as JANUS now");
    cmd(&mut a, "inspect /uploads/diner_0814.gif");
    assert!(screen(&a).contains("hasn't happened yet"));
    answer(&mut a, "eli_98_again");
    finish_chat(&mut a);
    answer(&mut a, "mara_98_again");
    finish_chat(&mut a);
    cmd(&mut a, "chat mara");
    pick(&mut a, "something happens to eli");
    pick(&mut a, "hasn't slept");
    finish_chat(&mut a);
    rebuilt(&mut a);

    // BUILD 1998.3: the last one. Changing anything now just brings dawn.
    assert_eq!(a.st.var("build"), 3);
    assert!(a.st.has("told_mara") && a.st.has("sabotaged_node4"));
    answer(&mut a, "ghost_98_loop");
    pick(&mut a, "three");
    finish_chat(&mut a);
    answer(&mut a, "eli_98_again");
    finish_chat(&mut a);
    cmd(&mut a, "chat eli");
    pick(&mut a, "don't run the archiver");
    pick(&mut a, "trust me");
    finish_chat(&mut a);
    cmd(&mut a, "users");
    cmd(&mut a, "users");
    assert!(a.seq.is_none(), "no fourth build");
    dawn(&mut a);

    cmd(&mut a, "reply eli");
    pick(&mut a, "You died");
    finish_chat(&mut a);
    assert!(a.st.has("eli_told_death") && a.st.has("eli_decided"));
    run(&mut a, 11.0);
    answer(&mut a, "parallax_breakin");
    pick(&mut a, "Both can be true");
    finish_chat(&mut a);
    run(&mut a, 7.0);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "a3_collapse"));
    skip(&mut a);
    assert!(a.st.has("seq:act3_end") || a.st.has("act3_complete"), "the act ends after the collapse");
    skip(&mut a);
    assert!(a.st.has("act3_complete"));
    assert_eq!(a.st.act, 4);
    assert!(!a.st.has("y1998"), "back in the present");
    assert!(clock::time_string(&a.st).starts_with("03:17"));

    cmd(&mut a, "view /var/janus/builds.log");
    let s = screen(&a);
    assert!(s.contains("1998.3") && s.contains("informed subject mara.q") && s.contains("terminated indexer"), "{s}");
}

#[test]
fn letting_eli_go_brings_dawn_and_you_can_send_him_the_article() {
    let mut a = app();
    act3(&mut a);
    a.st.set("eli_running");
    dawn(&mut a);
    assert_eq!(a.st.var("build"), 1, "no rebuilds if you never change anything");
    cmd(&mut a, "download /archive/news/eli_voss.txt");
    cmd(&mut a, "send eli eli_voss.txt");
    run(&mut a, 0.5);
    assert!(a.st.has("eli_article_sent"));
    assert!(a.st.has("chat:eli_98_reads"), "eli reads it himself");
}

#[test]
fn deleting_the_article_or_hanging_up_also_answer_eli() {
    let mut a = app();
    act3(&mut a);
    a.st.set("eli_running");
    dawn(&mut a);
    cmd(&mut a, "delete /archive/news/eli_voss.txt");
    cmd(&mut a, "y");
    assert!(a.st.has("eli_article_deleted") && a.st.has("eli_decided"));

    let mut a = app();
    act3(&mut a);
    a.st.set("eli_running");
    dawn(&mut a);
    cmd(&mut a, "logout");
    assert!(a.st.has("fled_eli"));
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "a3_redial"), "the modem dials back");
    skip(&mut a);
    assert!(!a.quit, "you don't actually get to leave");
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
