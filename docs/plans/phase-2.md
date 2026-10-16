# Phase 2 — Cross-platform collectors + honest compatibility

**Status:** adapters and portable plumbing in progress; native desktop QA pending. For actual implementation evidence and dependency research see [Phase 2 implementation](phase-2-implementation.md) and [QA log](../qa/phase-2.md).
**Goal:** same Watcher daemon, data model, CLI and OpenTUI on Linux X11, additional Wayland compositors, Windows, and macOS, with verified foreground and idle behavior wherever native APIs permit it.

Phase 2 is fundamentally an **integration and capability management** project, not an analytics rewrite. Refer to [architecture.md](architecture.md) for invariants, security, time semantics and module seams.

## Scope

### Included

- OS/compositor discovery and capability probes with distinct `Supported`, `Unsupported`, `PermissionDenied`, `TemporarilyUnavailable` results.
- Linux X11, Sway, Windows, and macOS native collector implementations and per-user launch/stop packaging.
- GNOME Wayland and KDE Plasma Wayland integration research/prototypes where required, with clear experimental/unsupported labels until verified.
- Minimal real-environment QA for all declared “supported” combinations; CI on Linux, Windows, macOS; separate X11 graphical tests.
- Collector reliability: disconnect/reconnect, session lock, suspend/resume, clock changes, app identity normalization, permission workflows.
- Portable IPC transport implementation (Windows named pipe, Unix sockets on Unix platforms) and platform-appropriate paths.

### Deferred

- Browser tab/domain tracking, WXT extension, browser native-host wiring (future phase).
- Attention inference, productivity classification, predictive insights, automatic cross-device synchronization, mobile OSes, stealth surveillance.
- Pretending GNOME, Plasma, and every Wayland compositor are interchangeable; support is a capability, not just an OS label.

## Collector seam and selection

The Phase 1 reducer receives the **same** normalized observation type on every platform. Adapters own OS permission checks, session-specific API connections, focus metadata and idle signals; they never calculate hours.

Some Windows/macOS native bindings may require `unsafe` FFI. Confine it to small reviewed adapter implementations, document pointer/lifetime assumptions, and expose safe observation types to the rest of the workspace. Keep the core, IPC and storage modules safe Rust.

Proposed internal contract (illustrative):

```rust
trait Collector {
    fn capabilities(&self) -> CapabilityReport;
    fn observe(&mut self, sink: ObservationSink, cancel: Cancellation) -> Result<(), CollectorError>;
}
```

Production behavior:

1. Identify OS, graphics session, compositor/desktop, and active interactive user session.
2. Probe collectors by *concrete capability*, not by checking `$WAYLAND_DISPLAY` alone. Select exactly one foreground owner for the current session, plus one trusted idle signal (possibly separate backend).
3. Emit normal observations with `source_id` and capabilities. If either focused-app or idle source disappears, emit an explicit coverage transition to unknown.
4. Reprobe after screen lock, compositor restart, login, sleep/resume or permission change. Never keep crediting the last known app without an upper bound.
5. Report selected and fallback collectors through `watcher doctor --verbose`. Save health transitions separately from usage.

## Operating-system workstreams

### A. Linux X11

**Adapter:** `linux_x11` in `watcher-platform`.

- Use XCB / EWMH (`_NET_ACTIVE_WINDOW`, `WM_CLASS`, relevant property notifications) to follow focused applications. Snapshot initially then subscribe to changes; handle window destruction/race and empty focus.
- Use XScreenSaver idle query/notifications or an equivalent native X11 input-idle signal; avoid spawning `xdotool`, `xprop` or `xprintidle` every second as the production strategy.
- Recognize multiple X displays, application ID normalization, lock screens and server disconnects. A nested XWayland instance is not full-X11 desktop support.
- QA: launch Xvfb or a dedicated X11 session, create two named windows, switch focus, idle/recover, query time intervals; compare against expected foreground/idle segments. Mark true idle test as unavailable if no real input-idle signal can be driven in the test display.

**Exit criterion:** deterministic real X11 window-focus integration and real-session idle QA, with no title-only heuristics or arbitrary command parsing.

### B. Linux Wayland: Sway and additional desktops

**Sway:** subscribe to Sway IPC focus/workspace/window events and query the tree to bootstrap state. Check version differences and IPC reconnect. Probe `ext-idle-notify` (or environment-supported idle source) separately. Ensure snapshot/subscription coherence.

**Hyprland:** reuse Phase 1 adapter, extend regression checks across supported releases and compositor restarts.

**GNOME Wayland:** evaluate a GNOME Shell extension or an explicitly sanctioned desktop interface. Assume there is **no universal compositor-independent focused-window query**. Require opt-in and account for GNOME version/API compatibility. If no reliable collector can be installed, say “foreground tracking unavailable.”

