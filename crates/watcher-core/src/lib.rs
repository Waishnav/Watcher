//! Pure activity interval reducer: no OS, database, or wall-clock dependencies.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputState {
    Active,
    Idle,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Active,
    Idle,
    Unknown,
    Paused,
}

impl ActivityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Unknown => "unknown",
            Self::Paused => "paused",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub start_ms: i64,
    pub end_ms: i64,
    pub app: Option<String>,
    pub state: ActivityState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppUsage {
    pub app: String,
    pub duration_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub from_ms: i64,
    pub to_ms: i64,
    pub active_ms: i64,
    pub idle_ms: i64,
    pub unknown_ms: i64,
    pub paused_ms: i64,
    pub apps: Vec<AppUsage>,
}

pub fn summarize(intervals: &[Interval], from_ms: i64, to_ms: i64) -> Report {
    let mut apps = BTreeMap::<String, i64>::new();
    let (mut active, mut idle, mut unknown, mut paused) = (0, 0, 0, 0);
    for i in intervals {
        let ms = (i.end_ms.min(to_ms) - i.start_ms.max(from_ms)).max(0);
        match i.state {
            ActivityState::Active => {
                if let Some(app) = &i.app {
                    *apps.entry(app.clone()).or_default() += ms;
                    active += ms;
                } else {
                    unknown += ms;
                }
            }
            ActivityState::Idle => idle += ms,
            ActivityState::Unknown => unknown += ms,
            ActivityState::Paused => paused += ms,
        }
    }
    let mut apps: Vec<_> = apps
        .into_iter()
        .map(|(app, duration_ms)| AppUsage { app, duration_ms })
        .collect();
    apps.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms).then(a.app.cmp(&b.app)));
    Report {
        from_ms,
        to_ms,
        active_ms: active,
        idle_ms: idle,
        unknown_ms: unknown,
        paused_ms: paused,
        apps,
    }
}

#[derive(Clone, Debug)]
pub enum Observation {
    Focus(Option<String>),
    FocusAvailable(bool),
    Input(InputState),
    InputAvailable(bool),
    Paused(bool),
    /// The compositor stopped delivering trustworthy information.
    Gap,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub app: Option<String>,
    pub input: InputState,
    pub state: ActivityState,
    pub focus_available: bool,
    pub input_available: bool,
    pub paused: bool,
    pub observed_ms: i64,
}

#[derive(Debug)]
pub struct Change {
    pub interval: Option<Interval>,
    /// Retroactively replace recent active intervals with idle time, when the
    /// compositor reported an idle transition after its threshold elapsed.
    pub idle_since_ms: Option<i64>,
}

pub struct Reducer {
    focus: Option<String>,
    focus_available: bool,
    input: InputState,
    input_available: bool,
    paused: bool,
    since_ms: i64,
    started_ms: i64,
}

impl Reducer {
    pub fn new(now_ms: i64) -> Self {
        Self {
            focus: None,
            focus_available: false,
            input: InputState::Unknown,
            input_available: false,
            paused: false,
            since_ms: now_ms,
            started_ms: now_ms,
        }
    }

    fn state(&self) -> ActivityState {
        if self.paused {
            ActivityState::Paused
        } else if !self.input_available || !self.focus_available {
            ActivityState::Unknown
        } else if self.input == InputState::Idle {
            ActivityState::Idle
        } else if self.input == InputState::Active && self.focus.is_some() {
            ActivityState::Active
        } else {
            ActivityState::Unknown
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            app: self.focus.clone(),
            input: self.input,
            state: self.state(),
            focus_available: self.focus_available,
            input_available: self.input_available,
            paused: self.paused,
            observed_ms: self.since_ms,
        }
    }

    pub fn tick(&mut self, now_ms: i64) -> Option<Interval> {
        let end_ms = now_ms.max(self.since_ms);
        if end_ms == self.since_ms {
            return None;
        }
        let interval = Interval {
            start_ms: self.since_ms,
            end_ms,
            app: self.focus.clone(),
            state: self.state(),
        };
        self.since_ms = end_ms;
        Some(interval)
    }

