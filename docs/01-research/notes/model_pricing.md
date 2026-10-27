# Claude API model prices (for the Facet cost meter)

Date checked: 2026-10-26.

These are Anthropic's first-party Claude API list prices in US dollars per million tokens (MTok). Facet uses them to turn the token counts in Claude Code transcripts into an estimated dollar cost. Subscription users (Pro, Max, Team, Enterprise) do not pay these rates per token, so for them the figure is an "API-equivalent" cost, not a bill.

Sources (all official, all read on 2026-10-26):

- Main source, every number in the table below: https://platform.claude.com/docs/en/about-claude/pricing (fetched as `https://platform.claude.com/docs/en/about-claude/pricing.md`)
- Cross-check of the main numbers, web search, fast mode and batch: https://claude.com/pricing It shows the same figures for every model it lists.
- Claude Code fast mode: https://code.claude.com/docs/en/fast-mode

Nothing in the table is unverified. Items marked UNVERIFIED in the notes below are things the official pages do not state.

## Price table

Order is most specific prefix first, so a simple "first prefix that matches the model id" lookup gives the right row. Dated ids such as `claude-opus-4-1-20250805` or `claude-haiku-4-5-20251001` match by prefix.

| Model id prefix | Model | Input | Output | Cache write 5m | Cache write 1h | Cache read (hits and refreshes) |
|---|---|---|---|---|---|---|
| `claude-fable-5-1` | Claude Fable 5.1 | $10 | $50 | $12.50 | $20 | $0.25 |
| `claude-fable-5` | Claude Fable 5 | $10 | $50 | $12.50 | $20 | $1 |
| `claude-mythos-5-1` | Claude Mythos 5.1 (limited access) | $10 | $50 | $12.50 | $20 | $0.25 |
| `claude-mythos-5` | Claude Mythos 5 (limited access) | $10 | $50 | $12.50 | $20 | $1 |
| `claude-opus-5-5` | Claude Opus 5.5 | $4 | $20 | $5 | $8 | $0.20 |
| `claude-opus-5` | Claude Opus 5 | $5 | $25 | $6.25 | $10 | $0.50 |
| `claude-opus-4-8` | Claude Opus 4.8 | $5 | $25 | $6.25 | $10 | $0.50 |
| `claude-opus-4-7` | Claude Opus 4.7 | $5 | $25 | $6.25 | $10 | $0.50 |
| `claude-opus-4-6` | Claude Opus 4.6 | $5 | $25 | $6.25 | $10 | $0.50 |
| `claude-opus-4-5` | Claude Opus 4.5 | $5 | $25 | $6.25 | $10 | $0.50 |
| `claude-opus-4-1` | Claude Opus 4.1 (retired on the Claude API) | $15 | $75 | $18.75 | $30 | $1.50 |
| `claude-opus-4` | Claude Opus 4 (retired on the Claude API) | $15 | $75 | $18.75 | $30 | $1.50 |
| `claude-sonnet-5-5` | Claude Sonnet 5.5 | $2 | $10 | $2.50 | $4 | $0.10 |
| `claude-sonnet-5` | Claude Sonnet 5 | $2 | $10 | $2.50 | $4 | $0.20 |
| `claude-sonnet-4-6` | Claude Sonnet 4.6 | $3 | $15 | $3.75 | $6 | $0.30 |
| `claude-sonnet-4-5` | Claude Sonnet 4.5 | $3 | $15 | $3.75 | $6 | $0.30 |
| `claude-sonnet-4` | Claude Sonnet 4 (retired on the Claude API) | $3 | $15 | $3.75 | $6 | $0.30 |
| `claude-haiku-5-5` | Claude Haiku 5.5, prompts up to 100,000 tokens | $0.10 | $0.50 | $0.125 | $0.20 | $0.01 |
| `claude-haiku-5-5` (over 100K) | Claude Haiku 5.5, prompts over 100,000 tokens | $0.50 | $2.50 | $0.625 | $1 | $0.05 |
| `claude-haiku-4-5` | Claude Haiku 4.5 | $1 | $5 | $1.25 | $2 | $0.10 |
| `claude-3-5-haiku` | Claude Haiku 3.5 (retired on the Claude API) | $0.80 | $4 | $1 | $1.60 | $0.08 |

Source for every row: https://platform.claude.com/docs/en/about-claude/pricing#model-pricing

