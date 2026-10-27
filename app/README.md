# app/

The Facet desktop app: Tauri 2 (Rust core) + React + TypeScript + Vite. Linux first (Phase 1).

Unofficial, not affiliated with Anthropic. The rules in `../CLAUDE.md` apply to every line here; `../docs/04-plan/DECISIONS.md` D-012 to D-014 record how Phase 1 implements them.

## Setup (Linux)

Done on the development machine on 2026-10-26:
- Rust via rustup (user-level): stable 1.99.1 with clippy and rustfmt.
- Tauri 2 system packages (sudo apt): `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`.
- Node 20 (nvm). Test tools are pinned to versions that support Node 20 (Vitest 4.1, jsdom 29).
- For screenshots without touching the desktop: `xvfb` and `imagemagick`.

Then: `npm ci`.

Dependencies are pinned to exact versions published at least two weeks earlier (supply-chain rule).

## Run

| Command | What it does |
|---|---|
| `npm run dev` | Browser preview at http://127.0.0.1:1420 with **sample data** (clearly labelled; touches nothing) |
| `npx tauri dev` | The real app with hot reload |
| `npx tauri build --bundles deb` | Release `.deb` (about 3.4 MB) in `src-tauri/target/release/bundle/deb/` |
| `npx tauri build --bundles appimage` | Release AppImage (about 85 MB, bundles WebKitGTK) |
| `facet --launch <id>` | Start or bring forward an account's Desktop instance (used by menu entries) |
| `facet --switcher` | Open the quick switcher in the running Facet (bind it to a global shortcut) |
| `cargo run --example health_report [-- --sign-in]` | Real Health checks without the UI; set `FACET_PROFILES` to try another accounts file |

Facet keeps its own files in `~/.config/facet/` (`profiles.toml` 0600, `rules.toml`) and `~/.local/state/facet/launched.json`. It writes, only when you ask: `~/.local/bin/claude-<id>`, `~/.local/bin/claude-auto`, `~/.local/share/applications/facet-<id>.desktop`, `~/.local/share/facet/` (icons, terminal start files). It never writes inside Claude's folders in Phase 1.

## Test

| Command | Covers |
|---|---|
| `cd src-tauri && cargo test` | 97 tests: env scrubbing (and a lint that every process goes through `env::command`), profiles (badges, rename, reorder), pictures and launcher icons, `/proc` scanning, launch and resume commands, terminals, shims, launchers, registry allow-list, sign-in parsing, Health rules, folder rules (runs the generated script with `sh`), launch records, Control, Studio, transcript usage and cost (no message text kept), timeline events, Ecosystem scans and inventory, Bridge handoffs (real git repo) |
| `cd src-tauri && cargo clippy --all-targets -- -D warnings` | Lints |
| `npm test` | 76 tests: app flows on the sample backend (launch, errors with retry, scope and tint, add/edit/remove, reserved names, dialog focus, Health scope, quick switcher, folder rules), parity with Rust helpers, design rules (tokens copy, no raw hex, no physical left/right, product naming) |
| `npx tsc --noEmit` | Strict types |
| `npm run tokens` | Copy `docs/05-design/tokens.css` into the app (a test fails if they drift) |
| `cargo run --release --example usage_report` | Usage and cost per account from your transcripts, compared with Claude Code's own figure (totals only) |
| `cargo run --release --example ecosystem_report` | What each account has installed, with risk levels (names and counts only) |

## Layout

```
app/
  src/                     React UI
    App.tsx                shell: account rail, section nav, views, tint, quick switcher
    components/            design-system components (ui.tsx), icons, toast, command palette
    views/                 Today, Timeline, Accounts (+ editor), Costs, Control, Studio, Ecosystem, Sessions, Folder rules, Bridge, Health, Settings
    lib/api.ts             the contract with the Rust core; lib/sample.ts is the sample backend
    styles/                tokens.css (copied), fonts.css (bundled OFL fonts), app.css
  src-tauri/src/
    env.rs                 the only way to start a process: removes override variables
    profile.rs             accounts store (no secrets), validation, detection of existing setups
    adapter/linux.rs       find/launch/quit Desktop, /proc scan, terminals, shims, launchers, X11 identity
    registry.rs            Claude Code session registry (allow-listed), misplaced-session detection
    auth.rs                `claude auth status --json`
    settings.rs            read-only settings checks (retention, override env, apiKeyHelper)
    health.rs              Health checks (pure, tested) and fact gathering
    rules.rs               folder rules and the claude-auto script
    launched.rs            instances Facet started, with process start times
    avatar.rs              account pictures and launcher icons
    usage.rs               usage, cost and timeline events from transcript metadata (D-019)
    ecosystem.rs           read-only plugin/repo safety scan and per-account inventory (D-020)
    bridge.rs              handoff files (D-021)
    control.rs, effective.rs, studio.rs   settings editor, effective settings, Studio
    commands.rs, lib.rs    Tauri commands, tray, single instance, state poller
```

## Phase 1 status (roadmap)

| Roadmap item | Status |
|---|---|
| Tauri skeleton, design tokens, account rail, Accounts view | Done |
| Profile store, process supervisor, Linux adapter (instances, X11 identity, launchers) | Done |
| Environment scrubbing | Done, enforced by a test (D-013) |
| Terminal per account; `claude-<id>` commands; folder rules with `claude-auto` | Done |
| Tray, single instance, quick switcher (Ctrl K, `facet --switcher`) | Done. Binding a global shortcut through the GlobalShortcuts portal is not built yet; bind `facet --switcher` in GNOME keyboard settings meanwhile |
| Health, first checks | Done (installs and official builds, override variables, shell files, settings, retention, sign-in, misplaced Code sessions, unknown instances) |
| Packages: AppImage and `.deb` | Built and smoke-tested on Ubuntu 26.04 |
| Guided sign-in with email verification | Blocked on Phase 0 test 5 (needs the user); the email check via `claude auth status` already works |

## Phase 2 status

| Item | Status |
|---|---|
| Control: per-account settings editor (model, output style, retention, permission mode and rules, extra folders, inbound messages, hooks, project MCP servers, skill shell commands, auto memory, environment) | Done: diff before save, private backups (last 30), restore with preview, refusal of files with sign-in values, secret values hidden (D-015) |
| Studio v1: library, templates, live lint, placement per account with drift and backups, import | Done |
| Effective settings across user, project, local and managed layers | Done |
| Copy settings between accounts | Done. Presets and keychain secrets: not yet |
| Full account control: rename (with `claude-<id>`), pictures, badges, notes, dock icon, folders, order (D-018) | Done |
| Today, Costs, Sessions from transcript metadata, with resume and CSV export (D-019; Phase 3 items pulled forward) | Done |
| Timeline: lanes, semantic zoom, inspector, replay (Phase 3, pulled forward) | Done (v1 from transcripts; OTel feed later) |
| Ecosystem: inventory, plugin on/off, safety preview, reviewed-version marks (D-020; Phase 4, pulled forward) | Done |
| Bridge v1: handoff files and per-account inbound messages (D-010, D-021) | Done |
| MCP server editing (through `claude mcp`) | Not yet |
| Hook workbench, trigger lab (`claude plugin eval`), AI-assisted drafting | Not yet (Studio v2, Phase 4) |
| Windows and macOS adapters | Not yet (needs those machines; test 9) |
