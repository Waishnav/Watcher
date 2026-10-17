# Cross-platform CI and isolated compositor QA

Cross-platform checks belong on the **`feat/cross-platform-ci`** branch, stacked on **`feat/rust-rewrite-phase-1`**. The intended PR topology is `main ← feat/rust-rewrite-phase-1 ← feat/cross-platform-ci`; cross-platform CI does **not** modify production data or launch a collector against a user's actual desktop session.

## Check coverage

| Workflow | Trigger | What it can prove | What it cannot prove |
| --- | --- | --- | --- |
| `ci.yml` / Rust quality | PR, push to main, manual | Rust formatting, strict Linux Clippy, Unix shell script and JS bridge syntax | macOS/Windows runtime behavior |
| `ci.yml` / Rust 1.85 | PR, push to main, manual | Manifest's declared minimum supported Rust version builds under 1.85 | Native Windows/macOS toolchains |
| `ci.yml` / Rust native matrix | PR, push to main, manual | **Native** build of every workspace target, all Rust tests, CLI `--help` startup; matrix: Ubuntu 24.04, Windows 2025, macOS 15 | Desktop focus, input idling, screen lock, suspend, IPC access controls in a real session |
| `ci.yml` / OpenTUI matrix | PR, push to main, manual | Bun frozen dependency install, TypeScript typecheck and bundling on the same three OS runners | Human terminal interaction and native UI lifecycle |
| `ci.yml` / WXT extension matrix | PR, push to main, manual | Chrome and Firefox bundles, TypeScript, IndexedDB/domain tests on Ubuntu, Windows, macOS | Real Firefox sidebar/native-host runtime behavior |
| `ci.yml` / isolated Chrome browser runtime | PR, push to main, manual | A real headless Chrome extension with a temporary profile, offline tab A→B, nonfocused window, full browser restart, Native Messaging host, SQLite replay with inert collector | Firefox runtime, desktop-idle/lock or user-display behavior |
| `desktop-qa.yml` / X11 | PR, manual, weekly | A disposable Xvfb display with Openbox, app A→B focus, collector totals, SQLite integrity and no overlapping intervals | Real X11 hardware, lock/suspend |
| `sway-qa.yml` / experimental Sway | Manual dispatch once on main, or opt-in PR label `qa-sway` | Own headless wlroots compositor, private runtime/socket, separate foot windows, independent compositor vs Watcher focus A→B | Active input-idle tracking (headless environments may not expose v2), lock/suspend, GNOME/KDE |

None of these jobs uses a self-hosted runner or stores database secrets. Test databases and logs remain in disposable directories and are cleaned up by shell traps. The Sway script force-selects `WLR_BACKENDS=headless`, creates a **new** `XDG_RUNTIME_DIR`, and never reuses the caller's compositor IPC socket. X11 QA refuses to run unless `WATCHER_XVFB_QA=1` and `DISPLAY` belongs to an actual Xvfb process.

Sway's opt-in compositor test is deliberately **not a required pull-request check** until one first run is available to validate its headless environment and installer package behavior. PR maintainers can add the `qa-sway` label to exercise the job before merge. Direct `workflow_dispatch` becomes available after this workflow exists on the default branch (a GitHub restriction). The job should fail rather than mark focus QA successful when the compositor or client doesn't initialize.

The workflows use GitHub hosted Ubuntu, Windows and macOS runners with `fail-fast: false` in matrices so a failing OS does not cancel the other OS builds. CI pass means **code compiled and deterministic tests passed**, not that native focus/idle adapters are production-verified.

## Current constraints

- As of this change, the origin repository `Waishnav/Watcher` is **archived**, so pushes and pull requests cannot be created. A repository owner must unarchive it or point Git at a writable remote before these workflows can execute remotely.
- The current local Arch development machine has no installed Windows/macOS Rust standard library targets, no Xvfb, and no Sway compositor. Those checks must execute on their runners. Syntax checks, Linux Rust checks, and the safety guards can be performed locally.
- Do not unarchive or run desktop-changing QA without the owner's permission. A PR is not a substitute for independent interactive testing on Windows/macOS.

## Follow-up release gates

After CI is enabled and the native builds are green, fix any target-specific compile/link findings and separately test interactive Windows/macOS application switches, idle/resume, lock/unlock, sleep/wake, and CLI/TUI reports. GNOME/KDE bridges also need opt-in compositor compatibility tests. Keep the experimental Sway workflow manual until it has a reliable runner baseline.
