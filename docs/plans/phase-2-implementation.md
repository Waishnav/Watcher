# Phase 2 implementation and dependency assessment

**State:** initial native collector implementations written, **not yet a supported cross-platform release**. A source build on Arch/Hyprland cannot certify other environments. This document distinguishes implementation from verification so later maintainers do not silently advertise unsupported desktops.

## Chosen dependencies and native mechanisms

| Target | Focus source | Idle source | Rust dependencies | What is implemented | Live verified |
| --- | --- | --- | --- | --- | --- |
| Hyprland | IPC `.socket2.sock`, `activewindow` notifications | Wayland `ext_idle_notifier_v1` **v2** | `wayland-client 0.31`, `wayland-protocols 0.32`, native Unix socket + `serde_json` | Yes; passive live QA | **Yes**, for basic foreground and input events |
| Sway | Sway IPC `subscribe` (window/workspace/close) | same Wayland input-idle protocol where advertised | `swayipc 4.0`, Wayland crates | Adapter integrated in auto-detection | **No** (no Sway compositor locally) |
| X11 desktop | EWMH `_NET_ACTIVE_WINDOW` root PropertyNotify, `WM_CLASS` | XScreenSaver `query_info` once per second | `x11rb 0.14` with `screensaver` | Adapter integrated; isolated Xvfb CI script | **Not yet** (Xvfb test awaits CI) |
| Windows | `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` and process image identity | `GetLastInputInfo` sampled every 500ms | `windows-sys 0.61` | Collector + current-user daemon named-pipe transport + CLI client | **No** (native compile/interactive desktop pending) |
| macOS | `NSWorkspace.frontmostApplication` sampled every 500ms | `CGEventSourceSecondsSinceLastEventType` sampled every 500ms | `objc2-app-kit 0.3`, linked CoreGraphics | Collector + Unix socket daemon/CLI | **No** (native compile/interactive Mac pending) |
| GNOME Wayland | Opt-in GNOME Shell extension for focus notifications | Wayland idle notifier v2 if supported | `zbus 5.13` (Rust MSRV 1.85), GJS + session D-Bus | Opt-in GJS script and D-Bus app-ID receiver implemented; not yet desktop-verified | No |
| KDE Plasma Wayland | Opt-in KWin 6 script using windowActivated | idle protocol if available | `zbus 5.13` + KWin script + session D-Bus | Opt-in KWin script and D-Bus app-ID receiver implemented; not yet desktop-verified | No |

### Research and authoritative references

- X11: [`x11rb` docs](https://docs.rs/x11rb/latest/x11rb/) for X11 property notifications, optional `screensaver` protocol and query. Chosen instead of spawning `xdotool` or `xprintidle` on a schedule. A 1s native query is unavoidable with this supported idle mechanism.
- Sway: [`swayipc` docs](https://docs.rs/crate/swayipc/4.0.0) and [Sway IPC specification](https://github.com/swaywm/sway/blob/master/sway/sway-ipc.7.scd). The synchronous crate matches Watcher's std-thread implementation without adding Tokio.
- Windows: [Microsoft SetWinEventHook](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook) requires a message loop; [`GetLastInputInfo`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlastinputinfo) is session-local and its tick count is not guaranteed to be monotonic. `windows-sys` exposes Win32 without another runtime.
- macOS: [Apple NSWorkspace activation notifications](https://developer.apple.com/documentation/appkit/nsworkspace/didactivateapplicationnotification) and [frontmost app](https://developer.apple.com/documentation/appkit/nsworkspace/frontmostapplication); [CoreGraphics last-input interval](https://developer.apple.com/documentation/coregraphics/cgeventsource/secondssincelasteventtype%28_%3Aeventtype%3A%29). Initial implementation samples `NSWorkspace` rather than wiring its main-runloop notifications; optimize after real Mac QA.
- GNOME: [focused-window D-Bus extension pattern](https://github.com/flexagoon/focused-window-dbus) and [GNOME bridge tradeoffs](https://github.com/fixing-wayland/w11/blob/main/gnome/README.md). GNOME does not expose unrestricted cross-desktop focused-window state to an arbitrary Wayland client.
- KDE: [official KWin scripting documentation](https://develop.kde.org/docs/plasma/kwin/). A versioned opt-in script needs to be designed and verified on Plasma 6; this is not a generic Wayland collector.

## Architecture

`Platform::detect()` chooses **Hyprland → Sway → native X11** on Linux via environment evidence; Windows and macOS use cfg-gated native modules. `Platform::start(sender, threshold)` launches platform-specific focus and input collection but only publishes `CollectorEvent` (normalized observations) through the bounded channel to the Rust reducer. The core, SQLite data model, CLI reports, and OpenTUI DTOs have no OS-specific calculations.

Focus and input are separate capabilities. Unsupported sources do not produce fabricated active time. A temporarily disconnected collector sends an unavailable capability; suspend-like gaps mark input unknown until a new real input signal arrives. Hyprland Wayland idle v2 uses a second input-resumed watcher to avoid falsely marking an already-idle startup as active; the first genuine input establishes Active.

On Windows, the per-user daemon uses a local named pipe (no TCP), with an OS-backed lock and data in `%LOCALAPPDATA%`. On macOS, it uses a Unix socket and `~/Library/Application Support/watcher`. Neither has been end-to-end verified on its native OS.

## Known code and validation risks

1. `windows-sys` modules and `objc2-app-kit` bindings were chosen from published docs, but there is **no locally installed Windows or macOS Rust target**. Passing Linux `cargo check` does **not** typecheck cfg-gated Windows/macOS code. Native CI is required to resolve compile/link problems.
2. Windows interactive session hooks and named pipes require desktop QA for focus, lock/unlock, multiple sessions, permissions, pipe ACLs, and clean shutdown. A Windows Server CI runner proves compilation, not user desktop behavior.
3. macOS uses foreground polling initially; upgrade to `NSWorkspace` activation notifications in a main run loop and test login/sleep/session notifications. A macOS hosted runner may be headless.
4. X11's `XScreenSaver` idle source is sampled, while X11 foreground is event-driven. `Xvfb` integration exercises a **disposable** display, not a human input session.
5. GNOME and KDE now have source-only opt-in desktop focus bridges at `integrations/`, using a local D-Bus receiver and app IDs. The GNOME extension publishes a 5s heartbeat; the KWin script publishes only changes so the receiver expires after 300s of silence. Neither has passed desktop integration QA. Do not fall back to inspecting XWayland or process titles and call the result supported.
6. Do not enable auto-start or persist a test collector on the maintainer's desktop. Separate QA DB/socket/pipe paths remain mandatory.
7. Remote CI cannot run until branch publication; as of this work, the GitHub origin is archived/read-only and rejected prior pushes with HTTP 403.

## Release gate

Every named platform requires: native build, collector start/stop and capability errors, live focus A→B with real app identity, idle→resume, lock/unlock, sleep/wake, daemon restart/persistence checks, CLI+TUI parity, user-private IPC verification, documented install/uninstall and a redacted evidence trace. Until all relevant rows pass, label a platform **experimental** or **unsupported**, not verified.
