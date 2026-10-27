# Decision log

Newest first. Add entries with `/decide`.

## D-022: Open each account's links in its own browser profile
Date: 2026-10-26 · Status: accepted
Context: The user's default browser is Chrome with separate personal and work profiles. Links opened by a Claude window (sign-in, links in chat) went to whichever Chrome window was last used, so the work account's links landed in the personal profile.
Decision: Opt-in. When turned on, Facet registers `facet-links.desktop` (`facet --open-url %u`, http and https only) as the default web browser, after backing up `mimeapps.list` and remembering the previous default; local HTML files and about: links stay with the previous browser. The opener finds the account from its own inherited `FACET_ACCOUNT` (set on every Desktop window, terminal and `claude-<id>` command Facet starts) or `CLAUDE_CONFIG_DIR`; it reads no other variable values. Tested on this machine: GNOME re-parents the handler, so the parent process chain is not used. It starts the chosen browser with `--profile-directory` (Chromium family) or `-P` (Firefox) and the URL as the last argument, with the sign-in variables removed. Links from windows Facet cannot tie to an account go to a fallback profile. Facet reads only each Chromium profile's display name and signed-in emails from `Preferences` (an allow-listed parse) to label profiles and suggest the one matching an account's email; never cookies, passwords, keys or `Local State`. A recent-links list keeps only the site name of the last 30 links, locally. Turning it off restores the previous default.
Consequences: Facet becomes part of the user's link handling while on; if another app takes the default, Settings says so. The Personal Desktop window started from the system menu has no account tag, so its links use the fallback (set to the personal profile).

