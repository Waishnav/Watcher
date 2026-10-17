# Phase 1 — Rust foundation + Hyprland end-to-end

**Status:** implementation completed in the rewrite branch; local CI-equivalent checks and passive Hyprland QA passed. Full real-session acceptance still needs non-disruptive controlled focus-switch and suspend/lock checks, plus hosted CI results. See [QA evidence](../qa/phase-1.md).
**Goal:** a real local Watcher instance on Linux/Hyprland that records foreground **active interaction time**, survives routine desktop changes, and returns correct daily/weekly totals through a Rust CLI and an OpenTUI client.

Read [architecture.md](architecture.md) and [the domain language](../../CONTEXT.md) first. This phase is a thin, verified vertical slice, **not** a rewrite of every Python feature.

## Scope and non-goals

### Included

- Per-user `watcherd`, foreground for development, single-instance in production.
- Event-driven Hyprland foreground window observation and a reliable supported idle signal, with explicit degraded state when signals are unavailable.
- Interval reconciliation, SQLite database, crash recovery, day/week/timeline queries, a minimal colored Rust CLI.
- A working OpenTUI view for today, apps, current status, with normal keyboard navigation/exit.
- Local IPC with typed/versioned requests, a `doctor` command, isolated QA/dev profile.
- Minimal CI for all supported Rust targets, TypeScript TUI typecheck/build, deterministic core invariants; hands-on DevSpace Hyprland QA.

### Explicitly excluded

- Windows, macOS, X11, Sway, GNOME and Plasma collectors (Phase 2).
- WXT browser extension, Native Messaging, domain/URL breakdown (future phase).
- Attention inference, productivity scoring, cloud sync, screenshots, terminal contents, keystroke capture, categories, blocking apps, GUI.
- Legacy CSV migration. Old code stays read-only in `legacy/` until this phase is accepted.

## Step 0 — repository baseline (setup work)

**Deliverables before implementing any activity logic:**

1. Cargo workspace: `watcher-core`, `watcher-platform`, `watcher-storage`, `watcher-ipc`; binaries `watcher` and `watcherd`. TypeScript/Bun `apps/tui` based on `@opentui/core`.
2. Root development instructions, `.gitignore`, Rust toolchain declaration, lockfiles, minimal GitHub Actions checks, proposed architecture and acceptance criteria.
3. All previous Python source, historical scripts, and old docs moved under `legacy/` unchanged (not installed and not called).
4. Run the baseline build, format, typecheck, CI-equivalent commands locally. Confirm project setup does not start any persistent process or alter real activity data.

The initial scaffold was not completion of Phase 1; the binaries and TUI are now functional. This section is retained as the original implementation plan.

## Step 1 — formalize the domain and reducer

### Implementation

- In `watcher-core`, create explicit `AppIdentity`, `SourceId`, `Observation`, `InputState` (active/idle/unknown), `FocusState`, `Coverage`, and `ActivityInterval` types.
- Favor a pure `reconcile(previous_state, observation) -> (new_state, interval_changes)` interface. Pass a clock reference into orchestrators; do not read system time in reducer rules.
- Represent time as monotonic elapsed spans for measuring and UTC wall-clock boundaries for storage; both need sensible overflow and clock-jump behavior. Track the last trustworthy observation per source.
- Close intervals at transitions rather than accumulating `+1 second` per poll. Represent unknown/unobservable time as a gap, not as continuous activity.
- Define source precedence and deduplication; a new focus event, idle event, lock/unlock, suspend/resume, disconnect and recovery must all be representable.
- Ensure adjacent half-open intervals can be aggregated without gaps or overlaps when coverage is known. Never assign one interval to two apps.
- Decide a configurable idle timeout, initially 60 seconds. Backdate idle transition to last input when the native source supports it; otherwise mark precision as approximate.

### Verification

- Deterministic tests using synthetic events + virtual clock for app switches, idle enter/exit, duplicated/reordered events, midnight rollovers, sleep/resume, wall-clock changes, daemon crash.
- Property-level invariant check: duration is non-negative; activity intervals for one session do not overlap; per-app active total never exceeds covered non-idle elapsed time.
- Explicit fixtures for missing idle capability. No fake “active” totals if the idle source is unknown.

**Gate:** events in, validated interval operations out; no OS, database, or terminal in the core test interface.

## Step 2 — durable SQLite model and analytics

