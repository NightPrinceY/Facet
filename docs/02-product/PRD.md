# Product requirements

Status: draft v0.3 · 2026-10-26 (adds M11 Studio, deep search, harness control; see D-009) · Owner: Yahya

Each module lists the user problem, the requirements (MUST for v1, SHOULD for v1.x, LATER), acceptance criteria and open questions. Pro marks paid features. Phases refer to `docs/04-plan/ROADMAP.md`.

---

## M1. Accounts (Phase 1, Free)

**Problem.** Running several Claude accounts is fragile and easy to get wrong: sign-in links land in the wrong window, environment variables collapse accounts into one, windows look identical.

**Requirements**
- MUST create an account: name, colour, plan, surfaces (Desktop app, Claude Code, both).
- MUST give each account its own Claude Desktop data folder and Claude Code config folder, and never share sign-in data.
- MUST launch, focus and quit each account's Desktop instance; show running state.
- MUST start each Desktop instance with that account's `CLAUDE_CONFIG_DIR`, so its Code tab keeps transcripts, settings and skills in the right folder (Desktop supplies the sign-in itself; test 2, D-012). Health flags an instance running without it.
- MUST open a terminal as an account (`claude-<name>` shim or `CLAUDE_CONFIG_DIR`).
- MUST run a guided sign-in: close other instances, prefer email-code sign-in, reopen others afterwards, then verify the signed-in email matches what the user expects, using the official `claude auth status --json` for that profile (`loggedIn`, `email`, `orgId`, `configDirectory`), never `~/.claude.json` or credential files.
- MUST scrub `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` from every process Facet starts.
- MUST tint windows with the account colour where the platform allows; otherwise badge and title.
- SHOULD create menu or Start entries and launchers per account.
- SHOULD import an existing setup (detect `~/.config/Claude-*`, `~/.claude-*`).
- LATER: per-account wallpapers for the app window, account-specific notification sounds.

**Acceptance**
- Two accounts signed in at once on Linux, both survive a reboot and an app update.
- A sign-in started for account B never completes in account A's window.
- With `ANTHROPIC_API_KEY` exported in the shell, Facet-launched sessions still use their own sign-in.

**Open questions:** test 2 (Desktop-launched Code sessions honour `CLAUDE_CONFIG_DIR`?), test 5 (deep links).

---

## M2. Timeline (Phase 3, core Free for one account, cross-account Pro)

Full spec: `TIMELINE-SPEC.md`.

**Problem.** Users cannot see what happened, when, in which account, and what it cost, at any level of detail.

**Requirements (summary)**
- MUST show all accounts on one time axis with semantic zoom from 30 days to 1 second.
- MUST show events: user prompts, model calls (tokens, cost, duration, model), tool runs (tool, target, duration, output size, success), errors, permission decisions, sessions and tasks.
- MUST select any event and show it to the millisecond; step event by event with the keyboard.
- MUST rank the most expensive conversations, tasks and tool runs in the visible range.
- MUST replay a range at 10×, 60× or 600×.
- SHOULD overlay limit windows (5-hour, weekly), budgets and anomalies.
- SHOULD support bookmarks and notes on the timeline, and exporting a range.
- LATER: compare two ranges, share a redacted replay.

---

## M3. Costs (Phase 3, Pro for cross-account)

**Problem.** "What did this cost?" has no answer across accounts, clients or tasks.

**Requirements**
- MUST show tokens (input, output, cache read, cache write) and API-equivalent cost per account, project, conversation, task and model, for 24 h, 7 d, 30 d and custom ranges.
- MUST de-duplicate cached-token counts (some tools over-count 2–3×).
- MUST drill from account → project → conversation → task → tool call. Pro
- MUST support client budgets per folder or account, with alerts at a chosen threshold. Pro
- MUST export CSV grouped by day, client, project and conversation for invoicing. Pro
- SHOULD show plan value: usage value per dollar of each plan.
- SHOULD give saving tips grounded in data (big tool outputs, model choice, cache misses).
- LATER: plan advisor; monthly PDF statement per client.

**Acceptance:** totals match OpenTelemetry `claude_code.cost.usage` within 2% over a week (test 3).

---

