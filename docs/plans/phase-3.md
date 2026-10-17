# Phase 3 — browser intelligence (draft implementation)

Status: **draft/experimental, stacked on `feat/cross-platform-ci` (PR #54)**. No browser store releases, interactive browser QA, nor system autostart is implied by green build checks. Browser data remains local and is never uploaded to a remote backend.

## Decision and invariants

- WXT + React + Tailwind 4 with shadcn-compatible local UI source; ReUI may supply additional registry components once the panel's live UX is verified. No separate web server or remote account is necessary.
- Browser-only mode is **first-class**: observations persist to extension IndexedDB regardless of daemon availability, and remain viewable locally.
- A short-lived Rust native host (`watcher-native-host`) is launched by the browser's Native Messaging interface and proxies a strictly allowlisted subset of versioned JSON IPC requests to the daemon. It cannot pause, resume or stop the daemon. Chrome needs an explicit extension-ID native host registration; Firefox has a fixed ID of `watcher@watcher.local`.
- **At-least-once delivery with idempotent writes**: the browser writes to IndexedDB first, sends up to 64 unsynced intervals per attempt, and marks only daemon-acknowledged intervals synced. A daemon restart, native-host failure, or duplicate replay must not discard or double-insert local records.
- Browser-observed totals are not **device-verified active interaction time** and must never be added to the OS active total. The new v3 `browser_intervals` table is independent of `activity_intervals`. Device report in the sidepanel uses the existing daemon's OS interval report.
- Never store full URLs, queries, paths, fragments, page titles or incognito tab data. Domain names are parsed with the Public Suffix List-aware `tldts` package. Browser permissions are `tabs`, `idle`, `storage`, `alarms`, `nativeMessaging`; by default browsers do not grant extension access in private windows.
- MV3 service workers may sleep. Store all closed intervals transactionally in IndexedDB. On a worker restart, do **not** attribute unobserved downtime; cap interval length to two minutes and checkpoint on a browser alarm.

## Present vertical slice

1. `apps/extension`: WXT builds `chrome-mv3` and `firefox-mv2`; sidepanel has live browser-domain leaderboard and per-device active/idle/unknown app totals when available. Domain-only idle/active tab observations go into IndexedDB. Closed intervals remain locally even after synchronization.
2. `apps/native-host`: small Rust stdio framed JSON Native Messaging adapter (host name `dev.watcher.bridge`). Browser-facing operations are limited to status, device report, browser ingest, browser-observed domain report. No shell commands, file reading, or privileged operations. Binary framing needs binary CRT mode on Windows.
3. `watcherd` / SQLite v3: `browser_intervals(source_id, seq, start_ms, end_ms, domain)` with unique sequence per installation, timestamp/domain validation and transactionally idempotent batch insertion. Native host transparently reuses the existing Unix/named-pipe transport. Database v1→v3 and v2→v3 migrations are backed up before mutation.
4. `scripts/register-native-host.py`: explicit developer-only registration on Linux/macOS with native-host executable and (for Chrome) a real browser extension ID. It never touches the user's browser settings unless explicitly invoked.

## Development

```sh
cargo build --workspace --locked
cd apps/extension
bun install --frozen-lockfile
bun run check && bun run test
bun run build && bun run build:firefox
```

Chrome: visit `chrome://extensions`, enable Developer Mode, load `apps/extension/.output/chrome-mv3`. Firefox: use `about:debugging#/runtime/this-firefox` and load `apps/extension/.output/firefox-mv2/manifest.json`. Browser-only mode works without any native companion; only device reports and ingestion need it.

For native-host testing without touching the installed companion, build `watcher-native-host` then run the per-user registration helper with the **real** extension ID shown in Chrome's extension manager. Note that this helper currently registers a normal release-profile host, not a `--dev` test profile. For isolated integration tests, run the host directly with `--dev` and `WATCHER_DEV_HOME` instead of registering it in your daily browser.

Firefox has explicitly declared `websiteActivity` data-collection metadata because domain observations can leave the extension for local daemon storage. **No information is transmitted to an external service.** Explicit opt-in to private-window access should remain off; if enabled manually, a collector-level private-window exclusion is still required before distribution.

## Work remaining before Phase 3 acceptance

- Runtime QA with Chrome and Firefox: sidepanel open, tab A→B, browser focus loss, browser idle/lock, restart of MV3 worker, private windows, multiple windows/profiles, daemon offline/online replay, and native-host registration/uninstallation.
- Improve source identity and OS/browser foreground-window matching. Current browser totals are **browser-observed only**. A domain may not be labeled OS-verified merely because the browser app is foreground, until window/profile matching is reliable.
- Explicit detection and exclusion of private windows inside the collector regardless of browser settings; permission/onboarding UX and opt-in retention/erase controls.
- Enforce a bounded IndexedDB retention/outbox policy and warnings for quota failures; handle long idle gaps and inaccurate wall-clock timestamps conservatively. Add tests exercising IndexedDB transactions and the service worker lifecycle, not just pure domain parsing.
- Windows per-user native host registration and true Windows/Firefox/Chrome runtime tests. macOS/Linux installer integration and browser store signing.
- Additional ReUI/shadcn components and accessible interaction QA once the basic data pipeline has been verified. Full browser/device reconciliation in Rust with explicit coverage remains separate from browser-only analytics.
