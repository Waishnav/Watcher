//! Platform signals only. No analytics or persistence here.

use std::sync::mpsc::SyncSender;
use watcher_core::Observation;

/// A platform adapter publishes the same normalized events regardless of the
/// underlying compositor or OS. Focus and input are independent capabilities.
/// No adapter performs screen-time calculations or touches the database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Hyprland,
    Sway,
    X11,
    GnomeWayland,
    PlasmaWayland,
    Windows,
    MacOS,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hyprland => "hyprland",
            Self::Sway => "sway",
            Self::X11 => "x11",
            Self::GnomeWayland => "gnome-wayland",
            Self::PlasmaWayland => "plasma-wayland",
            Self::Windows => "windows",
            Self::MacOS => "macos",
        }
    }

    pub fn detect() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        return Ok(Self::Windows);
        #[cfg(target_os = "macos")]
        return Ok(Self::MacOS);
        #[cfg(target_os = "linux")]
        {
            use std::env;
            if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
                Ok(Self::Hyprland)
            } else if env::var_os("SWAYSOCK").is_some() {
                Ok(Self::Sway)
            } else if env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("x11")
                && env::var_os("DISPLAY").is_some()
            {
                Ok(Self::X11)
            } else if env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland")
                && env::var("XDG_CURRENT_DESKTOP")
                    .unwrap_or_default()
                    .to_ascii_uppercase()
                    .contains("GNOME")
            {
                Ok(Self::GnomeWayland)
            } else if env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland")
                && env::var("XDG_CURRENT_DESKTOP")
                    .unwrap_or_default()
                    .to_ascii_uppercase()
                    .contains("KDE")
            {
                Ok(Self::PlasmaWayland)
            } else {
                Err("No supported desktop found: Hyprland, Sway, or native X11 required. GNOME/KWin Wayland need an opt-in integration; not silently treated as X11.".into())
            }
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Err("This operating system has no collector".into());
    }

    /// Start all signals owned by the adapter. The receiver must be bounded.
    pub fn start(self, events: SyncSender<CollectorEvent>, idle_threshold_ms: u32) {
        match self {
            #[cfg(target_os = "linux")]
            Self::Hyprland => {
                hyprland::spawn_focus(events.clone());
                hyprland::spawn_idle(events, idle_threshold_ms);
            }
            #[cfg(target_os = "linux")]
            Self::Sway => {
                sway::spawn_focus(events.clone());
                hyprland::spawn_idle(events, idle_threshold_ms);
            }
            #[cfg(target_os = "linux")]
            Self::X11 => x11::spawn(events, idle_threshold_ms),
            #[cfg(target_os = "linux")]
            Self::GnomeWayland => {
                bridge::spawn_focus(events.clone(), true);
                hyprland::spawn_idle(events, idle_threshold_ms);
            }
            #[cfg(target_os = "linux")]
            Self::PlasmaWayland => {
                bridge::spawn_focus(events.clone(), false);
                hyprland::spawn_idle(events, idle_threshold_ms);
            }
            #[cfg(target_os = "windows")]
            Self::Windows => windows::spawn(events, idle_threshold_ms),
            #[cfg(target_os = "macos")]
            Self::MacOS => macos::spawn(events, idle_threshold_ms),
            _ => {
                let _ = events.send(CollectorEvent::failure(
                    Observation::Gap,
                    "Collector is not built for this target".into(),
                ));
            }
        }
    }
}

pub struct CollectorEvent {
    pub observation: Observation,
    pub last_input_ms: Option<i64>,
    pub detail: Option<String>,
}

impl CollectorEvent {
    pub fn new(observation: Observation) -> Self {
        Self {
            observation,
            last_input_ms: None,
            detail: None,
        }
    }

    pub fn failure(observation: Observation, reason: String) -> Self {
        Self {
            observation,
            last_input_ms: None,
            detail: Some(reason),
        }
    }
}

