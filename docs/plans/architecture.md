# Watcher — architectural contracts

Status: **proposed design**, agreed direction for the Phase 1 and Phase 2 plans. This is a clean-slate rewrite; existing Python code is reference material only.

## Product invariant

Watcher is a **local-first activity observability tool**, not a productivity or attention classifier. The first metric is **active interaction time**, defined in [CONTEXT.md](../../CONTEXT.md). A foreground app is necessary but insufficient: time is active only while the user's input-idle signal says they are non-idle. Where idle support is unavailable, the tracker **must expose reduced coverage** instead of silently calling focused time “active.”

Passive media, when detectable, is a separate view. The product must never double-count browser breakdowns as extra desktop time. No accounts, sync, screenshots, keylogging, content capture, or attention inference in Phases 1–2.

## Deployment and process ownership

```text
               native collectors (event-driven first)
       Hyprland / X11 / Sway / Windows / macOS / ...
                            |
                     observations
                            v
                  +-------------------+
                  | watcherd (Rust)   |
                  | collector manager |
                  | activity reducer  |
                  | single DB writer  |
                  +---------+---------+
                            |
                       SQLite WAL
                            ^
                            | read/query via daemon
                  local versioned IPC
                       /          \
              watcher CLI      OpenTUI (Bun/TS)
              (Rust)           (client only)

         future WXT extension -> native host -> daemon
```

All activity data belongs to the **per-user daemon**, which is the only writer and enforces time reconciliation. A CLI invocation or a TUI session must not open a second collector or write to the SQLite store independently. The TUI is a separate TypeScript client; the CLI remains a small, formatted, colored Rust program. No embedded HTTP server or cloud backend is needed.

## Modules (one job each)

| Module | Owns | Must not own |
| --- | --- | --- |
| `watcher-core` | Domain types, state transitions, interval reconciliation, report calculations | OS APIs, SQLite, terminals |
| `watcher-platform` | Collector capability probes and OS/compositor integrations | Storage policy, screen-time arithmetic |
| `watcher-storage` | SQLite schema, migrations, durable writes, projections | Focus/idle heuristics |
| `watcher-ipc` | Versioned request/response DTOs, transport and error contracts | Product calculations |
| `watcherd` | Single-instance lifecycle, collector supervision, clock, reducer and storage wiring | CLI/TUI presentation |
| `watcher` | Commands, simple colored/text + machine-readable outputs | Independent collection |
| `apps/tui` | OpenTUI rendering, keyboard navigation, querying the daemon | Collector access, database mutations |

Avoid a crate per compositor before there is a concrete need. Keep OS integrations behind a small collector seam. The technical framework behind the seam can use OS-specific code without making the reducer OS-specific.

## Proposed domain types

Illustrative, **not** a Rust API frozen in this scaffold:

```rust
enum InputState { Active, Idle, Unknown }
enum FocusState { Known(AppIdentity), Unknown }
enum Coverage { Supported, PermissionDenied, Unsupported, TemporarilyUnavailable }

struct Observation {
    source: SourceId,
    source_seq: Option<u64>,
    observed_at_utc: Timestamp,
    monotonic_marker: Option<MonotonicMark>,
    payload: ObservationKind,
}
struct ActivityInterval {
    start_utc: Timestamp,
    end_utc: Timestamp, // exclusive
    app: Option<AppIdentity>,
    input_state: InputState,
    source: SourceId,
}
```

Observations include focus changed, idle entered/exited, collector disconnected/recovered, session locked/unlocked, and suspend/resume. The reducer accepts observations in a documented order and emits interval mutations; the database commits these atomically. Unknown is not implicitly Active. `AppIdentity` favors stable IDs (class/app_id/executable/bundle identifier) over mutable window titles. Raw titles and exact URLs are off by default.

## Time and attribution invariants

