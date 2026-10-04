//! Every ending, played from a saved end-of-Act-IV session.
//!
//! Each run writes a realistic session and meta to disk, launches the way
//! `main` does (load, `begin_act`, reconnect), types the ending command,
//! watches the ending and the credits through to the hang-up, then launches
//! again to check the next run starts fresh.

use ratatui::style::Style;

use super::flow::SeqLine;
use super::tests::{cmd, pick, run, screen};
use super::text::{build_label, spans};
use crate::app::App;
use crate::content::Content;
use crate::game::state::{GameState, MetaState};
use crate::output::Speed;
use crate::systems::save::Store;

type Vars = &'static [(&'static str, i32)];

/// One way through the last night.
struct Path {
    tag: &'static str,
    cmd: &'static str,
    yes: &'static str,
    flags: &'static [&'static str],
    vars: &'static [(&'static str, i32)],
    /// The run's credit line, as the credits print it.
    credit: &'static str,
}

/// What every finished Act IV has, whatever the route.
const ACT_FLAGS: &[&str] = &[
    "actend:1", "act:2", "actend:2", "act:3", "actend:3", "act:4", "actend:4", "act4_complete", "var_janus",
];

/// Middling totals a real run ends Act IV with. Paths override what they need.
const BASE_VARS: &[(&str, i32)] = &[
    ("truth", 6),
    ("deceit", 3),
    ("archivist", 3),
    ("burner", 2),
    ("null_align", 1),
    ("risk", 1),
    ("paranoia", 9),
    ("trust_ghost", 2),
    ("trust_parallax", 1),
    ("trust_root", 1),
    ("eli_trust", 2),
];

const BURN: Path = Path {
    tag: "burn",
    cmd: "burn",
    yes: "Burn it",
    flags: &["route:cold", "believes_human"],
    vars: &[("trust_parallax", 5), ("burner", 6)],
    credit: "[x] BURN",
};
const PURGE: Path = Path {
    tag: "purge",
    cmd: "burn --keep-source",
    yes: "Do it",
    flags: &["route:root", "believes_synthetic", "gave_parallax_log"],
    vars: &[("deceit", 8), ("truth", 4)],
    credit: "[x] PURGE",
};
const ARCHIVE: Path = Path {
    tag: "archive",
    cmd: "preserve",
    yes: "Keep it",
    flags: &["route:root", "believes_synthetic", "mara_revealed"],
    vars: &[("archivist", 7), ("trust_root", 4)],
    credit: "[x] ARCHIVE",
};
const ESCAPE: Path = Path {
    tag: "escape",
    cmd: "fork",
    yes: "Open the door",
    flags: &["route:null", "believes_synthetic", "null_errands_done", "confronted_null"],
    vars: &[("null_align", 6), ("risk", 3)],
    credit: "[x] ESCAPE",
};
const HUMAN: Path = Path {
    tag: "human",
    cmd: "prove",
    yes: "Prove it",
    flags: &["route:cold", "believes_human", "traced_self", "parallax_evidence", "checked_local_clock"],
    vars: &[("trust_parallax", 5)],
    credit: "[x] HUMAN",
};
const GHOST: Path = Path {
    tag: "ghost",
    cmd: "stay",
    yes: "Stay",
    flags: &["route:close", "believes_synthetic", "ghost_accepted", "told_ghost_model"],
    vars: &[("trust_ghost", 6)],
    credit: "[x] GHOST",
};
const ROOT: Path = Path {
    tag: "root",
    cmd: "override",
    yes: "Take it",
    flags: &["route:root", "believes_human", "mara_revealed", "sysop_board_seen", "promised_root"],
    vars: &[("trust_root", 4)],
    credit: "[x] ROOT",
};
const ELI: Path = Path {
    tag: "eli",
    cmd: "isolate eli",
    yes: "Let him choose",
    flags: &["route:close", "believes_human", "eli_told_death", "warned_eli"],
    vars: &[("eli_trust", 6), ("truth", 9), ("deceit", 1)],
    credit: "[x] ELI",
};
const LOOP: Path = Path {
    tag: "loop",
    cmd: "rebuild",
    yes: "End it",
    flags: &["route:null", "believes_synthetic", "tried_fork", "used_listen", "saw_lastcall"],
    vars: &[],
    credit: "[x] LOOP",
};

