# Facet

Facet is a cross-platform desktop app (Linux first, then Windows, then macOS) that keeps a person's Claude accounts separate, shows what each one costs down to the second, gives full control of each account's Claude Code settings, and previews what a skill, plugin or MCP server will do before it is installed.

Working name only: run a trademark search before any public launch (see `docs/02-product/NAMING.md`).

## Where things are

| Path | What it holds |
|---|---|
| `docs/00-context/SESSION-HISTORY.md` | How the project started, the proven Linux setup, every decision so far. Read first. |
| `docs/01-research/REPORT.md` | The research verdict (demand, competitors, feasibility, terms, business). Notes in `notes/`. |
| `docs/02-product/` | Vision, PRD, the Timeline spec, personas and pricing, naming. |
| `docs/03-architecture/` | Architecture, data model, platform adapters, security and compliance. `EXTENSION-SURFACE.md` is the reference for every Claude Code file and CLI command Facet may read or write (skills, agents, hooks, plugins, MCP, settings). |
| `docs/04-plan/` | Roadmap, Phase 0 tests, risks, decision log. |
| `docs/05-design/` | Design system, tokens (`tokens.css`, `tokens.json`), component specs. Browsable copy: https://claude.ai/artifact/6wMDMEo9PBxuFNoqWF1J5h (files mirrored in `design-system-artifact/`). |
| `prototype/facet.html` | Interactive prototype (single HTML file, sample data). Published copy: https://claude.ai/artifact/C5rDdF5JtC5tebDtZ99Dpc |
| `spikes/` | Phase 0 experiments. Each has a README with the question, the method and a Result section. |
| `reference/linux-setup/` | The working hand-made setup (second Claude Desktop instance + dock fix). |
| `app/` | The real app (Tauri 2 + Rust + React + TypeScript). Phase 1 skeleton built; how to run, test and build is in `app/README.md`. |

## Current phase

**Phase 1 (Linux core) built and in daily-use testing; Phase 2 under way (Control and Studio v1 built).** Phase 0: tests 1, 3, 4, 8 and 11 done; 2 and 7 answered with a user check left; tests 5 and 6 need the user and gate guided sign-in and Desktop config editing (status column in `docs/04-plan/PHASE0-TESTS.md`). Phase 1 status table: `app/README.md`. Next: Phase 2 Control and Studio v1. Before any change in `app/`, run the checks listed in `app/README.md` (they also run in `.github/workflows/ci.yml`).

## Non-negotiable rules

These come from Anthropic's terms and the research. Every change must respect them.

1. Launch only the official, unmodified Claude binaries. Never patch, repackage or re-sign them.
2. Never read, copy, back up, sync or log sign-in data: `.credentials.json`, keychain entries, Electron cookies or session stores, OAuth tokens.
3. Facet makes no network calls to Anthropic. Telemetry is received on localhost only.
4. No automatic account rotation or limit pooling, and never market "more quota".
5. Never click or type inside the Claude apps (no UI automation). Cross-session features use only documented Claude Code interfaces, triggered by the user.
6. Scrub `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` and `CLAUDE_CODE_OAUTH_TOKEN` from every process Facet starts.
7. Back up any config before editing it. Edit Claude Desktop config only while that instance is stopped. Never touch managed or enterprise policy files.
8. No "Claude" in the product name, logo, domain, bundle id or package name. Say "unofficial" and "not affiliated with Anthropic".
9. Prompt text and message content are off by default, stored only locally, and never in telemetry or crash reports.

When in doubt, use the `compliance-reviewer` agent in `.claude/agents/`.

## How to work here

- Write docs and code in plain English. Keep the Arabic summary at the end of user-facing reports.
- Prefer facts with sources. Mark anything unverified as such.
- Record every product or architecture decision in `docs/04-plan/DECISIONS.md` (use `/decide`).
- Use the design tokens in `docs/05-design/tokens.css`. Do not add raw hex values, font sizes or spacing outside the scales.
- Commit in small steps with clear messages once the user asks for commits.

## Useful commands

- `/phase0` runs or continues the Phase 0 test plan.
- `/decide` records a decision with context and consequences.
- `/design-check` checks a UI change against the design system.
- `/research <question>` runs sourced, parallel research into `docs/01-research/notes/`.
- `/new-spike` creates a Phase 0 spike folder and registers it.
- `/lint-skills [dir]` lints skills, commands and agents, and runs `claude plugin validate` on plugin roots.
- `/surface-check` re-checks `EXTENSION-SURFACE.md` after a Claude Code update.
- `.claude/hooks/guard-secrets.py` is a PreToolUse hook that blocks any tool from touching sign-in data (rule 2). It must be registered in `.claude/settings.json` to take effect.