#[cfg(target_os = "linux")]
pub mod hyprland {
    use super::*;
    use std::{
        env,
        io::{BufRead, BufReader},
        os::unix::net::UnixStream,
        path::PathBuf,
        process::Command,
        sync::mpsc::SyncSender,
        thread,
        time::Duration,
    };
    use watcher_core::InputState;

    fn clean_app_id(raw: &str) -> Option<String> {
        let id: String = raw.chars().filter(|c| !c.is_control()).take(128).collect();
        let id = id.trim();
        (!id.is_empty()).then(|| id.to_owned())
    }

    fn socket_path() -> Option<PathBuf> {
        let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        let name = format!("hypr/{signature}/.socket2.sock");
        let runtime = env::var("XDG_RUNTIME_DIR").ok()?;
        let path = PathBuf::from(runtime).join(&name);
        if path.exists() {
            Some(path)
        } else {
            let fallback = PathBuf::from("/tmp").join(name);
            fallback.exists().then_some(fallback)
        }
    }

    fn current_app() -> Option<String> {
        let output = Command::new("hyprctl")
            .args(["-j", "activewindow"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
        value.get("class")?.as_str().and_then(clean_app_id)
    }

    fn parse_focus_line(line: &str) -> Option<Option<String>> {
        let payload = line.strip_prefix("activewindow>>")?;
        // Hyprland emits CLASS,TITLE. Titles may contain commas and are never
        // included in observations or logs.
        let class = payload.split(',').next().unwrap_or("");
        Some(clean_app_id(class))
    }

    pub fn spawn_focus(tx: SyncSender<CollectorEvent>) {
        thread::spawn(move || {
            loop {
                let Some(path) = socket_path() else {
                    let _ = tx.send(CollectorEvent::failure(
                        Observation::FocusAvailable(false),
                        "Hyprland event socket not found".into(),
                    ));
                    thread::sleep(Duration::from_secs(3));
                    continue;
                };
                match UnixStream::connect(path) {
                    Ok(stream) => {
                        // Subscribe before fetching the current app. Subsequent events
                        // reconcile the snapshot if focus changes during the query.
                        let _ = tx.send(CollectorEvent::new(Observation::FocusAvailable(true)));
                        let _ = tx.send(CollectorEvent::new(Observation::Focus(current_app())));
                        for line in BufReader::new(stream).lines() {
                            let Ok(line) = line else {
                                break;
                            };
                            if let Some(app) = parse_focus_line(&line) {
                                if tx
                                    .send(CollectorEvent::new(Observation::Focus(app)))
                                    .is_err()
                                {
                                    return;
                                }
                            } else if line.starts_with("closelayer>>") {
                                // Layer events are not a proof that focus changed.
                            }
                        }
                    }
                    Err(err) => eprintln!("watcherd: Hyprland focus unavailable: {err}"),
                }
                if tx
                    .send(CollectorEvent::failure(
                        Observation::FocusAvailable(false),
                        "Hyprland event stream disconnected".into(),
                    ))
                    .is_err()
                {
                    return;
                }
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    pub fn spawn_idle(tx: SyncSender<CollectorEvent>, threshold_ms: u32) {
        thread::spawn(move || {
            loop {
                if let Err(err) = watch_idle(tx.clone(), threshold_ms) {
                    eprintln!("watcherd: Wayland input idle unavailable: {err}");
                    let _ = tx.send(CollectorEvent::failure(
                        Observation::InputAvailable(false),
                        err,
                    ));
                }
                if tx
                    .send(CollectorEvent::new(Observation::InputAvailable(false)))
                    .is_err()
                {
                    return;
                }
                thread::sleep(Duration::from_secs(3));
            }
        });
    }

    fn watch_idle(tx: SyncSender<CollectorEvent>, threshold_ms: u32) -> Result<(), String> {
        use wayland_client::{
            Connection, Dispatch, QueueHandle, delegate_noop,
            globals::{GlobalListContents, registry_queue_init},
            protocol::{wl_registry, wl_seat},
        };
        use wayland_protocols::ext::idle_notify::v1::client::{
            ext_idle_notification_v1::{self, ExtIdleNotificationV1},
            ext_idle_notifier_v1::ExtIdleNotifierV1,
        };

        struct IdleHandler {
            tx: SyncSender<CollectorEvent>,
            threshold_ms: u32,
            seen_input: bool,
        }
        #[derive(Clone, Copy)]
        enum IdleWatch {
            Threshold,
            FirstInput,
        }
        impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for IdleHandler {
            fn event(
                _: &mut Self,
                _: &wl_registry::WlRegistry,
                _: wl_registry::Event,
                _: &GlobalListContents,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
        delegate_noop!(IdleHandler: ignore wl_seat::WlSeat);
        delegate_noop!(IdleHandler: ExtIdleNotifierV1);
        impl Dispatch<ExtIdleNotificationV1, IdleWatch> for IdleHandler {
            fn event(
                state: &mut Self,
                _: &ExtIdleNotificationV1,
                event: ext_idle_notification_v1::Event,
                kind: &IdleWatch,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                let observation = match (kind, event) {
                    (IdleWatch::Threshold, ext_idle_notification_v1::Event::Idled) => {
                        state.seen_input = false;
                        CollectorEvent {
                            observation: Observation::Input(InputState::Idle),
                            last_input_ms: Some(
                                unix_ms().saturating_sub(i64::from(state.threshold_ms)),
                            ),
                            detail: None,
                        }
                    }
                    (_, ext_idle_notification_v1::Event::Resumed) if !state.seen_input => {
                        state.seen_input = true;
                        CollectorEvent::new(Observation::Input(InputState::Active))
                    }
                    _ => return,
                };
                let _ = state.tx.send(observation);
            }
        }

        let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
        let (globals, mut events) =
            registry_queue_init::<IdleHandler>(&conn).map_err(|e| e.to_string())?;
        let qh = events.handle();
        let seat: wl_seat::WlSeat = globals
            .bind(&qh, 1..=9, ())
            .map_err(|e| format!("wl_seat is unavailable: {e}"))?;
        // Version 2 explicitly ignores idle inhibitors and reports user input.
        // Version 1 can be suppressed by video idle inhibitors; don't mistake
        // it for a reliable active-interaction signal.
        let notifier: ExtIdleNotifierV1 = globals
            .bind(&qh, 2..=2, ())
            .map_err(|e| format!("ext_idle_notifier_v1 v2 unavailable: {e}"))?;
        let _notification =
            notifier.get_input_idle_notification(threshold_ms, &seat, &qh, IdleWatch::Threshold);
        // A second, 1ms input-idle listener only emits *Resumed*. This lets
        // Watcher learn that genuine user input occurred shortly after
        // startup without assuming Active just because the daemon started.
        let _first_input =
            notifier.get_input_idle_notification(1, &seat, &qh, IdleWatch::FirstInput);
        let mut handler = IdleHandler {
            tx,
            threshold_ms,
            seen_input: false,
        };
        let _ = handler
            .tx
            .send(CollectorEvent::new(Observation::InputAvailable(true)));
        // There is no supported initial "last input" query in this protocol.
        // Do NOT fabricate Active just because the notifier was registered:
        // it might already be idle. Start unknown until idled/resumed arrives.
        events.roundtrip(&mut handler).map_err(|e| e.to_string())?;
        loop {
            events
                .blocking_dispatch(&mut handler)
                .map_err(|e| e.to_string())?;
        }
    }

    fn unix_ms() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_focus_discards_window_titles() {
            assert_eq!(
                parse_focus_line("activewindow>>firefox,Secret project, private tab"),
                Some(Some("firefox".into()))
            );
            assert_eq!(parse_focus_line("activewindow>>,"), Some(None));
            assert_eq!(parse_focus_line("workspace>>3"), None);
            assert_eq!(
                parse_focus_line("activewindow>>app\u{1b}[31m,title"),
                Some(Some("app[31m".into()))
            );
        }
    }
}

#[cfg(target_os = "linux")]
pub mod bridge;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod sway;
#[cfg(windows)]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod x11;