## D-021: Bridge v1 handoff contents and opening
Date: 2026-10-26 · Status: accepted
Context: D-010 chose a user-triggered handoff file. It needed concrete contents that respect rule 9 (no message text by default) and a documented way to open it in the other account.
Decision: A handoff holds session metadata (folder, branch, session id, counts, models), the user's own note, and optionally the project's uncommitted git changes. Git runs through `env::command` with a 10 s limit; diff lines that look like secrets are hidden and files such as `.env` or keys are shown by name only; the diff is cut at 120 KB. The user reads and may edit the text; saving refuses anything that still looks like a secret and writes it privately (0600) under `~/.local/share/facet/handoffs/`. "Open in <account>" is a button: it opens a terminal as that account in the project folder running the official `claude -- "Read the handoff file at <path> …"`. A work-account warning appears for Team or Enterprise plans. The same view sets `crossSessionInbound` per account through Control's reviewed diff and backup.
Consequences: No conversation text leaves its account unless the user types it into the note. Native cross-account messaging stays off (D-010).
Review follow-up (same day): git runs with `core.fsmonitor=false`, `core.untrackedCache=false`, an empty `diff.external`, `--no-ext-diff --no-textconv`, without `GIT_DIR`/`GIT_WORK_TREE`/`GIT_CONFIG_*`, in its own process group (a timeout kills everything it started). If the repository's own config defines filters, textconv, diff drivers or fsmonitor, the diff is left out and the handoff says so; the user's global and system config (for example Git LFS) are trusted. Files whose names suggest secrets (`.env*`, `*credential*`, `*secret*`, keys, `.tfvars`, `.pgpass`, Claude Code's own sign-in file) are shown by name only. The work-account warning checks both accounts and also appears when the plan is unknown; a work account's code changes need an explicit confirmation. The first message of the receiving session is shown word for word before opening.

## D-020: Ecosystem preview reads files only
Date: 2026-10-26 · Status: accepted
Context: PRD M5 needs a safety preview of plugins, skill packs and repositories before they run, and an inventory of what each account has installed. Running `claude mcp list` starts servers, and reading `.claude.json` touches a file that also holds account details.
Decision: `ecosystem.rs` walks the folder without following links (4,000 files, 512 KB per text file), parses plugin.json, hooks.json, .mcp.json and settings files, lints skills, commands and agents with Studio's rules, and lists hosts and environment variables it mentions. Commands and URLs are redacted before they leave the core. From Claude Code's state file only `mcpServers` (user and per-project) is deserialised; every other field is skipped by the parser. Risk is low, medium or high with plain reasons. A content hash per plugin supports a "reviewed" mark that resets when files change.
Consequences: No network, no execution, no MCP start. Findings are heuristics (marked as such in the UI); a clean scan is not a guarantee.
Review follow-up (same day): the state file is read only if it is a regular file under 32 MB, and the raw text is dropped as soon as the MCP parts are parsed. Redaction hides the value after secret flags (`--token`, `--api-key`, `-H`, `Authorization:`, `Bearer`), URL queries and key-like URL path parts. Plugin install paths with `..` are refused; the walk caps folders as well as files; the reviewed-version store is private. MCP servers are added and removed only through the official `claude mcp add|remove`, run as the account with the override variables removed; Facet refuses sign-in and routing variables for servers and redacts the command's output.

## D-019: Usage and cost from transcript metadata
Date: 2026-10-26 · Status: accepted
Context: Today, Costs and Sessions need per-account usage without OTel being set up, and rule 9 keeps message text out.
Decision: `usage.rs` reads `<config>/projects/**/*.jsonl` (and subagent transcripts) with a serde model that keeps only timestamps, ids, model, usage counts, tool names, cwd, branch, entrypoint and version; text, thinking, tool inputs and results are skipped while parsing. Replies are de-duplicated by message and request id (Claude Code writes one line per content block). Cost uses the official API price table checked 2026-10-26 (`docs/01-research/notes/model_pricing.md`), including cache writes by TTL, Haiku 5.5's over-100K rate, fast mode and web search. On this machine the estimate is within 0.5% of Claude Code's own `cost-state` figure over 42 sessions. Money shows two decimals (four under a cent), always labelled "API-equivalent", never compared with plan prices (compliance review, rule 4). Resume opens a terminal running the official `claude --resume <uuid>`; only UUID-shaped ids are accepted, and the shell scrubs the override variables again right before the command.
Consequences: Prices must be re-checked when models change (unknown models are counted but shown as unpriced). OTel remains the source for live, per-event data later (Timeline).

## D-018: Full account control
Date: 2026-10-26 · Status: accepted
Context: The user asked to edit account names and every option. Names, short names (`claude-<id>`), folders and looks were partly fixed after adding.
Decision: Every field is editable. A short-name change installs the new `claude-<id>` command and launcher first, then switches the accounts file, then removes the old ones, and moves folder rules and backups. Folder changes are refused while that account's Desktop app or Code sessions run; Facet never moves Claude's files. Pictures are stored in Facet's data folder after a content check (PNG, JPEG, WebP, GIF, or SVG without scripts or outside links). The launcher icon defaults to Facet's square; "Claude app icon with badge" is opt-in and composed on the user's computer from the icon the official package installed, never shipped, shown in screenshots or marketing (compliance note on rule 8; ask counsel at the trademark search).
Consequences: Launchers and shims are rewritten on save when installed. Pictures and notes stay local.

## D-017: Studio placement is gated in the core; account files are never followed through links
Date: 2026-10-26 · Status: accepted
Context: The compliance review of Studio v1 found that risky items (shell injection, hooks, MCP servers, broad shell access) could be placed with no warning, that commands without frontmatter skipped all checks, that the secret scan missed later matches and common token formats, that account files were read and written through symlinks (so a linked file could leak into a diff or backup, or a write could land in `skills/synced`), and that Studio backups were not private. The design review found the diff caption named the wrong file, Close dropped unsaved edits, severity showed by colour only, and the placement badges were unreadable.
Decision:
1. The core lints the library text at placement: errors (including secrets) block; risks need an explicit acknowledgement (`risk_ack`) that the UI collects with a required checkbox listing the risks.
2. Whole-file checks run on every item (shell injection, curl or wget piped into a shell, secrets), with or without frontmatter. The secret scan checks every occurrence of many token formats (Anthropic, GitHub, GitLab, Slack, OpenAI, Stripe, npm, Hugging Face, Google, AWS AKIA/ASIA, private keys, JWTs, literal Authorization and x-api-key headers, passwords in URLs) and ignores `${VAR}` references.
3. Account files are read and written only as regular files reached without links (each path component checked, O_NOFOLLOW), never in `skills/synced/` or managed folders. A copy that is a link or holds a secret is shown as "left alone" and is never diffed, backed up or replaced.
4. Studio backups use Control's private helpers (0700/0600, fsynced, create-new, last 30 kept). Library deletes remove only the item's file.
5. UI: shared `DiffView` names the file; unsaved editor text is kept per item and Close or switching asks first; findings show their severity in words; placement state is written out ("In sync: Work · Different: Personal"); a live status line replaces the noisy live region; the Meter follows the spec (account colour, amber at 80%, overage in words, valid ARIA).
Consequences: Placing anything that runs commands takes one deliberate extra click, enforced even if the UI is bypassed. A real personal skill (`hf-cli`) is now flagged for piping a download into a shell.

## D-016: Leave CLAUDE_CONFIG_DIR unset for the default folder
Date: 2026-10-26 · Status: accepted
Context: On this machine the work account keeps Claude Code's state file in `~/.claude-work/.claude.json`, and a small `~/.claude/.claude.json` (created 2026-10-13, not by Facet) sits next to the real `~/.claude.json`. So when `CLAUDE_CONFIG_DIR` is set, Claude Code reads `$CLAUDE_CONFIG_DIR/.claude.json`; when unset, `~/.claude.json`. Facet was setting `CLAUDE_CONFIG_DIR=~/.claude` for the personal account, which would make its sessions read the near-empty file and lose the account's MCP servers and project trust.
Decision: For an account whose folder is `~/.claude`, every process, terminal, bash start file, `claude-<id>` shim and `claude-auto` rule removes `CLAUDE_CONFIG_DIR` instead of setting it (`env::is_default_config_dir`). Other accounts keep setting it. Health reports a stray `~/.claude/.claude.json` as information and never deletes it.
Consequences: Personal sessions started through Facet see the same state as sessions started by typing `claude`. Spike 1's note that Desktop must get `CLAUDE_CONFIG_DIR` applies to non-default accounts only (the personal Desktop instance correctly runs without it).

## D-015: Control never copies sign-in values; backups are private and previewed
Date: 2026-10-26 · Status: accepted
Context: The compliance review of Control (Phase 2) found that backing up a settings.json whose `env` holds an API key or token would put a second copy of a sign-in value in Facet's folder (rule 2), and that env values reached the webview and the diff. The design review found that a dialog rendered inside the inert app shell could not be used, that Save could write edits that differed from the diff shown, and that restore wrote without a preview.
Decision:
1. A settings file holding any sign-in, provider, credential or TLS-bypass variable (`control::REFUSED_ENV`, case-insensitive: Anthropic keys and tokens, base URLs and custom headers, Bedrock/Vertex/Foundry switches and keys, AWS and Google credentials, client-key passphrase, NODE_TLS_REJECT_UNAUTHORIZED, CLAUDE_CONFIG_DIR) is not read out, backed up or edited by Facet; the user removes the value by hand. Facet never sets these names.
2. Secret-looking env values (names containing KEY, TOKEN, SECRET, PASSWORD, PASSPHRASE, HEADERS, CREDENTIAL, COOKIE, AUTH) are replaced by a marker before reaching the UI and in every diff; environment edits are per key, so untouched values never leave the file.
3. Backups live in `~/.local/share/facet/backups/<account>/` (folder 0700, files 0600, fsynced, created with O_NOFOLLOW), the last 30 are kept, and restore takes a backup name (pattern-checked, regular files only), shows a diff, checks the file hash and backs up the current file first.
4. Facet does not write through a symlinked settings.json or into managed-policy folders; preview warns about proxies, extra CA certificates, client certificates, shell prefixes, prompt logging, non-local telemetry endpoints, rules that reach another account's folder, and running sessions of the account.
5. Dialogs render in a portal outside the inert shell; Save applies the snapshot of edits that the diff was computed from.
Consequences: Users with an API key in settings.json must remove it by hand before Facet edits that account, which matches rule 2 and the scrubbing rule. A lawyer may still be asked whether a user-typed key counts as sign-in data; the engineering rule makes the answer moot.

## D-014: Token fixes from the first app design review
Date: 2026-10-26 · Status: accepted
Context: The design review of the Phase 1 UI measured light `--ok` #08875C on `--ok-soft` at 3.96:1 (fails AA for 12px pill text; the 4.54 in the system was measured on white only). The modal backdrop mixed `--ink`, which is near-white in dark mode, so dialogs washed the window white. Component sizes (avatar, toggle, modal, toast) lived in app.css instead of tokens.
Decision: Light `--ok` becomes #067A53 (5.36:1 on surface, 4.68:1 on ok-soft). New `--scrim` token (light rgba(13,16,21,.4), dark rgba(0,0,0,.6)). Component sizes move into tokens.css (`--avatar-sm/md/lg`, `--toggle-*`, `--step-dot`, `--card-min`, `--modal-width`, `--drawer-width`, `--toast-width`, `--spin`). Small text on `--sunk` uses `--ink-2` (`--ink-3` on sunk is 4.19:1). The Add-account stepper is Name → Folders → Review until guided sign-in exists (test 5); the spec's Create → Sign in → Check steps come with it.
Consequences: Pills and status text pass AA in both themes; the prototype should pick up the new token on its next update.

## D-013: Phase 1 implementation rules from the app compliance review
Date: 2026-10-26 · Status: accepted (items 2 and 3 pending a lawyer's view before public launch)
Context: The compliance review of the Phase 1 skeleton (`app/`) found no high-risk issues and asked for explicit records on judgement calls.
Decision:
1. Every process Facet starts goes through `env::command`, which removes the three override variables; a unit test fails if `Command::new` appears anywhere else. Claude processes also get the account's `CLAUDE_CONFIG_DIR`.
2. Window identity on X11 uses `xdotool` with only the `search` and `set_window` subcommands (class property), enforced at run time and by a test. This is window management, not UI automation: nothing is clicked, typed, focused or activated (rule 5). Bringing a window forward uses Electron's own second-start forwarding.
3. Helper commands `claude-<id>` and `claude-auto` are user-side conveniences, not product names (rule 8 covers product, logo, domain, bundle id and package). Ids that could hide official commands (`desktop`, `code`, `auto`, `cli`, `app`, `facet`) are reserved, and Facet refuses a name that already exists anywhere on PATH. The release checklist line on binary names applies to Facet's own binary.
4. Health never reads the environment of Claude processes. Whether a Desktop instance has the right folder is inferred: instances Facet started are recorded with their kernel start time (`~/.local/state/facet/launched.json`), and sessions registered in the wrong account's folder are detected through `/proc/<pid>/exe` and the session registry (D-012).
5. Facet checks that it launches official builds: the Desktop package maintainer must be Anthropic, and `claude` must resolve to the official install folders; otherwise Health warns. The `FACET_CLAUDE_DESKTOP` override works in debug builds only.
6. Bash terminals opened by Facet use an rcfile that sources `~/.bashrc` first and re-applies the account afterwards; for other shells Health explains that shell files can override the account.
7. `profiles.toml` is owner-only (0600) because it holds expected emails; sign-in checks carry a `personal` flag so future logs or exports redact them.
Consequences: Rule 6 is enforced mechanically. Two naming and window-management interpretations are written down for legal review. `claude auth status` may itself contact Anthropic (from the official binary, on the user's request); the release network capture must confirm Facet makes no such call of its own.

## D-012: Phase 0 go for the Phase 1 skeleton; Facet must set CLAUDE_CONFIG_DIR on every Desktop it starts
Date: 2026-10-26 · Status: accepted
Context: Tests 1, 3, 4, 8 and 11 have results; test 2 is answered by inspection; test 7 facts are in. Nothing found blocks the product. Every result went in Facet's favour or simplified the design (D-008, D-010, D-011). Test 2 found that a Desktop instance's Code tab signs in with the Desktop's own account (Desktop injects it) but writes transcripts, settings, skills and the session registry into `CLAUDE_CONFIG_DIR`, which defaults to the personal `~/.claude`. The hand-made work launcher lacked it, so it was fixed (backup kept). Test 7 shows the GlobalShortcuts portal on GNOME 50.
Decision: Go for the Phase 1 skeleton in `app/` (the CLAUDE.md gate, tests 1, 3 and 4, is met). The Linux adapter MUST start each Desktop instance with that account's `CLAUDE_CONFIG_DIR`, and Health MUST flag a Desktop instance running without it. The quick switcher uses the GlobalShortcuts portal where present, with the CLI as fallback. Tests 5, 6 and the Arabic typing check in test 7 need the user and must finish before guided sign-in (test 5) and Desktop config editing (test 6) are built. They do not block the skeleton.
Consequences: Phase 1 can start once the Rust toolchain and WebKitGTK dev packages are installed (needs sudo; ask first, D-002). Guided sign-in and Desktop config editing stay behind tests 5 and 6.

## D-011: Telemetry is the cost source of truth; attribute accounts by account uuid, not by overriding resource attributes
Date: 2026-10-26 · Status: accepted
Context: Test 3 (`spikes/03-otel-per-account/README.md`): per-account events carry `session.id`, `prompt.id`, `user.account_uuid`, tokens and `cost_usd`; OTel cost equals Claude Code's own `total_cost_usd` exactly; prompt and response text are redacted by default; delivery lag is at most one export interval. A `settings.json` `env` value replaces the process's `OTEL_RESOURCE_ATTRIBUTES` entirely, and Claude Desktop sets its own value on its Code children (spike 1).
Decision: Costs use OTel `api_request.cost_usd` when telemetry is on, transcripts plus the price table otherwise (refines D-008). The receiver maps events to profiles by `user.account_uuid` (checked against the profile's `claude auth status --json`), with an optional per-account localhost port as a second key. Facet does not set `OTEL_RESOURCE_ATTRIBUTES` for profiles that run Desktop. `user.email` is dropped on ingest; `user.id`, `user.account_id` and `organization.id` are kept local only and are HMAC-keyed with a random per-install key if stored. Facet never turns on prompt-content export by default.
Consequences: Exact costs per account, prompt and tool call without fragile tagging. The event store must handle personal data (email) carefully. Plugin, hook and MCP events give Studio and Health their data for free.

## D-010: Bridge: handoff file by default; native cross-account delivery stays Lab-only until Anthropic clarifies it
Date: 2026-10-26 · Status: accepted (revised the same day after compliance review)
Context: Test 1 (`spikes/01-cross-account-messaging/README.md`): `ListAgents` and send-by-name only see sessions under the same `CLAUDE_CONFIG_DIR`, but SendMessage to a session's `uds:` socket address is delivered across accounts both ways, and receivers can reply. Session registry files (`<config>/sessions/<pid>.json`) hold name, cwd, status and `messagingSocketPath` without secrets. `crossSessionInbound` (`accept|hold|refuse`) is a documented per-account setting.
Decision: The default Bridge is the user-triggered handoff file. Delivery across config folders is undocumented and may be a gap rather than a feature, so building on it could be read as getting around account separation. The native path stays Lab-only and opt-in, and is not shipped until Anthropic documents it or confirms it is intended (question drafted in `docs/04-plan/ANTHROPIC-QUESTIONS.md`, to be sent by the user). If it ships: Facet lists live sessions from `<config>/sessions/<pid>.json` with an allow-list parser (never the `.key` files), the user asks a session to message another, Facet never writes to sockets, every account gets `crossSessionInbound: "hold"` while the Bridge is on (`refuse` for walled accounts), and checks run only when the user clicks "Check now". Copy talks about sharing context, never offloading work. It is never offered when a limit is hit and never routes automatically (rule 4), and it warns that work-account data may leave the organisation.
Consequences: Bridge v1 is the handoff file, which is cheap and fully documented. The native path waits on Anthropic and a legal review. Control (M4) still gains the inbound-messaging setting per account, because any same-user session can reach any other session's socket and users should be able to hold or refuse that.

## D-009: Add Studio (create, test, place extensions), whole-harness control and deep search
Date: 2026-10-26 · Status: accepted (Studio placement and evals pending test 11)
Context: The user asked for more abilities: creating skills and local skills, more control of the Claude Code harness, deep search. Research (`docs/01-research/notes/extensibility_and_studio.md`, `docs/03-architecture/EXTENSION-SURFACE.md`) found a crowded but account-blind ecosystem (skills.sh 600k+ skills, aitmpl, opcode, skills managers), top complaints about skills not triggering, silent YAML and truncation failures, hook debugging and no testing, and a sharp rise in malicious skills, plugins and MCP servers (Snyk ToxicSkills 13.4% critical, Check Point CVEs, postmark-mcp). Claude Code documents everything needed: SKILL.md and agent formats, 33 hook events, plugin and marketplace manifests, `claude plugin validate` and `claude plugin eval`.
Decision: Add PRD module M11 Studio (library, editors, linter, hook workbench, per-account placement with drift diff; later AI-assisted drafting via the user's own `claude -p`, trigger lab via `claude plugin eval`, local marketplaces, usage analytics). Extend M4 Control to the whole harness (memory, rules, output styles, status line, workflows, messaging settings, context budget, transcript retention). Extend M5 with risk flags, install hashes, update diff and an update gate. Turn M6 into deep search (metadata free, opt-in FTS5 full text Pro). Add test 11.
Consequences: Facet becomes the place where extensions are made and governed per account, which survives native multi-account. More surface to keep in step with Claude Code releases: parsers must ignore unknown fields, and `EXTENSION-SURFACE.md` is re-checked each minor release. Drafting and evals spend the user's quota, so Facet shows a cost ceiling first and only runs them on request. Facet still makes no network calls and never uses `--bare` (it would need an API key). Compliance review (2026-10-26) added guard rails now in the PRD: eval stop condition (no token variables, no copied sign-in files), archive and session titles behind opt-in, enforced secret scan for the library, MCP edits only via `claude mcp`, and a legal check before charging for AI-run features (RISKS).

## D-008: Transcripts are a first-class data source; de-duplicate by message id
Date: 2026-10-25 · Status: accepted
Context: Spike 4 showed millisecond timestamps, usage on every assistant record, 100% tool_use↔tool_result joins and promptId on user records. It also showed each API response repeated across ~2.23 records with identical usage.
Decision: The transcript indexer is a primary source for the Timeline (timestamps, tasks via promptId, tool joins, tokens). Tokens are counted once per `message.id` (last record wins). Cost comes from OpenTelemetry when available, otherwise from a versioned price table, always labelled as an estimate. (Refined by D-011.)
Consequences: Tool-call-level costs and second-level timelines are feasible from day one, even without telemetry. A price table must be maintained. Test 3 still decides whether OTel cost is used for accuracy.

## D-007: Timeline is a core module
Date: 2026-10-25 · Status: accepted
Context: The user wants full visibility of every second: which conversation, task and tool run cost the most. Claude Code's OpenTelemetry events (user_prompt, api_request, tool_result, tool_decision, api_error) and transcripts provide the data.
Decision: Build the Timeline (spec in `docs/02-product/TIMELINE-SPEC.md`) as the signature view in Phase 3. One-account Timeline is free; cross-account is Pro.
Consequences: OTLP receiver and rollups become Phase 3 foundations; test 3 and 4 are blocking.

## D-006: Working name Facet
Date: 2026-10-25 · Status: accepted (pending trademark search)
Context: "Switchyard" described switching; "Lanes" and "Tessera" are taken by AI coding tools; no "Claude" allowed.
Decision: Use Facet. See `docs/02-product/NAMING.md`.
Consequences: Trademark search required before launch.

## D-005: Open core with a Pro subscription
Date: 2026-10-25 · Status: proposed (validate in test 10)
Context: Isolation sells for $10–30 one-time with little traction; upkeep across three OSes is ongoing.
Decision: Free open-source core; Pro at about $9/month or $89/year; Team later.
Consequences: Need licensing and a merchant of record in Phase 4.

## D-004: Bridge stays in the lab
Date: 2026-10-25 · Status: accepted, refined by D-010
Context: No demand evidence found; highest policy exposure; native same-account messaging exists.
Decision: Do not build a custom transport. After test 1, either a UI over native messaging or a user-triggered handoff file.
Consequences: Bridge is out of v1 scope.

## D-003: Do not take over claude:// links
Date: 2026-10-25 · Status: accepted
Context: Runtime scheme registration is impossible on macOS and hostile to Anthropic's app elsewhere.
Decision: Guided, serialised sign-in with email verification instead.
Consequences: Sign-in briefly closes other instances.

## D-002: Linux first, Tauri 2
Date: 2026-10-25 · Status: accepted
Context: Linux is proven on the founder's machine and least served; Tauri 2 is proven by cc-switch and opcode.
Decision: Phase 1 on Linux with Tauri 2 + Rust + React + TypeScript.
Consequences: Rust toolchain and WebKitGTK dev packages needed (sudo, ask first).

## D-001: Compliance by construction
Date: 2026-10-25 · Status: accepted
Context: Anthropic enforces against token handling, spoofing and reselling.
Decision: The nine rules in `CLAUDE.md` are non-negotiable and checked by the compliance-reviewer agent.
Consequences: Some features (rotation, Desktop automation, scheme takeover) are permanently out.
