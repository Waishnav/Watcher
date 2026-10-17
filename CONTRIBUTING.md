# Contributing to Watcher

Watcher Phase 1 implements a Linux/Hyprland vertical slice. Read the [architecture](docs/plans/architecture.md), [Phase 1](docs/plans/phase-1.md) / [Phase 2](docs/plans/phase-2.md) plans, and [manual QA notes](docs/qa/phase-1.md) before starting implementation.

Keep platform-specific functionality behind collector interfaces. Ensure unsupported or permission-denied behavior is explicit, not disguised as zero usage. Avoid reading personal window titles or browser URLs in CI fixtures. Prefer small, focused changes with manual verification notes and screenshots or event traces when relevant.

## Baseline checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd apps/tui && bun install --frozen-lockfile && bun run check)
```

Never run a manual QA script that switches the user's current workspace or steals focus without an isolated compositor session. Prefer passive observation of ordinary desktop changes. The `--dev` profile uses isolated data and socket paths; never use production data for QA.

```sh
# User-session smoke, only when testing locally:
./target/debug/watcherd --dev
# In another terminal:
./target/debug/watcher --dev doctor
./target/debug/watcher --dev today --json
./target/debug/watcher --dev shutdown
```

The previous Python implementation and contribution notes live in [legacy/](legacy/CONTRIBUTING.md); they are not the guidance for new contributions.
