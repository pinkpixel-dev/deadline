//! The BBS clock. Every session on DEADLINE starts at 03:17:42 AM.

use crate::game::state::GameState;

const START: u64 = 3 * 3600 + 17 * 60 + 42;

pub fn time_string(st: &GameState) -> String {
    let secs = (START + st.elapsed as u64) % 86_400;
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
    if st.has("clock:1998") {
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
}