### Implementation

- `watcher-storage` owns migrations, one writer, indexed `activity_intervals`, source/coverage metadata, and read-only queries. Favor `rusqlite` + SQL over a large ORM; confirm the dependency set when implementing.
- Use WAL, bounded transaction batches, prepared statements, and explicit schema versioning. Store intervals/coverage transitions, **not just daily CSV aggregates**.
- Implement idempotent upsert/finalization and recovery markers. Persist the last safe instant frequently enough to bound data loss without a database write per input poll.
- Implement query semantics: `today`, `week` (ISO Monday–Sunday), `timeline --date`, `apps --range`. Split UTC intervals against the user's selected local timezone at query time; handle DST and non-whole-hour offsets.
- Show coverage gaps and unavailable idle signals alongside the time total, so zero activity is distinguishable from absent data.
- Store data in OS-native user data location; QA profile gets a separate path and must never rewrite the real user's database.

### Verification

- Run queries against a disposable database with known focus/idle transitions; compare active totals to expected intervals.
- Restart daemon/storage mid-session and ensure at most the bounded, explicitly recorded uncertainty is lost.
- Inspect SQLite manually with `sqlite3`; verify WAL/migrations, no overlap and no duplicate daily totals.

**Gate:** time projections agree with source intervals, including cross-midnight reporting and restart.

## Step 3 — Hyprland collector and idle capability

### Implementation

- Build `watcher-platform` with a small observation-stream interface, `capabilities()`, lifecycle start/stop, and typed errors. No app usage calculations here.
- Discover active Hyprland instance/IPC endpoints from the actual **current user session**; query initial active window and subscribe to focus/window events over Hyprland IPC. Resolve the snapshot/subscription race (subscribe then reconcile snapshot, or equivalent verified behavior).
- Prefer stable class/app ID metadata; avoid persisting mutable window titles by default. Handle workspace changes, fullscreen windows, multiple monitors, no focused window, compositor restart, transient disconnected socket.
- Probe a **real user-input idle signal** available under the environment (e.g., compositor-supported idle notification protocol). Do not equate audio playback with human input. On unsupported idle notifications, report `Unsupported` and stop presenting totals as verified active interaction time.
- Include bounded reconnect/backoff, health/capability transitions, structured diagnostic observations, and shutdown cleanup.
- Distinguish collectors' raw event timestamp uncertainty from reducer timestamps; record approximate accuracy if the backend cannot provide precise last-input instant.

### Verification (real Hyprland desktop)

1. `watcher doctor` confirms correct compositor session, focus capability, idle capability and permission state.
2. Foreground terminal for a controlled period, switch to Chromium, switch back. Verify the database has contiguous, non-overlapping intervals attributed to correct apps.
3. Wait beyond configured idle timeout, then resume input. Verify active interval is closed at reasonable boundary; idle is separate, not credited to active.
4. Switch workspaces, lock session, sleep/resume (manual controlled QA), close all windows, disconnect/restart collector. Confirm no runaway attribution.
5. Inspect redacted diagnostic logs and developer database, verifying that personal titles and full URLs aren't accidentally stored.

**Gate:** demonstrate actual recorded focus and idle transitions on Hyprland, not merely mock tests or a successful build.

## Step 4 — daemon lifecycle and IPC

### Implementation

- `watcherd` exclusively owns collector, reducer, database and local IPC listener; enforce one daemon per user+profile. Second launch reports existing daemon rather than starting concurrent writes.
- Foreground `--dev`/debug mode first. Distinguish fatal startup failure, unsupported collector, and partial capability operation. Do not hide errors in detached logs.
- Versioned, typed IPC requests: `status`, `doctor`, `summary`, `timeline`, `pause`, `resume`, `shutdown`. Include explicit errors, request deadlines and protocol version mismatch behavior.
- Unix-domain socket under a private runtime directory on Linux, strict file permissions. Client connection failure gives friendly actionable error. Avoid an HTTP server for v1.
- Support developer profile/environment overrides (`WATCHER_DEV_HOME` or an explicit `--profile dev`, final naming to be decided); config/data/log/socket all redirected. No ability for a generic reset operation to delete the production data directory.
- On signals/shutdown, flush one final safe commit, close an open interval without counting post-exit time, and release IPC/socket resources. Background login/startup integration only after live verification.

### Verification

