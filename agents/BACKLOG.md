# micro-watcher Modernization Backlog

- **Source:** [OUTLINE.md](./OUTLINE.md) (items O-1 … O-11)
- **Status:** In progress — MW-001 … MW-003 complete; MW-006, MW-017, MW-019, and MW-035 are partially complete; MW-004 is next
- **Ordering rule:** Each milestone is safe to ship on its own. Do not start a later milestone before its dependencies are checked off.
- **Confirmed decisions:** Python stays (D1); distribution `micro-watcher`, import `micro_watcher`, command `micro-watcher` (D2); GNOME Wayland → KDE → wlroots, X11 preserved (D3); history preserved via migration (D4); Python 3.11+ (D5); browser domains only, focused tab for screen time (D6/D8); no repo rename (D7).

---

## At a glance

| Milestone | Theme | Tasks | Exit criterion |
| --- | --- | --- | --- |
| M0 | Safety stopgap | MW-001 … MW-004 | Nothing in the repo clones remote code, edits shell rc files, or accepts traversal/injection input |
| M1 | Runnable, packaged, tested | MW-005 … MW-010 | `pipx install .` works without root and CI runs tests |
| M2 | Security and correctness | MW-011 … MW-015 | No `os.popen`, typed CLI, correct time/week math, report tests |
| M3 | Backend abstraction and Wayland | MW-016 … MW-020 | `micro-watcher doctor` reports a working backend on GNOME Wayland; X11 preserved |
| M4 | Durable storage | MW-021 … MW-025 | Crash-safe history, CSV migration, reports read one source of truth |
| M5 | Lifecycle and configuration | MW-026 … MW-028 | systemd user service, config file, privacy/export controls |
| M6 | Browser domain tracking | MW-029 … MW-034 | Domain-only focused-tab reports, local extension/native host, separate background media metric |
| M7 | Docs, cleanup, release | MW-035 … MW-038 | Verified README, clean tree, tagged release |
| M8 | Distribution packages | MW-039 … MW-043 | AUR and `.deb` artifacts install/uninstall cleanly; channel policy documented |

### Why this order

1. **M0 first.** The current installer downloads unpinned code and runs it as root, deletes lines from `~/.xinitrc`, and the CLI accepts shell metacharacters in date/week arguments. Those are live hazards, not refactors.
2. **M1 before any large refactor.** Tests and packaging are the safety net. Without them, every subsequent change risks silently corrupting reports.
3. **M2 before backend/storage work.** Backend code should not inherit `os.popen`; storage keys and CLI parsing must be stable first.
4. **M3 (Wayland) then M4 (storage), or in parallel.** Wayland is the adoption blocker; storage is the trust blocker. They touch different modules after M2 and can be developed concurrently by separate contributors.
5. **M5 after M3/M4.** Config keys and the service entry point should reflect the final backend/storage shape.
6. **M6 after storage/config/privacy.** Browser domains need a durable schema, ignored-domain controls, and a reliable browser-focus signal; they should not be bolted onto the CSV format.
7. **M7 last,** except for the M0 README safety patch. Release only after the documentation commands have been executed on a clean system.
8. **M8 after the release candidate.** AUR and Debian packaging need stable install paths, the systemd unit, and the GNOME extension as package data. `pamac`/`yay` consume AUR; an official `pacman` repository needs a Trusted User sponsor.

### Workflow rules

- One task per pull request; reference the task ID in the commit and PR title.
- Every behavior-changing PR adds or updates a test. Pure refactors must keep existing tests green.
- Do not mix refactoring and behavior changes in the same PR.
- Data migrations must be idempotent, backed up, and tested against a copy of real data.
- Update the checkbox here in the same PR that completes the task.
- Keep `OUTLINE.md` as the rationale; this file is the execution plan.

---

# M0 — Safety stopgap

*Ship this even if everything else stalls. It removes the actively harmful behavior without changing how tracking works.*

### MW-001 — Remove the network-cloning installer and `curl | bash` path

- [x] **Scope**
  - **Completed 2026-11-20:** `install` now copies the local checkout into `~/.local/share/micro-watcher` without network or `sudo`; `update` delegates to it; the README no longer uses `curl | bash`.
  - **Size:** S · **Depends on:** — · **Outline:** O-1, O-9
  - **Why now:** The installer currently ignores the checkout, clones an upstream branch, and installs it with `sudo`. A fork's installer must never install someone else's unpinned code.
  - **Work:**
    - Delete `git clone … && cd Watcher` from `install`; require execution from the repository root and fail with a clear message otherwise.
    - Make `install` copy from the local checkout only; remove any network access.
    - Replace the destructive `rm -rf /usr/share/Watcher/*` in `update` with a safe, non-destructive update path (or mark it deprecated in favor of the M1 packaging flow).
    - Remove the `bash <(curl …)` instructions and the contradictory second `./install` step from `README.md`; document the temporary "run from checkout" command until M1 lands.
  - **Done when:**
    - `rg "git clone|curl|wget" install update README.md` finds no installation path that downloads code.
    - `install` run from a copy without the checkout exits nonzero without writing anything.
    - A reviewer can trace every installed file back to the checkout.

### MW-002 — Stop modifying `~/.xprofile` and `~/.xinitrc`

- [x] **Scope**
  - **Completed 2026-11-20:** The installer and uninstaller no longer touch `~/.xprofile` or `~/.xinitrc`; the installer prints manual-start instructions, and uninstall keeps data unless confirmed.
  - **Size:** S · **Depends on:** MW-001 · **Outline:** O-1, O-6
  - **Why now:** `sed -i '/xprofile/d' ~/.xinitrc` deletes unrelated user configuration, and the append-only autostart has no uninstall path. This is destructive and unnecessary.
  - **Work:**
    - Delete the `sed` block and the legacy `watcher --start &` append from `install`.
    - Document manual start (`micro-watcher --start`) until the systemd service exists in MW-027.
    - Make `uninstall` use `read -r` and avoid deleting data unless explicitly confirmed.
  - **Done when:**
    - No installer or uninstaller reads or writes shell startup files.
    - `rg "xprofile|xinitrc"` only finds documentation of the legacy cleanup in MW-027.

### MW-003 — Hotfix input validation and shell injection

