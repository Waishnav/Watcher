# micro-watcher Modernization Outline

- **Status:** In progress — M0 tasks MW-001 … MW-003 are implemented; the GNOME Wayland backend is partially implemented (extension smoke-tested, pending first-login verification); MW-004 and the remaining milestones are proposed
- **Baseline:** `65146e8` (`main`, fork of `Waishnav/Watcher` v2.0), inspected 2026-11-20; the working tree also contains the M0 changes
- **Companion:** [BACKLOG.md](./BACKLOG.md) — ordered, checkable implementation tasks
- **Goal:** make micro-watcher installable without root, correct on modern Linux desktops, and durable enough that users can trust the recorded history.

---

## 1. Current snapshot

| Area | Today | Consequence |
| --- | --- | --- |
| Distribution | **M0:** local, user-level install from the checkout; no network or `sudo`. Baseline cloned upstream `v2.0` and copied files as root. | Fixed for the unsafe path; packaging still needs M1 |
| Runtime | Python standard library; X11 tools (`xdotool`, `xprop`, `xprintidle`); GNOME 45+ Shell extension on Wayland | No dependency manifest; the extension needs a one-time re-login and KDE/wlroots remain |
| Storage | One tab-separated CSV per day, fully rewritten every second | Corruption on crash, data loss, unnecessary disk churn |
| Lifecycle | **M0:** no shell startup files touched. Baseline edited `~/.xprofile` / `~/.xinitrc`. | No supervision or supported autostart yet |
| Quality | **M0:** strict date/week validation with 25 passing tests. No packaging, linting, or CI yet. | Refactors are still risky; contributors have no full guardrails |

Verified in the current environment (EndeavourOS, GNOME **Wayland**, Python 3.14):

- `xdotool`, `xprintidle`, and `pacmd` are absent; only `pactl`, `gdbus`, and `xprop` exist.
- At baseline, running `src/bin/micro-watcher` from a checkout failed with `ModuleNotFoundError`. It now resolves `src/micro_watcher` relative to the script, so checkout runs work.
- GNOME Shell is 51.1. The focus extension's DBus interface was smoke-tested in a headless nested Shell; the running session needs one log-out/login to load the newly installed extension.
- `python3 -m pytest` passes the validation, CLI-safety, and display-name tests.

---

## 2. Principles

1. **No root by default.** A personal screen-time tracker has no reason to write to `/usr/share` or `/usr/local/bin`.
2. **Never modify files we do not own.** No `sed` on shell startup files; use documented integration points.
3. **No shell interpolation of data.** Arguments and window titles must never pass through a shell string.
4. **Durability over cleverness.** A crash must not lose the day's history; prefer transactions over whole-file rewrites.
5. **One backend contract.** X11 and each Wayland compositor implement the same interface; unsupported sessions fail loudly.
6. **Testable by construction.** Pure logic (time math, week math, aggregation) is separated from system calls.
7. **Preserve user data.** Every format or path change ships with a tested, idempotent migration.
8. **Boring, standard tooling.** `pyproject.toml`, `pytest`, `ruff`, and CI instead of bespoke scripts.

---

## 3. Improvements

### O-1 — Fix distribution and installation

**What:** Replace the network-cloning installer and `curl | bash` instructions with an install path that uses the reviewed local checkout.

**Status:** M0 implemented the local, user-level installer; M1 packaging (`pipx install .`) is the supported long-term replacement.

**Why change it:**

