# Spike 4: transcript structure for second-level timeline and tool-call costs

**Question.** Do Claude Code transcripts (`<config>/projects/**/*.jsonl`) carry per-message timestamps, per-message token usage, and `tool_use` ↔ `tool_result` ids we can join? That decides whether the Timeline can show tool-call costs and exact seconds.

## Method
`python3 analyze.py ~/.claude --max-files 200`

The script is read-only and prints structure only (record types, keys, usage fields, timestamp precision, join rate). It never prints message text, prompts, tool inputs or outputs.

## Result (2026-10-25, Claude Code 2.1.299, personal account, 115 recent transcripts, 110,136 records)

**Answer: yes. Transcripts support a second-level timeline and tool-call-level costs.** Raw report: `result-raw.md` (structure only).

| Question | Finding |
|---|---|
| Timestamps | ISO 8601 with **milliseconds** on 75% of records (all conversational ones) |
| Token usage | On **100%** of assistant records: `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, plus `service_tier`, `speed`, `server_tool_use` |
| Tool joins | `tool_use.id` ↔ `tool_result.tool_use_id`: **100%** joined (17,276 of 17,278) |
| Tasks | `promptId` on every user record (20,301) → groups a prompt with what followed |
| Linkage | `sessionId` 97%, `uuid`/`parentUuid` chain 70%, `cwd` and `gitBranch` for project attribution, `requestId` on assistant records |
| Model | `message.model` on every assistant record |
| Cadence | median 1.1 s between records, p95 35 s |

**Critical finding: usage is repeated.** One API response is split into ~2.23 records (max 15) that share `message.id` and carry the **same usage** (20,717 identical repeats, 85 that differ while streaming). Summing every record over-counts by about 2.2×, which explains the over-counting other tools show. **Facet must de-duplicate by `message.id`, keeping the last record's usage.**

No cost field is stored in transcripts. Cost must come from OpenTelemetry (`cost_usd`) or from a price table applied to de-duplicated tokens.

Other record types worth knowing: `api-request`, `cost-state`, `file-history-snapshot`/`delta` (edit history), `permission-mode`, `ai-title`/`custom-title` (conversation titles for the Sessions view).
