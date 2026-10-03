//! Act IV playthroughs, starting from a finished Act III.

use super::tests::{app, cmd, finish_chat, pick, run, screen};
use super::tests_act2::{answer, next_login, skip};
use crate::app::App;
use crate::systems::save::Store;

/// A session that just finished Act III (close route, told Eli the truth),
/// with real storage so JANUS can write snapshots, logging back in.
fn act4(tag: &str) -> App {
    let mut a = app();
    let dir = std::env::temp_dir().join(format!("deadline-act4-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    a.store = Some(Store::at(dir).expect("temp store"));
    for f in [
        "actend:1", "act:2", "route:close", "archive_revealed", "archive_unlocked", "archive_mounted", "actend:2",
        "act2_complete", "act:3", "actend:3", "act3_complete", "var_janus", "eli_told_death", "eli_decided", "promised_root",
    ] {
        a.st.set(f);
    }
    a.st.set_var("trust_ghost", 2);
    a.st.act = 4;
    a.st.act_started = 3;
    next_login(&mut a);
    a
}

fn see_my_model(a: &mut App) {
    cmd(a, "inspect /var/janus/models/player_04");
    assert!(a.st.has("saw_player_model"));
}

#[test]
fn act_four_opens_on_the_models_folder() {
    let mut a = act4("open");
    assert!(a.st.has("act:4") && a.st.has("a4_bulletin"));
    assert!(screen(&a).contains("/var/janus IS NOT A PUBLIC DIRECTORY"));

    cmd(&mut a, "files /var/janus/models");
    let s = screen(&a);
    for m in ["mara.q", "eli.v", "ghost17", "parallax.partial", "player_04"] {
        assert!(s.contains(m), "{m} should be in the models folder\n{s}");
    }
    see_my_model(&mut a);
    let s = screen(&a);
    assert!(s.contains("MEMORY SEED") && s.contains("synthetic") && s.contains("91.7%") && s.contains("CONNECTED"), "{s}");
    cmd(&mut a, "whoami");
    assert!(screen(&a).contains("player_04"));
    cmd(&mut a, "view /var/janus/models/eli.v");
    assert!(screen(&a).contains("aware of 08/15/1998"), "eli.v reflects telling him in act three");
    cmd(&mut a, "reply janus");
    assert!(screen(&a).contains("Not yet."), "JANUS doesn't talk until the end of the act");
}

#[test]
fn telling_ghost_or_hiding_their_model() {
    let mut a = act4("ghost-tell");
    answer(&mut a, "ghost_a4_hello");
    pick(&mut a, "i'll look");
    finish_chat(&mut a);
    cmd(&mut a, "view /var/janus/models/ghost17");
    answer(&mut a, "ghost_a4_ask");
    pick(&mut a, "there's one called ghost17");
    finish_chat(&mut a);
    assert!(a.st.has("told_ghost_model"));
    assert!(!a.st.has("ghost_blames_you"), "ghost trusts you enough not to blame you");

    let mut a = act4("ghost-hide");
    answer(&mut a, "ghost_a4_hello");
    pick(&mut a, "i'll look");
    finish_chat(&mut a);
    cmd(&mut a, "view /var/janus/models/ghost17");
    answer(&mut a, "ghost_a4_ask");
    pick(&mut a, "nothing about you");
    finish_chat(&mut a);
    let trust = a.st.var("trust_ghost");
    answer(&mut a, "ghost_a4_found");
    pick(&mut a, "didn't know how");
    finish_chat(&mut a);
    assert!(a.st.has("ghost_found_out"), "they find out anyway");
    assert!(a.st.var("trust_ghost") < trust);
}

#[test]
fn janus_plants_before_you_and_restoring_it_sends_you_back() {
    let mut a = act4("plant");
    see_my_model(&mut a);
    answer(&mut a, "parallax_a4");
    pick(&mut a, "how do I know");
    finish_chat(&mut a);
    assert!(a.st.has("parallax_proof") && a.st.var("explanations") == 1);
    for _ in 0..4 {
        cmd(&mut a, "users");
    }
    assert!(a.st.has("before_you_planted"));
    assert!(a.st.notes.iter().any(|(_, n)| n.contains("restore before_you")), "JANUS writes in your journal");
    cmd(&mut a, "snapshots");
    assert!(screen(&a).contains("before_you"), "a snapshot you never made");

    if a.chat.is_some() {
        a.end_chat(false);
    }
    let paranoia = a.st.var("paranoia");
    cmd(&mut a, "restore before_you");
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "a4_alt"), "the other timeline plays\n{}", screen(&a));
    skip(&mut a);
    assert!(a.st.has("planted") && a.st.has("alt_ghost_gone"), "close route flips to the night you gave ghost up");
    assert_eq!(a.st.var("paranoia"), paranoia, "it's rebuilt from who you are now");
    cmd(&mut a, "finger ghost_17");
    assert!(screen(&a).contains("USER DOES NOT EXIST"));
    cmd(&mut a, "snapshot mine");
    assert!(screen(&a).contains("isn't yours to keep"));

    answer(&mut a, "root_alt");
    pick(&mut a, "Why are you");
    finish_chat(&mut a);
    run(&mut a, 0.5);
    skip(&mut a);
    assert!(!a.st.has("planted") && !a.st.has("alt_timeline"), "back in your own timeline");
    assert!(a.st.has("saw_before_you") && a.st.has("before_you_planted") && a.st.has("parallax_proof"));
    assert_eq!(a.meta.total_restores, 1);
    assert!(screen(&a).contains("you didn't type restore"));
}

