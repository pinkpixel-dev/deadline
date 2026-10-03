//! The BBS clock. Every session on DEADLINE starts at 03:17:42 AM.

use crate::game::state::GameState;

const START: u64 = 3 * 3600 + 17 * 60 + 42;

/// Parse `HH:MM` or `HH:MM:SS` (24 hour) into seconds past midnight.
pub fn parse(t: &str) -> Option<u32> {
    let parts: Vec<u32> = t.split(':').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
    let (h, m, s) = match parts[..] {
        [h, m] => (h, m, 0),
        [h, m, s] => (h, m, s),
        _ => return None,
    };
    (h < 24 && m < 60 && s < 60).then_some(h * 3600 + m * 60 + s)
}

pub fn time_string(st: &GameState) -> String {
    let secs = match st.clock {
        Some((at, base)) => (base as u64 + (st.elapsed - at).max(0.0) as u64) % 86_400,
        None => (START + st.elapsed as u64) % 86_400,
    };
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    let (h12, ampm) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12:02}:{m:02}:{s:02} {ampm}")
}

/// The system date. Normally today, but some timelines disagree.
pub fn date_string(st: &GameState) -> String {
    if st.has("clock:0815") {
        "08/15/1998".to_string()
    } else if st.has("clock:1998") {
        "08/14/1998".to_string()
    } else {
        chrono::Local::now().format("%m/%d/%Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_starts_at_three_seventeen() {
        let mut st = GameState::new("x");
        assert_eq!(time_string(&st), "03:17:42 AM");
        st.elapsed = 18.0 + 60.0 * 42.0;
        assert_eq!(time_string(&st), "04:00:00 AM");
        st.set("clock:1998");
        assert_eq!(date_string(&st), "08/14/1998");
    }

    #[test]
    fn the_clock_can_be_set_and_keeps_running() {
        let mut st = GameState::new("x");
        st.elapsed = 500.0;
        st.clock = Some((500.0, parse("21:04").unwrap()));
        assert_eq!(time_string(&st), "09:04:00 PM");
        st.elapsed += 61.0;
        assert_eq!(time_string(&st), "09:05:01 PM");
        assert_eq!(parse("23:41:30"), Some(23 * 3600 + 41 * 60 + 30));
        assert_eq!(parse("25:00"), None);
        assert_eq!(parse("nine"), None);
    }
}