- At baseline, `install` line 2 ran `git clone https://github.com/Waishnav/Watcher -b v2.0` and installed from that clone. A user running this fork's installer got upstream code, not the fork; local fixes never shipped and offline installation was impossible.
- The baseline README's `bash <(curl -s .../install)` pattern executed remote code as the current user, followed by `sudo cp` on whatever was downloaded. There was no tag pin, commit pin, or checksum, so the installed bytes were not reproducible or auditable.
- The baseline `install` wrote to `/usr/share/Watcher` and `/usr/local/bin/watcher` with `sudo`; `update` emptied `/usr/share/Watcher/*` first and assumed the layout created by `install`. An interrupted update left a partially installed application.
- The baseline `install` required `git`, but the documented dependency list mentioned only `xprintidle` and `xdotool`.
- The baseline README's two installation steps contradicted each other: the `curl` command ran the installer, and the next step told the user to run `./install`, which did not exist in the directory the first command left behind.
- Because installation is opaque, distro packagers and `pipx` users have no supported path.

**Where:** `install`, `update`, `uninstall`, `README.md` (Installation section).

**Target state:** `pipx install .` (or `python -m pip install --user .`) from the checkout, plus a small wrapper script if desired. No network access during install beyond explicitly installing dependencies, no `sudo`, and an uninstall that reverses every change.

---

### O-2 — Add packaging metadata and a proper entry point

**What:** Add `pyproject.toml`, a `src/micro_watcher/` package, an executable entry point, and a single source of truth for the version.

**Status:** M0 renamed the module directory and entry point, made `sys.path` relative to the script, and converted imports to the `micro_watcher` package name. Packaging, console scripts, and version metadata remain M1.

**Why change it:**

- At baseline, `src/bin/micro-watcher` hardcoded `sys.path.insert(0, "/usr/share/Watcher/")`; from a checkout this failed, and the only reason it worked after installation was that files were copied to that exact absolute path.
- The version is a hardcoded string (`2.0.0`) in the CLI, unrelated to any package metadata. There is no way to query, bump, or verify the installed version.
- There is no dependency declaration, so missing system tools are discovered at runtime (or silently ignored). There is no way to express "this feature requires `xdotool` on X11".
- Tests and ad-hoc runs need `PYTHONPATH` or working-directory tricks; there is no editable install, wheel, or sdist.
- Packaging is a prerequisite for every later improvement: CI needs an importable package, `pipx` needs a console script, and migration tooling needs a stable module layout.

**Where:** new `pyproject.toml`, `src/bin/micro-watcher`, `src/micro_watcher/` (imports), `.gitignore`.

**Target state:** `pip install -e .` works; `micro-watcher` and `python -m micro_watcher` both start the app; the version comes from package metadata; runtime dependencies are documented and system-tool checks produce actionable errors.

---

### O-3 — Add a Wayland-capable backend

**What:** Introduce a backend abstraction and implement GNOME Wayland support, keeping X11 as a fallback.

**Status:** GNOME Wayland is implemented with a user-level Shell extension for focus and Mutter's `IdleMonitor` for idle time; X11 is preserved. The extension was smoke-tested in a headless GNOME Shell 51 session; the running session requires a log-out/login before it can load. The formal backend interface and KDE/wlroots implementations remain.

**Why change it:**

- At baseline the tracker was X11-only: `get_windows.py` shelled out to `xdotool` and `xprop`, and `afk.py` used `xprintidle`. Native Wayland clients are invisible to X11 tools, so on a Wayland session the tracker recorded an empty window/app (attributed to Home-Screen/Unknown) instead of failing visibly.
- The current development machine is GNOME Wayland and does not have `xdotool` or `xprintidle`; the baseline app could not work here even after a successful install.
- `pacmd` is a PulseAudio-specific tool that is commonly absent on PipeWire systems. The code interprets "no output" as "no media playback", which changes AFK behavior without notifying the user.
- Screen-time data is only useful if it is complete. Silent misattribution is worse than an explicit "unsupported session" error, because the user trusts a wrong report.
- A backend contract also makes tests deterministic (fake backend) and allows future compositors without touching sampling logic.