    pub fn apply(
        &mut self,
        now_ms: i64,
        observation: Observation,
        last_input_ms: Option<i64>,
    ) -> Change {
        let interval = self.tick(now_ms);
        let idle_since_ms = if matches!(observation, Observation::Input(InputState::Idle))
            && !self.paused
            && self.input_available
            && self.input != InputState::Idle
        {
            last_input_ms.map(|t| t.clamp(self.started_ms, self.since_ms))
        } else {
            None
        };
        match observation {
            Observation::Focus(app) => self.focus = app,
            Observation::FocusAvailable(available) => {
                self.focus_available = available;
                if !available {
                    self.focus = None;
                }
            }
            Observation::Input(state) => self.input = state,
            Observation::InputAvailable(available) => {
                self.input_available = available;
                if !available {
                    self.input = InputState::Unknown;
                }
            }
            Observation::Paused(paused) => self.paused = paused,
            Observation::Gap => {
                self.focus_available = false;
                self.focus = None;
                self.input_available = false;
                self.input = InputState::Unknown;
            }
        }
        Change {
            interval,
            idle_since_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_credit_without_both_capabilities() {
        let mut reducer = Reducer::new(0);
        reducer.apply(0, Observation::Focus(Some("Terminal".into())), None);
        reducer.apply(0, Observation::FocusAvailable(true), None);
        reducer.apply(0, Observation::Input(InputState::Active), None);
        assert_eq!(reducer.snapshot().state, ActivityState::Unknown);
        reducer.apply(0, Observation::InputAvailable(true), None);
        assert_eq!(reducer.snapshot().state, ActivityState::Active);
        assert_eq!(reducer.tick(1_500).unwrap().end_ms, 1_500);
    }

    #[test]
    fn switching_and_pause_do_not_overlap() {
        let mut reducer = Reducer::new(100);
        reducer.apply(100, Observation::FocusAvailable(true), None);
        reducer.apply(100, Observation::InputAvailable(true), None);
        reducer.apply(100, Observation::Input(InputState::Active), None);
        reducer.apply(100, Observation::Focus(Some("A".into())), None);
        let change = reducer.apply(1100, Observation::Focus(Some("B".into())), None);
        let a = change.interval.unwrap();
        let b = reducer
            .apply(2100, Observation::Paused(true), None)
            .interval
            .unwrap();
        assert_eq!(
            (a.start_ms, a.end_ms, a.app.as_deref()),
            (100, 1100, Some("A"))
        );
        assert_eq!(
            (b.start_ms, b.end_ms, b.app.as_deref()),
            (1100, 2100, Some("B"))
        );
        assert_eq!(reducer.snapshot().state, ActivityState::Paused);
        assert_eq!(reducer.tick(3000).unwrap().state, ActivityState::Paused);
    }

    #[test]
    fn idle_cutoff_is_clamped_to_current_session() {
        let mut reducer = Reducer::new(1000);
        reducer.apply(1000, Observation::InputAvailable(true), None);
        let change = reducer.apply(62_000, Observation::Input(InputState::Idle), Some(0));
        assert_eq!(change.idle_since_ms, Some(1000));
        assert_eq!(reducer.snapshot().state, ActivityState::Unknown);
        reducer.apply(63_000, Observation::Input(InputState::Active), None);
        assert_eq!(reducer.snapshot().input, InputState::Active);
    }

    #[test]
    fn backwards_clock_does_not_create_negative_duration() {
        let mut reducer = Reducer::new(2000);
        assert!(reducer.tick(1000).is_none());
        assert_eq!(reducer.tick(3000).unwrap().start_ms, 2000);
    }

    #[test]
    fn report_clips_intervals_to_requested_range() {
        let intervals = vec![Interval {
            start_ms: 0,
            end_ms: 2000,
            app: Some("App".into()),
            state: ActivityState::Active,
        }];
        let report = summarize(&intervals, 1000, 5000);
        assert_eq!(report.active_ms, 1000);
        assert_eq!(report.apps[0].duration_ms, 1000);
    }
}
