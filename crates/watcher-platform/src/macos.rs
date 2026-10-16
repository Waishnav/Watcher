//! macOS native app foreground state + Quartz event-idle sampling.
//! NSWorkspace polling is a fallback pending a native main-runloop notification
//! integration. No Accessibility permission or window titles are requested.
use crate::CollectorEvent;
use objc2_app_kit::NSWorkspace;
use std::{sync::mpsc::SyncSender, thread, time::Duration};
use watcher_core::{InputState, Observation};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    // kCGEventSourceStateCombinedSessionState = 0, kCGAnyInputEventType = UINT32_MAX
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}

pub fn spawn(tx: SyncSender<CollectorEvent>, threshold_ms: u32) {
    let focus_tx = tx.clone();
    thread::spawn(move || {
        let mut previous: Option<String> = None;
        let mut initialized = false;
        loop {
            let app = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .and_then(|a| a.bundleIdentifier())
                .map(|s| s.to_string());
            if !initialized {
                if focus_tx
                    .send(CollectorEvent::new(Observation::FocusAvailable(true)))
                    .is_err()
                {
                    return;
                }
                initialized = true;
            }
            if app != previous {
                if focus_tx
                    .send(CollectorEvent::new(Observation::Focus(app.clone())))
                    .is_err()
                {
                    return;
                }
                previous = app;
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
    thread::spawn(move || {
        let mut previous = InputState::Unknown;
        loop {
            let elapsed = unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) };
            if !elapsed.is_finite() || elapsed < 0.0 {
                let _ = tx.send(CollectorEvent::failure(
                    Observation::InputAvailable(false),
                    "Quartz idle input state unavailable".into(),
                ));
                previous = InputState::Unknown;
                thread::sleep(Duration::from_secs(2));
                continue;
            }
            let elapsed_ms = (elapsed * 1000.0).min(u32::MAX as f64) as u32;
            let state = if elapsed_ms >= threshold_ms {
                InputState::Idle
            } else {
                InputState::Active
            };
            if previous == InputState::Unknown {
                if tx
                    .send(CollectorEvent::new(Observation::InputAvailable(true)))
                    .is_err()
                {
                    return;
                }
            }
            if previous != state {
                let mut event = CollectorEvent::new(Observation::Input(state));
                if state == InputState::Idle {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    event.last_input_ms = Some(now.saturating_sub(i64::from(elapsed_ms)));
                }
                if tx.send(event).is_err() {
                    return;
                }
                previous = state;
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