**Where:** `src/micro_watcher/get_windows.py`, `src/micro_watcher/afk.py`, `src/micro_watcher/watch_log.py`, new backend modules, plus in-repo compositor plugins: `extensions/gnome/` (GJS/ESM) and `extensions/kde/` (KWin script/QML). The core stays Python; the plugins are installed as data files and only expose focus/idle facts over the local session bus.

**Target state:** A `Backend` protocol providing `active_window()` and `idle_seconds()`. X11 behavior is preserved. GNOME Wayland works through `org.gnome.Mutter.IdleMonitor` for idle time and a small, documented GNOME Shell extension (or AT-SPI fallback) for focus. Other sessions either work through KDE/wlroots integrations or exit with a clear, actionable message. Extensions are versioned per shell/compositor and shipped from this repository; they are not a separate application stack.

---

### O-4 — Replace shell calls, harden input, and use `argparse`

**What:** Remove `os.popen`, validate all user-supplied dates/weeks, reject path traversal, and replace the ad-hoc `sys.argv` parser.

**Status:** M0 closed the injection/traversal hotfix (strict validators, no user input in a shell, tests). The full `subprocess` and `argparse` refactor remains in M2.

**Why change it:**

- At baseline, `analysis.weekday_from_date()` interpolated its argument into `date -d "<input>"`. The `-ws` argument only had to be 8 characters long, so an 8-character payload such as `a;id;ls ` reached the shell. That was a command-injection vector (closed in M0).
- The same week argument is used in `~/.cache/Watcher/Analysis/<week>.csv`, and the `-ds` date is concatenated into a file path. An 8-character week like `../../xy` escapes the data directory (the `.csv` suffix limits, but does not eliminate, the traversal). Length checks are not validation.
- Commands are parsed by splitting human-readable `date`/`xprop`/`xdotool` output, which breaks when window titles contain quotes, tabs, or newlines. `get_windows.py` depends on `$(...)` command substitution inside a shell string.
- `commands.daily_summary(date=get_date())` and `week_summary(week=os.popen(...))` evaluate defaults at import time. That is mostly hidden by one process per invocation, but it is a real bug for tests, long-running services, and future daemon mode.
- Argument errors print a generic "Wrong option" block. Users get no indication that a date must be `YYYY-MM-DD` or a week `Www-YYYY`.
- Replacing shell calls with `subprocess.run([...])` removes quoting bugs and makes mocking possible in tests.

**Where:** `src/bin/micro-watcher`, `src/micro_watcher/analysis.py`, `src/micro_watcher/get_windows.py`, `src/micro_watcher/watch_log.py`, `src/micro_watcher/commands.py`.

**Target state:** `argparse` (or a small typed parser) with date/week validators, `subprocess.run` with argument lists and timeouts, no `shell=True`, no `os.popen`, and tests covering malicious and malformed inputs.

---

### O-5 — Make storage crash-safe and queryable

**What:** Replace per-second whole-file rewrites with a transactional store (SQLite in WAL mode is the default candidate) plus a tested migration from existing CSVs.

**Why change it:**

- `watch_log.update_csv()` opens the day file in `w` mode and rewrites every known app on every sample. A crash, power loss, or full disk during that write can truncate the file and lose the entire day.
- There is no locking: the autostart instance and a manually started instance both write the same file and overwrite each other's data.
- Rewriting the full file every second is O(number of apps) per sample and causes unnecessary disk writes/SSD wear over a day.
- Time values are stored as hand-formatted `HH:MM:SS` strings. Arithmetic wraps at 24 hours in `convert()`, and aggregation is O(n²) in `sort_data()`.
- Weekly reports re-read every daily file and rewrite `~/.cache/Watcher/Analysis/<week>.csv`, duplicating derived state that can drift from the source.
- The recorded history is the product. Losing it silently is the most damaging failure mode in this codebase.

**Where:** `src/micro_watcher/watch_log.py`, `src/micro_watcher/analysis.py`, `src/micro_watcher/time_operations.py`, new `storage` module.