## M4. Control (Phase 2, basic Free, templates and copy Pro)

**Problem.** Claude Code settings are JSON files per account. Keeping accounts consistent, safe and correct is manual and error-prone.

**Requirements**
- MUST edit per account: model, effort, fast mode, output style, permission mode, allow/ask/deny rules, hooks, environment, enabled plugins, MCP servers (Code and Desktop). User- and local-scope MCP servers are changed through `claude mcp add/remove` (scrubbed environment), never by writing `~/.claude.json`, which Claude Code rewrites while running and which holds account details; Facet reads only its `mcpServers` keys.
- MUST validate against Claude Code's JSON schema, show a diff, back up, write atomically.
- MUST keep secrets in the system keychain and reference them, not write them in plain text. The secret store and `env` editor refuse `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` and `ANTHROPIC_BASE_URL`.
- MUST never edit managed or enterprise policy files; show them read-only with their source.
- MUST edit Desktop's `claude_desktop_config.json` only while that instance is stopped.
- SHOULD offer presets (strict client work, power user, learning) and copy settings between accounts. Pro
- SHOULD show "effective settings": what actually applies after user, project, local and managed layers merge.
- MUST cover the whole harness per account, not just `settings.json`: memory (`CLAUDE.md`, `.claude/rules/`, auto memory on/off), output styles, status line, keybindings, workflows on/off, cross-session messaging (`crossSessionInbound`, `isolatePeerMachines`), skill shell execution (`disableSkillShellExecution`). Reference: `docs/03-architecture/EXTENSION-SURFACE.md`.
- MUST show a **context budget** per account and project: estimated startup tokens of skill descriptions, MCP tool definitions, CLAUDE.md and rules, with one-click disable or scope-to-project for each item.
- MUST warn when `cleanupPeriodDays` will delete transcripts (default 30 days) and offer to raise it, with a backup.
- MUST never write `apiKeyHelper` or any value that produces credentials.
- LATER: policy packs for teams, drift alerts when a file changes outside Facet.

---

## M5. Ecosystem (Phase 4, preview Free, risk score Pro)

**Problem.** Skill packs, plugins and MCP servers run code on the user's machine. Nobody shows what they do before install.

**Requirements**
- MUST parse a skill, plugin or MCP package without executing it and list: skills, commands, agents, hooks (event + command), permissions requested, environment variables read, network endpoints, files written, start command and whether it is version-pinned.
- MUST install into chosen accounts only, with a backup first.
- MUST show what is installed per account and allow removing per account.
- SHOULD score risk (low/medium/high) with plain reasons. Pro
- SHOULD alert when an installed item updates and its permissions or hooks change.
- MUST flag, with a plain reason: `` !`cmd` `` shell injection in skills, hooks that return `allow` or `updatedInput`, `http` hooks, `SessionStart` hooks, `bin/` on PATH, `env` overrides of base URLs, `enableAllProjectMcpServers`, unpinned marketplace sources (no `sha`, `sha256` or exact version), `curl | sh` patterns.
- MUST record a content hash at install and show a diff before any update is accepted (rug-pull defence). SHOULD hold auto-updates of marketplaces for review per account (update gate), using documented settings only.
- MUST treat a freshly cloned repo's `.claude/` and `.mcp.json` as untrusted and offer a preview before the user opens it in Claude Code.
- SHOULD run bundled static rules offline; optional external scanners (mcp-scan, Cisco skill-scanner) only if the user installs them, in static or offline modes only. LLM-judge modes that call Anthropic or need an API key are not supported from Facet.
- LATER: curated catalogue, team allow-lists.

---

