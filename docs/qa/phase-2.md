# Phase 2 QA log

## 2026-10-16 — Arch/Hyprland, non-disruptive verification

- Refactored collectors to a shared `Platform::detect/start` seam. Existing Hyprland focus and Wayland idle collectors still compile on the development machine.
- Real passive Hyprland QA in an isolated `.qa/p2-live/` data/socket profile: both focus and input capabilities available, current app identified, actual input counted as active. New initial-coverage behavior correctly began unknown; SQLite integrity `ok`, overlapping interval count zero.
- No external window activation, keyboard input injection, screen lock, suspend, compositor reconfiguration, or changes to live user desktop. QA daemon shut down successfully after testing; production data untouched.
- Rust fmt, Clippy, build and core/storage regression tests run locally on Arch. Frontend Bun typecheck and bundle run locally.
- Isolated X11 focus integration recipe at `scripts/qa/x11-isolated.sh` and workflow `.github/workflows/desktop-qa.yml`; script hard-refuses to run without the explicit Xvfb QA marker. No Xvfb binary in this development environment; CI execution pending.
- Windows and macOS code has **not** been built by their native Rust compiler targets or run on their desktops. Experimental GNOME and KDE compositor bridge source is implemented, but **not natively verified**. Do not claim Phase 2 complete.

## Private D-Bus protocol smoke

- Ran `dbus-run-session` with synthetic `XDG_CURRENT_DESKTOP=GNOME`, using a disposable `.qa/bridge-test` database and socket, and deliberately no GNOME Shell installation. Called `org.watcher.FocusSink.SetActiveApp` with `org.test.qa`; the daemon reported `collector=gnome-wayland`, `app=org.test.qa`, `focus_available=true`. Shut the daemon down through the CLI. **This verifies the D-Bus receiver, not a real GNOME desktop.**
- GNOME and KWin script syntax can be parsed with `node --check`, but native shell/KWin APIs, extension installation, actual focus delivery, and liveness remain unverified.

## Native QA still required

- [ ] Xvfb X11 focus A→B and XScreenSaver query, from scheduled/dispatch workflow
- [ ] Real X11 desktop lock/idle/resume and persistence
- [ ] Real Sway IPC focus, idle, workspace, compositor restart
- [ ] Windows native compilation and interactive focus/idle/lock/suspend/named-pipe/TUI
- [ ] macOS native compilation and interactive focus/idle/sleep/launch-agent/TUI
- [ ] GNOME Shell 45+ extension installation, permissions, heartbeat/disconnect and versioned real-desktop QA
- [ ] KDE Plasma 6 script installation, liveness/permission handling and versioned real-desktop QA
- [ ] Published hosted CI status on all target OSes
