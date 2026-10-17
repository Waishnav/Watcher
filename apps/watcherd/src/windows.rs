//! Session-local Windows named-pipe server. No TCP listener or Windows service.
use crate::engine::{Service, now_ms};
use fs2::FileExt;
use std::{
    fs,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    os::windows::io::{FromRawHandle, RawHandle},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use watcher_core::Observation;
use watcher_ipc::{PROTOCOL_VERSION, Request, Response, paths};
use watcher_platform::{CollectorEvent, Platform};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, GetLastError, INVALID_HANDLE_VALUE},
    Storage::FileSystem::PIPE_ACCESS_DUPLEX,
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
        PIPE_TYPE_BYTE, PIPE_WAIT,
    },
};

fn serve(pipe_name: &str, state: &Arc<Mutex<Service>>, stop: &AtomicBool) -> Result<(), String> {
    let name: Vec<u16> = pipe_name.encode_utf16().chain(std::iter::once(0)).collect();
    while !stop.load(Ordering::Relaxed) {
        unsafe {
            // Default Windows DACL plus PIPE_REJECT_REMOTE_CLIENTS: per-user
            // interactive pipe, never remotely reachable. Each client gets a
            // new instance so failed requests cannot wedge future connections.
            let pipe = CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                1024 * 1024,
                64 * 1024,
                5000,
                std::ptr::null(),
            );
            if pipe == INVALID_HANDLE_VALUE {
                return Err(format!(
                    "CreateNamedPipeW: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let connected = ConnectNamedPipe(pipe, std::ptr::null_mut()) != 0
                || GetLastError() == ERROR_PIPE_CONNECTED;
            if !connected {
                CloseHandle(pipe);
                continue;
            }
            let file = File::from_raw_handle(pipe as RawHandle);
            let mut line = String::new();
            let result = BufReader::new((&file).take(64 * 1024)).read_line(&mut line);
            let reply = match result {
                Ok(0) => None,
                Ok(_) if !line.ends_with('\n') => Some(Response::Error {
                    version: PROTOCOL_VERSION,
                    message: "request too large or incomplete".into(),
                }),
                Ok(_) => Some(match serde_json::from_str::<Request>(&line) {
                    Ok(req) => state.lock().unwrap().request(req, stop),
                    Err(err) => Response::Error {
                        version: PROTOCOL_VERSION,
                        message: format!("bad request: {err}"),
                    },
                }),
                Err(err) => Some(Response::Error {
                    version: PROTOCOL_VERSION,
                    message: err.to_string(),
                }),
            };
            if let Some(reply) = reply {
                if let Ok(json) = serde_json::to_string(&reply) {
                    let _ = writeln!(&file, "{json}");
                }
            }
            // Closing the handle disconnects this client; no handle survives
            // past the request. Serialized requests prevent concurrent writes.
        }
    }
    Ok(())
}

pub fn run() -> Result<(), String> {
    let platform = Platform::detect()?;
    let dev = std::env::args().any(|arg| arg == "--dev");
    let config = paths(dev)?;
    fs::create_dir_all(&config.data).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(config.data.join("daemon.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|e| format!("daemon already active: {e}"))?;
    let service = Arc::new(Mutex::new(Service::open(platform, &config.data)?));
    let stop = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let stop = stop.clone();
        move || stop.store(true, Ordering::Relaxed)
    })
    .map_err(|e| e.to_string())?;
    let pipe_name = config.socket.to_string_lossy().into_owned();
    let server = {
        let state = service.clone();
        let stop = stop.clone();
        let name = pipe_name.clone();
        thread::spawn(move || serve(&name, &state, &stop))
    };
    let (tx, rx) = mpsc::sync_channel::<CollectorEvent>(256);
    let timeout = std::env::var("WATCHER_IDLE_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|v| (5..=3600).contains(v))
        .unwrap_or(60)
        * 1000;
    platform.start(tx, timeout);
    println!("watcherd: Windows collector started (pipe {pipe_name})");
    let mut previous_tick_wall = now_ms();
    while !stop.load(Ordering::Relaxed) {
        let next = rx.recv_timeout(Duration::from_secs(5));
        let now = now_ms();
        if now.saturating_sub(previous_tick_wall) > 20_000 {
            service.lock().unwrap().receive(
                previous_tick_wall,
                Observation::TimeDiscontinuity,
                None,
            )?;
        }
        previous_tick_wall = now;
        let result = match next {
            Ok(event) => service.lock().unwrap().observation(event),
            Err(mpsc::RecvTimeoutError::Timeout) => service.lock().unwrap().tick(now_ms()),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if let Err(err) = result {
            eprintln!("watcherd: persistence error: {err}");
            stop.store(true, Ordering::Relaxed);
        }
    }
    service.lock().unwrap().tick(now_ms())?;
    // Wake a blocking ConnectNamedPipe after Ctrl+C. Shutdown IPC itself is
    // already serviced by this same server, so this is only for signal exits.
    let _ = OpenOptions::new().read(true).write(true).open(pipe_name);
    server
        .join()
        .map_err(|_| "pipe server panicked".to_string())??;
    Ok(())
}