## M6. Sessions and deep search (Phase 3, metadata search Free, full-text and semantic Pro)
- MUST search session id, folder, branch, date, model, tools used and files touched across accounts. Session titles and summaries are built from prompts, so they count as message content and are searchable only after opt-in (rule 9).
- MUST resume a session in the account it belongs to (`claude --resume <id>` launched in that account, by the user).
- MUST filter by facets: account, project, branch, model, tool, file, cost, date.
- SHOULD offer full-text search of message text after opt-in (rule 9): SQLite FTS5 with the trigram tokenizer (code, paths, Arabic), indexed incrementally by byte offset. Tool results (file and command output) are excluded by default. Withdrawing opt-in deletes the index. Indexed text never goes to logs or crash reports. Pro
- SHOULD keep a sealed local archive so search survives Claude Code's transcript clean-up. **Off by default**, behind the same opt-in as full text. Sealed means: copies only `projects/**/*.jsonl`, token patterns (`sk-ant-`, `ghp_`, `AKIA`, `Bearer …`) redacted before storing, encrypted at rest with a local key, never in backups, sync or crash reports, user-set retention, one-click wipe. Pro
- LATER: local semantic search with an in-process embedding model (no remote API), merged with lexical results.
- SHOULD show sessions waiting for input across accounts, with notifications.

## M7. Folder rules (Phase 2, Free)
- MUST map folder patterns to accounts; a `claude` shim picks the account from the current folder.
- MUST show which rule matched and allow a manual override.

## M8. Bridge (Lab; test 1 done, D-010)
- v1 MUST be the user-triggered handoff file (summary, diff, file list) that the user opens in the target account.
- MUST let the user set `crossSessionInbound` per account (`refuse` for walled accounts) and show which accounts accept inbound messages, because any same-user session can reach any other session's socket.
- LATER, only if Anthropic documents cross-account delivery (`docs/04-plan/ANTHROPIC-QUESTIONS.md`) and legal review passes, opt-in Lab only:
  - list live sessions of all accounts from `<config>/sessions/<pid>.json`, parsed with an allow-list (`pid`, `name`, `cwd`, `status`, `kind`, `entrypoint`, `version`, `messagingSocketPath`, `peerProtocol`, `sessionId`; everything else dropped, never the `.key` files);
  - help the user ask one session to message another (Facet never writes to sockets);
  - set every account to `hold` while it is on;
  - re-check delivery only when the user clicks "Check now", with the cost shown.
- MUST describe it as sharing context, never offloading work. MUST NOT offer it when a limit is hit, and MUST NOT route automatically (rule 4). MUST warn that a work account's data may leave the organisation.
- MUST respect walls: walled accounts never send or receive.
- MUST never inject prompts without a user action.

