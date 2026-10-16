//! Opt-in GNOME Shell and Plasma KWin bridge receiver over the session D-Bus.
//! The shell/KWin plugin pushes ONLY stable application IDs; window titles and
//! window contents never cross this interface. No input is captured here.
use crate::CollectorEvent;
use std::{
    sync::{Arc, Mutex, mpsc::SyncSender},
    thread,
    time::{Duration, Instant},
};
use watcher_core::Observation;

struct FocusSink {
    tx: SyncSender<CollectorEvent>,
    last_seen: Arc<Mutex<Instant>>,
}

fn sanitize_app_id(app_id: &str) -> Option<String> {
    let cleaned = app_id
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect::<String>();
    (!cleaned.trim().is_empty()).then_some(cleaned)
}

#[zbus::interface(name = "org.watcher.FocusSink")]
impl FocusSink {
    /// Invoked by a trusted, user-installed desktop bridge at every focus
    /// transition (and once when the bridge loads).
    fn set_active_app(&self, app_id: &str) -> bool {
        let focused = sanitize_app_id(app_id);
        *self.last_seen.lock().unwrap() = Instant::now();
        self.tx
            .try_send(CollectorEvent::new(Observation::FocusAvailable(true)))
            .is_ok()
            && self
                .tx
                .try_send(CollectorEvent::new(Observation::Focus(focused)))
                .is_ok()
    }
}

pub fn spawn_focus(tx: SyncSender<CollectorEvent>, heartbeat_expected: bool) {
    thread::spawn(move || {
        loop {
            let _ = tx.send(CollectorEvent::failure(
            Observation::FocusAvailable(false),
            "GNOME/KWin requires the opt-in Watcher desktop focus bridge. See docs/plans/phase-2-implementation.md".into(),
        ));
            let last_seen = Arc::new(Mutex::new(Instant::now()));
            match zbus::blocking::connection::Builder::session()
                .and_then(|b| b.name("org.watcher.FocusSink"))
                .and_then(|b| {
                    b.serve_at(
                        "/org/watcher/FocusSink",
                        FocusSink {
                            tx: tx.clone(),
                            last_seen: Arc::clone(&last_seen),
                        },
                    )
                })
                .and_then(|b| b.build())
            {
                Ok(_connection) => {
                    // GNOME extension sends a heartbeat every five seconds.
                    // KWin scripts currently lack a confirmed timer API;
                    // both integrations expire, conservatively, when stale.
                    let deadline = Duration::from_secs(if heartbeat_expected { 20 } else { 300 });
                    let mut unavailable = false;
                    loop {
                        thread::sleep(Duration::from_secs(2));
                        let stale = last_seen.lock().unwrap().elapsed() > deadline;
                        if stale && !unavailable
                            && tx.send(CollectorEvent::failure(
                                Observation::FocusAvailable(false),
                                "Focus bridge has stopped reporting; foreground coverage is unknown".into(),
                            )).is_err()
                        {
                            return;
                        }
                        unavailable = stale;
                    }
                }
                Err(err) => {
                    eprintln!("watcherd: focus bridge D-Bus unavailable: {err}");
                    if tx
                        .send(CollectorEvent::failure(
                            Observation::FocusAvailable(false),
                            err.to_string(),
                        ))
                        .is_err()
                    {
                        return;
                    }
                    thread::sleep(Duration::from_secs(5));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_ids_are_bounded_and_never_include_control_chars() {
        assert_eq!(
            sanitize_app_id("\u{1b}org.test.qa\n"),
            Some("org.test.qa".into())
        );
        assert_eq!(sanitize_app_id(""), None);
        assert_eq!(sanitize_app_id(&"a".repeat(200)).unwrap().len(), 128);
    }
}