- [x] **Scope**
  - **Completed 2026-11-20:** Added `src/micro_watcher/validation.py`, validated dates/weeks at every path boundary, removed user input from the shell, and added 25 passing regression tests. The CLI rejects 8-character payloads such as `x;id;ls ` and `../../xy` before any file or command is touched.
  - **Size:** S · **Depends on:** — · **Outline:** O-4
  - **Why now:** `-ws` accepts any 8-character string and passes it to `date -d "<arg>"`, and both date/week values are concatenated into file paths. This is exploitable today; it cannot wait for the full `argparse` refactor.
  - **Work:**
    - Add validators: date `YYYY-MM-DD` that resolves to a real calendar date; week `Www-YYYY` with `01 ≤ ww ≤ 53`.
    - Full-match, reject `.` / `/` path segments and any shell metacharacters before use.
    - Validate before constructing file paths or invoking external commands.
    - Replace the `date` shell invocation reached by user input with Python `datetime` (implemented in the M0 hotfix; the general `subprocess` refactor remains in MW-011).
    - Add regression tests for `a;id;ls ` (8 chars), `../../xy` (8 chars), and `../../etc/` (10 chars).
  - **Done when:**
    - Malicious payloads exit with a usage error and create/touch no file outside `~/.cache/Watcher`.
    - The shell-injection call path no longer exists.
    - Tests or a documented script reproduce the checks.

### MW-004 — Interim crash safety for the CSV writer

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-001 · **Outline:** O-5
  - **Why now:** The day file is truncated and rewritten every second. A crash mid-write loses the whole day, and two instances overwrite each other. This is a stopgap until the storage layer in M4.
  - **Work:**
    - Write to a temporary file in the same directory, `flush`/`fsync`, then `os.replace` onto the target.
    - Add a per-user lock (for example `fcntl.flock` on a lockfile) so a second `micro-watcher --start` exits with a message instead of clobbering data.
    - Handle `SIGINT`/`SIGTERM` by finishing the current write before exiting.
    - Remove the per-second `print(data)` that floods the terminal/journal.
  - **Done when:**
    - `SIGKILL` during a simulated run leaves the last complete file parseable.
    - Starting a second instance exits nonzero with an explanatory message.
    - No per-sample stdout spam.

---

# M1 — Runnable, packaged, tested

*Goal: a fresh checkout installs into an isolated environment without root, and CI exists before refactoring begins.*

### MW-005 — Add `pyproject.toml` and console entry point

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-001 · **Outline:** O-2, O-8
  - **Why now:** There is no package metadata, no dependency declaration, no installable entry point, and the version is a hardcoded string. Nothing downstream (CI, `pipx`, distro packaging) works without this.
  - **Work:**
    - Create `pyproject.toml` with name, description, license, `requires-python = ">=3.11"`, classifiers, and project URLs.
    - Single-source the version from package metadata; remove the hardcoded `2.0.0`.
    - Declare the console script `micro-watcher = "micro_watcher.cli:main"`.
    - Add optional `dev` dependencies (`pytest`, `ruff`, `build`).
    - Choose and document a build backend; verify an offline build with `--no-build-isolation` for environments without network.
  - **Done when:**
    - `python -m build` produces both sdist and wheel.
    - Installing the wheel in a clean venv provides `micro-watcher` on `PATH`.
    - `micro-watcher --version` and `importlib.metadata.version("micro-watcher")` agree.

### MW-006 — Finalize the `src/micro_watcher/` package layout

- [ ] **Scope**
  - **Partial 2026-11-20:** Added `src/micro_watcher/__init__.py`, converted modules to absolute `micro_watcher.*` imports, and renamed the module directory/entry point. The CLI shim, `python -m micro_watcher`, and packaging remain.
  - **Size:** M · **Depends on:** MW-005 · **Outline:** O-2, O-10
  - **Why now:** The module directory is now `src/micro_watcher` with absolute imports, but the CLI still mutates `sys.path` and there is no `cli.py`/`__main__.py`. Packaging is only real once the checkout and installed layouts are identical.
  - **Work:**
    - Add `src/micro_watcher/__init__.py`; move the CLI logic to `src/micro_watcher/cli.py` and add `src/micro_watcher/__main__.py`. Keep `src/bin/micro-watcher` only as a thin development shim or remove it after packaging.
    - Convert imports to absolute package imports (`from micro_watcher.analysis import …`); remove `sys.path.insert`, `/usr/share` references, and the `src/bin` path assumption.
    - Delete dead modules or scripts only after confirming nothing references them.
    - Update developer commands and temporary install docs.
  - **Done when:**
    - `python -m micro_watcher --help` and `micro-watcher --help` work from any directory after install.
    - `rg "/usr/share/Watcher|sys.path" src/` returns nothing.
    - Running from the checkout fails only with an actionable dependency message, never `ModuleNotFoundError`.

### MW-007 — Detect runtime tools with actionable errors

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-006 · **Outline:** O-2, O-3
  - **Why now:** Missing `xdotool`, `xprintidle`, `xprop`, or `pactl` currently produces empty output, tracebacks, or silent misbehavior. Users cannot tell what is wrong.
  - **Work:**
    - Add a small dependency-check module used at startup and by future `micro-watcher doctor`.
    - Report the missing tool, the feature it powers, and a distro-neutral install hint.
    - Distinguish required (backend) from optional (media/AFK) tools.
  - **Done when:**
    - On a machine without X11 tools, `micro-watcher --start` prints a clear unsupported-session message and exits nonzero.
    - No uncaught `ValueError`/`FileNotFoundError` from missing binaries.

### MW-008 — Add the pytest scaffold and characterization tests

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-006 · **Outline:** O-8
  - **Why now:** There are no tests, and the riskiest logic (time arithmetic, ISO weeks, aggregation) is about to be rewritten. Tests must exist before refactoring, even if some document current bugs.
  - **Work:**
    - Add `tests/` with `conftest.py` fixtures for a temporary `HOME` and temporary data directory.
    - Test `time_operations` (`time_difference`, `time_addition`, `convert`, `format_time`, `convert_into_sec`) and report aggregation.
    - Test week/date helpers against year boundaries, week 1, week 53, and the `6.9`-day hack in `get_sunday_of_week`.
    - Mark known-invalid behavior with `xfail` referencing the M2 task that fixes it, rather than enshrining it.
  - **Done when:**
    - `pytest` passes offline in a fresh venv.
    - The suite covers zero, sub-minute, multi-hour, >24-hour totals, and midnight crossings.
    - Every `xfail` has a task ID.

