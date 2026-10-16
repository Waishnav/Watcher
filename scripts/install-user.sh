#!/usr/bin/env bash
# Install the Phase 1 binaries in the current user's home, without root access.
set -euo pipefail

repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="${WATCHER_BIN_DIR:-${HOME}/.local/bin}"
tui_dir="${WATCHER_TUI_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/watcher/tui}"

command -v cargo >/dev/null || { echo "Rust/cargo required" >&2; exit 1; }
command -v bun >/dev/null || { echo "Bun required for OpenTUI" >&2; exit 1; }
cd "$repo"
cargo build --release --locked -p watcher -p watcherd
mkdir -p "$bin_dir" "$tui_dir/src"
install -m 755 target/release/watcher "$bin_dir/watcher"
install -m 755 target/release/watcherd "$bin_dir/watcherd"
cp apps/tui/package.json apps/tui/bun.lock apps/tui/tsconfig.json "$tui_dir/"
cp apps/tui/src/index.ts "$tui_dir/src/index.ts"
(
  cd "$tui_dir"
  bun install --frozen-lockfile --production
)
printf 'Installed Watcher to %s and OpenTUI to %s\n' "$bin_dir" "$tui_dir"
echo "Nothing was added to autostart. See README.md for optional Hyprland autostart."
