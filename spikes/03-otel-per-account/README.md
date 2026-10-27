# Spike 3: telemetry per account

**Question.** With telemetry turned on in each account's settings, do account, session, tokens, cost and tool events arrive separately per account at a local collector? Is there a `prompt.id` to group events into tasks?

**Also covers test 6 notes** (Desktop config rewrites) when convenient.

## Method
1. Start the collector: `python3 collector.py`
2. For each test account, add to `<CLAUDE_CONFIG_DIR>/settings.json` (back it up first):
   ```json
   "env": {
     "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
     "OTEL_LOGS_EXPORTER": "otlp",
     "OTEL_METRICS_EXPORTER": "otlp",
     "OTEL_EXPORTER_OTLP_PROTOCOL": "http/json",
     "OTEL_EXPORTER_OTLP_ENDPOINT": "http://127.0.0.1:4318",
     "OTEL_METRIC_EXPORT_INTERVAL": "10000",
     "OTEL_LOGS_EXPORT_INTERVAL": "5000",
     "OTEL_RESOURCE_ATTRIBUTES": "facet.account=personal"
   }
   ```
3. In each account, run one short prompt that uses a tool (for example "list the files here").
4. Read the collector output and `data/otel.jsonl` (git-ignored).
5. Remove the env block afterwards, or keep it if the user wants.

## What to record
- Event names seen, and attribute keys on each (`session.id`, `prompt.id`, `user.account_uuid`, `user.email`, tokens, cost, duration, tool name).
- Whether `facet.account` arrives on every record.
- Delay from action to arrival.
- Whether cost and tokens match `/usage` or `/cost` for the same session.

## Result (2026-10-26, Claude Code 2.1.310): done

**Answer: yes. Every event and metric arrives separately per account, with session id, prompt id, account uuid, tokens and exact cost, and prompt and response text are redacted by default.** Telemetry cost matches Claude Code's own reported cost exactly (to the last digit), so Costs can use OTel as the source of truth and transcripts as the fallback (D-008).

### Method actually used
Telemetry variables were passed **on the command line** (no edit to either account's `settings.json`, so no backup needed). One prompt per run: "List the files in the current directory using the Bash tool with ls, then say how many there are", in a scratch folder with two files, `--allowedTools "Bash(ls:*)"`, `--output-format json`, override variables scrubbed with `env -u`.

| Run | Account | How `facet.account` was set | Session |
|---|---|---|---|
| A | personal (`~/.claude`) | process env `OTEL_RESOURCE_ATTRIBUTES=facet.account=personal` | f16708ec |
| B | work (`~/.claude-work`) | process env `OTEL_RESOURCE_ATTRIBUTES=facet.account=work` | 61d650b3 |
| C | work | **Desktop-like:** process env `OTEL_RESOURCE_ATTRIBUTES=desktop.sim=1`, `OTEL_SERVICE_NAME=desktop-sim`; `--settings '{"env":{"OTEL_RESOURCE_ATTRIBUTES":"facet.account=work-settings"}}'` | 849f6301 |
| D | personal | as A, README intervals (logs 5 s, metrics 10 s), collector records arrival time | timing only |

Analysis scripts printed attribute **keys**, counts, sums and hashes only; payloads are in `data/` (git-ignored, local).

### Findings

| Question | Finding |
|---|---|
| Separate per account | Yes. Resource `facet.account` on every log and metric; `user.account_uuid` differs per account (hash `710cb97b` personal, `fb95b073` work; same for B and C) |
| Event names | `user_prompt`, `api_request`, `tool_decision`, `tool_result`, `assistant_response`, `mcp_server_connection`, `plugin_loaded`, `hook_registered`, `hook_execution_start`, `hook_execution_complete`, `managed_settings_resolved` (log body `claude_code.<name>`) |
| Common keys on every event | `session.id`, `user.account_uuid`, `user.account_id`, `user.id`, `user.email`, `organization.id`, `terminal.type`, `event.timestamp`, `event.sequence` |
| `prompt.id` | Present on `user_prompt`, `api_request`, `tool_decision`, `tool_result`, `assistant_response`, `mcp_server_connection`; one per prompt. Groups events into tasks |
| `api_request` keys | `model`, `cost_usd`, `cost_usd_micros`, `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`, `cache_creation_5m_tokens`, `cache_creation_1h_tokens`, `duration_ms`, `ttft_ms`, `effort`, `speed`, `query_source`, `request_id`, `client_request_id` |
| `tool_result` keys | `tool_name`, `tool_use_id`, `duration_ms`, `success`, `tool_input_size_bytes`, `tool_result_size_bytes`, `request_id` (joins to transcripts by `tool_use_id`) |
| Metrics | `claude_code.cost.usage`, `claude_code.token.usage`, `claude_code.session.count`, `claude_code.active_time.total` |
| Cost vs Claude Code | A: OTel 0.243776 = `-p` `total_cost_usd` 0.2437758; B: 0.175203 = 0.1752034; C: 0.148731 = 0.1487308. Tokens identical to `modelUsage` |
| Content | `user_prompt.prompt`, `user_prompt.prompt_text`, `assistant_response.response` are all `<REDACTED>` by default; `prompt_length` is kept |
| Delay | Log events arrive within the export interval: median 1.3–5.0 s, max 5.07 s with `OTEL_LOGS_EXPORT_INTERVAL=5000`; pending events flush when the session exits |
| **Settings env vs process env** | In C, the resource had `facet.account=work-settings` and **no** `desktop.sim`: the `env` value from settings **replaces** the process variable entirely. `OTEL_SERVICE_NAME=desktop-sim` (not overridden) was kept |

### Consequences
1. **Do not rely on overriding `OTEL_RESOURCE_ATTRIBUTES` for Desktop-launched sessions.** Claude Desktop sets its own `OTEL_RESOURCE_ATTRIBUTES` on its Code children (spike 1, test 2 notes). Setting it in `settings.json` would silently drop Desktop's attributes. Instead, map events to accounts by `user.account_uuid`, which is on every event. Facet learns the uuid from the first telemetry event per profile; that is not a credential. Optionally also give each account its own local endpoint port (`OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:43NN`) as a second key that touches no Desktop variable.
2. Telemetry carries `user.email`. The local event store is personal data: keep it local, exclude it from crash reports, and hash or drop email on ingest.
3. `plugin_loaded` (`skill_path_count`, `has_hooks`, `has_mcp`), `hook_execution_*` (`num_blocking`, `total_duration_ms`) and `mcp_server_connection` (`status`, `duration_ms`) feed Studio usage analytics, the hook workbench ("did my hook fire?") and Health, with no transcript reading.
4. Never set `OTEL_LOG_USER_PROMPTS` or similar content flags by default (rule 9).
5. Still open: confirm that a Desktop-launched Code session honours telemetry `env` from its config folder's `settings.json` (needs the work Desktop restarted with `CLAUDE_CONFIG_DIR`, see spike 1).
