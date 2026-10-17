//! Windows foreground WinEvent hook and session-scoped GetLastInputInfo.
//! Requires a logged-in interactive user; a Windows service in session 0 does
//! not receive the same user focus or keyboard events.
use crate::CollectorEvent;
use std::{
    ptr,
    sync::{OnceLock, mpsc::SyncSender},
    thread,
    time::Duration,
};
use watcher_core::{InputState, Observation};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HWND},
    System::{
        SystemInformation::GetTickCount64,
        Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW},
    },
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
        WindowsAndMessaging::{
            EVENT_SYSTEM_FOREGROUND, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
            MSG, WINEVENT_OUTOFCONTEXT,
        },
    },
};

static FOCUS_SINK: OnceLock<SyncSender<CollectorEvent>> = OnceLock::new();

fn foreground() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.chars().filter(|c| !c.is_control()).take(128).collect())
    }
}

unsafe extern "system" fn focus_changed(
    _: HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if let Some(tx) = FOCUS_SINK.get() {
        // Executed on the hook thread's Win32 message loop, not the UI thread.
        let _ = tx.try_send(CollectorEvent::new(Observation::Focus(foreground())));
    }
}

pub fn spawn(tx: SyncSender<CollectorEvent>, threshold_ms: u32) {
    let focus_tx = tx.clone();
    thread::spawn(move || {
        let _ = FOCUS_SINK.set(focus_tx.clone());
        unsafe {
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                ptr::null_mut(),
                Some(focus_changed),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.is_null() {
                let _ = focus_tx.send(CollectorEvent::failure(
                    Observation::FocusAvailable(false),
                    "SetWinEventHook failed: no interactive desktop or permission denied".into(),
                ));
                return;
            }
            let _ = focus_tx.send(CollectorEvent::new(Observation::FocusAvailable(true)));
            let _ = focus_tx.send(CollectorEvent::new(Observation::Focus(foreground())));
            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {}
            UnhookWinEvent(hook);
        }
        let _ = focus_tx.send(CollectorEvent::new(Observation::FocusAvailable(false)));
    });
    thread::spawn(move || {
        let mut previous = InputState::Unknown;
        loop {
            unsafe {
                let mut info = LASTINPUTINFO {
                    cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                    dwTime: 0,
                };
                if GetLastInputInfo(&mut info) == 0 {
                    let _ = tx.send(CollectorEvent::failure(
                        Observation::InputAvailable(false),
                        "GetLastInputInfo failed for current session".into(),
                    ));
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }
                let elapsed = (GetTickCount64() as u32).wrapping_sub(info.dwTime);
                let state = if elapsed >= threshold_ms {
                    InputState::Idle
                } else {
                    InputState::Active
                };
                if previous == InputState::Unknown {
                    let _ = tx.send(CollectorEvent::new(Observation::InputAvailable(true)));
                }
                if state != previous {
                    let mut event = CollectorEvent::new(Observation::Input(state));
                    if state == InputState::Idle {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as i64;
                        event.last_input_ms = Some(now.saturating_sub(i64::from(elapsed)));
                    }
                    if tx.send(event).is_err() {
                        return;
                    }
                    previous = state;
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