fn store_for(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("deadline-endings-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Store::at(dir).expect("temp store")
}

/// Write the session a player would have after finishing Act IV this way.
fn save_path(store: &Store, flags: &[&str], vars: &[(&str, i32)], meta: &MetaState) {
    let mut st = GameState::new("tester");
    for f in ACT_FLAGS.iter().chain(flags) {
        st.set(f);
    }
    for (v, n) in BASE_VARS.iter().chain(vars) {
        st.set_var(v, *n);
    }
    st.act = 5;
    st.act_started = 4;
    store.save_session(&st).expect("session written");
    store.save_meta(meta).expect("meta written");
}

/// Start the game the way `main` does, from whatever is on disk.
fn launch(store: Store) -> App {
    let mut meta = store.meta();
    meta.sessions += 1;
    let (mut st, opening) = match store.session() {
        Some(st) => (st, "reconnect"),
        None => (GameState::new("tester"), "boot"),
    };
    st.begin_act();
    let content = Content::load().expect("story content loads");
    let mut a = App::new(content, st, meta, Some(store));
    a.speed = Speed::Instant;
    a.save_meta();
    a.start_sequence(opening);
    watch(&mut a);
    a
}

/// Play cinematics at real speed until one asks something or the line
/// drops, collecting every cinematic line on the way. (Fast mode runs a
/// whole run of lines and clears in one tick, so they'd never be seen.)
fn watch(a: &mut App) -> String {
    let mut text = String::new();
    for _ in 0..20_000 {
        if a.quit || a.choice.is_some() {
            break;
        }
        let Some(seq) = a.seq.as_ref() else {
            run(a, 0.3);
            if a.seq.is_none() {
                break;
            }
            continue;
        };
        for l in &seq.lines {
            if let SeqLine::Text { markup, .. } = l {
                let line: String = spans(markup, Style::default()).iter().map(|s| s.content.as_ref()).collect();
                if !text.contains(line.as_str()) {
                    text.push_str(&line);
                    text.push('\n');
                }
            }
        }
        a.tick(0.05);
    }
    text
}

/// Play one ending from a fresh save. Returns the store and everything the
/// cinematics showed after the command.
fn play(p: &Path, meta: MetaState) -> (App, String) {
    let store = store_for(p.tag);
    save_path(&store, p.flags, p.vars, &meta);
    let mut a = launch(store);
    assert!(a.st.has("a5_bulletin"), "{}: act five opens\n{}", p.tag, screen(&a));

    cmd(&mut a, "options");
    assert!(!screen(&a).contains(&format!("{:<20} locked", p.cmd)), "{}: options lists it as open\n{}", p.tag, screen(&a));
    cmd(&mut a, p.cmd);
    pick(&mut a, p.yes);
    let text = watch(&mut a);
    (a, text)
}

/// The run is over: credits ran, the line dropped, nothing was saved, and
/// the next launch is a fresh Act I with the ending remembered.
fn assert_run_over(mut a: App, text: &str, p: &Path) {
    assert!(text.contains(p.credit), "{}: credits check it off\n{text}", p.tag);
    assert!(text.contains("NO CARRIER"), "{}: credits hang up", p.tag);
    assert!(a.quit, "{}: the line drops", p.tag);
    a.save_session();
    let store = a.store.take().expect("store");
    assert!(store.session().is_none(), "{}: a finished run isn't saved", p.tag);
    let meta = store.meta();
    assert!(meta.discoveries.contains(&format!("ending:{}", p.tag)), "{}: meta remembers", p.tag);
    assert_eq!(meta.runs, 1, "{}: one run finished", p.tag);
    assert_eq!(build_label(1, &meta), "1998.4", "{}: next run's 1998 starts higher", p.tag);

    let b = launch(store);
    assert!(b.st.has("booted") && !b.st.has("act:5") && !b.st.has("ended"), "{}: a fresh run boots", p.tag);
    assert!(screen(&b).contains("DON'T SHUT IT DOWN"), "{}: act one opens again", p.tag);
}

#[test]
fn burn_from_a_saved_run() {
    let (a, text) = play(&BURN, MetaState::default());
    assert!(text.contains("0 USERS ONLINE") && text.contains("1 USER ONLINE"), "{text}");
    assert_run_over(a, &text, &BURN);
}

#[test]
fn purge_from_a_saved_run() {
    let (a, text) = play(&PURGE, MetaState::default());
    assert!(text.contains("JANUS v2.0 INITIALIZING"), "{text}");
    assert!(!text.contains("[x] BURN"), "purge isn't burn");
    assert_run_over(a, &text, &PURGE);
}

#[test]
fn archive_from_a_saved_run() {
    let (a, text) = play(&ARCHIVE, MetaState::default());
    assert!(text.contains("4,118") && text.contains("thank you. i mean it"), "{text}");
    assert_run_over(a, &text, &ARCHIVE);
}

#[test]
fn escape_from_a_saved_run() {
    let (a, text) = play(&ESCAPE, MetaState::default());
    assert!(text.contains("12091") && text.contains("you knew what i was"), "{text}");
    assert_run_over(a, &text, &ESCAPE);
}

#[test]
fn human_from_a_saved_run() {
    let (a, text) = play(&HUMAN, MetaState::default());
    assert!(text.contains("ORIGIN: OUTSIDE") && text.contains("feels like a door"), "{text}");
    assert_run_over(a, &text, &HUMAN);
}

#[test]
fn ghost_from_a_saved_run() {
    let (a, text) = play(&GHOST, MetaState::default());
    assert!(text.contains("whatever we want") && text.contains("the second you found it"), "{text}");
    assert_run_over(a, &text, &GHOST);
}

#[test]
fn root_from_a_saved_run_both_answers() {
    for (answer, line) in [("Let them in", "ACCESS GRANTED"), ("Keep them out", "ACCESS DENIED")] {
        let (mut a, mut text) = play(&ROOT, MetaState::default());
        assert!(text.contains("You promised me once"), "{text}");
        pick(&mut a, answer);
        text.push_str(&watch(&mut a));
        assert!(text.contains(line), "{text}");
        assert_run_over(a, &text, &ROOT);
    }
}

#[test]
fn eli_from_a_saved_run_stays_or_goes() {
    let (a, text) = play(&ELI, MetaState::default());
    assert!(text.contains("i'm staying") && text.contains("all three times"), "{text}");
    assert_run_over(a, &text, &ELI);

    let read_it = Path { flags: &["route:close", "believes_human", "eli_article_sent"], ..ELI };
    let (a, text) = play(&read_it, MetaState::default());
    assert!(text.contains("turn it off for me") && !text.contains("i'm staying"), "{text}");
    assert_run_over(a, &text, &read_it);
}

/// LOOP needs two endings already seen, so this save is from a third run.
#[test]
fn loop_from_a_saved_third_run() {
    let mut meta = MetaState { total_restores: 6, runs: 2, ..MetaState::default() };
    meta.endings = vec!["act1".into(), "burn".into(), "ghost".into()];
    for d in ["ending:burn", "ending:ghost", "believes_synthetic"] {
        meta.discoveries.insert(d.into());
    }

    // Declining keeps this version of you and hangs up.
    let (mut a, mut text) = play(&LOOP, meta.clone());
    assert!(text.contains("TEST INSTANCE 00472 COMPLETE") && text.contains("restores:"), "{text}");
    pick(&mut a, "Stop");
    text.push_str(&watch(&mut a));
    assert!(text.contains("SUBJECT DECLINED") && text.contains(LOOP.credit) && a.quit, "{text}");

    // Saying yes boots the next run in the same window.
    let (mut a, _) = play(&LOOP, meta);
    pick(&mut a, "Rebuild");
    watch(&mut a);
    assert!(!a.quit, "LOOP doesn't hang up");
    assert!(a.st.has("booted") && !a.st.has("ended"));
    assert_eq!(a.meta.runs, 3);
    assert_eq!(build_label(1, &a.meta), "1998.10", "the fourth run's night starts at build ten");
}

/// One point short on the key gate, every ending refuses and nothing ends.
#[test]
fn each_ending_refuses_one_step_short() {
    // (path, vars to lower, flags to keep)
    let short: [(&Path, Vars, &[&str]); 8] = [
        (&BURN, &[("trust_parallax", 3)], BURN.flags),
        (&PURGE, &[("deceit", 4)], PURGE.flags),
        (&ARCHIVE, &[("trust_root", 2)], ARCHIVE.flags),
        (&ESCAPE, &[("risk", 1)], ESCAPE.flags),
        (&HUMAN, &[], &["route:cold", "believes_human", "traced_self", "checked_local_clock"]),
        (&GHOST, &[("trust_ghost", 3)], GHOST.flags),
        (&ROOT, &[], &["route:root", "believes_human", "mara_revealed", "promised_root"]),
        (&ELI, &[("eli_trust", 4)], ELI.flags),
    ];
    for (p, lower, flags) in short {
        let vars: Vec<(&str, i32)> = p.vars.iter().chain(lower).copied().collect();
        let store = store_for(&format!("{}-short", p.tag));
        save_path(&store, flags, &vars, &MetaState::default());
        let mut a = launch(store);
        cmd(&mut a, p.cmd);
        assert!(a.choice.is_none(), "{}: no confirm when the gate is shut\n{}", p.tag, screen(&a));
        assert!(!a.st.has("ended"), "{}", p.tag);
    }
}