#[test]
fn restoring_before_you_from_inside_it_still_leads_home() {
    let mut a = act4("plant-twice");
    see_my_model(&mut a);
    answer(&mut a, "parallax_a4");
    pick(&mut a, "how do I know");
    finish_chat(&mut a);
    for _ in 0..4 {
        cmd(&mut a, "users");
    }
    assert!(a.st.has("before_you_planted"));
    if a.chat.is_some() {
        a.end_chat(false);
    }
    cmd(&mut a, "restore before_you");
    skip(&mut a);
    cmd(&mut a, "restore before_you");
    skip(&mut a);
    assert!(a.st.has("planted"), "still inside the replay");

    answer(&mut a, "root_alt");
    pick(&mut a, "Why are you");
    finish_chat(&mut a);
    run(&mut a, 0.5);
    skip(&mut a);
    assert!(!a.st.has("planted") && !a.st.has("alt_timeline"), "back in your own timeline, not the replay");
    assert!(a.st.has("parallax_proof"));
}

#[test]
fn act_four_autosaves_without_a_logout() {
    let mut a = act4("autosave");
    for _ in 0..10 {
        cmd(&mut a, "users");
    }
    let saved = a.store.as_ref().and_then(|s| s.session()).expect("autosaved session");
    assert_eq!(saved.act_started, 4, "closing the window mid-act keeps the act");
    assert_eq!(saved.commands % 10, 0);
}

#[test]
fn the_local_clock_is_parallax_s_proof_and_null_gets_traced() {
    let mut a = act4("proof");
    see_my_model(&mut a);
    answer(&mut a, "parallax_a4");
    pick(&mut a, "Why do you care");
    finish_chat(&mut a);
    cmd(&mut a, "date --local");
    assert!(screen(&a).contains("LOCAL CLOCK") && a.st.has("checked_local_clock"));
    answer(&mut a, "parallax_a4_proof");
    pick(&mut a, "I'm real");
    finish_chat(&mut a);
    assert!(a.st.has("parallax_evidence"));

    answer(&mut a, "null_a4");
    pick(&mut a, "how many times");
    finish_chat(&mut a);
    cmd(&mut a, "trace null");
    assert!(screen(&a).contains("escape.o") && a.st.has("null_exposed"), "{}", screen(&a));
    answer(&mut a, "null_a4_exposed");
    pick(&mut a, "maybe");
    finish_chat(&mut a);
    assert!(a.st.has("confronted_null"));
}

#[test]
fn three_theories_then_janus_asks_and_the_act_ends() {
    let mut a = act4("end");
    see_my_model(&mut a);
    answer(&mut a, "parallax_a4");
    pick(&mut a, "how do I know");
    finish_chat(&mut a);
    answer(&mut a, "null_a4");
    pick(&mut a, "you're lying");
    finish_chat(&mut a);
    answer(&mut a, "root_a4");
    pick(&mut a, "What are you");
    finish_chat(&mut a);
    assert!(!a.st.has("mara_revealed"), "ROOT only slips if it trusts you");
    assert_eq!(a.st.var("explanations"), 3);
    assert!(screen(&a).contains("most human thing"), "ROOT remembers your act one promise");

    for _ in 0..4 {
        if a.st.has("janus_ready") {
            break;
        }
        cmd(&mut a, "users");
    }
    run(&mut a, 5.0);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "a4_janus"), "JANUS speaks\n{}", screen(&a));
    skip(&mut a);
    if a.chat.is_some() {
        a.end_chat(false);
    }
    cmd(&mut a, "reply janus");
    assert!(a.chat.as_ref().is_some_and(|c| c.id == "janus_ask"));
    pick(&mut a, "A person");
    finish_chat(&mut a);
    assert!(a.st.has("believes_human"));
    assert_eq!(a.st.act, 5);
    skip(&mut a);
    assert!(a.st.has("act4_complete"));
    run(&mut a, 305.0);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "kick_4"), "ROOT hangs up after the act");
}

#[test]
fn act_two_ambient_lines_stay_in_act_two() {
    let mut a = act4("ambient");
    a.st.set("node07_open");
    for _ in 0..300 {
        a.next_ambient = 0.0;
        a.ambient();
    }
    let log: Vec<String> = a.log.iter().map(crate::game::text::plain).collect();
    assert!(log.iter().any(|l| l.contains("5 models loaded")), "act four's own lines still show up");
    for stale in ["indexing 06/1998", "archive mount verified", "something is typing"] {
        assert!(!log.iter().any(|l| l.contains(stale)), "act two line leaked into act four: {stale}");
    }
}