**KDE Plasma Wayland:** investigate KWin-sanctioned integrations and version-specific scripting interfaces. Avoid depending on incidental behavior of XWayland or unprivileged process enumeration. Package installation and permission details alongside any plugin.

**Other compositors:** capability-discover if a compatible protocol is actually present; otherwise leave unsupported. Do not announce general Wayland support based on a single compositor.

**Exit criterion:** Sway reaches supported only after real focus + idle QA. GNOME/Plasma may remain experimental/unsupported if their integration is not trustworthy; that is an honest Phase 2 outcome.

### C. Windows

**Adapter:** `windows`, behind `#[cfg(windows)]`.

- Foreground focus through Win32 `GetForegroundWindow` and related process/window metadata. Where appropriate, subscribe to WinEvent foreground notifications rather than busy polling.
- Input-idle through `GetLastInputInfo`; handle tick counter semantics, locked sessions, fast user switching, remote desktop and sign-out. Avoid session 0 and system services for interactive tracking.
- Create native Windows named-pipe IPC secured for the current user, using a single per-user daemon. User data in an OS-appropriate app data directory. Start at user logon with an explicitly installed task/autostart hook, not a machine-wide privileged service.
- Avoid sensitive window titles by default; report missing accessibility/permission requirements clearly.
- QA on a **real interactive Windows user desktop**: two focused windows, no input, resume, lock/unlock, terminate/restart. Hosted Windows Server CI compilation and API smoke checks are not enough to certify Windows desktop behavior.

**Exit criterion:** end-to-end active time totals observed and confirmed in an interactive Windows session, verified native pipe and per-user launch lifecycle.

### D. macOS

**Adapter:** `macos`, behind `#[cfg(target_os = "macos")]`.

- Use NSWorkspace's frontmost application / activation notifications for app-level foreground attribution; resolve stable bundle ID and process identity. Detailed per-window titles are optional and may require Accessibility privileges (not a default Phase 2 need).
- Use supported system idle-input APIs such as IOKit/IOHID system-state queries, confirm applicability on current macOS versions and permission settings, and implement bounded sampling if event subscription is unavailable.
- Handle screen lock, sleep/wake, fast user switching and app termination. Account for App Nap/lifecycle as needed; never let suspended collection stretch an open active interval.
- Use Unix socket IPC in user-private runtime path, per-user `launchd` agent on explicit installation, and a macOS-native app support directory.
- QA on a **real interactive Mac**: foreground app switch, inactivity, resume, lock/unlock, restart, TUI terminal restoration. GitHub-hosted macOS compile/tests do not guarantee keyboard/focus permission behavior.

**Exit criterion:** app focus + idle activity genuinely verified on supported macOS version(s), permissions documented and a launch agent cleanly uninstalls.

## Platform compatibility is a matrix, not a boolean

Maintain a compatibility table listing **OS and versions, desktop/compositor and versions, foreground capability, idle capability, packaging, and verification evidence**.

| Target | Baseline automated signal | Required manual/live proof before “supported” |
| --- | --- | --- |
| Linux Hyprland | Existing Phase 1 + regressions | Focus, idle, lock/sleep and app-switch QA |
| Linux X11 | Xvfb focus integration | Live X11 input-idle and session QA |
| Linux Sway | IPC fixture tests | Sway session focus + idle |
| Linux GNOME Wayland | Extension build/permission checks if implemented | Real versioned Shell session |
| Linux KDE Plasma Wayland | KWin integration build/probe if implemented | Real Plasma session |
| Windows | Windows hosted Rust compile/tests + API smoke | Interactive Windows desktop |
| macOS | macOS hosted Rust compile/tests + API smoke | Interactive Mac |

Do **not** equate CI green with desktop support. Record date, exact version, observed events, coverage gaps and known limitations for each supported entry. Tests should not require exposing user activity traces.

## Minimal CI design

### Required on every PR

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` on Linux.
- `cargo build --workspace`, `cargo test --workspace` on pinned Ubuntu, Windows and macOS GitHub-hosted runners.
- TUI `bun install --frozen-lockfile`, typecheck and bundle; start on Linux, expand to OS matrix as OpenTUI packaging evolves.
- Synthetic reducer/time-boundary fixtures, storage migration/open tests and collector capability/error-handling smoke tests.
- Compilation via `#[cfg]` on OS-native runner is important; a Linux-only cross-compile cannot prove correct Windows/macOS linking.
- Trigger on PR and push to main; no self-hosted runner on untrusted public PRs.

### Desktop integration workflow, incremental

- Start with Xvfb + controlled X11 window focus in a separate GitHub Actions job that produces logs and small structured interval traces.
- Add compositor-level integration only when a disposable session is reliable (e.g., dedicated Sway/Weston/Hyprland environment where dependencies and focus control are actually usable). Keep clear provenance: Weston does **not** prove Hyprland.
- Run heavier graphical tests manually or on a scheduled workflow; diagnostics retained as short-lived artifacts, with redacted titles and no personal URLs.
- For Windows/macOS interactive tests, use dedicated disposable VMs or accessible machines **after** securing untrusted code execution, rather than checking public PRs out on the maintainer's personal desktop.
- Do not let flaky or permission-constrained hosted environment smoke jobs masquerade as active-user desktop tests.

