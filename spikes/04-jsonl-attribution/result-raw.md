# Transcript structure report

Config dir: `/home/you/.claude` · files scanned: 115 · records: 110136
Lines per file: median 277, max 10028

## Record types (`type`)
- `assistant`: 37739
- `user`: 20309
- `attachment`: 17671
- `last-prompt`: 5700
- `ai-title`: 3247
- `queue-operation`: 3152
- `atis-latch`: 3093
- `bridge-session`: 3070
- `mode`: 2750
- `custom-title`: 2634
- `agent-name`: 2424
- `file-history-snapshot`: 1988
- `permission-mode`: 1620
- `frame-link`: 1403
- `system`: 1320
- `file-history-delta`: 875
- `artifact-autoreact-ledger`: 300
- `relocated`: 282
- `api-request-blob`: 230
- `api-request`: 171
- `artifact-comment-monitor`: 102
- `cost-state`: 36
- `api-request-shape`: 15
- `history-suppression`: 5

## Top-level keys
- `type`: 110136
- `sessionId`: 107273
- `timestamp`: 82655
- `version`: 77225
- `parentUuid`: 77039
- `isSidechain`: 77039
- `uuid`: 77039
- `userType`: 77039
- `entrypoint`: 77039
- `cwd`: 77039
- `gitBranch`: 77039
- `message`: 58278
- `requestId`: 37708
- `slug`: 26674
- `effort`: 24159
- `promptId`: 20300
- `sourceToolAssistantUUID`: 18033
- `attachment`: 17671
- `toolUseResult`: 17580
- `apiBlockIndex`: 13672
- `perTurnEffort`: 12150
- `serverClassifierRequest`: 11932
- `advisorModel`: 11824
- `rendered`: 8815
- `renderedRole`: 6962

## `message` keys
- `role`: 58278
- `content`: 58278
- `model`: 37739
- `id`: 37739
- `type`: 37739
- `stop_reason`: 37739
- `stop_sequence`: 37739
- `stop_details`: 37739
- `usage`: 37739
- `diagnostics`: 37705
- `context_management`: 12217
- `container`: 12211
- `input_transformations`: 11825
- `output_config`: 7

## Content block types
- `tool_use`: 18088
- `tool_result`: 18087
- `thinking`: 11528
- `text`: 9064
- `image`: 79
- `tool_addition`: 2
- `tool_removal`: 1

## Timestamp format
- `ISO, 3 fractional digits`: 82655

## Token usage on assistant messages
- assistant records: 37758, with `usage`: 37739 (100%)

## `usage` keys
- `input_tokens`: 37739
- `cache_creation_input_tokens`: 37739
- `cache_read_input_tokens`: 37739
- `output_tokens`: 37739
- `service_tier`: 37739
- `cache_creation`: 37739
- `inference_geo`: 37739
- `speed`: 37435
- `server_tool_use`: 37180
- `iterations`: 37180
- `output_tokens_details`: 21208
- `fallback_credit`: 10015

## tool_use ↔ tool_result join
- tool_use ids: 17278, tool_result ids: 17276, joined: 17276 (100%)

## Linkage fields present (share of records)
- `sessionId`: 97%
- `cwd`: 70%
- `uuid`: 70%
- `parentUuid`: 70%

## Time between consecutive records
- median 1.1 s, p95 34.8 s
