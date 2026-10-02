use chrono::{DateTime, Local};
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Work,
    Break,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Work => "work",
            Mode::Break => "break",
        }
    }

    pub fn other(self) -> Mode {
        match self {
            Mode::Work => Mode::Break,
            Mode::Break => Mode::Work,
        }
    }
}

/// A finished interval, ready to be appended to storage.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub started_at: DateTime<Local>,
    pub mode: Mode,
    pub length_s: u64,
    pub threshold_s: u64,
}

/// What the UI needs to render one frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    pub mode: Mode,
    pub display_s: u64,
    pub counting_up: bool,
    pub paused: bool,
    pub threshold_s: u64,
}

/// One running interval. Time spent paused is not counted.
///
/// The display is always derived from the *current* threshold, so editing the
/// threshold mid-interval shifts the countdown by exactly the delta.
pub struct Timer {
    mode: Mode,
    started_at: DateTime<Local>,
    accumulated: Duration,
    running_since: Option<Instant>,
    notified: bool,
}

impl Timer {
    pub fn new(mode: Mode, now: Instant) -> Self {
        Timer {
            mode,
            started_at: Local::now(),
            accumulated: Duration::ZERO,
            running_since: Some(now),
            notified: false,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn is_paused(&self) -> bool {
        self.running_since.is_none()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.accumulated + self.running_since.map_or(Duration::ZERO, |s| now - s)
    }

    pub fn pause(&mut self, now: Instant) {
        if let Some(since) = self.running_since.take() {
            self.accumulated += now - since;
        }
    }

    pub fn toggle_pause(&mut self, now: Instant) {
        if self.is_paused() {
            self.running_since = Some(now);
        } else {
            self.pause(now);
        }
    }

    /// Restart this interval from zero, discarding it. Keeps the paused state.
    pub fn reset(&mut self, now: Instant) {
        let paused = self.is_paused();
        *self = Timer::new(self.mode, now);
        if paused {
            self.running_since = None;
        }
    }

    pub fn session(&self, now: Instant, threshold_s: u64) -> Session {
        Session {
            started_at: self.started_at,
            mode: self.mode,
            length_s: self.elapsed(now).as_secs_f64().round() as u64,
            threshold_s,
        }
    }

    /// Start a fresh interval of the other mode.
    pub fn switch(&mut self, now: Instant) {
        *self = Timer::new(self.mode.other(), now);
    }

    pub fn view(&self, now: Instant, threshold_s: u64) -> View {
        let elapsed_s = self.elapsed(now).as_secs();
        let counting_up = elapsed_s >= threshold_s;
        View {
            mode: self.mode,
            display_s: if counting_up { elapsed_s } else { threshold_s - elapsed_s },
            counting_up,
            paused: self.is_paused(),
            threshold_s,
        }
    }

    /// True exactly once each time the elapsed time crosses the threshold.
    pub fn check_threshold(&mut self, now: Instant, threshold_s: u64) -> bool {
        let reached = self.elapsed(now) >= Duration::from_secs(threshold_s);
        let fire = reached && !self.notified;
        self.notified = reached;
        fire
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn counts_down_then_up() {
        let t0 = Instant::now();
        let t = Timer::new(Mode::Work, t0);
        let v = t.view(t0, 60);
        assert_eq!((v.display_s, v.counting_up), (60, false));
        let v = t.view(t0 + S(59), 60);
        assert_eq!((v.display_s, v.counting_up), (1, false));
        let v = t.view(t0 + S(60), 60);
        assert_eq!((v.display_s, v.counting_up), (60, true));
        let v = t.view(t0 + S(75), 60);
        assert_eq!((v.display_s, v.counting_up), (75, true));
    }

    #[test]
    fn pause_is_excluded_from_length() {
        let t0 = Instant::now();
        let mut t = Timer::new(Mode::Work, t0);
        t.toggle_pause(t0 + S(10));
        t.toggle_pause(t0 + S(100));
        assert_eq!(t.session(t0 + S(105), 60).length_s, 15);
        t.pause(t0 + S(110));
        t.pause(t0 + S(200)); // idempotent
        assert_eq!(t.session(t0 + S(300), 60).length_s, 20);
    }

    #[test]
    fn threshold_change_shifts_by_delta() {
        let t0 = Instant::now();
        let t = Timer::new(Mode::Break, t0);
        assert_eq!(t.view(t0 + S(100), 300).display_s, 200);
        assert_eq!(t.view(t0 + S(100), 420).display_s, 320);
        // lowering below elapsed flips to counting up
        let v = t.view(t0 + S(100), 60);
        assert_eq!((v.display_s, v.counting_up), (100, true));
    }

    #[test]
    fn threshold_notifies_once_and_rearms() {
        let t0 = Instant::now();
        let mut t = Timer::new(Mode::Work, t0);
        assert!(!t.check_threshold(t0 + S(59), 60));
        assert!(t.check_threshold(t0 + S(60), 60));
        assert!(!t.check_threshold(t0 + S(61), 60));
        // threshold raised beyond elapsed, then crossed again
        assert!(!t.check_threshold(t0 + S(62), 120));
        assert!(t.check_threshold(t0 + S(120), 120));
    }

    #[test]
    fn reset_restarts_and_keeps_pause() {
        let t0 = Instant::now();
        let mut t = Timer::new(Mode::Work, t0);
        t.pause(t0 + S(30));
        t.reset(t0 + S(40));
        assert!(t.is_paused());
        assert_eq!(t.elapsed(t0 + S(50)), Duration::ZERO);
        assert_eq!(t.mode(), Mode::Work);
    }

    #[test]
    fn switch_produces_session_and_flips_mode() {
        let t0 = Instant::now();
        let mut t = Timer::new(Mode::Work, t0);
        let s = t.session(t0 + S(1700), 1500);
        assert_eq!((s.mode, s.length_s, s.threshold_s), (Mode::Work, 1700, 1500));
        t.switch(t0 + S(1700));
        assert_eq!(t.mode(), Mode::Break);
        assert_eq!(t.elapsed(t0 + S(1700)), Duration::ZERO);
    }
}
