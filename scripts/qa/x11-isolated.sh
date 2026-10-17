#!/usr/bin/env bash
# Run only *inside* xvfb-run with a disposable DISPLAY. NEVER use on a desktop.
set -euo pipefail
if [[ "${WATCHER_XVFB_QA:-}" != "1" ]]; then
  echo 'Refusing to run: WATCHER_XVFB_QA=1 is required, via xvfb-run.' >&2
  exit 2
fi
if [[ -z "${DISPLAY:-}" ]] || ! (pgrep -a -x Xvfb | grep -F -- "${DISPLAY}" >/dev/null); then
  echo "Refusing to run: DISPLAY must belong to a running Xvfb process, not your desktop." >&2
  exit 2
fi
repo="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
export XDG_SESSION_TYPE=x11
unset HYPRLAND_INSTANCE_SIGNATURE SWAYSOCK WAYLAND_DISPLAY
export WATCHER_DEV_HOME="$(mktemp -d)"
cleanup() {
  if [[ -n "${daemon_pid:-}" ]]; then kill "$daemon_pid" 2>/dev/null || true; wait "$daemon_pid" 2>/dev/null || true; fi
  if [[ -n "${a_pid:-}" ]]; then kill "$a_pid" 2>/dev/null || true; wait "$a_pid" 2>/dev/null || true; fi
  if [[ -n "${b_pid:-}" ]]; then kill "$b_pid" 2>/dev/null || true; wait "$b_pid" 2>/dev/null || true; fi
  if [[ -n "${wm_pid:-}" ]]; then kill "$wm_pid" 2>/dev/null || true; wait "$wm_pid" 2>/dev/null || true; fi
  rm -rf -- "$WATCHER_DEV_HOME"
}
trap cleanup EXIT
openbox --sm-disable >"$WATCHER_DEV_HOME/openbox.log" 2>&1 & wm_pid=$!
sleep 2
"$repo/target/debug/watcherd" --dev >"$WATCHER_DEV_HOME/daemon.log" 2>&1 & daemon_pid=$!
for i in {1..30}; do [[ -S "$WATCHER_DEV_HOME/daemon.sock" ]] && break; sleep 0.2; done
"$repo/target/debug/watcher" --dev doctor
xterm -class WatcherQaA -e sleep 30 >"$WATCHER_DEV_HOME/a.log" 2>&1 & a_pid=$!
xterm -class WatcherQaB -e sleep 30 >"$WATCHER_DEV_HOME/b.log" 2>&1 & b_pid=$!
sleep 2
window_a="$(xdotool search --onlyvisible --class WatcherQaA | head -1)"
window_b="$(xdotool search --onlyvisible --class WatcherQaB | head -1)"
[[ -n "$window_a" && -n "$window_b" ]] || { echo 'X11 QA windows unavailable' >&2; exit 1; }
xdotool windowactivate --sync "$window_a"; xdotool mousemove 100 100; sleep 3
xdotool windowactivate --sync "$window_b"; xdotool mousemove 110 110; sleep 3
"$repo/target/debug/watcher" --dev today --json >"$WATCHER_DEV_HOME/report.json"
python3 - "$WATCHER_DEV_HOME/report.json" "$WATCHER_DEV_HOME/watcher.sqlite" <<'PY'
import json, sqlite3, sys
report = json.load(open(sys.argv[1]))
assert report['kind']=='report', report
apps = {r['app']:r['duration_ms'] for r in report['report']['apps']}
print('X11 QA observed application totals:', apps)
assert apps.get('WatcherQaA', 0) > 500, 'No credited active time for A'
assert apps.get('WatcherQaB', 0) > 500, 'No credited active time for B'
db = sqlite3.connect(sys.argv[2])
assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
overlaps = db.execute('''SELECT count(*) FROM activity_intervals a
  JOIN activity_intervals b ON a.id < b.id
  AND a.start_ms < b.end_ms AND b.start_ms < a.end_ms''').fetchone()[0]
assert overlaps == 0, f'{overlaps} overlaps found'
print('X11 QA: PASS — isolated focus switching and SQLite integrity')
PY
"$repo/target/debug/watcher" --dev shutdown