### MW-009 — Add `ruff` and CI

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-005, MW-008 · **Outline:** O-8
  - **Why now:** Contributors have no objective definition of passing, and lint would have caught unused imports, shadowed loop variables, and dead code.
  - **Work:**
    - Configure `ruff check` and `ruff format --check` in `pyproject.toml`.
    - Fix only mechanical issues first; keep behavior changes separate.
    - Add `.github/workflows/ci.yml` running tests and lint on push/PR for Python 3.11 through the current stable release.
  - **Done when:**
    - CI is green on the default branch.
    - A deliberately broken test or lint violation fails CI.
    - The README/developer docs state the exact local commands.

### MW-010 — Rework `uninstall` and retire the destructive `update`

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-005, MW-006 · **Outline:** O-1, O-6
  - **Why now:** `update` clears the install directory before copying, and `uninstall` uses `rm -rf` on fixed system paths while leaving startup entries behind. Packaging changes the upgrade story.
  - **Work:**
    - Replace `update` with documented `pipx upgrade` / `pip install -U` flows; delete the script or make it a thin, safe wrapper.
    - Make `uninstall` remove the service/autostart entries created by M5, legacy binaries, and only prompt for data deletion with a safe default.
    - Remove unconditional `rm -rf` globs; quote paths; verify the target belongs to this app.
    - Clean up legacy `~/.xprofile` lines by exact match only, and only when the user opts in.
  - **Done when:**
    - Uninstall on a clean install leaves no Watcher files except user data (when declined).
    - No wildcard deletion remains.
    - `shellcheck install uninstall` reports no errors (if the scripts survive M1).

---

# M2 — Security and correctness

*Goal: remove the shell layer, make date/week handling correct and testable, and replace ad-hoc argument parsing.*

### MW-011 — Replace `os.popen` with a `subprocess` runner

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-003, MW-008 · **Outline:** O-4
  - **Why now:** Shell strings break on quotes/newlines in window titles and are the root cause of the injection vector. A single runner also makes mocking and timeouts possible.
  - **Work:**
    - Add a helper that calls `subprocess.run(args, capture_output=True, text=True, timeout=…)` and returns a structured result.
    - Migrate `get_windows.py`, `afk.py`, `watch_log.py`, `analysis.py`, and `commands.py`.
    - Parse `xprop` by property name rather than fixed character offsets; handle missing/denied windows explicitly.
    - Remove command substitution (`$(xdotool …)`) and all `shell=True`.
  - **Done when:**
    - `rg "os.popen|shell=True" src/` returns nothing.
    - Window titles containing `"`, `'`, tabs, and newlines do not break tracking.
    - Missing tools produce the MW-007 error, not an exception; tests monkeypatch the runner.

### MW-012 — Replace `date(1)` with `datetime` and fix ISO week math

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-008 · **Outline:** O-4
  - **Why now:** Localized/human-readable `date` output and the `timedelta(days=6.9)` hack are fragile across locales and year boundaries. Screen-time reports depend on exact dates.
  - **Work:**
    - Replace `get_date`, yesterday, current-week, previous-week, and weekday calls with `datetime.date`/`datetime` and `timedelta`.
    - Rewrite `get_sunday_of_week` using `isocalendar()` / `fromisocalendar()`; validate week ranges.
    - Keep `xprintidle` as a subprocess (it is hardware idle time, not a clock), but wrap it per MW-011.
    - Ensure the process is locale-independent.
  - **Done when:**
    - No `date +` / `date -d` command remains.
    - Table-driven tests pass for ISO week 1/52/53, Dec/Jan boundaries, leap years, and yesterday/today across midnight.
    - The default week matches `datetime.date.today().strftime("%G-W%V")` semantics (ISO year/week).

### MW-013 — Replace `sys.argv` parsing with `argparse`

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-003, MW-012 · **Outline:** O-4
  - **Why now:** The current parser accepts arguments by length, prints a generic error, evaluates defaults at import time, and has no place to attach validation. Divergent behavior between flags is easy to introduce.
  - **Work:**
    - Preserve the documented interface: `-ds/--day-summary [date]`, `-y/--yesterday` (keep legacy `--yestarday` alias), `-ws/--week-summary [week]`, `-pw/--previous-week`, `-s/--start`, `-v/--version`, `-h/--help`.
    - Use typed validators for dates and weeks; compute defaults inside handlers, never as import-time defaults.
    - Errors go to stderr with usage and exit code 2; successful runs keep exit code 0.
    - Add a `--data-dir` / `--config` override if not already covered by M5.
  - **Done when:**
    - Every existing documented example command behaves the same or better.
    - Invalid values produce a specific error naming the accepted format.
    - `python -m micro_watcher` and the console script share one parser; tests cover each flag and alias.

### MW-014 — Replace hand-written time arithmetic with a duration type

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-008 · **Outline:** O-4, O-5
  - **Why now:** The current functions encode time as strings and branch manually on negative components. `convert()` wraps at 24 hours, which corrupts multi-day or weekly totals and makes AFK subtraction error-prone.
  - **Work:**
    - Introduce a small `Duration`/`timedelta`-based module for parse, add, subtract, sum, and format.
    - Define behavior for negative differences (report/clamp, never wrap silently) and for totals ≥ 24h.
    - Keep display output backward compatible (`HH:MM:SS` plus the existing human format).
    - Migrate call sites in `watch_log`, `analysis`, and `commands`.
  - **Done when:**
    - Property/round-trip tests cover 0s, 59s, 1h, 23:59:59, 24h, 100h, and negative deltas.
    - No manual carry/borrow branches remain.
    - Weekly totals can exceed 24h and display correctly.

