# Explicitly opt-in desktop focus bridges (experimental)

GNOME Shell and KDE Plasma Wayland cannot be treated as generic Wayland focus APIs. These two minimal scripts publish **only the app ID/class** to the per-user local Watcher D-Bus service `org.watcher.FocusSink`. No window title, keyboard event, or app content is collected. A separate Wayland input-idle notifier v2 must also be exposed by the compositor for **active** time to be credited.

**Neither bridge has been installed/tested against a real GNOME/KWin desktop.** Installing or enabling extensions can affect your desktop; never do this in a normal QA session or automatically. They are source-level experimental integrations, not a supported release yet.

## GNOME Shell (45+ syntax)

Copy `gnome/watcher-focus@watcher` to `~/.local/share/gnome-shell/extensions/watcher-focus@watcher`, restart the shell/session as required **at your discretion**, and enable `watcher-focus@watcher` via the desktop extension manager. The extension listens to `notify::focus-window`; it publishes changes asynchronously and republishes the current app every five seconds to recover after daemon restarts.

## KDE Plasma / KWin 6

Package `kwin/watcher-focus` as a KWin script with `kpackagetool6 --type=KWin/Script --install <package-dir>` only when deliberately testing on KDE Plasma 6. Enable it through KWin Scripts preferences; check version-specific KWin behavior. This script emits `windowActivated` and an initial snapshot, without polling. Unlike GNOME, no heartbeat exists yet, so daemon-side liveness and recovery must be verified before promoting Plasma to supported.

## Developer security note

The bridge method can be called by other applications owned by the current user on the same session bus. Treat app IDs as untrusted and cap their length. In future releases, authenticating the sender or using a private capability token can reduce injection risks. For now, these desktop integrations are experimental.
