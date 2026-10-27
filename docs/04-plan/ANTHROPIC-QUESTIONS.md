# Questions for Anthropic (drafts; the user decides whether and where to send them)

Facet does not contact Anthropic itself. These are drafts for the user to send, for example as a GitHub issue on `anthropics/claude-code`, a support request, or (for item 1, if it looks like a gap) a security report through Anthropic's disclosure channel.

## 1. Is cross-session messaging meant to cross `CLAUDE_CONFIG_DIR` boundaries?

**Draft (GitHub issue or security report):**

> On Claude Code 2.1.310 (Linux), `/peers` and `ListAgents` only list sessions that share the same `CLAUDE_CONFIG_DIR`, which matches the docs (https://code.claude.com/docs/en/cross-session-messaging.md). However, `SendMessage` to a raw `uds:/run/user/<uid>/cc-socks/<pid>.sock` address delivers to a session running under a *different* `CLAUDE_CONFIG_DIR` (a different signed-in account), and that session can reply the same way. The receiving session processed the message without a prompt to its user because both ran in compatible permission modes.
>
> Questions: (a) Is delivery across config dirs intended, and will it stay? (b) If not intended, is `crossSessionInbound: "hold"` or `"refuse"` the recommended way to stop it today? (c) Is there a documented way for a user to tell two of their own accounts' sessions about each other?
>
> Steps: start session A with `CLAUDE_CONFIG_DIR=~/.claude-a` and session B with the default dir; in A call `SendMessage` with `to: "uds:/run/user/$UID/cc-socks/<B's pid>.sock"`.

Why Facet needs the answer: D-010 keeps the native Bridge in the Lab until this is documented.

## 2. Third-party apps that run `claude -p` / `claude plugin eval` on a user's subscription

**Draft (support request):**

> We are building an unofficial, local-only companion app. At the user's click it would run the official `claude -p` or `claude plugin eval` in the user's own account, to draft or test a skill the user is writing. The app sends no requests itself and never handles tokens. Is this an acceptable use of a Pro/Max subscription, and may such features be part of a paid app?

Why: RISKS row "Paid Studio features"; PRD M11 open question.

## 3. Desktop's Code tab and `CLAUDE_CONFIG_DIR`

**Draft (GitHub issue, low priority):**

> Claude Desktop's Code tab inherits `CLAUDE_CONFIG_DIR` from the Desktop process environment and otherwise uses `~/.claude`, while it signs in with the Desktop's own account. With a second Desktop instance (`--user-data-dir`) signed into another account, its Code sessions write transcripts and settings into the first account's folder unless the launcher sets `CLAUDE_CONFIG_DIR`. Is a per-instance setting for this planned?