- Concurrent `watcher today`/TUI reads while daemon writes; one-writer behavior; graceful shutdown and interrupted process restart; stale IPC socket recovery; bad permissions; daemon unavailable.
- Kill/start manually in isolated QA profile, observe that changes are reflected within a bounded query delay.

**Gate:** process lifecycle robust and diagnostics actionable in a controlled local profile.

## Step 5 — interfaces: colored Rust CLI + OpenTUI

### Rust CLI

- Use `clap` (proposed) for `watcher today`, `watcher week`, `watcher apps`, `watcher timeline`, `watcher status`, `watcher doctor`, `watcher pause/resume`.
- Default human-facing output follows the original Watcher spirit: simple aligned tables, readable ANSI color (auto-disable on non-TTY or `NO_COLOR`), compact time formatting, zero/empty states, and sensible process exit codes.
- Offer JSON on read commands for scripting; time totals always include coverage caveats/metadata when relevant.
- `watcher tui` delegates to the separate OpenTUI application with documented installation resolution, not a second implementation of analytics.

### OpenTUI

- Bun + `@opentui/core`, TypeScript. Single client layer querying versioned IPC. Initially: overview/today, per-app totals, timeline list, collector health. No React framework unless UX complexity justifies one. The scaffold's `bun run build` externalizes dependencies to avoid bundling native binaries for all OSes; standalone packaging requires a separate verified design.
- Keyboard navigation `1/2/3`, `q` to quit, refresh with bounded scheduling and cleanup, resize, short terminals, noninteractive failures, CTRL+C restoration.
- Reuse daemon summary DTOs; the CLI and TUI must show the same calculations for the same time zone and range.
- Provide clear empty/offline states, not fake data. Support a dev-only deterministic dataset view for layout QA.

### Verification

- Check CLI in color and plain/redirected modes; compare JSON response against SQLite projections.
- Run OpenTUI under a real PTY/`tmux`: navigate, resize, exit, inspect terminal recovery. Compare displayed totals to CLI, disconnect daemon while TUI open, confirm graceful error.

**Gate:** all three perspectives (SQLite, CLI, TUI) match for a fixed query.

## Step 6 — release readiness and legacy retirement gate

- Phase 1 CI checks formatting, Clippy, Rust builds/tests on Linux, Windows, macOS (portable crates and scaffolds), and OpenTUI typecheck/build. A passing foreign-OS build does **not** claim collector support.
- Verify Hyprland interactive QA locally in DevSpace using an isolated data directory; do not automate focus switching on the user's everyday session.
- Add a safe per-user installation/startup procedure with uninstall/stop instructions, never root service or remote shell-piped installer as baseline.
- Document known capability limitations, resource use (CPU/memory/DB size), permissions and data retention defaults from measurements.
- Require an explicit Phase 1 acceptance: real active time observed; idle threshold behavior explained; daemon restarts safely; correct week/day reports; OpenTUI and CLI render correct values.
- **Only after these gates pass:** propose deleting `legacy/` in a separate change. Until then, retain all old files and Git history.

## Suggested acceptance checklist

- [x] Live Hyprland foreground snapshot and idle signal verified (controlled focus-switch QA pending)
- [x] No active attribution during unknown or disconnected coverage
- [ ] No inflated totals through focus switches, suspend, or midnight
- [x] SQLite initial schema, restart gaps, and query results verified
- [x] One daemon per profile; safe stop and restart
- [x] Colored Rust CLI and OpenTUI show identical real totals
- [x] Dev profile, redacted diagnostics, doctor; no destructive reset command exposed
- [ ] Linux + Windows + macOS portable builds; actual Hyprland desktop QA
- [x] Documentation covers installation, known limitations and how to reproduce failures
- [ ] Maintainer accepts replacement; legacy retirement separately approved

## Proposed implementation sequence (reviewable changes)

1. **P1-A:** core events/reducer and synthetic fixtures.
2. **P1-B:** schema/migrations/queries with disposable DB.
3. **P1-C:** Hyprland observation adapter and idle capability/health.
4. **P1-D:** daemon IPC/lifecycle and dev profile/doctor.
5. **P1-E:** CLI and OpenTUI connected to real IPC.
6. **P1-F:** real desktop QA, bug fixing, packaging, documentation and acceptance.

Don't turn every step into a new crate. Each review should have one observable contract, a minimal verification story, and no fake cross-platform support claims.
