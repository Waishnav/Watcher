//! Per-user collector, interval reducer and local JSON-lines server.
mod engine;
#[cfg(unix)]
mod linux {
    use crate::engine::{Service, now_ms};
    use fs2::FileExt;
    use std::{
        env, fs,
        fs::{File, OpenOptions},
        io::{BufRead, BufReader, Read, Write},
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
    use watcher_core::Observation;
    use watcher_ipc::{PROTOCOL_VERSION, Paths, Request, Response, paths};
    use watcher_platform::{CollectorEvent, Platform};

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
        let parse = BufReader::new((&stream).take(64 * 1024 + 1)).read_line(&mut line);
        let response = match parse {
            Ok(0) => return,
            Ok(bytes) if bytes > 64 * 1024 || !line.ends_with('\n') => Response::Error {
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
        let platform = Platform::detect()?;
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
        let service = Arc::new(Mutex::new(Service::open(platform, &data)?));
        let stop = Arc::new(AtomicBool::new(false));
        ctrlc::set_handler({
            let stop = Arc::clone(&stop);
            move || {
                stop.store(true, Ordering::Relaxed);
            }
        })
        .map_err(|e| e.to_string())?;

        let (tx, rx) = mpsc::sync_channel::<CollectorEvent>(256);
        let threshold_ms = env::var("WATCHER_IDLE_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|n| (5..=3600).contains(n))
            .unwrap_or(60)
            * 1000;
        platform.start(tx, threshold_ms);
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
            let next = rx.recv_timeout(Duration::from_secs(5));
            let now = now_ms();
            let elapsed_wall = now.saturating_sub(previous_tick_wall);
            let elapsed_mono = previous_tick_mono.elapsed().as_millis() as i64;
            if elapsed_wall > elapsed_mono + 5_000 || elapsed_wall > 20_000 {
                // Likely suspend or clock discontinuity; do not charge the gap.
                if let Err(err) = service.lock().unwrap().receive(
                    previous_tick_wall,
                    Observation::TimeDiscontinuity,
                    None,
                ) {
                    eprintln!("watcherd: suspend recovery: {err}");
                    stop.store(true, Ordering::Relaxed);
                }
            }
            previous_tick_wall = now;
            previous_tick_mono = Instant::now();
            match next {
                Ok(event) => {
                    if let Err(err) = service.lock().unwrap().observation(event) {
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

#[cfg(windows)]
mod windows;

fn main() {
    #[cfg(unix)]
    if let Err(err) = linux::run() {
        eprintln!("watcherd: {err}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    if let Err(err) = windows::run() {
        eprintln!("watcherd: {err}");
        std::process::exit(1);
    }
}
