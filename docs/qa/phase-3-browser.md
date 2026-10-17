# Phase 3: isolated browser acceptance QA

Status: **Chrome headless behavior verified in an isolated profile; Firefox graphical runtime remains unverified.** This is engineering QA for experimental Phase 3, not a claim that all browser/OS features are release-ready.

## Reproducible automated Chrome coverage

After Rust binaries and the WXT extension have been built, run:

```bash
cargo build --workspace --locked
cd apps/extension
bun install --frozen-lockfile
bun run qa:chrome
```

`qa/chrome-e2e.ts` launches actual open-source Chromium (headless), creates a completely temporary **Chrome user-data-dir**, HOME, runtime directory, daemon database, and localhost HTTP fixture. It never launches a headed window, attaches to an existing browser profile, uses the real user's compositor, or registers a native host in the real user's Chrome configuration. The browser's Native Messaging host manifest is installed **inside the disposable browser user-data directory**. The daemon deliberately uses an inert, disconnected GNOME-wayland collector under a separate runtime directory so it cannot attribute any real user's application focus; desktop activity is expected to remain **unverified**. Cleanup stops this disposable daemon/browser and deletes the temp profile. CI installs Playwright's unbranded Chromium because some branded Chrome releases do not support unpacked-extension sideloading flags.

The automated test requires independent evidence for these scenarios:

| Scenario | Independent evidence required | Verified behavior |
| --- | --- | --- |
| Native host, no daemon | Actual Chrome Native Messaging response | Native host reports `daemon_unavailable`; extension remains functional |
| Offline tab A → B | Real `chrome.tabs.create` in isolated Chromium, WXT worker receives tab events | Offline domain intervals persist in extension IndexedDB and are displayed as browser-observed |
| Unfocused browser window | `chrome.windows.getAll` reports a distinct window with `focused: false`, another with `focused: true` | Background-window domain receives zero credit; otherwise scenario must be reported **inconclusive** |
| Browser restart while daemon offline | `context.close()` followed by a fresh Chromium process with the **same disposable profile** | Pending IndexedDB intervals persist, extension source identity stays stable |
| Native daemon reconnect | New `watcherd` process becomes available, Chrome actually calls Native Messaging bridge | Pending count drops to zero only after daemon acknowledges imported data |
| Repeated synchronization | Repeat extension report calls and compare daemon's own persisted `browser_report` results | Identical totals; replay does not duplicate records |
| Browser/device separation | Daemon collector signals are deliberately unavailable | Browser domain history exists; `device.active_ms` remains **zero** |

Smoke results from interactive **agent-browser** exploratory QA on 2026-10-18 corroborated the same basic flow: `alpha.test` / `beta.test` tab switches produced five queued intervals, Chrome Native Messaging returned `daemon_unavailable` without `watcherd`, a later refresh synchronized all five into a disposable v3 SQLite database, and a second refresh retained an unchanged row count. After shutting down the daemon, two additional records stayed queued across a full Chrome process restart and were synchronized later. SQLite's integrity check returned `ok` and OS-active time remained zero. An independently unfocused window was also observed without accumulating domain activity.

## Important remaining gates

1. **Firefox live integration**: compile/bundle success does not establish actual sidebar, native messaging registration, or Firefox background lifecycle. Run the extension using a temporary Firefox profile, verify tab A/B focus and correct pending/ack behavior without touching a maintainer's browser.
2. **MV3 worker sleep and suspension**: explicitly terminate/restart worker independently of the browser, verify no invented time and restoration, and check alarms/queue retention under realistic browser power policies.
3. **Private windows**: test with browser private-window permission *explicitly enabled*, verify that incognito tabs are excluded at the collector layer. Default browser denial alone is not enough.
4. **Idle/locked periods, clock jumps, and long activity**: test independent browser idle observations, foreground inactivity, laptop suspend/resume and wall-clock changes. Never turn unknown gaps into active time.
5. **Production native host registration**: current script is opt-in Linux/macOS developer registration only. Windows registration, packaging, lifecycle and uninstall are pending.
6. **OS window ↔ browser tab identity matching**: unavailable for many supported desktops. Do not present unverified domain time as device-verified; avoid double counting.
7. **Retention/quota controls and polished UX**: offline IndexedDB queue currently has no bounded retention or user-facing data erasure management. The UI foundation is shadcn-compatible; richer ReUI interactions need focused accessibility/visual QA.

Always record independent evidence of actual browser window/tab focus transitions. A scenario that fails to produce the required focus change is **inconclusive**, never an accuracy pass.
