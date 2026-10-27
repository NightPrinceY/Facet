# Claude Code extension surface (what Facet may read and write)

Reference for the Studio, Control and Ecosystem modules. Checked 2026-10-26 against the docs at https://code.claude.com/docs/en/ and the local CLI (`claude --version` → 2.1.310). **[unverified]** = not confirmed in docs or on this machine. Re-check on every Claude Code minor release; Facet's parsers must ignore unknown fields.

Everything below is a documented file or CLI command. Facet only reads and writes these files (with backup and diff) or runs the official `claude` binary on the user's request, with override variables scrubbed.

## Where things live (per account = per `CLAUDE_CONFIG_DIR`)

| Thing | Account level (`<config>/…`) | Project level | Plugin |
|---|---|---|---|
| Skills | `skills/<name>/SKILL.md`; `skills/synced/<bucket>/<name>/` = claude.ai account skills synced by Claude Code (read-only for Facet; spike 11) | `.claude/skills/<name>/SKILL.md` (also nested `<subdir>/.claude/skills`) | `skills/<name>/SKILL.md` → `/plugin:skill` |
| Commands (legacy, merged into skills) | `commands/*.md` | `.claude/commands/*.md` | `commands/` |
| Subagents | `agents/*.md` (recursive) | `.claude/agents/*.md` | `agents/` |
| Hooks | `settings.json` → `hooks` | `.claude/settings.json`, `.claude/settings.local.json` | `hooks/hooks.json`; also skill/agent frontmatter `hooks` |
| MCP servers | `~/.claude.json` for the default folder, `$CLAUDE_CONFIG_DIR/.claude.json` otherwise (verified 2026-10-26, D-016; user and local scope; also holds `oauthAccount`: change only via `claude mcp`, read only `mcpServers`) | `.mcp.json` | `.mcp.json` |
| Output styles | `output-styles/*.md` | `.claude/output-styles/*.md` | `output-styles/` |
| Workflows | `workflows/*.js` | `.claude/workflows/*.js` | `workflows/` |
| Memory | `CLAUDE.md`, auto memory `projects/<p>/memory/` | `CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md`, `.claude/rules/*.md` (`paths` frontmatter) | — |
| Keybindings | `keybindings.json` | — | — |
| Session registry | `sessions/<pid>.json`, parsed with an allow-list (`pid`, `name`, `cwd`, `status`, `kind`, `entrypoint`, `version`, `messagingSocketPath`, `peerProtocol`, `sessionId`); `<pid>.*.key`: **never read** | — | — |

Settings precedence: managed (never written by Facet) > `--settings` > `.claude/settings.local.json` > `.claude/settings.json` > `<config>/settings.json`. List keys such as `permissions.allow` merge. Project files cannot set `permissions.defaultMode` to `auto` or `bypassPermissions`.

## Skills (`SKILL.md`)

