# Phase 1 QA — Linux / Hyprland

**Environment observed:** Arch Linux (x86_64), Hyprland 0.56.2, Rust 1.98.1, Bun 1.3.14, Chromium 148 and Firefox 157 present. User desktop is **shared**; QA must not programmatically change active windows, desktops, keyboard focus, idle state, or compositor configuration.

## Repeatable local checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
cargo test --workspace --locked
(cd apps/tui && bun install --frozen-lockfile && bun run check && bun run build)
git diff --check
```

All are safe without a graphical session. Tests use synthetic events for time attribution and in-memory/temporary SQLite databases.

## Passive real-desktop session

Test with `WATCHER_IDLE_SECONDS=5 ./target/debug/watcherd --dev` to shorten idle transitions. Do this only in an isolated `.watcher-dev` profile. **This never switches apps or moves focus itself.** Use ordinary user interaction as it happens; do not assume the user switched windows at a predetermined time.

1. `watcher --dev doctor`: both foreground and input-idle capabilities were detected on the real Hyprland session.
2. `watcher --dev status` showed a real current foreground application class. Neither window title nor URL was stored.
3. Over a short live session, real `active` → `idle` → `active` changes were observed while user input naturally came and went. Idle notifications from Wayland `ext_idle_notifier_v1` v2 were available.
4. `watcher --dev today`, `week`, `timeline` and OpenTUI were queried from the running daemon. Totals matched persisted rows; `sqlite3 ... 'pragma integrity_check'` returned `ok`; an overlap query returned zero overlaps.
5. `watcher --dev pause` / `resume` produced a separate paused interval without touching the desktop.
6. A second `watcherd --dev` refused the existing profile lock. `watcher --dev shutdown` exited cleanly and removed the socket. Subsequent CLI calls failed explicitly instead of manufacturing zero usage.
7. OpenTUI launched under a PTY, displayed real report and status responses, changed views using keyboard input, and restored the terminal on `q`.

8. A clean **release** build was installed with the per-user installer into a disposable QA prefix. The installed Rust CLI, daemon, and installed OpenTUI source/native dependencies worked together. No files were installed to the normal user bin directory.
9. Following daemon shutdown and restart, offline elapsed time was recorded as an unknown interval, shown separately instead of crediting it to any app.

Only a disposable QA database was used; the normal XDG Watcher data directory was untouched.

## Additional verification and known limitations

- **Focused-app transitions:** reducer scenarios and Hyprland event parser are covered by deterministic tests. The passive live session demonstrated focused-app discovery; a deliberately controlled *A → B* compositor focus transition was **not** forced on the user's working desktop. Don't claim this manual scenario passed until independently exercised.
- **Lock/unlock and suspend/resume:** interval gap handling and clock regressions are designed to avoid overcounting, but physical suspend and lock transitions were not intentionally triggered on the working machine. These need a dedicated session before an accuracy guarantee can be made for those cases.
- **Other OSes/compositors:** CI builds portable code on hosted Windows/macOS, but no Windows/macOS tracking is implemented in Phase 1. Remote GitHub Actions results only exist after branch push and workflow completion.
- **Service autostart:** intentionally not turned on during QA. `scripts/install-user.sh` supports per-user installation, without modifying user startup configuration.
- **Privacy:** the QA database contains real user activity classes from the passive run; it is gitignored. Don't upload the database or paste its raw contents into public issues.

## Phase 1 acceptance boundary

Core event processing, real Hyprland idle and focused-app observation, persistence, local IPC, CLI and OpenTUI are present. Full reliability sign-off should separately confirm manual app switching and lock/suspend/recovery in a non-disruptive environment. Keep `legacy/` until the maintainer explicitly accepts the replacement; no automatic deletion.
