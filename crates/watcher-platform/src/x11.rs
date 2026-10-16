//! Native X11 EWMH focus events + XScreenSaver idle query.
//! The X11 extension has no portable input-idle change callback, so idle is
//! sampled once a second; focused-window tracking remains event-driven.
use crate::CollectorEvent;
use std::{
    sync::mpsc::SyncSender,
    thread,
    time::{Duration, Instant},
};
use watcher_core::{InputState, Observation};
use x11rb::{
    connection::{Connection, RequestConnection},
    protocol::{
        Event,
        screensaver::ConnectionExt as ScreenSaverExt,
        xproto::{AtomEnum, ChangeWindowAttributesAux, ConnectionExt as XprotoExt, EventMask},
    },
    rust_connection::RustConnection,
};

fn atom(conn: &RustConnection, name: &[u8]) -> Result<u32, String> {
    conn.intern_atom(false, name)
        .map_err(|e| e.to_string())?
        .reply()
        .map(|r| r.atom)
        .map_err(|e| e.to_string())
}

fn focused_app(
    conn: &RustConnection,
    root: u32,
    active_atom: u32,
    class_atom: u32,
) -> Result<Option<String>, String> {
    let property = conn
        .get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    let Some(window) = property
        .value32()
        .and_then(|mut it| it.next())
        .filter(|w| *w != 0)
    else {
        return Ok(None);
    };
    let reply = match conn
        .get_property(false, window, class_atom, AtomEnum::STRING, 0, 128)
        .map_err(|e| e.to_string())?
        .reply()
    {
        Ok(value) => value,
        Err(_) => return Ok(None), // the focused window may close between requests
    };
    let mut parts = reply.value.split(|b| *b == 0);
    let instance = parts.next().unwrap_or_default();
    let class = parts.next().filter(|s| !s.is_empty()).unwrap_or(instance);
    let name = String::from_utf8_lossy(class)
        .chars()
        .filter(|c| !c.is_control())
        .take(128)
        .collect::<String>();
    Ok((!name.trim().is_empty()).then_some(name))
}

pub fn spawn(tx: SyncSender<CollectorEvent>, idle_threshold_ms: u32) {
    thread::spawn(move || {
        loop {
            let result = run(&tx, idle_threshold_ms);
            let reason = result
                .err()
                .unwrap_or_else(|| "X11 connection ended".into());
            if tx
                .send(CollectorEvent::failure(
                    Observation::FocusAvailable(false),
                    reason,
                ))
                .is_err()
            {
                return;
            }
            if tx
                .send(CollectorEvent::new(Observation::InputAvailable(false)))
                .is_err()
            {
                return;
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

fn run(tx: &SyncSender<CollectorEvent>, idle_threshold_ms: u32) -> Result<(), String> {
    let (conn, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let root = conn.setup().roots[screen].root;
    let active_atom = atom(&conn, b"_NET_ACTIVE_WINDOW")?;
    let class_atom = atom(&conn, b"WM_CLASS")?;
    let has_idle = conn
        .extension_information("MIT-SCREEN-SAVER")
        .map_err(|e| e.to_string())?
        .is_some();
    // Listen for updates; never grab keyboard/pointer or change focus.
    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )
    .map_err(|e| e.to_string())?
    .check()
    .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;
    tx.send(CollectorEvent::new(Observation::FocusAvailable(true)))
        .map_err(|e| e.to_string())?;
    tx.send(CollectorEvent::new(Observation::Focus(focused_app(
        &conn,
        root,
        active_atom,
        class_atom,
    )?)))
    .map_err(|e| e.to_string())?;
    tx.send(CollectorEvent::new(Observation::InputAvailable(has_idle)))
        .map_err(|e| e.to_string())?;
    let mut previous = InputState::Unknown;
    let mut last_poll = Instant::now() - Duration::from_secs(2);
    loop {
        // Poll the X socket for pending property notifications, not the focus API.
        while let Some(event) = conn.poll_for_event().map_err(|e| e.to_string())? {
            if let Event::PropertyNotify(e) = event {
                if e.window == root && e.atom == active_atom {
                    tx.send(CollectorEvent::new(Observation::Focus(focused_app(
                        &conn,
                        root,
                        active_atom,
                        class_atom,
                    )?)))
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        if has_idle && last_poll.elapsed() >= Duration::from_secs(1) {
            last_poll = Instant::now();
            let idle = conn
                .screensaver_query_info(root)
                .map_err(|e| e.to_string())?
                .reply()
                .map_err(|e| e.to_string())?
                .ms_since_user_input;
            let state = if idle >= idle_threshold_ms {
                InputState::Idle
            } else {
                InputState::Active
            };
            if state != previous {
                let mut event = CollectorEvent::new(Observation::Input(state));
                if state == InputState::Idle {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    event.last_input_ms = Some(now_ms.saturating_sub(i64::from(idle)));
                }
                tx.send(event).map_err(|e| e.to_string())?;
                previous = state;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}