**Target state:** A storage layer with atomic transactions, a single-writer instance lock, monotonic timestamps, indexed queries, and idempotent CSV import. CSV export remains available for compatibility and inspection.

---

### O-6 — Replace shell-startup hacking with a proper user service

**What:** Run tracking through a systemd user unit bound to the graphical session, with an XDG autostart fallback; stop editing `~/.xprofile` and `~/.xinitrc`.

**Status:** M0 removed all shell-startup edits. The supervised service remains M5.

**Why change it:**

- `install` removes every line containing `xprofile` from `~/.xinitrc` with `sed -i '/xprofile/d'`. This can delete unrelated lines a user depends on, and it is destructive by design.
- At baseline, appending `watcher --start &` to `~/.xprofile` only worked for X sessions that source that file. It did not work reliably on Wayland, and it offered no restart, logging, or dependency ordering.
- There is no duplicate-instance protection, so a manual start plus autostart creates two writers.
- `uninstall` removes binaries but leaves the startup line behind, so the next login attempts to run a missing command.
- Users cannot easily see whether the tracker is running or why it exited. `journalctl --user` solves that for free with systemd.

**Where:** `install`, `uninstall`, new service templates, `README.md`.

**Target state:** `micro-watcher service install` writes a user unit, enables it for `graphical-session.target`, and reports status; `micro-watcher service uninstall` stops and removes it. No shell rc files are modified. An XDG `.desktop` autostart file is available for non-systemd sessions.

---

### O-7 — Add a configuration file

**What:** Move tunables out of code into a validated TOML config under the XDG config directory, with CLI and environment overrides.

**Why change it:**

- AFK timeout (`afkTimeout = 1` minute), poll interval (1 second), the terminal-emulator list, and cache paths are hardcoded. Users must edit installed Python to change normal behavior.
- Tests cannot isolate behavior from user settings or the real home directory.
- Different machines need different policies: ignore work apps, disable AFK, choose a backend, change retention, or relocate data.
- A documented config is also the natural place to expose privacy controls (ignored windows, title capture, retention).

**Where:** new `config` module, `afk.py`, `watch_log.py`, `get_windows.py`, `commands.py`, documentation.

**Target state:** `${XDG_CONFIG_HOME:-~/.config}/micro-watcher/config.toml` with validated keys, precedence `CLI > environment > file > defaults`, a commented sample config, and tests using temporary config directories.

---

### O-8 — Add tests, linting, and CI

**What:** Add `pytest`, `ruff`, and GitHub Actions, starting with the pure logic and expanding with each refactor.

**Status:** M0 added 25 validation and CLI-safety tests. The full suite, linting, and CI remain in M1/M7.

**Why change it:**

- At baseline there were zero tests. The most error-prone code is hand-written time arithmetic (`time_difference`, `time_addition`, `convert`), ISO week calculation (`get_sunday_of_week`, including a `timedelta(days=6.9)` hack), CSV parsing, and report aggregation.
- Every improvement in this outline changes behavior in those areas. Without tests, refactors will silently corrupt reports.
- Linting catches unused imports, shadowed variables (for example `x` used for both a module alias and a loop variable), and dead branches that make review harder.
- CI gives contributors an objective definition of "working" and prevents platform-specific regressions.
- The pure functions are cheap to test; this is the highest-leverage investment after packaging.

**Where:** new `tests/`, `pyproject.toml` tool sections, `.github/workflows/`.

**Target state:** `pytest` runs offline in a fresh checkout; `ruff check` and `ruff format --check` pass; CI runs on supported Python versions for pushes and pull requests; core logic has meaningful coverage (edge cases, not just line coverage).

---

### O-9 — Rewrite the documentation to match reality

**What:** Replace the README installation/feature sections with verified instructions, an accurate feature status, a platform support matrix, and privacy/uninstall documentation.

**Status:** M0 removed the unsafe install instructions and updated the command names, dependency notes, and data/uninstall basics. The full rewrite remains in M7.