Points worth knowing when computing cost:

- Cache writes are 1.25x the input price (5 minutes) and 2x (1 hour) on every model. Cache reads are 0.1x on most models, but 0.05x on Opus 5.5 and Sonnet 5.5, and 0.025x on Fable 5.1 and Mythos 5.1. Do not derive cache read from a single multiplier. Source: https://platform.claude.com/docs/en/about-claude/pricing#prompt-caching
- Sonnet 5 at $2 / $10 is now its standard price. The planned rise to $3 / $15 on 2026-09-01 did not happen (footnote 3 on the pricing page).
- The `claude-api` skill bundled with Claude Code (cached 2026-10-06) says Sonnet 5.5 cache reads cost $0.20. Both official pages say $0.10. Use $0.10.
- Claude 3.x: only Haiku 3.5 is still on the pricing page. Claude 3 Haiku, 3 Opus, 3.5 Sonnet and 3.7 Sonnet are not listed any more, so their prices are UNVERIFIED as of today. Facet should show "price unknown" for them rather than guess.
- `claude-mythos-preview` has no row on the pricing page (UNVERIFIED).
- Retired models are still priced on the page because old transcripts may contain them, and they remain on some cloud platforms.
- Claude Code also writes `"model": "<synthetic>"` on messages it creates itself (UNVERIFIED: not in the official docs, and not checked against a local transcript for this note). Those carry no real usage and should cost $0.

## Web search

$10 per 1,000 searches, on top of normal token costs. Each search counts once however many results it returns, and a failed search is not billed. Transcripts show the count as `usage.server_tool_use.web_search_requests`.
Source: https://platform.claude.com/docs/en/about-claude/pricing#web-search-tool (same figure on https://claude.com/pricing).

Web fetch has no extra charge, only token costs. Code execution is free when used with the `_20260209` web search or web fetch tools. Otherwise it costs $0.05 per container-hour after 1,550 free hours a month per organization. Source: same page, "Specific tool pricing".

## Long context (over 200K tokens)

There is no longer a premium for long context on current models. Claude 4.6 and later models (except Haiku 5.5) get the full 1M-token window at standard prices: "A 900k-token request is billed at the same per-token rate as a 9k-token request."
Source: https://platform.claude.com/docs/en/about-claude/pricing#long-context-pricing

The one exception is Claude Haiku 5.5. It charges per request by prompt length, and the threshold is 100,000 tokens, not 200K. A request whose prompt (input plus cache reads plus cache writes) is over 100,000 tokens pays the higher row for all of its tokens, output included. Each request is priced on its own. The rates are in the table above.

Older models (Sonnet 4.5, Sonnet 4, Opus 4.5 and earlier) are believed to have had a 200K window, or a 1M beta with premium pricing, in the past (UNVERIFIED, from memory, not on any current official page). The current page shows no long-context premium for them, so Facet should use the standard rates. Whether any old transcripts were billed at a past premium is UNVERIFIED.

## Fast mode (speed premium)

Fast mode runs the same Opus model up to 2.5x faster at a higher price per token.

| Model | Fast input | Fast output | Multiplier vs standard |
|---|---|---|---|
| Claude Opus 5.5 | $8 | $40 | 2x |
| Claude Opus 5 | $10 | $50 | 2x |
| Claude Opus 4.8 | $10 | $50 | 2x |

Sources: https://platform.claude.com/docs/en/about-claude/pricing#fast-mode-pricing and https://code.claude.com/docs/en/fast-mode

- Prompt caching multipliers apply on top of the fast rates, so cache writes and reads under fast mode are 2x the standard cache prices.
- Fast pricing is flat across the whole 1M window.
- Opus 4.7 fast mode was removed on 2026-07-24. Sonnet, Haiku and Fable have no fast mode.
- In Claude Code, subscription users pay for fast mode from usage credits, outside their plan limits. Fast and standard speed keep separate caches, so switching speed mid-conversation re-reads the context uncached.
- The API returns `usage.speed` to say which speed ran. Whether Claude Code writes this field into its transcript JSONL is UNVERIFIED. Facet should check a real fast-mode transcript before relying on it.

Priority Tier: the pricing page gives no Priority Tier prices (they are set by contract). UNVERIFIED, and not relevant to Claude Code transcripts.

## Other modifiers

