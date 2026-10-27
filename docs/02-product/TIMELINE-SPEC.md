# Timeline spec

The Timeline is Facet's signature view: everything every account did, on one time axis, from a month down to a millisecond, with costs attached to each piece.

## Questions it must answer
- What was each account doing at 14:21:57 today?
- Which conversation cost the most this week? Which task inside it? Which single tool run caused it?
- How long did that `npm test` take, how much output did it produce, and how much did feeding that output to the model cost?
- When do I hit limits, and what was running when I did?
- Replay the last hour of this session, second by second.

## Vocabulary
| Term | Meaning | Source |
|---|---|---|
| Account | One Claude identity (Desktop data folder + Code config folder) | Facet |
| Conversation | One Claude Code session | `session.id` |
| Task | Everything from one user prompt until the next prompt in the same conversation | `claude_code.user_prompt` event, then `prompt.id` if present (verify, test 3) |
| Model call | One API request | `claude_code.api_request` event |
| Tool run | One tool execution | `claude_code.tool_result` event |
| Decision | Allow or deny of a tool permission | `claude_code.tool_decision` event |
| Error | Failed API request | `claude_code.api_error` event |
| Chat span | Time a Desktop chat window was active | Not available from Claude; future, if at all |

## Data per event (target fields)
Common: `ts` (ms), `account_id`, `session_id`, `task_id`, `type`, `seq`.
- Prompt: length in characters; text only if the user turned capture on.
- Model call: model, input/output/cache-read/cache-write tokens, estimated cost, duration, success.
- Tool run: tool name, target (file path, command, MCP server and tool), duration, success, output size; for Edit, lines added and removed.
- Decision: tool, decision, source (config, user, hook).
- Error: status, message class, retry attempt.

Where fields come from is fixed by tests 3 and 4: OpenTelemetry events first, transcript JSONL second (timestamps, `tool_use` ↔ `tool_result` ids, per-message usage), for joins OTel lacks.

## Zoom levels (semantic zoom)
| Visible span | What lanes show | Histogram bucket |
|---|---|---|
| > 2 days | Daily activity blocks per account | 1 hour |
| 2 h – 2 days | Task spans (bars from prompt to last event) | 1–5 min |
| 45 min – 2 h | Task spans with model-call ticks | 30 s |
| ≤ 45 min | Individual events: prompt bars, model-call dots sized by cost, tool squares with duration tails | 1–5 s |
| ≤ 1 min | Same, with labels and second ticks | 100 ms |

Zoom range: 30 days to 5 seconds. Tick labels switch to `HH:MM:SS` below one minute. Timestamps in the inspector always show milliseconds.

## Layout
1. **Toolbar:** zoom presets (Day, 1 h, 10 min, 1 min, 10 s), zoom in/out, visible range readout, event filter, replay button and speed.
2. **Spend histogram:** stacked by account colour, peak value labelled.
3. **Lanes:** one per account (scope via the account rail). Expand a lane to one sub-lane per conversation (v1.x).
4. **Overlays:** now line, replay playhead, limit-window shading (5 h, weekly), budget thresholds, anomaly markers, bookmarks.
5. **Minimap:** the whole range with the viewport rectangle; drag to move.
6. **Inspector:** the selected event to the millisecond, with account, conversation, task and type-specific fields; previous/next; zoom to task.
7. **Leaderboard:** most expensive conversations, tasks, tool runs (and later files and MCP servers) in the visible range; clicking zooms there.
8. **Range summary** when nothing is selected: spend, tokens, model calls, tool runs.

## Interactions
| Input | Action |
|---|---|
| Wheel / pinch | Zoom around the cursor |
| Drag | Pan |
| Click | Select event (fine zoom) or task (coarse zoom) |
| Double-click | Zoom to the task |
| ← / → | Previous / next event (respects filter); pans if needed |
| + / − | Zoom in / out |
| Shift-drag (v1.x) | Select a range, then summarise or export |
| B (v1.x) | Bookmark the selected moment |

## Replay
Moves a playhead through the range at 10×, 60× or 600×, selecting each event as it passes and keeping the playhead in view. Stops at "now". v1.x: show the prompt (if captured), tool targets and diffs as they happen.

## Cost attribution rules
- Model call cost is its own estimated cost from telemetry.
- Task cost = sum of model calls in the task.
- Tool-run cost = cost of the next model call in the same task that consumed its output, labelled "fed to the next call" (it is an attribution, not a bill).
- Conversation cost = sum of its tasks.
- All costs are API-equivalent estimates; plans are flat-priced. Label this everywhere.

## Anomalies (v1.x)
- Retry loop: same tool and target 3+ times within 2 minutes with failures.
- Runaway output: a tool output above 20K tokens.
- Cost spike: a 1-minute bucket above 4× the account's median.
- Stuck: a session waiting for permission for more than 10 minutes.

## Performance budget
- 200,000 events in a 30-day range at 60 fps pan and zoom.
- Pre-aggregated rollups per minute and hour (`DATA-MODEL.md`); raw events loaded only for spans under 2 hours.
- Canvas rendering, hit testing by binary search on sorted timestamps.

## Privacy
- Prompt text and tool output content are not stored unless the user turns capture on per account.
- Retention setting: 30, 90, 365 days or forever (default 90). Raw events older than retention are deleted; rollups kept.

## Prototype
`prototype/facet.html` → Timeline. Implements zoom presets, wheel zoom, drag pan, minimap, event inspector to the millisecond, keyboard stepping, leaderboards and replay with sample data.
