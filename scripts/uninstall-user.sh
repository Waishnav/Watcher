#!/usr/bin/env bash
# Removes installed executables and OpenTUI; historical activity is not deleted.
set -euo pipefail
bin_dir="${WATCHER_BIN_DIR:-${HOME}/.local/bin}"
if [[ "$(uname -s)" == "Darwin" ]]; then
  tui_dir="${WATCHER_TUI_DIR:-${HOME}/Library/Application Support/watcher/tui}"
else
  tui_dir="${WATCHER_TUI_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/watcher/tui}"
fi
echo "Stop the daemon first: $bin_dir/watcher shutdown"
for executable in watcher watcherd; do
  if [[ -f "$bin_dir/$executable" ]]; then
    rm -- "$bin_dir/$executable"
  fi
done
if [[ -d "$tui_dir" ]]; then rm -r -- "$tui_dir"; fi
echo "Removed binaries and OpenTUI. Activity database retained."
