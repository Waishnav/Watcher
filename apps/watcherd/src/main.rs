//! Per-user collector, interval reducer and local JSON-lines server.
#[cfg(unix)]
mod linux {
    use chrono::Utc;
    use fs2::FileExt;
    use std::{
        env, fs,
        fs::{File, OpenOptions},
        io::{BufRead, BufReader, Write},
        os::unix::{
            fs::PermissionsExt,
            net::{UnixListener, UnixStream},
        },
        path::Path,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::{Duration, Instant},
    };
    use watcher_core::{Observation, Reducer, summarize};
    use watcher_ipc::{PROTOCOL_VERSION, Paths, Request, Response, paths};
    use watcher_platform::CollectorEvent;
    use watcher_storage::Store;

    struct Service {
        reducer: Reducer,
        storage: Store,
        focus_issue: Option<String>,
        input_issue: Option<String>,
    }

    impl Service {
        fn receive(
            &mut self,
            now: i64,
            event: Observation,
            last_input: Option<i64>,
        ) -> Result<(), String> {
            let change = self.reducer.apply(now, event, last_input);
            if let Some(segment) = change.interval {
                self.storage.append(&segment).map_err(|e| e.to_string())?;
            }
            if let Some(since) = change.idle_since_ms {
                self.storage
                    .replace_recent_with_idle(since, now)
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }

        fn tick(&mut self, now: i64) -> Result<(), String> {
            if let Some(segment) = self.reducer.tick(now) {
                self.storage.append(&segment).map_err(|e| e.to_string())?;
            }
            Ok(())
        }

        fn request(&mut self, request: Request, stop: &AtomicBool) -> Response {
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
                        collector: "hyprland".into(),
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

    fn now_ms() -> i64 {
        Utc::now().timestamp_millis()
    }

    fn private_dir(dir: &Path) -> Result<(), String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
    }

    fn try_lock(path: &Path) -> Result<File, String> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.try_lock_exclusive()
            .map_err(|e| format!("another daemon is running for this profile: {e}"))?;
        Ok(file)
    }