- Batch API: 50% off input and output on every model. Caching multipliers stack with it. Fast mode is not available with batch. Claude Code does not use batch, so Facet can ignore it. Source: https://platform.claude.com/docs/en/about-claude/pricing#batch-processing
- US-only inference (`inference_geo: "us"`): 1.1x on all token types for Claude 4.6 and later models. Source: https://platform.claude.com/docs/en/about-claude/pricing#data-residency-pricing
- Tokenizer: Claude 4.7 and later models produce about 30% more tokens for the same text. This changes token counts, not prices.
- Amazon Bedrock and Google Cloud (Google managed) set their own prices. Claude Platform on AWS and Microsoft Foundry bill the standard rates.

## Machine-readable copy

Haiku 5.5 is given at its prompts-up-to-100K rates. For a request whose prompt is over 100,000 tokens, multiply every Haiku 5.5 rate by 5. Fast mode on Opus 5.5, Opus 5 and Opus 4.8 is 2x all rates.

```json
{"checked":"2026-10-26","source":"https://platform.claude.com/docs/en/about-claude/pricing","models":[{"prefix":"claude-fable-5-1","input":10,"output":50,"cacheWrite5m":12.5,"cacheWrite1h":20,"cacheRead":0.25},{"prefix":"claude-fable-5","input":10,"output":50,"cacheWrite5m":12.5,"cacheWrite1h":20,"cacheRead":1},{"prefix":"claude-mythos-5-1","input":10,"output":50,"cacheWrite5m":12.5,"cacheWrite1h":20,"cacheRead":0.25},{"prefix":"claude-mythos-5","input":10,"output":50,"cacheWrite5m":12.5,"cacheWrite1h":20,"cacheRead":1},{"prefix":"claude-opus-5-5","input":4,"output":20,"cacheWrite5m":5,"cacheWrite1h":8,"cacheRead":0.2},{"prefix":"claude-opus-5","input":5,"output":25,"cacheWrite5m":6.25,"cacheWrite1h":10,"cacheRead":0.5},{"prefix":"claude-opus-4-8","input":5,"output":25,"cacheWrite5m":6.25,"cacheWrite1h":10,"cacheRead":0.5},{"prefix":"claude-opus-4-7","input":5,"output":25,"cacheWrite5m":6.25,"cacheWrite1h":10,"cacheRead":0.5},{"prefix":"claude-opus-4-6","input":5,"output":25,"cacheWrite5m":6.25,"cacheWrite1h":10,"cacheRead":0.5},{"prefix":"claude-opus-4-5","input":5,"output":25,"cacheWrite5m":6.25,"cacheWrite1h":10,"cacheRead":0.5},{"prefix":"claude-opus-4-1","input":15,"output":75,"cacheWrite5m":18.75,"cacheWrite1h":30,"cacheRead":1.5},{"prefix":"claude-opus-4","input":15,"output":75,"cacheWrite5m":18.75,"cacheWrite1h":30,"cacheRead":1.5},{"prefix":"claude-sonnet-5-5","input":2,"output":10,"cacheWrite5m":2.5,"cacheWrite1h":4,"cacheRead":0.1},{"prefix":"claude-sonnet-5","input":2,"output":10,"cacheWrite5m":2.5,"cacheWrite1h":4,"cacheRead":0.2},{"prefix":"claude-sonnet-4-6","input":3,"output":15,"cacheWrite5m":3.75,"cacheWrite1h":6,"cacheRead":0.3},{"prefix":"claude-sonnet-4-5","input":3,"output":15,"cacheWrite5m":3.75,"cacheWrite1h":6,"cacheRead":0.3},{"prefix":"claude-sonnet-4","input":3,"output":15,"cacheWrite5m":3.75,"cacheWrite1h":6,"cacheRead":0.3},{"prefix":"claude-haiku-5-5","input":0.1,"output":0.5,"cacheWrite5m":0.125,"cacheWrite1h":0.2,"cacheRead":0.01},{"prefix":"claude-haiku-4-5","input":1,"output":5,"cacheWrite5m":1.25,"cacheWrite1h":2,"cacheRead":0.1},{"prefix":"claude-3-5-haiku","input":0.8,"output":4,"cacheWrite5m":1,"cacheWrite1h":1.6,"cacheRead":0.08}],"webSearchPer1k":10}
```
