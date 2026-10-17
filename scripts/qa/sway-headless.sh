#!/usr/bin/env bash
# Sway focus QA on a fresh wlroots *headless* compositor and private Wayland bus.
# Never connect to an existing XDG_RUNTIME_DIR, SWAYSOCK or desktop session.
set -euo pipefail
if [[ "${WATCHER_HEADLESS_QA:-}" != "1" ]]; then
  echo 'Refusing to run outside an explicitly requested headless QA.' >&2
  exit 2
fi
repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
qa="$(mktemp -d)"
export XDG_RUNTIME_DIR="$qa/runtime"
export WATCHER_DEV_HOME="$qa/data"
mkdir -p "$XDG_RUNTIME_DIR" "$WATCHER_DEV_HOME"
chmod 700 "$XDG_RUNTIME_DIR" "$WATCHER_DEV_HOME"
cleanup() {
  rc=$?
  if (( rc != 0 )); then
    echo 'Headless Sway QA failed; test logs follow:' >&2
    for file in sway.log watcherd.log a.log b.log; do
      if [[ -f "$qa/$file" ]]; then
        echo "--- $file ---" >&2
        tail -80 "$qa/$file" >&2
      fi
    done
  fi
  for pid in "${watcherd_pid:-}" "${a_pid:-}" "${b_pid:-}" "${sway_pid:-}"; do
    if [[ -n "$pid" ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
  done
  rm -rf -- "$qa"
  exit "$rc"
}
trap cleanup EXIT
# Force an in-memory backend regardless of what desktop the parent shell uses.
export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=1 WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1 XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=sway
unset WAYLAND_DISPLAY WAYLAND_SOCKET DISPLAY SWAYSOCK HYPRLAND_INSTANCE_SIGNATURE
cat > "$qa/sway.conf" <<'CONFIG'
output * resolution 1280x720
focus_follows_mouse no
CONFIG
sway -c "$qa/sway.conf" --debug >"$qa/sway.log" 2>&1 & sway_pid=$!
for i in $(seq 1 100); do
  SWAYSOCK="$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'sway-ipc.*.sock' -print -quit)"
  WAYLAND_DISPLAY="$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -type s -name 'wayland-*' -printf '%f\n' -quit)"
  if [[ -n "$SWAYSOCK" && -n "$WAYLAND_DISPLAY" ]]; then break; fi
  kill -0 "$sway_pid" 2>/dev/null || { echo 'Headless compositor died at startup' >&2; exit 1; }
  sleep .1
done
[[ -n "${SWAYSOCK:-}" && -n "${WAYLAND_DISPLAY:-}" ]] || { echo 'Missing private Sway/Wayland sockets' >&2; exit 1; }
export SWAYSOCK WAYLAND_DISPLAY
swaymsg -s "$SWAYSOCK" -t get_outputs >/dev/null
"$repo/target/debug/watcherd" --dev >"$qa/watcherd.log" 2>&1 & watcherd_pid=$!
for i in $(seq 1 50); do
  [[ -S "$WATCHER_DEV_HOME/daemon.sock" ]] && break
  kill -0 "$watcherd_pid" 2>/dev/null || { echo 'watcherd died at startup' >&2; exit 1; }
  sleep .1
done
[[ -S "$WATCHER_DEV_HOME/daemon.sock" ]] || { echo 'Missing private Watcher socket' >&2; exit 1; }
foot --app-id=WatcherQaA sleep 40 >"$qa/a.log" 2>&1 & a_pid=$!
foot --app-id=WatcherQaB sleep 40 >"$qa/b.log" 2>&1 & b_pid=$!
for i in $(seq 1 100); do
  tree="$(swaymsg -s "$SWAYSOCK" -t get_tree)"
  if jq -e '[.. | objects | .app_id? // empty] | index("WatcherQaA") != null and index("WatcherQaB") != null' >/dev/null <<< "$tree"; then break; fi
  sleep .1
done
check_focus() {
  app="$1"
  swaymsg -s "$SWAYSOCK" "[app_id=\"$app\"] focus" >/dev/null
  for i in $(seq 1 40); do
    # Require independent compositor evidence AND a normalized Watcher event.
    compositor_app="$(swaymsg -s "$SWAYSOCK" -t get_tree | jq -r '.. | objects | select(.focused? == true) | .app_id? // empty' | head -1)"
    watcher_app="$("$repo/target/debug/watcher" --dev status --json | jq -r '.snapshot.app // empty')"
    if [[ "$compositor_app" == "$app" && "$watcher_app" == "$app" ]]; then
      echo "Verified private Sway focus: $app"
      return 0
    fi
    sleep .1
  done
  echo "Focus mismatch: compositor=$compositor_app watcher=$watcher_app expected=$app" >&2
  return 1
}
check_focus WatcherQaA
check_focus WatcherQaB
"$repo/target/debug/watcher" --dev shutdown >/dev/null
wait "$watcherd_pid"; watcherd_pid=''
echo 'Headless Sway QA: PASS (focus A -> B; idle protocol availability not assumed)'
