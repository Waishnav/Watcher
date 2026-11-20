# micro-watcher Focus (GNOME Shell extension)

GNOME does not expose the focused window to unprivileged processes. This
extension runs inside GNOME Shell and exposes the minimum needed by the local
micro-watcher process over the session bus:

- application ID
- window manager class
- window title (used only to distinguish Neovim/Vim inside a terminal; never
  written to the screen-time database)
- process ID

No network access is performed. The extension only answers local DBus calls
from the current user's session.

## Interface

- Bus name: `org.micro_watcher.Focus`
- Object path: `/org/micro_watcher/Focus`
- Interface: `org.micro_watcher.Focus`
- Methods: `GetActiveWindow() -> (ssss)` and `GetVersion() -> s`

## Installation

`./install` copies this directory to
`~/.local/share/gnome-shell/extensions/micro-watcher@alberto-caballero.github.io`
and enables it. GNOME Shell only scans new extensions at startup, so log out and
back in once after the first install. Verify with:

```bash
gnome-extensions info micro-watcher@alberto-caballero.github.io
gdbus call --session \
  --dest org.micro_watcher.Focus \
  --object-path /org/micro_watcher/Focus \
  --method org.micro_watcher.Focus.GetActiveWindow
```

To remove it without removing the tracker:

```bash
gnome-extensions disable micro-watcher@alberto-caballero.github.io
rm -rf ~/.local/share/gnome-shell/extensions/micro-watcher@alberto-caballero.github.io
```

## Compatibility

The metadata declares GNOME Shell 45–51. It uses the ESM extension API available
from GNOME 45 onward. Older GNOME versions are not supported by this extension.

## Privacy

This extension gives any process in the user's session access to the focused
window's app ID, class, title, and PID. That is the same kind of information
already available to X11 clients; it is not a sandbox boundary. Do not install it
if you do not trust local processes running as your user.