**Why change it:**

- At baseline, the README offered a `curl | bash` install and then a second `./install` step; the first does not leave the expected file layout, and the second assumes a different workflow. Users following the docs cannot tell which one is authoritative.
- It listed only `xprintidle` and `xdotool` as dependencies, omitting `git` (then required by the installer) and `pacmd` for AFK detection.
- It said AFK was not implemented in the gallery note while marking it complete in the To-do, and `afk.py` contains a basic implementation. The gallery and fun fact were based on that outdated behavior.
- There is no mention of Wayland support (or lack thereof), data location, uninstall, privacy (what is recorded), upgrade behavior, or how to run from a checkout.
- The project is branded "Watcher" while the fork/repo is `micro-watcher`; CONTRIBUTING still points at v1.0/v2.0 branches. This creates avoidable confusion for users and contributors.
- Documentation is the installation path for most users. If it recommends an unsafe command, the code's safety work is irrelevant.

**Where:** `README.md`, `CONTRIBUTING.md`, new `docs/` pages if the content outgrows the README.

**Target state:** A README whose commands have been executed on a clean system, with a support matrix (X11, GNOME Wayland, KDE, wlroots), dependency checks, privacy statement, uninstall instructions, troubleshooting, and development setup.

---

### O-10 — Housekeeping and repository hygiene

**What:** Clean up ignore rules, dead code, migration scripts, naming, and low-level repository rough edges.

**Status:** M0 renamed the module directory/entry point and expanded `.gitignore`. Dead code, migration tooling, and the remaining naming cleanup are M7.

**Why change it:**

- At baseline `.gitignore` contained only `src/Watcher/__pycache__/*`. Python caches elsewhere, virtual environments, coverage data, and build artifacts could be committed accidentally.
- `misc/export_to_2.0` uses `#!/usr/bin/python` (often absent), relies on `os.getlogin()` and hardcoded `/home/<user>`, and has no documentation. It will fail in non-interactive or differently laid-out environments.
- `watch_log.log_creation()` prints the entire data dictionary every second, contains an unused AFK branch, and uses `x` both as an imported module alias and a loop variable. `commands.py` and `analysis.py` contain commented-out code and unused imports.
- Hardcoded absolute paths (`/usr/share/Watcher`) and divergent names (`Watcher` vs `micro-watcher`) make the code harder to reason about and package.
- None of these items is individually severe, but together they slow review, hide real bugs, and make the project look less trustworthy than it is.

**Where:** `.gitignore`, `misc/export_to_2.0`, `src/micro_watcher/*.py`, `README.md`, `CONTRIBUTING.md`.

**Target state:** A clean tree with no generated files, no dead code, migration tooling that works from any checkout, consistent naming, and preserved MIT attribution to the upstream author.

---

### O-11 — Add privacy-preserving browser domain tracking

**What:** Track the registrable domain of the focused browser tab through a browser extension and a local native-messaging host, without URLs, page titles, history, cookies, or content.

**Why change it:**

- For most users the browser is where screen time goes, but the window backend can only report "Firefox" or "Chromium". The resulting report is not actionable.
- Reading browser history databases or scraping the address bar through accessibility APIs captures full URLs (paths, query strings, tokens), is brittle across browser versions, and is disproportionate for screen-time measurement.
- A tracker that claims to measure browsing while omitting the browser is misleading. An extension can report the smallest useful unit — the site — with permissions that users can inspect.
- Background tabs must not be conflated with screen time. Counting every open tab as attention inflates reports; focused-tab attention and background media playback need separate semantics.

**Design constraints:**

