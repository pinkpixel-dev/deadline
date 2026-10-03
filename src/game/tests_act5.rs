//! Act V: endings and rebuilding a run.

use super::tests::{app, run, screen};
use super::tests_act2::skip;
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
