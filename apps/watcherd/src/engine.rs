use chrono::Utc;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use watcher_core::{Observation, Reducer, summarize};
use watcher_ipc::{PROTOCOL_VERSION, Request, Response};
use watcher_platform::Platform;
use watcher_storage::Store;

pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(crate) struct Service {
    pub platform: Platform,
    pub reducer: Reducer,
    pub storage: Store,
    pub focus_issue: Option<String>,
    pub input_issue: Option<String>,
}

impl Service {
    pub fn open(platform: Platform, data: &Path) -> Result<Self, String> {
        let mut storage = Store::open(&data.join("watcher.sqlite")).map_err(|e| e.to_string())?;
        let mut started_ms = now_ms();
        if let Some(last) = storage.last_end().map_err(|e| e.to_string())? {
            if last < started_ms {
                storage
                    .append(&watcher_core::Interval {
                        start_ms: last,
                        end_ms: started_ms,
                        app: None,
                        state: watcher_core::ActivityState::Unknown,
                    })
                    .map_err(|e| e.to_string())?;
            } else if last > started_ms {
                started_ms = last;
            }
        }
        Ok(Self {
            platform,
            reducer: Reducer::new(started_ms),
            storage,
            focus_issue: None,
            input_issue: None,
        })
    }

    pub fn observation(&mut self, event: watcher_platform::CollectorEvent) -> Result<(), String> {
        match &event.observation {
            Observation::FocusAvailable(available) => {
                self.focus_issue = if *available {
                    None
                } else {
                    event.detail.clone()
                };
            }
            Observation::InputAvailable(available) => {
                if *available {
                    self.input_issue = None;
                } else if let Some(detail) = &event.detail {
                    self.input_issue = Some(detail.clone());
                }
            }
            _ => {}
        }
        self.receive(now_ms(), event.observation, event.last_input_ms)
    }

    pub fn receive(
        &mut self,
        now: i64,
        event: Observation,
        last_input: Option<i64>,
    ) -> Result<(), String> {
        let change = self.reducer.apply(now, event, last_input);
        self.storage
            .apply(change.interval.as_ref(), change.idle_since_ms, now)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn tick(&mut self, now: i64) -> Result<(), String> {
        if let Some(segment) = self.reducer.tick(now) {
            self.storage.append(&segment).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn request(&mut self, request: Request, stop: &AtomicBool) -> Response {
        let error = |message: String| Response::Error {
            version: PROTOCOL_VERSION,
            message,
        };
        if request.version() != PROTOCOL_VERSION {
            return error(format!("protocol mismatch: expected {PROTOCOL_VERSION}"));
        }
        if let Err(err) = self.tick(now_ms()) {
            return error(err);
        }
        match request {
            Request::Status { .. } => {
                let snapshot = self.reducer.snapshot();
                let detail = format!(
                    "focus={}, input={}, paused={}; focus detail: {}; idle detail: {}",
                    snapshot.focus_available,
                    snapshot.input_available,
                    snapshot.paused,
                    self.focus_issue.as_deref().unwrap_or("ok"),
                    self.input_issue.as_deref().unwrap_or("ok")
                );
                Response::Status {
                    version: PROTOCOL_VERSION,
                    snapshot,
                    collector: self.platform.label().into(),
                    detail,
                }
            }
            Request::Report { from_ms, to_ms, .. } => {
                if from_ms >= to_ms {
                    return error("invalid time range".into());
                }
                match self.storage.query(from_ms, to_ms, usize::MAX) {
                    Ok(intervals) => Response::Report {
                        version: PROTOCOL_VERSION,
                        report: summarize(&intervals, from_ms, to_ms),
                    },
                    Err(err) => error(err.to_string()),
                }
            }
            Request::Timeline {
                from_ms,
                to_ms,
                limit,
                ..
            } => {
                if from_ms >= to_ms {
                    return error("invalid time range".into());
                }
                match self.storage.query(from_ms, to_ms, limit.clamp(1, 5000)) {
                    Ok(intervals) => Response::Timeline {
                        version: PROTOCOL_VERSION,
                        intervals,
                    },
                    Err(err) => error(err.to_string()),
                }
            }
            Request::Pause { .. } => {
                match self.receive(now_ms(), Observation::Paused(true), None) {
                    Ok(()) => Response::Ok {
                        version: PROTOCOL_VERSION,
                        message: "tracking paused".into(),
                    },
                    Err(err) => error(err),
                }
            }
            Request::Resume { .. } => {
                match self.receive(now_ms(), Observation::Paused(false), None) {
                    Ok(()) => Response::Ok {
                        version: PROTOCOL_VERSION,
                        message: "tracking resumed".into(),
                    },
                    Err(err) => error(err),
                }
            }
            Request::Shutdown { .. } => {
                stop.store(true, Ordering::Relaxed);
                Response::Ok {
                    version: PROTOCOL_VERSION,
                    message: "shutting down".into(),
                }
            }
        }
    }
}
