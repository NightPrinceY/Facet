# Facet

**Keep your Claude accounts apart. See what each one costs. Control every setting. Know what a plugin does before you install it.**

Facet is an unofficial desktop app for people who use more than one Claude account: an employer seat and a personal plan, or one account per client. It is not affiliated with or endorsed by Anthropic. Claude is a trademark of Anthropic, PBC.

> Status: **0.2.4, Linux.** Working daily on Ubuntu (GNOME). Windows and macOS come later. Plan: `docs/04-plan/ROADMAP.md`.

## What it does

- **Accounts.** Each account gets its own Claude Desktop and Claude Code folders, launched side by side with its own name, colour and dock icon. Rename, reorder, add a picture or badge, open a terminal as any account (`claude-<name>`), and pick the account by folder (`claude-auto`).
- **Links per account.** Links that a Claude window opens go to that account's own browser profile (Chrome, Chromium, Brave, Edge, Vivaldi, Firefox).
- **Today, Costs, Sessions, Timeline.** Usage and API-equivalent cost per account, project, model and session, from Claude Code's own transcripts, without reading message text. Search sessions and resume one in the right account. Every account on one time axis, zoomable to the second.
- **Control.** Edit each account's Claude Code settings with a diff before saving, private backups and restore. See what applies after managed, project and account settings merge. Copy settings between accounts.
- **Studio.** Write skills, subagents and commands, lint them, and place them in chosen accounts.
- **Ecosystem.** What each account has installed, with a read-only safety check of plugins, MCP servers and hooks. Add or remove MCP servers through the official `claude mcp` command.
- **Bridge.** Share context between your own accounts with a handoff file you open in the other account.
- **Health.** Checks for the things that quietly break multi-account setups.

## Rules Facet keeps

- Launches only the official, unmodified Claude apps.
- Never reads, copies or backs up sign-in data.
- Makes no network calls to Anthropic.
- Never moves work between accounts or pools limits.
- Never clicks or types inside the Claude apps.
- Backs up any file before editing it.

The full list is in `CLAUDE.md`.

## Build and run

Requirements: Rust, Node 20 and the Tauri 2 Linux dependencies (WebKitGTK 4.1). Details, checks and packaging are in `app/README.md`.

```sh
cd app
npm install
npx tauri dev          # run
npx tauri build        # .deb and AppImage in src-tauri/target/release/bundle/
```

`npm run dev` opens a browser preview with clearly labelled sample data.

## Project map

`CLAUDE.md` maps the repository: research in `docs/01-research`, product in `docs/02-product`, architecture in `docs/03-architecture`, plan and decisions in `docs/04-plan`, design system in `docs/05-design`, validation spikes in `spikes/`, and the app in `app/`.