- Record the **registrable domain** only (for example `example.com`), never the path, query, fragment, page title, page content, form data, cookies, or history. Normalize IDNs to punycode and bundle a Public Suffix List snapshot so `co.uk`-style domains aggregate correctly.
- **Focused tab = screen time.** The extension reports the active tab of the focused browser window, and the tracker records it only while the browser is the backend's active app.
- **Background = media playback, not attention.** Track a background tab's domain only while it is audible/playing media (`visibility=background`, `audible=true`, plus `media_playing`), and aggregate it as **background media time** with the site, browser, and duration. Do not record merely-open tabs. Never add media time to screen time. Background media is enabled with browser tracking and can be disabled by configuration. Detecting muted video requires an explicit content-script opt-in that reports only a boolean playback state; it must not read the URL or title. Because background media inspects audible/media-playing tabs, document that larger exposure explicitly.
- **Extension:** Manifest V3 for Chromium browsers (Chrome, Edge, Brave, Vivaldi) and the Firefox equivalent. Use only `tabs`/`windows` APIs; no content scripts, no `webRequest`, no `history`, no cookies, no page DOM, and no network requests. Compute the domain locally and send it over native messaging.
- **Native host:** local stdin/stdout only, with no listening sockets and no outbound network. Restrict the manifest to the published extension IDs, validate message types and lengths, and version the protocol.
- **Incognito/private windows** are excluded by default and cannot be enabled from the host side. Pausing tracking stops both app and browser events.
- **Privacy controls:** ignored domains, pause/resume, retention, export, and purge apply to browser data exactly as they do to app data. The database stays local with `0600` permissions, and no telemetry is added.
- **Fallback:** without the extension, record only the browser app name. Do not scrape history or the address bar; document the reduced detail explicitly.

**Where:** new `browser/` extension sources and native-host manifests, `micro_watcher/browser.py`, the storage domain schema, config, reporting, and privacy documentation.

**Target state:** On a supported browser, reports show focused-tab time by domain, with full URLs and titles absent from logs, database, and exports by construction. Background media is a separate section reporting **what was playing (registrable domain and browser)** and **for how long**, never mixed into screen time. It is enabled with browser tracking and can be turned off. `micro-watcher doctor` shows extension and native-host status.

---

### O-12 — Publish installable packages

**What:** Ship a supported system-wide package for Arch-based systems (AUR, consumable by `pacman`/`pamac`/`yay`) and Debian/Ubuntu (`.deb`, plus an optional apt repository), including the GNOME extension, systemd user unit, man page, and license.

**Why change it:**

- The current installer requires a checkout and manual `./install`; it has no upgrade path, no checksummed artifact, and no system-wide uninstall through the package manager.
- Package managers are how most Linux users install and upgrade CLI tools. Requiring a git checkout excludes users who do not want a compiler/toolchain.
- `pamac` and `yay` are AUR helpers, not independent repositories. An official `pacman` package requires a Trusted User sponsor; an official apt package requires a Debian/Ubuntu sponsor. The realistic first release is AUR plus a signed `.deb` (GitHub Releases), then a PPA/self-hosted apt repository.
- GNOME extensions are shell-version-sensitive and must be installed into a per-user or system extension directory; packaging is the right place to pin compatibility and enable instructions.
- The systemd user unit must not be auto-enabled by a package (users may not want tracking); packages should install it and print the opt-in command.

**What needs to happen:**

- Finish M1 packaging (`pyproject.toml`, wheel, console script) and M5 service; make the GNOME extension installable as package data.
- AUR: `PKGBUILD` + `.SRCINFO` for stable and `-git` variants, `python-installer`/`python-wheel`, correct `depends`/`optdepends`, clean-chroot build, AUR account and maintainer identity.
- Debian/Ubuntu: `debian/` source package via `dh-python`/`pybuild`, dependencies for Python 3.11+ and X11 tools, `lintian` clean, build with `sbuild`/`pbuilder`; publish `.deb` artifacts and a PPA or signed repository if `apt install micro-watcher` without a filename is required.
- Arch official repo: only after a Trusted User sponsors the package; AUR is the supported first step.
- CI: build artifacts per release, verify install/uninstall in clean containers, sign packages/repository metadata, document the channel-specific commands.
- Policy: no root at runtime, no automatic service enable, no extension auto-enable beyond the user-level installer path.

