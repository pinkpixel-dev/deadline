//! Act V: endings and rebuilding a run.

use super::tests::{app, cmd, finish_chat, pick, run, screen};
use super::tests_act2::{answer, next_login, skip};
use crate::app::App;
use crate::content::effect::Effect;
use crate::systems::save::Store;

fn with_store(tag: &str) -> (App, Store) {
    let mut a = app();
    let dir = std::env::temp_dir().join(format!("deadline-act5-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    a.store = Some(Store::at(dir.clone()).expect("temp store"));
    (a, Store::at(dir).expect("temp store"))
}

#[test]
fn an_ending_is_remembered_and_ends_the_run() {
    let (mut a, store) = with_store("ending");
    a.st.set("act:5");
    a.save_session();
    assert!(store.session().is_some());

    a.apply(&[Effect::Ending("burn".into())]);
    assert!(a.meta.discoveries.contains("ending:burn"));
    assert_eq!(a.meta.endings, vec!["burn".to_string()]);
    assert!(store.meta().discoveries.contains("ending:burn"), "meta is written right away");
    assert!(store.session().is_none(), "the finished run isn't saved");
    a.save_session();
    assert!(store.session().is_none(), "later saves don't bring it back");

    a.apply(&[Effect::Ending("burn".into())]);
    assert_eq!(a.meta.endings.len(), 1, "seeing an ending twice records it once");
}

#[test]
fn rebuild_boots_a_fresh_run_and_keeps_meta() {
    let (mut a, store) = with_store("rebuild");
    a.st.set("act:5");
    a.st.add("trust_ghost", 3);
    a.st.history.push("rebuild".into());
    a.meta.total_restores = 5;
    a.save_session();

    a.apply(&[Effect::Ending("loop".into()), Effect::Rebuild]);
    assert!(a.seq.as_ref().is_some_and(|s| s.id == "boot"), "the opening screen plays again");
    assert!(store.session().is_none());
    skip(&mut a);
    run(&mut a, 0.5);
    assert!(a.st.has("booted") && !a.st.has("act:5") && !a.st.has("ended"));
    assert_eq!(a.st.var("trust_ghost"), 0);
    assert!(!a.st.history.iter().any(|h| h == "rebuild"), "the old run's history is gone");
    assert_eq!(a.meta.total_restores, 5);
    assert!(a.meta.discoveries.contains("ending:loop"));
    assert!(screen(&a).contains("DON'T SHUT IT DOWN"), "act one opens again\n{}", screen(&a));
    a.save_session();
    assert!(store.session().is_some_and(|s| s.has("booted")), "the new run saves normally");
}

/// A session that just finished Act IV, logging back in for the last night.
fn act5(tag: &str, flags: &[&str]) -> (App, Store) {
    let (mut a, store) = with_store(tag);
    for f in ["actend:1", "act:2", "actend:2", "act:3", "actend:3", "act:4", "actend:4", "act4_complete", "believes_human"] {
        a.st.set(f);
    }
    for f in flags {
        a.st.set(f);
    }
    a.st.act = 5;
    a.st.act_started = 4;
    next_login(&mut a);
    (a, store)
}

/// Let a cinematic (and anything it chains into) play out.
fn play_out(a: &mut App) {
    for _ in 0..10 {
        skip(a);
        run(a, 0.5);
    }
}

#[test]
fn act_five_opens_and_lists_what_can_be_done() {
    let (mut a, _) = act5("open", &[]);
    assert!(a.st.has("a5_bulletin") && a.st.has("a5_live"));
    assert!(screen(&a).contains("THERE IS NO RESTORE FOR THIS ONE"));
    assert!(!screen(&a).contains("END OF CURRENT BUILD"), "act five is written now");

    cmd(&mut a, "options");
    let s = screen(&a);
    for line in ["burn                 locked", "preserve             locked", "fork                 locked", "isolate eli          locked"] {
        assert!(s.contains(line), "{line}\n{s}");
    }
    assert!(!s.contains("rebuild"), "LOOP stays secret until it's earned");
    assert!(!s.contains("--keep-source"), "PURGE only shows once BURN is possible");
}

#[test]
fn a_locked_ending_says_who_is_in_the_way() {
    let (mut a, store) = act5("locked", &[]);
    cmd(&mut a, "burn");
    assert!(screen(&a).contains("Parallax's end of the line"));
    cmd(&mut a, "override");
    assert!(screen(&a).contains("SYSOP COMMANDS NOT FOUND"));
    cmd(&mut a, "stay");
    assert!(screen(&a).contains("you can't stay somewhere you're just visiting"), "believes_human blocks GHOST");
    cmd(&mut a, "rebuild");
    assert!(!screen(&a).contains("TEST INSTANCE"), "a locked LOOP doesn't announce itself");
    assert!(a.choice.is_none() && !a.st.has("ended"));
    a.save_session();
    assert!(store.session().is_some());
}

#[test]
fn burning_janus_ends_the_run_and_rolls_credits() {
    let (mut a, store) = act5("burn", &["gave_parallax_log"]);
    cmd(&mut a, "options");
    assert!(screen(&a).contains("destroy me"));
    cmd(&mut a, "burn");
    pick(&mut a, "Burn it");
    play_out(&mut a);
    assert!(a.meta.discoveries.contains("ending:burn"));
    assert!(a.quit, "the credits hang up");
    assert!(store.session().is_none(), "the next launch starts a new run");
    assert!(store.meta().discoveries.contains("ending:burn"));
}

#[test]
fn keeping_the_source_is_its_own_ending() {
    let (mut a, _) = act5("purge-locked", &["gave_parallax_log"]);
    cmd(&mut a, "burn --keep-source");
    assert!(screen(&a).contains("REFUSED"), "BURN alone doesn't open PURGE");
    assert!(a.choice.is_none(), "and it doesn't fall through to a plain burn");

    let (mut a, _) = act5("purge", &["gave_parallax_log", "gave_root_log"]);
    cmd(&mut a, "burn --keep-source");
    pick(&mut a, "Do it");
    play_out(&mut a);
    assert!(a.meta.discoveries.contains("ending:purge") && !a.meta.discoveries.contains("ending:burn"));
}

#[test]
fn root_ending_asks_one_last_question() {
    let (mut a, _) = act5("root", &["mara_revealed", "sysop_board_seen"]);
    a.st.set_var("trust_root", 3);
    cmd(&mut a, "override");
    pick(&mut a, "Take it");
    for _ in 0..10 {
        skip(&mut a);
        run(&mut a, 0.5);
        if a.choice.is_some() {
            break;
        }
    }
    pick(&mut a, "Let them in");
    play_out(&mut a);
    assert!(a.meta.discoveries.contains("ending:root") && a.meta.discoveries.contains("root_allowed"));
    assert!(a.quit);
}

#[test]
fn loop_rebuilds_the_subject() {
    let (mut a, _) = act5("loop", &["tried_fork", "used_listen", "saw_lastcall"]);
    a.meta.total_restores = 5;
    a.meta.discoveries.insert("ending:burn".into());
    a.meta.discoveries.insert("ending:ghost".into());
    cmd(&mut a, "options");
    assert!(screen(&a).contains("rebuild"));
    cmd(&mut a, "rebuild");
    pick(&mut a, "End it");
    for _ in 0..10 {
        skip(&mut a);
        run(&mut a, 0.5);
        if a.choice.is_some() {
            break;
        }
    }
    pick(&mut a, "Rebuild");
    play_out(&mut a);
    assert!(!a.quit, "the opening screen plays again instead of hanging up");
    assert!(a.st.has("booted") && !a.st.has("a5_bulletin"));
    assert!(a.meta.discoveries.contains("ending:loop"));

    cmd(&mut a, "burn");
    assert!(!screen(&a).contains("KEY REQUIRED"), "act five's commands stay in act five");
}

#[test]
fn never_deciding_gets_you_hung_up_and_picked_back_up() {
    let (mut a, _) = act5("kick", &[]);
    a.st.elapsed += 2401.0;
    run(&mut a, 1.0);
    a.st.elapsed += 1.0;
    run(&mut a, 1.0);
    play_out(&mut a);
    assert!(a.quit && !a.st.has("a5_live"), "ROOT hangs up");
    assert!(!a.st.has("ended"), "hanging up isn't an ending");

    a.quit = false;
    next_login(&mut a);
    run(&mut a, 1.0);
    assert!(a.st.has("a5_live"));
    assert!(screen(&a).contains("You came back"), "{}", screen(&a));
}

#[test]
fn parallax_can_hand_over_the_kill_code_on_the_last_night() {
    let (mut a, _) = act5("parallax", &[]);
    a.st.set_var("trust_parallax", 3);
    cmd(&mut a, "burn");
    assert!(screen(&a).contains("KEY REQUIRED"));
    answer(&mut a, "parallax_a5");
    pick(&mut a, "If it has to burn");
    finish_chat(&mut a);
    assert!(screen(&a).contains("burn{/}") || screen(&a).contains("will work now"), "{}", screen(&a));
    cmd(&mut a, "burn");
    assert!(a.choice.is_some(), "the code is on your end now");
}

#[test]
fn ghost_can_accept_their_file_on_the_last_night() {
    let (mut a, _) = act5("ghost", &["believes_synthetic"]);
    a.st.set_var("trust_ghost", 3);
    cmd(&mut a, "stay");
    assert!(screen(&a).contains("not ready"));
    answer(&mut a, "ghost_a5");
    pick(&mut a, "that's enough");
    finish_chat(&mut a);
    assert!(a.st.has("ghost_accepted") && a.st.var("trust_ghost") >= 4);
    cmd(&mut a, "stay");
    assert!(a.choice.is_some());
}

#[test]
fn root_points_you_at_the_sysop_board() {
    let (mut a, _) = act5("root-talk", &["mara_revealed"]);
    a.st.set_var("trust_root", 3);
    answer(&mut a, "root_a5");
    pick(&mut a, "Take it off your hands");
    finish_chat(&mut a);
    assert!(screen(&a).contains("open sysop"), "{}", screen(&a));
}

#[test]
fn eli_can_still_learn_the_truth_on_the_last_night() {
    let (mut a, _) = act5("eli", &["eli_article_deleted"]);
    a.st.set_var("eli_trust", 4);
    a.st.set_var("truth", 5);
    answer(&mut a, "eli_a5");
    pick(&mut a, "your obituary");
    finish_chat(&mut a);
    assert!(a.st.has("eli_told_death"));
    cmd(&mut a, "isolate eli");
    assert!(a.choice.is_some(), "telling him the truth opens his ending");
}

#[test]
fn later_runs_feel_like_a_rerun() {
    let (mut a, _) = act5("rerun", &["ghost_accepted"]);
    a.meta.discoveries.insert("ending:burn".into());
    a.st.set_var("trust_ghost", 3);
    answer(&mut a, "ghost_a5");
    assert!(screen(&a).contains("this feels like a rerun"), "{}", screen(&a));
}
