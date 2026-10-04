mod app;
mod content;
mod game;
mod input;
mod keys;
mod output;
mod systems;
mod ui;

use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;

use app::App;
use content::Content;
use game::state::GameState;
use systems::save::Store;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The account DEADLINE already has for you.
fn player_name() -> String {
    let raw = ["USER", "LOGNAME", "USERNAME"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .unwrap_or_default();
    let clean: String = raw
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(14)
        .collect();
    if clean.is_empty() { "visitor".into() } else { clean }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    if has("--help") || has("-h") {
        println!("deadline {VERSION}\n\nUSAGE: deadline [--reset] [--check] [--version]\n\n  --reset    hang up the old session and dial in fresh\n  --check    validate story content and exit");
        return Ok(());
    }
    if has("--version") || has("-V") {
        println!("deadline {VERSION}");
        return Ok(());
    }

    let content = Content::load()?;
    if has("--check") {
        let errors = content::validate::check(&content);
        for e in &errors {
            eprintln!("{e}");
        }
        println!("{} problems", errors.len());
        std::process::exit(if errors.is_empty() { 0 } else { 1 });
    }

    let store = Store::open().ok();
    if has("--reset") {
        if let Some(s) = &store {
            s.clear_session()?;
        }
    }
    let mut meta = store.as_ref().map(|s| s.meta()).unwrap_or_default();
    meta.sessions += 1;
    let player = player_name();
    let (mut st, opening) = match store.as_ref().and_then(|s| s.session()) {
        Some(st) => (st, "reconnect"),
        None => (GameState::new(&player), "boot"),
    };
    st.player = player;
    if st.begin_act()
        && let Some(Err(e)) = store.as_ref().map(|s| s.clear_planted())
    {
        eprintln!("could not clear planted snapshots: {e}");
    }

    let mut app = App::new(content, st, meta, store);
    app.save_meta();
    app.start_sequence(opening);

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    let result = ui::splash::play(&mut terminal).and_then(|_| run(&mut terminal, &mut app));
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    app.save_session();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    let frame = Duration::from_millis(33);
    let mut last = Instant::now();
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app))?;
        let timeout = frame.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(k) => app.on_key(k),
                Event::Mouse(m) => app.on_mouse(m),
                Event::Paste(s) => s.chars().filter(|c| !c.is_control()).for_each(|c| app.input.insert(c)),
                _ => {}
            }
        }
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f64().min(0.25);
        if dt >= 0.016 {
            app.tick(dt);
            last = now;
        }
    }
    Ok(())
}