### MW-015 — Fix report aggregation and performance

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-008, MW-014 · **Outline:** O-5, O-10
  - **Why now:** `sort_data()` is O(n²), duplicates overwrite silently, and weekly reports duplicate derived state in a second file. Tests can now pin correct output before the storage rewrite.
  - **Work:**
    - Replace nested sorting with `sorted(items, key=…, reverse=True)` and a deterministic tie-break by app name.
    - Decide and test filtering of empty, `Unknown`, and Home-Screen entries.
    - Aggregate weeks in one pass; stop treating `Analysis/<week>.csv` as a source of truth.
    - Add a golden dataset test for daily and weekly output.
  - **Done when:**
    - Report output is stable across runs and ordering is deterministic.
    - A synthetic one-year dataset aggregates in well under a second on a laptop.
    - Golden tests document intentional differences from the old output.

---

# M3 — Backend abstraction and Wayland

*Goal: the tracker works on the current GNOME Wayland machine and fails explicitly elsewhere. Requires M2's subprocess and date cleanup.*

### MW-016 — Define the backend contract and extract X11

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-011, MW-012 · **Outline:** O-3
  - **Why now:** X11 commands are embedded in the sampling loop and analysis modules. Without an interface, every new compositor forks the core loop.
  - **Work:**
    - Define a protocol with `name`, `is_available()`, `active_window()`, and `idle_seconds()` (plus an explicit error type for unsupported operations).
    - Extract current `xdotool`/`xprop`/`xprintidle` behavior into an X11 backend, preserving the terminal→Neovim/Vim mapping.
    - Add a fake/in-memory backend and inject the backend into `log_creation`.
  - **Done when:**
    - X11 behavior is unchanged against a recorded corpus of window/idle outputs.
    - The sampling loop contains no X11-specific commands.
    - Tests run with the fake backend and no display server.

### MW-017 — Implement the GNOME Wayland backend

- [ ] **Scope**
  - **Partial 2026-11-20:** Added `extensions/gnome/` (focus over DBus), Mutter `IdleMonitor` idle time, installer/uninstaller integration, and backend diagnostics. Extension DBus was smoke-tested in headless GNOME Shell 51.1; current-session loading awaits a log-out/login. Doctor output and formal backend contract remain.
  - **Size:** L · **Depends on:** MW-016 · **Outline:** O-3
  - **Why now:** GNOME Wayland is the default on the current machine and on modern Fedora/Ubuntu. The app currently records garbage there, and the user cannot tell.
  - **Work:**
    - Idle: query `org.gnome.Mutter.IdleMonitor` (`GetIdletime`) over DBus/gdbus; verify support and permissions.
    - Focus: implement a small GNOME Shell extension that exposes the focused app over a session DBus interface (the supported route), with an AT-SPI-based fallback if the extension is not installed.
    - Do not rely on `org.gnome.Shell.Eval`; it is disabled on supported GNOME versions.
    - Package/document the extension (source, install/uninstall, supported GNOME versions, security notes).
    - Degrade explicitly: if the extension is missing, explain how to install it instead of recording an empty app.
  - **Done when:**
    - On GNOME 45+ Wayland, native and XWayland focused apps are recorded with correct names.
    - Idle pause/resume toggles correctly after the configured timeout.
    - `micro-watcher doctor` reports backend, extension status, and idle source.
    - Uninstalling the app removes the extension or clearly documents its manual removal.

### MW-018 — Add KDE and wlroots backends (scoped)

- [ ] **Scope**
  - **Size:** L · **Depends on:** MW-016 · **Outline:** O-3
  - **Why now:** Wayland support that only covers GNOME excludes KDE and tiling-window-manager users. The interface exists; these are independent increments.
  - **Work:**
    - KDE: obtain the active window through a KWin script/DBus bridge and idle through the compositor's idle API.
    - wlroots: use `swaymsg -t get_tree` / `hyprctl activewindow -j` for focus; use the compositor's idle notification mechanism for idle time.
    - Ship one backend per PR, each with recorded-output fixtures and a documented minimum compositor version.
    - Detect and report "unsupported compositor" rather than guessing.
  - **Done when:**
    - Each backend either works end-to-end on its compositor or is not selected.
    - No backend silently returns an empty app for a full session.
    - Docs list the tested compositor versions.

### MW-019 — Replace `pacmd` and redesign AFK detection

- [ ] **Scope**
  - **Partial 2026-11-20:** `afk.py` now uses `pactl` and Mutter/`xprintidle` idle seconds; `pacmd` is gone. The explicit AFK state machine and double-call cleanup remain.
  - **Size:** M · **Depends on:** MW-016 · **Outline:** O-3, O-4
  - **Why now:** `pacmd` is absent on PipeWire systems, and `is_afk()` is called twice per loop with implicit side effects. Both produce incorrect accounting.
  - **Work:**
    - Use `pactl list sink-inputs` (PulseAudio/PipeWire compatible) and treat a missing tool as "unknown", not "no playback".
    - Create an explicit AFK state machine (`active`, `afk`, `returned`) evaluated once per sample.
    - Explain and test the idle-time subtraction so time is never double-counted or made negative.
    - Add a config toggle to disable media detection and/or AFK entirely (wired in M5).
  - **Done when:**
    - `rg pacmd` returns nothing.
    - Tracking continues without audio tooling; a warning is logged once.
    - Unit tests cover AFK entry/exit, active playback, and missing tools.

