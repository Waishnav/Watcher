# micro-watcher

A local-first screen-time tracker for Linux, written in Python. It records the
focused application, pauses when you are idle (unless media is playing), and
prints daily or weekly reports from a local CSV store.

This fork stays on Python 3; the upstream Go migration branch is not used here.
It currently targets **GNOME Wayland** and **X11**, with KDE and wlroots
backends planned.

## Status

Development snapshot. The GNOME Wayland backend, safety fixes, and user-local
installer are implemented; distro packages and browser tracking are not yet
published. See [`agents/OUTLINE.md`](agents/OUTLINE.md) for the design and
[`agents/BACKLOG.md`](agents/BACKLOG.md) for the ordered work plan.

## Features

- Daily and weekly screen-time reports per application.
- Active-window tracking on X11 and GNOME Wayland.
- Idle detection via `xprintidle` on X11 and Mutter's `IdleMonitor` on Wayland.
- Media-aware idle handling through `pactl` (PulseAudio/PipeWire).
- Local-only data; no network calls or telemetry.
- User-local installation with no `sudo`, no downloaded installer, and no
  edits to shell startup files.

Planned:

- KDE Wayland and wlroots backends.
- Browser domain tracking (focused tab only) with a separate **background media**
  metric; no URLs, page titles, history, or page content.
- Crash-safe storage and migration from the current CSVs.
- A systemd user service.
- AUR and Debian/Ubuntu packages.

## Support matrix

| Platform | Window tracking | Idle tracking | Status |
| --- | --- | --- | --- |
| X11 | `xdotool` + `xprop` | `xprintidle` | Supported |
| GNOME Wayland 45+ | GNOME Shell extension (`gdbus`) | Mutter `IdleMonitor` | Supported; one re-login after install |
| KDE Wayland | KWin plugin (planned) | Compositor idle API (planned) | Planned |
| wlroots (Sway/Hyprland) | `swaymsg` / `hyprctl` (planned) | Compositor idle API (planned) | Planned |
| GNOME < 45 | — | — | Not supported |
| Windows/macOS | — | — | Not planned |

The current GNOME test environment is GNOME Shell 51.1 on Wayland. X11 support
is preserved in code but is not exercised on that machine.

## Requirements

- Python **3.11 or newer**.
- **GNOME Wayland:** GNOME Shell 45+, `gdbus` (GLib), and the bundled
  `micro-watcher Focus` extension. No X11 tools are required.
- **X11:** `xdotool`, `xprintidle`, and `xprop` (usually in `x11-utils`).
- **Optional:** `pactl` from PulseAudio or PipeWire for media-aware AFK
  detection.

Arch-based systems (X11 only):

```bash
sudo pacman -S xdotool xprintidle x11-utils
```

Debian/Ubuntu (X11 only):

```bash
sudo apt install xdotool xprintidle x11-utils
```

Package-manager installation of micro-watcher itself is not available yet; see
[Distribution](#distribution).

## Installation

### From this checkout

```bash
# Clone the repository, then from its root:
./install
```

The installer:

- copies the reviewed checkout to `~/.local/share/micro-watcher`;
- creates the `~/.local/bin/micro-watcher` launcher;
- installs and queues the GNOME focus extension when GNOME is available;
- creates `~/.cache/Watcher` for recorded data.

It does **not** use `sudo`, download code, or edit `~/.xprofile` /
`~/.xinitrc`.

If `~/.local/bin` is not on `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

On GNOME Wayland, log out and back in once after the first install so GNOME
Shell loads the extension. Verify it with:

```bash
gnome-extensions info micro-watcher@alberto-caballero.github.io
gdbus call --session \
  --dest org.micro_watcher.Focus \
  --object-path /org/micro_watcher/Focus \
  --method org.micro_watcher.Focus.GetActiveWindow
```

Start tracking:

```bash
micro-watcher --start
```

On GNOME Wayland the command prints `Tracking via GNOME Wayland...` when the
extension is loaded. Before the first re-login it exits with an actionable
message instead of recording incorrect data.

### Uninstall

```bash
./uninstall
```

This removes the launcher, the user-local installation, and the GNOME extension.
Recorded data in `~/.cache/Watcher` is kept unless you explicitly confirm
deletion. A legacy system-wide install, if present, is reported for manual
removal.

## Usage

```bash
micro-watcher --help
micro-watcher -ds              # today's report
micro-watcher -ds 2026-11-20   # a specific day
micro-watcher -ds --yesterday  # yesterday
micro-watcher -ws              # this ISO week
micro-watcher -ws W01-2026     # a specific ISO week
micro-watcher -ws --previous_week
micro-watcher --start          # record until Ctrl+C
```

Dates use `YYYY-MM-DD`; weeks use `Www-YYYY` (ISO week and ISO year). Invalid
values are rejected before any file or command is touched.

## Data and privacy

- Data lives in `~/.cache/Watcher/daily_data/<date>.csv` (tab-separated).
- The tracker stores application class names and durations. Window titles are
  read transiently to distinguish Neovim/Vim inside a terminal; they are not
  written to disk.
- The GNOME extension exposes the focused window's app ID, class, title, and PID
  to processes in your user session. It has no network access. This is similar
  to what X11 already exposes to local clients.
- No telemetry, accounts, or cloud sync.

Browser tracking is planned as a separate extension and native-messaging host:
only the registrable domain is stored, never URLs, paths, queries, page titles,
history, cookies, or content. Background media will be reported separately from
screen time, with site, browser, and duration. See
[`agents/OUTLINE.md`](agents/OUTLINE.md).

## Development

Run the tests from the repository root:

```bash
python3 -m pytest
```

Design and execution documents:

- [`agents/OUTLINE.md`](agents/OUTLINE.md) — goals, privacy model, waypoints.
- [`agents/BACKLOG.md`](agents/BACKLOG.md) — ordered tasks (M0–M8).
- [`extensions/gnome/README.md`](extensions/gnome/README.md) — extension
  interface, installation, and local trust boundary.

## Distribution

No micro-watcher packages are published yet. The supported installation today
is `./install` from this checkout.

The plan, in order:

| Channel | Command | Status |
| --- | --- | --- |
| AUR (Arch) | `yay -S micro-watcher`, `pamac build micro-watcher` | Planned |
| Arch local package | `pacman -U micro-watcher-*.pkg.tar.zst` | Planned |
| Arch official repo | `pacman -S micro-watcher` | Requires a Trusted User sponsor, or a custom signed repo |
| Debian/Ubuntu package | `apt install ./micro-watcher_*.deb` | Planned |
| apt repository/PPA | `apt install micro-watcher` | Follow-up; requires a signed repository |

`pamac` and `yay` are AUR helpers, not separate package formats. `pacman -S`
does not read the AUR, and `apt install micro-watcher` requires a configured
repository. Packaging work is tracked as **M8** in the backlog and described in
[`agents/OUTLINE.md`](agents/OUTLINE.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Keep changes consistent with the
ordered plan; every behavior change should include tests.

## License

MIT. Upstream attribution is preserved in [LICENSE](LICENSE).