Frontmatter, all optional (unknown keys are ignored silently, so Facet's linter must warn about them): `name` (defaults to folder), `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context: fork`, `agent`, `background`, `hooks`, `paths`, `shell`, `metadata`, `license`, `compatibility` (≤500 chars).

- `description` + `when_to_use` are truncated at **1,536 characters** in the skill listing. Routing depends on the first words.
- Body loads only on use; keep under ~500 lines, link to `reference.md`, `scripts/` via `${CLAUDE_SKILL_DIR}`.
- Substitutions: `$ARGUMENTS`, `$N`, `$name`, `${CLAUDE_SKILL_DIR}`, `${CLAUDE_SESSION_ID}`, `${CLAUDE_PROJECT_DIR}`, `${CLAUDE_EFFORT}`, `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`.
- Shell injection: `` !`cmd` `` runs at load time. **The safety preview must list every one** (disable with `disableSkillShellExecution`).
- Portable subset (claude.ai, Skills API, other Agent Skills clients): `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools` only; other keys are a hard error there.

## Subagents (`agents/*.md`)

Required `name` (≤256 chars, no `:`), `description`. Optional `tools`, `disallowedTools`, `model` (`sonnet|opus|haiku|fable|inherit|<id>`), `permissionMode`, `maxTurns`, `skills`, `mcpServers`, `hooks`, `memory` (`user|project|local`), `background`, `isolation: worktree`, `effort`, `color`, `initialPrompt`. Plugin agents ignore `permissionMode`, `mcpServers`, `hooks`, `initialPrompt`.

## Hooks

33 events, among them `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Stop`, `SubagentStart/Stop`, `PreCompact`, `ConfigChange`, `FileChanged`, `InstructionsLoaded`, `Notification`. Handler types: `command`, `http`, `mcp_tool`, `prompt`, `agent` (experimental). Common fields: `matcher` (case-sensitive tool name), `if` (permission-rule filter such as `Bash(rm *)`), `timeout`, `statusMessage`.

Stdin JSON: `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`; tool events add `tool_name`, `tool_input`, `tool_use_id`. Exit `0` = parse stdout JSON; exit `2` = block, stderr is the reason; other codes do not block. `PreToolUse` JSON: `hookSpecificOutput.permissionDecision` (`allow|deny|ask|defer`), `updatedInput`, `additionalContext`.

**Risk flags for the safety preview:** hooks that return `allow` or `updatedInput`, `http` hooks (data leaves the machine), `SessionStart` hooks from a repo, `PermissionRequest` auto-approval.

## Plugins and marketplaces

- `.claude-plugin/plugin.json`: only `name` required (kebab-case). Others: `version`, `description`, `author`, `homepage`, `repository`, `license`, `keywords`, `defaultEnabled`, `dependencies`, `userConfig` (`sensitive` values go to secure storage, Facet never reads them), component paths, `experimental`.
- Layout: `skills/`, `commands/`, `agents/`, `hooks/hooks.json`, `.mcp.json`, `.lsp.json`, `output-styles/`, `workflows/`, `monitors/`, `bin/` (added to PATH: **flag in preview**), `settings.json`.
- `.claude-plugin/marketplace.json`: `name`, `owner`, `plugins[]` with `name` + `source` (`./path`, `github`, `url`, `git-subdir`, `npm`, `archive` with `sha256`, `command`). Unpinned sources (no `sha`/`sha256`/exact `version`) are an update risk.
- Settings: `enabledPlugins`, `extraKnownMarketplaces`, `blockedMarketplaces`, `pluginConfigs`. Official marketplaces auto-update by default.
- CLI: `claude plugin install|uninstall|enable|disable|list|update|details|configure|validate [--strict] [--json]|init|tag|prune|test|eval`, `claude plugin marketplace add|list|update|remove`, session flag `--plugin-dir <path>`.

## Testing: `claude plugin eval`

`evals/<case>/prompt.md` (frontmatter `runs`, `max_turns`, `timeout_seconds`, `allowed_tools`, `model`) + `evals/<case>/graders/*.md` (`regex`, `tool_used`, `tool_order`, `file_exists`, `llm`, `baseline`); MCP mocks in `evals/mocks/`. Flags: `--json`, `--threshold`, `--max-cost-usd`, `--ablation with-without`, `--runs`. Results: `results/<ts>/aggregate-result.json`, `report.html`. **The HTML report is published to claude.ai by default: always pass `--no-publish`.** `claude plugin eval init --bare <case>` writes a blank case. Verified graders (spike 11): `regex` (`pattern`), `tool_used` (`tool`, with-only for `Skill`), `llm` (judge haiku by default). Each run uses the parent account's credential internally; Facet passes no token. Exit 0 pass, 1 below threshold, 2 cost ceiling or auth. Runs in a fresh `CLAUDE_CONFIG_DIR` and `HOME` with only the plugin under test; **not an OS sandbox, network not blocked**. Uses the account's quota: Facet must show the cost ceiling before starting.

## MCP

Scopes local > project (`.mcp.json`) > user > plugin > claude.ai connectors; managed above all. CLI `claude mcp add|add-json|list|get|remove|login|logout`. `${VAR}` / `${VAR:-default}` expansion. Tool permission names `mcp__<server>__<tool>`. OAuth sign-in for remote servers happens in Claude (`/mcp`, `claude mcp login`), never in Facet.

## Headless (for user-started drafting and tests)

`claude -p` with `--output-format json|stream-json`, `--json-schema`, `--allowedTools`, `--append-system-prompt-file`, `--agents`, `--settings`, `--setting-sources`, `--plugin-dir`, `--permission-mode`, `--max-budget-usd`. `system/init` in stream-json lists `plugins`, `plugin_errors`, `mcp_servers`, `mcp_server_errors`: a way for Health to see what loaded, **but each run sends a prompt and spends quota**, so only when the user clicks "Check now", never on a timer or at startup. Keep only names from `system/init`; the stream also carries reply text. **Do not use `--bare`**: it authenticates with `ANTHROPIC_API_KEY`, which Facet scrubs.

## Cross-session messaging (test 1)

Documented at https://code.claude.com/docs/en/cross-session-messaging.md: `/peers`, `ListAgents`, `SendMessage`; same-machine messages stay local; inbound control `crossSessionInbound` = `accept|hold|refuse`; `isolatePeerMachines`. Cross-`CLAUDE_CONFIG_DIR` discovery is **not documented**; peer feature `reply_across_default_dirs` is undocumented.

## Settings keys Facet's Control module should expose

`permissions` (`allow`, `ask`, `deny`, `defaultMode`, `additionalDirectories`), `sandbox`, `env`, `model`, `effort`, `outputStyle`, `statusLine`, `hooks`, `disableAllHooks`, `enabledPlugins`, `extraKnownMarketplaces`, `blockedMarketplaces`, `enableAllProjectMcpServers`, `enabledMcpjsonServers`, `disabledMcpjsonServers`, `cleanupPeriodDays` (default 30), `autoMemoryEnabled`, `claudeMdExcludes`, `attribution`, `spinnerTipsEnabled`, `disableSkillShellExecution`, `disableWorkflows`, `crossSessionInbound`, `isolatePeerMachines`. Never write `apiKeyHelper`: it produces credentials.

Seen in a real account's settings.json (2026-10-26) but not yet in this reference: `skillOverrides` (per-skill entries keyed by skill name), `remoteControlAtStartup`, `theme`, `tui`. **[unverified meaning]**; Facet preserves them untouched and shows them read-only in "What applies".