**Channel matrix:**

| User command | What it actually requires | First supported release |
| --- | --- | --- |
| `yay -S micro-watcher` | AUR package (stable) | Yes |
| `pamac build micro-watcher` | AUR package (stable) | Yes |
| `pacman -U micro-watcher-*.pkg.tar.zst` | Locally built or downloaded Arch package | Yes |
| `pacman -S micro-watcher` | Official Arch repository, or a custom signed repository added to `pacman.conf` | Only with a Trusted User sponsor or a custom repo decision |
| `apt install ./micro-watcher_*.deb` | Signed `.deb` from GitHub Releases | Yes |
| `apt install micro-watcher` | PPA or self-hosted signed apt repository with `Release`/`InRelease` metadata | Follow-up |

`pamac` and `yay` are AUR helpers; there is no separate “pamac package”. `pacman -S` does not read AUR, and `apt install <name>` does not read a downloaded `.deb` unless a repository is configured.

**Dependency sets to encode in packaging:**

- Arch: `python` (>=3.11), `xdotool`, `xprintidle`, `x11-utils`; optdepends `gnome-shell`, `pipewire-pulse`/`pulseaudio`.
- Debian/Ubuntu: `python3 (>= 3.11)`, `xdotool`, `xprintidle`, `x11-utils`; Recommends `gnome-shell`; Suggests/Recommends `pipewire-pulse` or `pulseaudio-utils`.
- Both: install the GNOME extension system-wide and the systemd user unit disabled by default; never write to `$HOME` during package installation.

**Target state:** `yay -S micro-watcher` / `pamac build micro-watcher` works from AUR; `apt install ./micro-watcher_<version>_amd64.deb` works on Debian 12+ and Ubuntu 24.04+; the package installs the command, extension, service template, man page, license, and uninstall path. A signed apt repository or PPA is documented as the follow-up for `apt install micro-watcher`.

---

## 4. Decisions needed before implementation

These are genuine forks in the road. The backlog assumes the recommended default; confirm or change before starting the affected milestone.

| ID | Decision | Recommended default | Impact if changed |
| --- | --- | --- | --- |
| D1 | **Python or Go?** | **Confirmed:** continue in Python. The Go migration branch is out of scope for this effort. | Revisit only if someone commits to maintaining a parallel Go port. |
| D2 | **Distribution and command names** | **Confirmed:** distribution `micro-watcher`; import package `micro_watcher` (underscores are required in Python); command `micro-watcher`; service `micro-watcher.service`. | Check PyPI availability before publishing; renaming later breaks scripts and docs. Legacy data under `~/.cache/Watcher` stays readable until migration. |
| D3 | **Backend priority and plugin location** | **Confirmed:** GNOME Wayland first, then KDE, then wlroots; X11 preserved. Keep compositor plugins in this repo under `extensions/gnome/` and `extensions/kde/`; the Python daemon remains the core. | GNOME focus tracking needs a Shell extension and KDE needs a KWin script/QML; both add version-specific JS/QML and packaging work, but no separate language runtime beyond the compositor's own JS engine. |
| D4 | **Storage compatibility** | Preserve history: migrate existing CSVs, keep CSV export. | A clean break is simpler but loses users' existing reports unless they migrate manually. |
| D5 | **Minimum Python version** | **Confirmed:** 3.11+, developed and tested on the current stable interpreter. | Raising the floor to 3.12 drops Debian 12 (3.11); 3.14 would drop Ubuntu 24.04 (3.12) and most current LTS installs. Keep 3.11 until its security support ends in October 2027, then bump deliberately. |
| D6 | **Browser privacy defaults** | Registrable domain only; focused tab only for screen time; no URLs, paths, queries, titles, history, cookies, or content; background **media** tracked as a separate metric; incognito excluded; local-only extension/native host. | Full URLs improve detail but create a sensitive browsing log; background media is useful but is not attention and must never be added to screen time. |
| D7 | **Naming cleanup scope** | Keep the repository name; make the product, command, unit, config, and import names consistently `micro-watcher` / `micro_watcher`. Legacy paths remain until migration. | A full data-path rename requires a migration and rollback plan; only do it with M4. |
| D8 | **Background media metric** | Enabled by default when browser tracking is enabled; store only domains with audible/media playback in background tabs, with browser and duration, as a separate metric. Never add it to screen time. | Turning it off must remove the metric entirely; muted video without sound needs an explicit content-script opt-in, and leaving it on is an accepted privacy trade-off because only media-playing tabs are inspected. |
| D9 | **Distribution channels** | AUR first (covers `yay`/`pamac`), signed `.deb` on Releases, then a PPA/self-hosted apt repo and optionally a custom signed pacman repo; official Arch/Debian repositories only with a sponsor. | Official `pacman`/`apt` package names require distribution maintainers; promising them without a sponsor would block the release. A custom pacman repo is the only way to offer `pacman -S` without Arch sponsorship. |

