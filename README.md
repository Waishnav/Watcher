# Watcher

A minimal, local-first screen-time tracker. The Rust rewrite is on the `feat/rust-rewrite-phase-1` branch.

**Phase 1:** functional on **Linux + Hyprland**. A per-user Rust daemon observes focused applications and input inactivity, records intervals in SQLite, and serves a colored Rust CLI and an OpenTUI frontend. This is an early development release, not a claim of support for every OS or Wayland compositor.

The primary metric is **active interaction time**: time when a foreground app is known and the Wayland input-idle signal says the user is not idle. Idle time, paused time, and missing coverage are separate. After 60 seconds without input, the daemon retroactively reclassifies the idle threshold period. No window titles, keystrokes, screenshots, URLs, or website contents are stored.

## Run from source (safest way to try it)

Prerequisites: Rust stable, Linux/Hyprland, Bun >= 1.3.14, and working Wayland `ext_idle_notifier_v1` **version 2**. The compositor must provide both Hyprland IPC and this input-only idle protocol. If either is absent, Watcher reports it and **does not pretend that unknown time is active**.

```sh
cargo build --workspace --locked
cd apps/tui && bun install --frozen-lockfile && cd ../..

# Start a foreground daemon in an isolated, gitignored .watcher-dev directory.
./target/debug/watcherd --dev
```

In another terminal (no `sudo`):

```sh
./target/debug/watcher --dev doctor
./target/debug/watcher --dev today
./target/debug/watcher --dev week
./target/debug/watcher --dev apps
./target/debug/watcher --dev timeline
./target/debug/watcher --dev tui
./target/debug/watcher --dev pause
./target/debug/watcher --dev resume
./target/debug/watcher --dev shutdown
```

Run `watcher --help` for the full command syntax, including `--json` and `today --date YYYY-MM-DD`. Use `WATCHER_IDLE_SECONDS=5` when launching the **development** daemon to shorten manual idle QA; 60 seconds is the default. The app does **not** change window focus, launch browser windows, or change your compositor configuration.

The dev profile uses `.watcher-dev/` for its database, IPC socket, and daemon lock. You can set `WATCHER_DEV_HOME` to an absolute directory to isolate separate QA sessions. The production profile uses the normal XDG data and runtime directories.

### On-device database upgrades

Watcher currently stores activity in **SQLite schema v2**. When an updated daemon starts, it checks the on-device schema and automatically runs any missing ordered migrations before opening IPC or tracking activity. Existing v1 databases receive a **consistent local SQLite backup** beside `watcher.sqlite` first; v2 adds only a time-range query index and keeps historical usage unchanged. Bad, unversioned nonempty, or newer-than-supported databases fail safely rather than being overwritten or silently downgraded. Backups remain private, local, and are not automatically removed.

See [database migrations and recovery](docs/plans/storage-migrations.md).

## Install for your user

```sh
bash scripts/install-user.sh
$HOME/.local/bin/watcherd
```

The installer builds release Rust executables, installs them in `~/.local/bin`, and installs the OpenTUI program into the user data directory. Bun is currently required to launch OpenTUI. The daemon must run in the actual Hyprland user session so that `HYPRLAND_INSTANCE_SIGNATURE`, `WAYLAND_DISPLAY`, and `XDG_RUNTIME_DIR` are available.

**No autostart is enabled automatically.** If desired, add `exec-once = ~/.local/bin/watcherd` to your own Hyprland configuration (check that `~/.local/bin` resolves correctly), or use a user systemd service with the correct graphical-session environment imported. This is optional, and running both startup mechanisms would be a mistake. To stop tracking, use `watcher shutdown`.

Uninstall with `bash scripts/uninstall-user.sh` after stopping the daemon. It removes binaries and the TUI but **does not delete your recorded activity**. The original Python installer in `legacy/` should not be used with this rewrite.

## Development and QA

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
(cd apps/tui && bun install --frozen-lockfile && bun run check && bun run build)
```

The daemon records app-level intervals, not only daily aggregates. Timezone/date boundaries are applied when querying. Queries to the same daemon give the CLI and TUI consistent totals. The JSON-lines IPC protocol is local to the current user; on Linux the daemon socket is private and limited to the user's session. For reproducible QA that does not interfere with a personal desktop, use the development profile; tests do not need to manipulate window focus.

See [Phase 1 implementation plan](docs/plans/phase-1.md), [Phase 2 plan](docs/plans/phase-2.md), [architecture](docs/plans/architecture.md), and [Phase 1 QA notes](docs/qa/phase-1.md).

## Limitations

- Only Hyprland is implemented. Cross-platform Rust builds in CI are **not** equivalent to a working Windows/macOS collector.
- Sway, X11, Windows and macOS collectors are implemented experimentally for Phase 2; GNOME/KDE have source-level opt-in focus bridges. All still require native/live verification. No WXT extension yet.
- Idle input protocol support must be available from the Wayland compositor. Screen locks, suspend/recovery, and clock anomalies can produce unknown coverage.
- Media playback time is not classified yet; media playing while idle is **not** counted as active interaction. Attention inference is not implemented.
- OpenTUI currently runs via Bun from an installed TypeScript package; fully standalone bundling will be addressed separately.
- The original implementation remains in [`legacy/`](legacy/README.md) for reference until the replacement is accepted.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CONTEXT.md](CONTEXT.md). MIT licensed.

## Phase 2 development / experimental collectors

The shared collector seam now includes **X11**, **Sway**, **Windows**, and **macOS** adapters. These are **not yet declared production-supported**: Windows and macOS must pass native builds and real desktop verification; X11's isolated Xvfb job and Sway's real compositor QA are pending. GNOME Wayland and KDE Plasma Wayland have **experimental, opt-in focus bridge source code** under `integrations/` (no automatic installation); neither bridge is verified on its corresponding desktop. Unsupported or missing focus signals must not be counted as active usage. See [Phase 2 implementation/dependency research](docs/plans/phase-2-implementation.md) and [QA matrix](docs/qa/phase-2.md).

Windows development builds use native named pipes and per-user data under `%LOCALAPPDATA%`; PowerShell setup scripts are in `scripts/install-user.ps1` and `scripts/uninstall-user.ps1`. macOS uses AppKit/CoreGraphics and user data under `~/Library/Application Support/watcher`. These scripts are not yet verified on Windows/macOS hosts. An X11-only graphical integration test is designed to run under an isolated Xvfb display, never against your active desktop.