### MW-020 — Session detection and `micro-watcher doctor`

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-016, MW-017, MW-019 · **Outline:** O-3, O-9
  - **Why now:** Users cannot tell which backend is active or why nothing is recorded. A single diagnostic command turns silent failure into a supportable bug report.
  - **Work:**
    - Detect `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `XDG_CURRENT_DESKTOP`, and available binaries; select the backend deterministically.
    - Add `micro-watcher doctor` printing Python version, session, chosen backend, dependency status, data/config paths, service status, and recent errors.
    - Exit nonzero with an actionable message when no backend supports the session.
  - **Done when:**
    - On GNOME Wayland, `micro-watcher doctor` identifies the GNOME backend and extension state.
    - On X11, it identifies the X11 backend.
    - On an unsupported session, the message names supported environments and no data is recorded.

---

# M4 — Durable storage

*Goal: crash-safe, lock-protected history derived from one source of truth. Can run in parallel with M3 after M2.*

### MW-021 — Write the storage decision record

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-014 · **Outline:** O-5
  - **Why now:** Choosing SQLite versus an append-only log affects migration, querying, and packaging. Decide once, in writing, before writing code.
  - **Work:**
    - Compare SQLite WAL and append-only JSONL on crash safety, query performance, migration complexity, and inspectability.
    - Define the schema (at minimum: app identity, interval start/end or duration, date, source/backend), indices, retention, and file permissions.
    - Document the migration mapping from existing daily totals.
    - Store the record in `agents/decisions/0001-storage.md`.
  - **Done when:**
    - The ADR records the decision, alternatives, and consequences.
    - Schema and migration rules are reviewable without reading implementation code.

### MW-022 — Implement the storage layer

- [ ] **Scope**
  - **Size:** L · **Depends on:** MW-021 · **Outline:** O-5
  - **Why now:** This replaces the per-second file rewrite that can destroy a day's history.
  - **Work:**
    - Implement the chosen store with atomic transactions, WAL/busy timeout (for SQLite), and a single-writer instance lock.
    - Store data under `${XDG_DATA_HOME:-~/.local/share}/micro-watcher/` with `0600` permissions; migrate or honor the legacy cache path.
    - Provide a repository API: `record_interval`, `totals_for_day`, `totals_for_range`, `app_breakdown`.
    - Add tests for concurrent writers, crash during commit, empty ranges, and clock rollover.
  - **Done when:**
    - A simulated crash between samples preserves all committed intervals.
    - Two processes cannot silently overwrite each other; the second either waits or exits.
    - No code path rewrites an entire day file per sample.

### MW-023 — Migrate legacy CSV data

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-022 · **Outline:** O-5, O-10
  - **Why now:** Users already have `~/.cache/Watcher/daily_data/*.csv`. A storage rewrite that ignores them loses history and trust.
  - **Work:**
    - Read legacy tab-separated daily files, validate each row, and import daily aggregates idempotently.
    - Create a timestamped backup before writing; support `--dry-run` and report imported/skipped/malformed counts.
    - Verify imported totals match the old reporting logic within defined tolerances.
    - Keep CSV export available.
  - **Done when:**
    - Running the migration twice produces the same database.
    - Malformed rows are reported, not fatal, and never partially imported.
    - Reports from migrated data match the old output for the golden dataset.

### MW-024 — Read reports from storage

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-022, MW-023 · **Outline:** O-5
  - **Why now:** Two derived sources (daily CSVs plus `Analysis/<week>.csv`) drift and duplicate logic. Reports must come from the store.
  - **Work:**
    - Rewrite `analysis.py` and `commands.py` to query the repository API.
    - Remove or demote `Analysis/<week>.csv` to an explicit export command.
    - Keep CLI output and sorting stable; document intentional differences.
    - Add a performance test with one year of synthetic data.
  - **Done when:**
    - Daily and weekly reports read only the store.
    - Golden reports match MW-015 expectations.
    - One-year reports complete quickly on a laptop.

### MW-025 — Make the sampling loop resilient

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-019, MW-022, MW-024 · **Outline:** O-5, O-6
  - **Why now:** The loop sleeps a fixed second, can drift, has no backoff, and calls AFK detection twice with side effects. A service will expose these weaknesses.
  - **Work:**
    - Use a monotonic clock and drift-corrected scheduling; handle date rollover explicitly.
    - Flush/commit on `SIGINT`/`SIGTERM`; log through `logging` (journald-friendly), not `print`.
    - Add error backoff when the backend/tool fails transiently; never spin on exceptions.
    - Integrate the single-instance lock and AFK state machine from earlier tasks.
  - **Done when:**
    - A simulated 48-hour run with a fake clock keeps accurate totals and rolls over at midnight.
    - `SIGTERM` produces a clean exit and a final committed sample.
    - Repeated backend failures are rate-limited and visible in logs.

---

# M5 — Lifecycle and configuration

*Goal: a supervised user service, a validated config file, and explicit privacy controls.*

### MW-026 — Add validated TOML configuration

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-020, MW-024 · **Outline:** O-7
  - **Why now:** Poll interval, AFK timeout, ignored apps, paths, and backend are hardcoded. Users should not edit installed code, and tests need isolation.
  - **Work:**
    - Read `${XDG_CONFIG_HOME:-~/.config}/micro-watcher/config.toml` using `tomllib`; validate types and ranges.
    - Support at least: poll interval, AFK timeout, ignored apps, backend override, media detection, data directory, retention, log level.
    - Implement precedence `CLI > environment > config file > defaults`; provide a commented sample.
    - Fail with the file path and offending key; never silently fall back on invalid config.
  - **Done when:**
    - Invalid config exits with a precise message and nonzero status.
    - Tests cover defaults, precedence, and malformed TOML.
    - The sample config is documented in the README.

### MW-027 — Install a systemd user service

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-025, MW-026 · **Outline:** O-6
  - **Why now:** This replaces the `~/.xprofile` hack with a supervised, Wayland-compatible, log-visible lifecycle.
  - **Work:**
    - Add `micro-watcher service install|uninstall|status` writing `~/.config/systemd/user/micro-watcher.service`.
    - Bind the unit to `graphical-session.target` (`Wants`/`After`, `PartOf`), run `micro-watcher --start`, and set `Restart=on-failure`.
    - Provide an XDG `.desktop` autostart fallback for non-systemd sessions.
    - During upgrade, remove only the exact legacy `watcher --start &` line (written by the old installer) if the user opts in.
  - **Done when:**
    - `systemctl --user enable --now micro-watcher` records data after login on both Wayland and X11.
    - Logs are visible in `journalctl --user -u micro-watcher`.
    - `micro-watcher service uninstall` stops, disables, and removes the unit without touching data.

### MW-028 — Add privacy, export, and retention controls

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-026 · **Outline:** O-7, O-9
  - **Why now:** A screen-time tracker collects sensitive behavioral data. Users need to see, scope, export, and delete it without editing files by hand.
  - **Work:**
    - Honor `ignored_apps` at the backend/aggregation boundary so ignored windows are never persisted.
    - Add export (`micro-watcher export --format csv|json --output …`) and purge (`micro-watcher purge --before DATE`, with `--dry-run` and confirmation).
    - Implement configured retention.
    - Document exactly what is stored (app class names by default, no titles) and where.
  - **Done when:**
    - Ignored apps never appear in the store or reports.
    - Export/import round-trip is tested.
    - Purge requires explicit confirmation and reports what it would delete.

---

# M6 — Browser domain tracking

*Goal: make browser time actionable without turning the tracker into a browsing history. Focused tab time is screen time; background media is a separate metric showing the site, browser, and duration.*

### MW-029 — Write the browser privacy ADR and threat model

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-016, MW-022, MW-026 · **Outline:** O-11, D6, D8
  - **Why now:** Browser data is the most sensitive data this project will touch. The extension/native-host boundary and the focused-versus-background semantics must be decided before anyone writes permissions or a schema.
  - **Work:**
    - Compare an extension + native messaging (`tabs`/`windows` APIs) with history-database scraping and AT-SPI address-bar access; document why the first is the least invasive viable option.
    - Define the browser matrix: Chromium family (Chrome, Edge, Brave, Vivaldi) and Firefox; document unsupported browsers.
    - Specify the protocol: browser identity, registrable domain, event type, timestamp, visibility (`focused`/`background`), audibility, media-playing state, schema version. No URLs, paths, queries, fragments, titles, or page content.
    - Define permission minimization, incognito exclusion, pause behavior, native-host sender validation, and the local-only/no-network rule.
    - Record the decision in `agents/decisions/0002-browser-domains.md`.
  - **Done when:**
    - The ADR states exactly what is collected, where it is stored, and what is explicitly excluded.
    - Focused-tab screen time and background media playback are defined as separate metrics.
    - The extension permission set and native-host trust boundary are reviewable before implementation.

### MW-030 — Implement domain normalization and storage

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-022, MW-029 · **Outline:** O-11
  - **Why now:** Aggregating `mail.google.com`, `www.google.com`, and `google.co.uk` correctly requires a stable definition of "site" before data is written.
  - **Work:**
    - Normalize hostnames: lowercase, strip a trailing dot, convert IDNs to punycode, and reduce to the registrable domain (eTLD+1).
    - Bundle a Public Suffix List snapshot with its license and a documented update script; handle `co.uk`, private suffixes, IP literals, and `localhost` explicitly.
    - Add browser-domain storage to the repository API with `browser`, `domain`, interval, `visibility`, `audible`, and `media_playing` fields; store no full URL.
    - Add retention and `0600` permissions consistent with app data.
    - Test subdomains, multi-label suffixes, IDNs, malformed hosts, and every non-public host policy.
  - **Done when:**
    - `co.uk`-style domains aggregate correctly and no schema column can hold a path or query.
    - Normalization is deterministic and covered by table-driven tests.
    - Local/IP hosts follow the documented policy instead of being stored accidentally.

### MW-031 — Build the browser extension

- [ ] **Scope**
  - **Size:** L · **Depends on:** MW-029 · **Outline:** O-11
  - **Why now:** Only the browser knows which tab is focused. The extension is the smallest component that can answer that without reading the history database or the page itself.
  - **Work:**
    - Implement a Manifest V3 Chromium extension and the Firefox equivalent using only `tabs` and `windows` APIs.
    - Listen for window focus, tab activation, and URL changes; compute the registrable domain locally (or send the hostname to the native host for normalization) and send only the protocol fields from MW-029.
    - Request no content scripts, `webRequest`, `history`, `cookies`, host permissions, or network access; do not persist URLs in extension storage.
    - Exclude incognito/private windows by default and honor the global pause state.
    - Handle multiple windows and profiles without attributing an unfocused window's tab to screen time.
    - Provide build/packaging scripts and manual-install documentation for development.
  - **Done when:**
    - Switching tabs or windows emits the expected domain/focus events and nothing else.
    - Reviewing the manifest, source, and outgoing messages shows no URL, title, query, or page content.
    - Incognito windows produce no events; pausing produces no events.

### MW-032 — Implement the native messaging host and focus correlation

- [ ] **Scope**
  - **Size:** L · **Depends on:** MW-016, MW-030, MW-031 · **Outline:** O-11
  - **Why now:** The extension cannot write the database, and the backend must confirm that the browser is actually the focused application. The host is the trust boundary.
  - **Work:**
    - Add a native-messaging host installed with the app; generate Chromium and Firefox manifests in their required per-user locations with the published extension IDs.
    - Speak versioned JSON lines over stdin/stdout only: no listening sockets, no outbound network, strict message type/length validation.
    - Correlate browser events with the active backend window and timestamps; record a domain only while the browser is focused, deduplicate rapid events, and close intervals on blur/close.
    - Provide install/uninstall/status integration and surface extension/host state through `micro-watcher doctor`.
    - Add tests using a fake extension transport, including malformed, oversized, and spoofed messages.
  - **Done when:**
    - Domains are recorded only while the browser is the focused app; switching away closes the interval.
    - Invalid messages are rejected without a traceback, and the manifests are removed on uninstall.
    - `micro-watcher doctor` reports extension, host, and protocol status.

### MW-033 — Add background media metrics

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-030, MW-032 · **Outline:** O-11, D8
  - **Why now:** Background media (Spotify, YouTube, podcasts) is useful context, but counting every open tab as attention would inflate reports and collect unnecessary data. Track only media-playing tabs and present the result as what it is: background media, with source and duration.
  - **Work:**
    - Add a config switch (default on when browser tracking is enabled) for background media events.
    - Use the browser's `audible` flag for background tabs; store `visibility=background`, `audible=true`, `media_playing`, the browser, registrable domain, and interval start/end. Do not record merely-open tabs.
    - Keep muted-video detection behind a separate opt-in content script that reports only a boolean `mediaPlaying` state; it must not read URLs or titles.
    - Never include background media in screen-time totals or the app breakdown; report it in a dedicated **Background media** section grouped by domain, with browser and accumulated duration.
    - Apply retention and ignored-domain rules to background media data too.
    - Test that enabling the feature leaves focused screen-time totals unchanged and that durations aggregate correctly across play/pause cycles.
  - **Done when:**
    - Background media events are impossible to mix into screen time by construction.
    - Disabling the feature produces no background media events or storage.
    - Reports show site, browser, and duration separately and explain that this is not attention time.

### MW-034 — Add browser reporting and privacy controls

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-030, MW-031, MW-032, MW-033 · **Outline:** O-11
  - **Why now:** Domain data is only acceptable if users can inspect, limit, export, and delete it as easily as app data.
  - **Work:**
    - Add daily/weekly browser sections grouped by registrable domain, with browser filters and no URL detail.
    - Add a distinct **Background media** section showing domain, browser, and total duration; never merge it into screen-time or app-usage totals.
    - Support `ignored_domains` at the ingestion boundary so ignored domains are never persisted.
    - Extend pause, retention, export, and purge to browser data; keep exports domain-only and documented.
    - Update the privacy documentation: what is collected, what is never collected, where it lives, and how to remove it.
    - Add golden-report tests for subdomain aggregation and background separation.
  - **Done when:**
    - Ignored domains never appear in storage, reports, or exports.
    - Export/purge round-trips cover domains and app data consistently.
    - Documentation and `micro-watcher doctor` agree with actual behavior.

---

# M7 — Documentation, cleanup, release

*Goal: a release that a new user can install, understand, and remove using only the documentation.*

### MW-035 — Rewrite the README and platform documentation

- [ ] **Scope**
  - **Partial 2026-11-20:** README rewritten for current features, GNOME/X11 support, privacy, installation/uninstall, and the AUR/.deb roadmap; obsolete gallery/HACTOBERFEST/Go content removed. Final verification after KDE/browser/package work remains.
  - **Size:** M · **Depends on:** MW-020, MW-027, MW-034 · **Outline:** O-9
  - **Why now:** M0 fixed the unsafe install command and basic command names, but the README still lacks a verified support matrix, migration/privacy details, browser setup, and troubleshooting. Docs are the install path for most users.
  - **Work:**
    - Document `pipx install .` (and `python -m pip install --user .`), first run, data location, uninstall, and upgrade.
    - Add a support matrix: X11, GNOME Wayland, KDE Wayland, wlroots, plus known limitations per compositor.
    - Replace the outdated AFK status, dependency list, and naming; remove `curl | bash` permanently.
    - Document browser extension installation, permissions, what domains are stored, and how to disable background metrics.
    - Add privacy, troubleshooting, migration, and development sections; verify every command on a clean environment.
  - **Done when:**
    - No command in the README is unverified.
    - The support matrix matches `micro-watcher doctor` behavior.
    - The install/uninstall instructions work without `sudo` and without editing shell rc files.

### MW-036 — Document architecture and refresh CONTRIBUTING

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-035 · **Outline:** O-9, O-10
  - **Why now:** The code has moved from six path-dependent scripts to a package with a backend contract, storage layer, and browser trust boundary. Contributors need an accurate map.
  - **Work:**
    - Add `docs/architecture.md`: module map, sampling flow, backend contract, storage schema, browser extension/native-host protocol, config precedence, service lifecycle.
    - Update `CONTRIBUTING.md`: environment setup, tests, lint, PR expectations, adding a backend or browser, data migration rules.
    - Link the decisions directory and this backlog.
  - **Done when:**
    - A new contributor can run the app and tests using only the docs.
    - The documented module layout matches the repository.
    - The backend-extension and browser-extension guides are sufficient to add a fake component end-to-end.

### MW-037 — Repository housekeeping

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-006, MW-010 · **Outline:** O-10
  - **Why now:** Low risk, but it removes review noise and prevents accidental commits before the release tag.
  - **Work:**
    - Finish `.gitignore` coverage: `.venv/`, `build/`, `dist/`, `.pytest_cache/`, `.ruff_cache/`, coverage files, and browser-extension build outputs.
    - Remove dead code, commented-out blocks, unused imports, shadowed names, and the per-second debug print (if not already removed).
    - Fix `misc/export_to_2.0`: remove `os.getlogin()`/hardcoded `/home`, use `Path.home()`, replace the `#!/usr/bin/python` shebang with `#!/usr/bin/env python3`, and move the logic under `micro-watcher migrate-v1` (the file is already executable).
    - Preserve the MIT license text and upstream attribution; add a short NOTICE or README acknowledgment for the fork.
  - **Done when:**
    - `git status` is clean after a full test/lint run.
    - No generated files are tracked.
    - The migration tool runs from any working directory or has been retired with documented replacement.

### MW-038 — Cut the release

- [ ] **Scope**
  - **Size:** S · **Depends on:** MW-035, MW-036, MW-037 · **Outline:** O-1 … O-11
  - **Why now:** All user-facing behavior, paths, and install instructions change in this effort. Shipping without a versioned release and migration notes leaves users stranded.
  - **Work:**
    - Choose the version according to the packaging decision (suggest `2.1.0` for the first packaged Python release; document the legacy install as superseded).
    - Add a `CHANGELOG.md` covering the installer change, service migration, storage migration, Wayland support, and browser permission/privacy model.
    - Build sdist/wheel, install from the sdist in a clean venv, run `micro-watcher doctor`, record a sample, and produce a report.
    - Tag the release; publish notes with upgrade and rollback steps.
  - **Done when:**
    - The exact release artifact is installable without the repository checkout.
    - `micro-watcher --version`, `micro-watcher doctor`, and a full record/report cycle pass.
    - Release notes explain how legacy data and autostart entries are handled.

---

# M8 — Distribution packages

*Goal: install with the distribution's package manager, not a checkout. AUR covers `pamac` and `yay`; `.deb` covers apt until a signed repository/PPA exists.*

### MW-039 — Define the system install layout and package data

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-006, MW-027, MW-035 · **Outline:** O-12
  - **Why now:** AUR and Debian packages must agree on where the command, Python modules, GNOME extension, systemd unit, man page, and license live. Changing this after publishing breaks upgrades.
  - **Work:**
    - Define the FHS layout: `/usr/bin/micro-watcher`, `/usr/lib/python3/dist-packages/micro_watcher`, `/usr/share/gnome-shell/extensions/<uuid>`, `/usr/lib/systemd/user/micro-watcher.service`, `/usr/share/man/man1/micro-watcher.1`, `/usr/share/licenses/micro-watcher/`.
    - Make the GNOME extension, service template, and documentation installable as package data with a single version source.
    - Decide the policy for the user service: install it disabled and print `systemctl --user enable --now micro-watcher`; never auto-enable tracking.
    - Add a man page and shell completions if the command surface is stable.
  - **Done when:**
    - A staging install (`DESTDIR`/prefix) contains exactly the agreed files.
    - The wheel and the system package use the same version and entry point.
    - No package writes into a user's home directory at install time.

### MW-040 — Build and publish the AUR package

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-038, MW-039 · **Outline:** O-12, D9
  - **Why now:** `pamac` and `yay` are AUR helpers; an AUR package is the supported way to get `micro-watcher` onto Arch-based systems without an official repository sponsor.
  - **Work:**
    - Create `packaging/aur/PKGBUILD` for the stable release and `micro-watcher-git` for the checkout; include `.SRCINFO`, license, source checksums, and `python`/X11/`libpulse` dependencies plus `gnome-shell`/`pipewire-pulse` optdepends.
    - Install the wheel or source with `python -m installer` into `$pkgdir`; install the GNOME extension under `/usr/share/gnome-shell/extensions/`.
    - Test `makepkg` in a clean chroot (`devtools`), install with `pacman -U`, verify `micro-watcher --version`/`--doctor`, then remove.
    - Publish under the maintainer's AUR account and document `yay -S micro-watcher` / `pamac build micro-watcher`.
  - **Done when:**
    - A clean chroot builds without network surprises and `namcap` reports no errors.
    - `pacman -U` and AUR-helper installs produce a working command and extension.
    - The package removes cleanly and keeps user data unless explicitly purged.

### MW-041 — Build and publish the Debian/Ubuntu package

- [ ] **Scope**
  - **Size:** L · **Depends on:** MW-038, MW-039 · **Outline:** O-12, D9
  - **Why now:** apt users need a native package; a `.deb` is the minimum viable channel and avoids asking users to run a checkout script.
  - **Work:**
    - Add `debian/` (`control`, `rules`, `install`, `copyright`, `changelog`, `source/format`) using `dh-python`/`pybuild`; declare `python3 (>= 3.11)`, X11 tools, `x11-utils`, and optional `pipewire-pulse`/`pulseaudio-utils`; recommend `gnome-shell`.
    - Install the GNOME extension into `/usr/share/gnome-shell/extensions/` and the systemd user unit into `/usr/lib/systemd/user/`; keep the service disabled by default.
    - Build with `dpkg-buildpackage` and `sbuild`/`pbuilder`; run `lintian`; test install/remove on Debian 12/13 and Ubuntu 24.04.
    - Publish the `.deb` and checksums on GitHub Releases; document `apt install ./micro-watcher_<version>_amd64.deb`.
  - **Done when:**
    - `lintian` has no errors and the package installs/removes cleanly in a clean container.
    - The extension is discoverable by GNOME and the unit is present but not enabled.
    - `apt install ./…deb` works on every documented target release.

### MW-042 — Decide and implement the apt repository/PPA and official Arch path

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-040, MW-041 · **Outline:** O-12, D9
  - **Why now:** `apt install micro-watcher` without a filename requires a signed repository; `pacman -S micro-watcher` requires the package to be in an official Arch repository. These are separate trust/ownership decisions.
  - **Work:**
    - Choose a Launchpad PPA or a self-hosted signed apt repository (GPG key, `dists/`, `pool/`, `Packages`/`Release`, repository docs). Implement only the chosen option.
    - For Arch, document that AUR is the supported channel. If `pacman -S micro-watcher` is required without a sponsor, add a custom signed pacman repository (repo database + package + signature) and document `pacman.conf`/key setup; otherwise prepare an official-repository submission only if a Trusted User agrees to sponsor it.
    - Add install/upgrade/remove instructions and key-verification steps to the README for every published channel.
  - **Done when:**
    - `apt update && apt install micro-watcher` works from the documented repository on supported Ubuntu/Debian versions, or the release explicitly documents `.deb`-only installation.
    - The Arch channel is unambiguous (AUR supported by default; a custom repo is documented if implemented; official repo not promised without a sponsor).
    - Signing keys and repository metadata are documented and rotated safely.

### MW-043 — Add packaging CI, signing, and clean-environment verification

- [ ] **Scope**
  - **Size:** M · **Depends on:** MW-040, MW-041, MW-042 · **Outline:** O-12
  - **Why now:** Hand-built packages drift. Packaging must be reproducible and verified on every release, including the extension and service integration.
  - **Work:**
    - Add CI jobs that build the wheel, `.deb`, and AUR artifacts, run `lintian`/`namcap`, and install/uninstall in clean Debian/Ubuntu/Arch containers.
    - Sign release artifacts and repository metadata with a documented key; publish checksums.
    - Smoke-test the installed command, GNOME extension presence, and systemd unit on each target.
    - Document the release checklist and rollback procedure.
  - **Done when:**
    - A release tag produces verified artifacts automatically.
    - CI catches a broken entry point, missing extension asset, or service path.
    - The release page lists checksums, signatures, and per-channel commands.

---

## Parking lot (deliberately unscheduled)

Do not start these until the release above exists:

- GUI/Tauri frontend.
- Cloud sync, accounts, or telemetry.
- Notifications and goal/limit alerts.
- Flatpak, Snap, or AppImage packages.
- Additional browser integrations (Safari, mobile browsers) or editor plugins.
- Automatic app categorization / machine learning.
- Windows or macOS backends.
- Go rewrite (see D1).

---

## Definition of done for this backlog

- [ ] No installer path downloads or executes remote code, and no installer needs root.
- [ ] The CLI validates all dates/weeks and never passes data through a shell.
- [ ] X11 and at least GNOME Wayland work end-to-end; unsupported sessions fail with actionable errors.
- [ ] History survives crashes, concurrent starts, and upgrades; legacy CSVs are migrated.
- [ ] A systemd user service replaces shell startup edits; uninstall is clean.
- [ ] Config, privacy, export, and retention controls are documented and tested.
- [ ] Browser reports are domain-only; focused-tab time is distinct from background media, which reports site, browser, and duration, with no URL, title, or page content stored.
- [ ] CI runs tests and lint; the README commands are verified on a clean system.
- [ ] A tagged release includes migration and rollback notes.
- [ ] AUR (`yay`/`pamac`) and `.deb` artifacts install, upgrade, and remove cleanly on the documented distro versions.