## M9. Health (Phase 4, Free)
- MUST NOT read the environment of Claude processes (Desktop's Code children and eval children hold sign-in tokens). Override-variable checks look only at Facet's own environment (presence, never values), the login environment and shell files (`grep -o` so only names are output). `ANTHROPIC_BASE_URL` set by Desktop on its own children is expected and is not flagged. Whether a Desktop instance has the right `CLAUDE_CONFIG_DIR` is known from how Facet launched it; for instances Facet did not start, look for the new `<config>/sessions/<pid>.json` with `entrypoint: claude-desktop` in the expected folder.
- MUST check: Claude version and update source, override variables, account identity on this OS, deep-link handler, stale profile locks, failing MCP servers, telemetry pipeline, cache size, backups.
- MUST offer one-click fixes where safe.

## M10. Settings and platform (Phases 1–4)
- Tray or menu bar, start at login, quick switcher (global shortcut where the OS allows, CLI fallback on GNOME Wayland), notifications, weekly report, language (English, Arabic RTL), theme, data retention, prompt-text capture (off by default).

---

## M11. Studio: create, test and place extensions (Phase 2 library and editors, Phase 4 tests; authoring Free, trigger lab and evals Pro)

**Problem.** People write skills, agents, hooks and plugins by hand. Skills fail to trigger, YAML mistakes fail silently, hooks are hard to debug, there is no way to test, and nothing puts the right extension into the right account. Research: `docs/01-research/notes/extensibility_and_studio.md`.

**Requirements**
- MUST keep one local **library** of skills, subagents, commands, hooks, output styles, workflows and plugins, versioned with git, with no secrets in it. Enforced: a secret scan on every write, commit and push blocks literal tokens (in `.mcp.json` `env`, hook headers, settings `env`) and offers a `${VAR}` reference instead; `pluginConfigs` values, `${CLAUDE_PLUGIN_DATA}`, `.credentials.json` and `*.key` are never imported, even inside a plugin folder; the library ships a `.gitignore`.
- MUST create each kind from a form or a template and write the documented file format (`docs/03-architecture/EXTENSION-SURFACE.md`), showing the file before saving.
- MUST show `<config>/skills/synced/` (claude.ai account skills synced by Claude Code) read-only, labelled as synced, and never write into it.
- MUST **place** a library item into chosen accounts (`<config>/skills/…`) or projects (`.claude/skills/…`) by copy, with backup; show where each item is placed and when a placed copy has drifted from the library. Placing hooks or marketplaces writes `settings.json` with backup, diff and atomic write, never managed-settings paths.
- MUST **lint** skills and agents: valid YAML, single-line description, unknown keys (Claude Code ignores them silently), description + `when_to_use` budget (1,536 characters) with a live meter, trigger words first, body length, broken relative links, portability to the open Agent Skills subset.
- MUST provide a **hook workbench**: pick event and matcher from lists (case-correct tool names), and dry-run the hook locally against an edited sample input JSON, showing exit code, stdout, stderr and whether it would block. Recording real inputs is off by default and stays local (they carry prompts). Dry-runs scrub the three override variables, and third-party hooks run only after the M5 safety preview.
- MUST validate plugins and marketplaces with the official `claude plugin validate --json`.
- SHOULD offer **AI-assisted drafting**: "describe what the skill should do" launches the user's own official `claude -p` in the chosen account, then shows the result as a diff. Facet itself makes no network call. Guard rails: one visible user click per run, the exact command and prompt shown first (with "copy command" to run it themselves), never on a timer or in the background; override variables scrubbed; `--tools` limited to Write and Edit, working folder = a scratch folder, deny rules for credential paths passed via `--settings`, never `bypassPermissions`; runs stay in the chosen account and never fall back to another account when a limit is hit.
- SHOULD run a **trigger lab**: should-trigger and should-not-trigger prompts per skill, run through `claude plugin eval` (with-without ablation, override variables scrubbed, same guard rails as drafting) after showing a cost ceiling (`--max-cost-usd`), and chart pass rate per description version. Test 11 part C showed that eval signs in through Claude Code itself (no token variable, no copied files), so the stop condition is not triggered; it stays in force if that changes. Never script `claude setup-token` and never copy sign-in files. MUST always pass `--no-publish` (eval otherwise publishes its report, with prompts, to claude.ai). `--trust-plugin` only for a single run the user clicked, after the M5 preview, with a warning that the plugin's code runs on this machine; never a stored default. Eval results and traces contain prompts and replies, so they stay in Facet's scratch area, local only, out of crash reports and search, and deletable.
- SHOULD scaffold a **local marketplace** (`marketplace.json`, `plugin.json`) in a folder or git remote the user owns, and add it to chosen accounts via `extraKnownMarketplaces`.
- SHOULD show **usage analytics** per extension from transcripts and local telemetry: how often each skill, agent and MCP tool ran, cost per skill, and items never used (prune list).
- MUST run every library item through the M5 safety preview before placing it, including the user's own.
- LATER: import a skill from skills.sh or GitHub into the library (after preview), team-shared libraries.

**Acceptance**
- A skill created in Studio and placed in account B appears in a Claude Code session of account B and not in account A.
- The linter catches each of: multi-line description, unknown key, description over budget, missing `name`/folder mismatch.
- A hook dry-run reproduces the exit code and block decision of the same hook in a real session.

**Test 11 (done):** account-level skills load only in their own `CLAUDE_CONFIG_DIR`; `claude plugin validate` checks plugin roots only; `claude plugin eval` gives with/without trigger deltas per account.

**Open questions:** legal opinion on charging for features that run on the user's subscription through headless Claude Code (see RISKS) before Studio v2; whether AI-run features should be free.

---

## Success metrics
- Activation: a second account signed in within 5 minutes of install.
- Retention: weekly active use of Timeline or Costs.
- Conversion: 1–3% of active free users to Pro within 60 days of launch.
- Trust: zero incidents involving credentials; compliance review passes on every release.

## Non-goals
- More quota, account rotation, pooling, proxies.
- A replacement for Claude Desktop or Claude Code.
- Provider switching to other model vendors (cc-switch already does this).
- Cloud sync in v1.
