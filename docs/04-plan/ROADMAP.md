# Roadmap

Durations assume one developer part-time and are estimates, not promises.

## Phase 0: Validate (now, ~1–2 weeks)
Goal: replace unknowns with facts before writing app code.
- Run the tests in `PHASE0-TESTS.md`; results go in each `spikes/*/README.md`.
- Interview 8–10 consultants or freelancers; open a landing page with a Pro waitlist.
- Trademark pre-check for "Facet".
**Exit:** tests 1, 3, 4, 5, 8, 11 have results; decisions logged; go/no-go written in `DECISIONS.md`.

## Phase 1: Linux core (~4 weeks)
- Tauri 2 app skeleton, design tokens, account rail, Accounts view.
- Profile store, process supervisor, Linux adapter (instance, X11 identity, launchers).
- Guided sign-in with email verification; environment scrubbing.
- Terminal per account; `claude` shim with folder rules.
- Tray, quick switcher (portal or CLI fallback), Health (first checks).
- Packages: AppImage and .deb.
**Exit:** Yahya runs all four accounts daily from Facet for a week without falling back to the manual setup.
**Status (2026-10-26):** built except guided sign-in (waits on test 5) and portal-bound global shortcut (CLI fallback works). Details in `app/README.md`. The exit test (a week of daily use) is the user's.

## Phase 2: Control and the other platforms (~4 weeks)
- Control: settings editor (schema, diff, backup, atomic write, effective settings, keychain secrets, presets).
- MCP config editing for Code and Desktop (Desktop only while stopped).
- Whole-harness control: memory and rules, output styles, status line, workflows, messaging settings, transcript retention guard, context budget view.
- Studio v1: library, editors and templates for skills, agents, commands, hooks; linter; hook workbench; placement per account and project with drift diff; `claude plugin validate`.
- Windows adapter (Store alias or portable path, decided by test 9) and Microsoft Store packaging.
- macOS adapter (launcher bundles), signing and notarization.
**Exit:** same daily-use bar on Windows and macOS test machines.
**Status (2026-10-26):** Control (settings editor with diff, private backups, restore, refusals of sign-in values; D-015) and Studio v1 (library, live lint, templates, placement with drift and backups, import) are built. Not yet: effective settings across layers, keychain secrets, presets and copy between accounts, MCP editing, hook workbench, Windows and macOS adapters.

## Phase 3: Timeline and costs (~5 weeks)
- OTLP receiver, telemetry wiring per account, transcript indexer, SQLite rollups.
- Timeline (canvas, semantic zoom, inspector, keyboard, leaderboards, replay).
- Costs drill-down, client budgets, CSV export, weekly report.
- Sessions deep search (metadata across accounts, opt-in full text via FTS5) and resume.
- Extension usage analytics (what ran, cost per skill, prune list).
**Exit:** totals match telemetry within 2% over a week; Timeline holds 60 fps on 200K events.

## Phase 4: Ecosystem, care and launch (~4 weeks)
- Package inspector and safety preview v2 (risk flags, install hash, diff before update, update gate, untrusted-repo check); install per account; update alerts.
- Studio v2: AI-assisted drafting via the user's own `claude -p`, trigger lab via `claude plugin eval`, local marketplace scaffolding.
- Backups and restore; full Health suite; auto-update; Arabic RTL.
- Pro licensing (merchant of record), landing page, docs.
- Open-source launch: Show HN, r/ClaudeAI, awesome-claude-code lists, Linux communities, one transparent post in #18435.

## Phase 5: After launch
- Bridge, if test 1 and user demand justify it.
- Anomaly alerts, bookmarks, range compare.
- Team edition: policy packs, approved catalogue, audit, sync.
- Phone companion.

## Kill or pivot criteria (60 days after open-source launch)
- Fewer than ~300–500 GitHub stars, or
- Fewer than 100 people on the Pro waitlist, or
- Anthropic ships Desktop multi-account and Pro conversion stays under 1%.
Then: stop investing in isolation; keep Timeline, costs, control and safety as a standalone tool (works with one account per OS user), or offer it to an existing ecosystem such as cc-switch.
