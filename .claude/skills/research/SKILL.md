---
name: research
description: Deep, sourced research for Facet product or technical questions; writes a note under docs/01-research/notes/ with URLs and [unverified] marks. Use when the user asks to research, investigate, compare competitors, or "search deeply".
argument-hint: "<question>"
---

Research this question for Facet: $ARGUMENTS

1. Read `docs/01-research/REPORT.md` and list `docs/01-research/notes/` so you build on what is known. Do not redo settled questions.
2. Split the question into 2–4 independent strands (for example: official docs, competitors, user complaints, security). Run each strand as a background agent in one message: `claude-code-guide` for anything about Claude Code, the API or the Agent SDK; `general-purpose` with web search for the rest. Ask each for URLs and for `[unverified]` on single-source claims.
3. While they run, check facts you can verify locally (CLI `--help`, file layouts by name only). Never read credentials, cookies, keychain items or `*.key` files.
4. Merge into `docs/01-research/notes/<topic>.md`: verdict first, then findings with sources, then ranked ideas checked against the nine rules in `CLAUDE.md`. Facts and claims stay separate.
5. If the findings change scope or architecture, update the PRD or architecture docs and record it with `/decide`.
6. Report in plain English with the Arabic summary at the end.