### Release gate

- Required CI green, plus a signed-off live QA checklist for **every platform whose support status is being newly promoted**.
- If a platform is unverified, ship with `experimental` / `unsupported` explicit, even when the binary works.
- Collect diagnostic reproduction commands and minimal event traces; avoid raw, personal browsing/window information.

## Reliability work and failure injection

For every collector, exercise:

1. Start with no foreground window; focus A, focus B, close B, switch workspaces, quit A.
2. Idle timeout triggers; no input then resume; input during multimedia playback is treated as input, playback alone does not force Active.
3. Suspend/resume and clock adjustment; session lock/unlock; compositor or Explorer/Dock restart.
4. Permissions denied, permissions granted mid-session, no idle protocol, temporary IPC loss, malformed/duplicated/reordered observations.
5. Daemon kill/restart and query during reconnection; no duplicate time or unbounded usage after disconnection.
6. Two independent user login sessions if feasible; never cross-attribute sessions/users.
7. No native crash from startup/shutdown; resource monitoring for periodic polling and idle callbacks.

Keep high-level QA recipes reproducible through `watcher doctor`, `watcher --profile qa status`, `watcher --profile qa today --json` (exact names finalized in Phase 1). Collector outputs should be inspectable without requiring writing a new test for every observation.

## Packaging and release readiness

- Distribution target: per-user installs, no root service. Provide OS-specific install/uninstall and autostart procedures, clear data-location and retention documentation.
- CLI + daemon are Rust builds. OpenTUI is a separate Bun/TypeScript application; the scaffold development bundle externalizes native packages and still needs installed dependencies. Verify release artifact/native-library compatibility on each OS before claiming a one-command bundle.
- Avoid invasive permissions for capabilities we don't ship. Signing/notarization requirements on macOS and Windows release security prompts must be documented before general availability.
- Respect `NO_COLOR`, accessible/UTF-8 output, terminal restore behavior and non-interactive CLI across OSes. Commands and JSON format should not depend on shell-specific features.
- Report fallback collectors, missing capabilities and protocol/API version incompatibilities prominently.

## Work breakdown (in order)

1. **P2-A — portability gate:** stabilize core/IPC abstractions, paths, per-user daemon lifecycle and all-OS build matrix.
2. **P2-B — X11:** native foreground observation, idle detection, Xvfb workflow and live X11 QA.
3. **P2-C — Sway:** IPC adapter, idle capability, disposable compositor QA; regression on Hyprland.
4. **P2-D — Windows:** Win32 collector, named-pipe IPC, per-user packaging and real desktop QA.
5. **P2-E — macOS:** NSWorkspace/idle collector, launch agent/permissions, real desktop QA.
6. **P2-F — GNOME/KDE evaluation:** integration prototypes and capability-based support decision; do not block verified targets on uncertain compositor extensions.
7. **P2-G — release matrix:** regression runs, documentation, installers, resource characterization and versioned support matrix.

Workstreams may be parallelized, but do not promote a platform to supported before it passes its own live verification.

## Acceptance checklist

- [ ] Portable crates, IPC DTOs and reporting semantics unchanged across OSes
- [ ] Linux, Windows, macOS compile/test on GitHub-hosted runners
- [ ] X11 active-window integration passes isolated Xvfb exercise and real idle QA
- [ ] Sway observed correctly in an actual compositor session
- [ ] Windows verified interactively, including idle, lock/resume and single-user daemon
- [ ] macOS verified interactively, including idle, lock/resume and per-user launch agent
- [ ] GNOME/KDE status labeled honestly, with documented limitations/prototypes
- [ ] No phantom active time during disconnections/unknown idle coverage on any claimed supported platform
- [ ] Minimal CI and a separate desktop QA workflow with safe diagnostic artifacts
- [ ] Colorful Rust CLI and OpenTUI reports have cross-platform parity
- [ ] Permissions, isolation, install/uninstall and privacy policy documented

## Risks and explicit decisions to revisit

- **Wayland is not a universal API.** Other desktops may need extensions/scripting; unsupported is an acceptable outcome until evidence says otherwise.
- **Hosted CI is often headless.** APIs may be present while user focus/idle aren't; use interaction-capable QA to earn support claims.
- **Observer accuracy varies.** Track capability and precision metadata; do not compare platform totals to millisecond exactness when signals lack timestamps.
- **No browser integration in Phase 2.** Keep room in observation source metadata for later browser enrichment without double-counting desktop activity.
- **No legacy deletion tied to Phase 2.** Legacy cleanup is gated by Phase 1 acceptance as documented there.