---

## 5. Non-goals

- GUI/Tauri frontend (the README's "300 stars" idea) — separate project, blocked on a stable backend/storage layer.
- Cloud sync, accounts, or remote telemetry — conflicts with the local-first privacy model.
- Windows/macOS support — different window/idle APIs; out of scope for this outline.
- Rewriting the project in Go — see D1; not part of the Python backlog.

---

## 6. Definition of success

The modernization is done when, on a clean supported system:

1. Installation needs no root and no network beyond dependency installation, and `micro-watcher --version` reflects package metadata.
2. `micro-watcher --start` either records accurate app/idle data or exits with a clear "unsupported session/backend" message.
3. Killing the process (including `SIGKILL`) does not corrupt previously recorded history.
4. A second instance refuses to run concurrently instead of clobbering data.
5. Weekly and daily reports are derived from one source of truth and are covered by tests.
6. Uninstall removes the service, binaries, and startup integration while leaving data unless explicitly purged.
7. Browser reports show focused-tab time by domain without storing URLs, paths, queries, titles, or page content; background media is reported separately with site, browser, and duration.
8. CI runs tests and lint on every pull request; the README commands have been verified end-to-end.

---

## 7. Outline to backlog mapping

| Outline item | Backlog milestone | Notes |
| --- | --- | --- |
| O-1 Distribution | M0, M1 | Unsafe behavior is neutralized in M0; the supported installer arrives with packaging in M1. |
| O-2 Packaging | M1 | Prerequisite for isolated installs, CI, and entry points. |
| O-3 Wayland | M3 | Depends on the backend contract and subprocess cleanup. |
| O-4 Shell/input | M0 (hotfix), M2 (full refactor) | Injection/traversal fixes cannot wait for the full CLI rewrite. |
| O-5 Storage | M4 | Independent of M3; can run in parallel after M2. |
| O-6 Autostart | M5 | Replaces the M0 stopgap with a supported service. |
| O-7 Config | M5 | Implemented after the service so paths and backend overrides are meaningful. |
| O-8 Tests/CI | M1 | Pulled ahead of refactors; expanded continuously in every later milestone. |
| O-9 Docs | M0 (safety patch), M7 (full rewrite) | The dangerous install command is removed immediately; the accurate rewrite ships with the release. |
| O-10 Housekeeping | M0 (partial), M7 | Ignore rules and module names were fixed with M0; dead code and migration tooling remain. |
| O-11 Browser domains | M6 | Depends on storage, config, privacy controls, and the backend focus signal. |
| O-12 Distribution packages | M8 | AUR and `.deb` packaging after the application is packaged and released. |