1. Intervals are `[start, end)` in UTC; the viewer selects local timezone and date boundaries at query time. Split reporting windows at midnight or ISO week edges for aggregation, never by resetting active state every midnight.
2. Use a monotonic clock to measure live durations, with wall-clock timestamps for storage/reporting. Explicitly handle suspend/resume, wall-clock corrections, negative deltas, out-of-order signals, and daemon restarts; never let a sleep gap become active usage.
3. At most one foreground app is credited at a time **per tracked desktop/user session**. If focus is unknown, record unknown coverage rather than charge the previous app indefinitely.
4. Active usage = foreground app intersected with non-idle intervals. Non-foreground browser windows do not add time. Media playback cannot turn idle state into active user input.
5. Initial idle threshold is configurable (proposal: 60 seconds). On idle notification, backdate the active interval to the platform-reported last-input boundary when available; otherwise state the precision limitation. Do not claim exact subsecond accuracy from coarse polling.
6. Treat observation timestamps as untrusted when they come from external sources; clamp or reject impossible or stale values with diagnostics. Deduplicate reconnect/replay observations using source and sequence/identity where possible.
7. Limit loss on power failure through small batched commits/checkpoints, without flushing to disk once per second. Recovery closes open intervals at the last **reliably observed** instant and creates a coverage gap until collection resumes.

## Storage and IPC

- SQLite in OS-appropriate per-user **data** directories (not disposable cache). WAL, migrations, bounded writes, one writer; optional read transactions and derived indexed queries. Do not indefinitely store an unbounded debug event log unless explicitly opted in.
- Planned tables: `schema_migrations`, `activity_intervals`, `collector_health` / source metadata, and optional bounded diagnostic observations. Summaries are SQL projections, not truth independently maintained in weekly CSVs.
- Daemon `status/doctor`, `today/week/timeline`, `pause/resume` and `shutdown` commands cross a **versioned local IPC interface**. Include protocol version, typed errors, time range, timezone, capabilities, and coverage in responses.
- Linux/macOS: per-user Unix-domain socket (private runtime directory, ownership and restrictive permissions). Windows: named pipe with per-user ACL. Authenticate the local peer where feasible; never expose unauthenticated TCP by default.
- Define backpressure and bounded queues: collector reconnection with finite retries/backoff, no unbounded event buffering. Instrument queue overflows and collector downtime as coverage gaps.
- Provide `watcher doctor` and structured diagnostic logs with redaction. Avoid logging application titles, complete browser URLs, or private browsing context by default.

## Security, permissions and packaging

- Run as a **normal user** in the interactive desktop session; no root service or arbitrary desktop introspection permissions.
- Require explicit user opt-in for sensitive OS permissions; surface `PermissionDenied` separately from `Unsupported`.
- No side effects when running `cargo build`, `cargo test`, or merely opening the TUI. No automatic launch of the preserved legacy installer.
- A developer/QA profile redirects config, data, logs, and IPC into an isolated directory. Reset commands must refuse to touch the standard production path.
- Phase 2 platform support is only “verified” after a **live focus/idle exercise** on that OS/compositor, not by compiling against its SDK.
- Browser integration belongs to a later phase; reserve the architectural seam only. No browser extension implementation in Phases 1–2.

## Alternatives considered / scope control

- **No separate client/server over TCP:** a per-user daemon plus IPC has fewer security and lifecycle complexities, yet still gives the CLI and OpenTUI independent clients.
- **No wholesale Rust UI:** Rust handles the engine and CLI; Bun + OpenTUI handles terminal rendering where it is strongest.
- **No cloud, message broker, ORM abstraction layer, or enterprise plugin framework:** SQLite and composited collector adapters are sufficient.
- **No pretending all Wayland desktops expose focused windows:** capability probes, extensions, or compositor IPC are required per environment.
- **No legacy data migration until needed:** preserving the Python app is temporary; importing historical aggregate CSV cannot reconstruct past intervals.

## Design review questions before locking Phase 1 interfaces

- What should “unknown idle signal” contribute to totals? Default: **zero credited active time**, plus explicit unavailable coverage.
- What is the first-session idle threshold and whether users may override it? Default proposal: **60 seconds**, configurable.
- Which identifiers should be stored by default? Default: stable application ID/name and timestamps only.
- Which clock precision can a collector guarantee? Decide based on actual Hyprland + Wayland idle-protocol QA, not assumed support.