    fn handle_client(mut stream: UnixStream, service: &Arc<Mutex<Service>>, stop: &AtomicBool) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
        let mut line = String::new();
        let parse = BufReader::new(&stream).read_line(&mut line);
        let response = match parse {
            Ok(0) => return,
            Ok(bytes) if bytes > 64 * 1024 => Response::Error {
                version: PROTOCOL_VERSION,
                message: "request too large".into(),
            },
            Ok(_) => match serde_json::from_str::<Request>(&line) {
                Ok(req) => service.lock().unwrap().request(req, stop),
                Err(err) => Response::Error {
                    version: PROTOCOL_VERSION,
                    message: format!("bad request: {err}"),
                },
            },
            Err(err) => Response::Error {
                version: PROTOCOL_VERSION,
                message: format!("read failure: {err}"),
            },
        };
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = writeln!(stream, "{json}");
        }
    }

    pub fn run() -> Result<(), String> {
        if !cfg!(target_os = "linux") {
            return Err("the Phase 1 daemon supports Linux/Hyprland only".into());
        }
        if !env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .eq_ignore_ascii_case("hyprland")
            && env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
        {
            return Err("Hyprland session not detected".into());
        }
        let dev = env::args().any(|arg| arg == "--dev");
        let Paths { data, socket } = paths(dev)?;
        private_dir(&data)?;
        private_dir(socket.parent().ok_or("invalid socket path")?)?;
        let _lock = try_lock(&data.join("daemon.lock"))?;
        let listener = if socket.exists() {
            // Our exclusive lock proves that no instance using this profile is
            // alive; only then is it safe to replace a stale socket.
            fs::remove_file(&socket).map_err(|e| e.to_string())?;
            UnixListener::bind(&socket).map_err(|e| e.to_string())?
        } else {
            UnixListener::bind(&socket).map_err(|e| e.to_string())?
        };
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let mut storage = Store::open(&data.join("watcher.sqlite")).map_err(|e| e.to_string())?;
        let mut started_ms = now_ms();
        if let Some(previous_end) = storage.last_end().map_err(|e| e.to_string())? {
            if previous_end < started_ms {
                storage
                    .append(&watcher_core::Interval {
                        start_ms: previous_end,
                        end_ms: started_ms,
                        app: None,
                        state: watcher_core::ActivityState::Unknown,
                    })
                    .map_err(|e| e.to_string())?;
            } else if previous_end > started_ms {
                eprintln!(
                    "watcherd: system clock precedes stored activity; continuing after previous endpoint"
                );
                started_ms = previous_end;
            }
        }
        let service = Arc::new(Mutex::new(Service {
            reducer: Reducer::new(started_ms),
            storage,
            focus_issue: None,
            input_issue: None,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        ctrlc::set_handler({
            let stop = Arc::clone(&stop);
            move || {
                stop.store(true, Ordering::Relaxed);
            }
        })
        .map_err(|e| e.to_string())?;

        let (tx, rx) = mpsc::sync_channel::<CollectorEvent>(256);
        #[cfg(target_os = "linux")]
        {
            watcher_platform::hyprland::spawn_focus(tx.clone());
            let threshold_ms = env::var("WATCHER_IDLE_SECONDS")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|n| (5..=3600).contains(n))
                .unwrap_or(60)
                * 1000;
            watcher_platform::hyprland::spawn_idle(tx, threshold_ms);
        }
        println!(
            "watcherd: running; socket={} (Ctrl+C stops)",
            socket.display()
        );

        let server = {
            let service = Arc::clone(&service);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => handle_client(stream, &service, &stop),
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(40))
                        }
                        Err(err) => eprintln!("watcherd: IPC listener error: {err}"),
                    }
                }
            })
        };
        let mut previous_tick_wall = now_ms();
        let mut previous_tick_mono = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            let now = now_ms();
            let elapsed_wall = now.saturating_sub(previous_tick_wall);
            let elapsed_mono = previous_tick_mono.elapsed().as_millis() as i64;
            if elapsed_wall > elapsed_mono + 5_000 {
                // Likely suspend or clock discontinuity; do not charge the gap.
                if let Err(err) =
                    service
                        .lock()
                        .unwrap()
                        .receive(previous_tick_wall, Observation::Gap, None)
                {
                    eprintln!("watcherd: suspend recovery: {err}");
                    stop.store(true, Ordering::Relaxed);
                }
            }
            previous_tick_wall = now;
            previous_tick_mono = Instant::now();
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    let mut service = service.lock().unwrap();
                    match &event.observation {
                        Observation::FocusAvailable(available) => {
                            service.focus_issue = if *available {
                                None
                            } else {
                                event.detail.clone()
                            };
                        }
                        Observation::InputAvailable(available) => {
                            if *available {
                                service.input_issue = None;
                            } else if let Some(detail) = &event.detail {
                                service.input_issue = Some(detail.clone());
                            }
                        }
                        _ => {}
                    }
                    if let Err(err) =
                        service.receive(now_ms(), event.observation, event.last_input_ms)
                    {
                        eprintln!("watcherd: observation rejected: {err}");
                        stop.store(true, Ordering::Relaxed);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Err(err) = service.lock().unwrap().tick(now_ms()) {
                        eprintln!("watcherd: persistence failed: {err}");
                        stop.store(true, Ordering::Relaxed);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        service.lock().unwrap().tick(now_ms())?;
        let _ = server.join();
        fs::remove_file(&socket).map_err(|e| e.to_string())?;
        println!("watcherd: stopped");
        Ok(())
    }
}

fn main() {
    #[cfg(unix)]
    if let Err(err) = linux::run() {
        eprintln!("watcherd: {err}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        eprintln!("watcherd: the Phase 1 collector supports Linux/Hyprland only");
        std::process::exit(2);
    }
}
