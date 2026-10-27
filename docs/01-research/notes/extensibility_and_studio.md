# Extensibility, authoring and deep search (research, 2026-10-26)

Question: should Facet go beyond previewing extensions and help people **create, test, place and govern** Claude Code skills, agents, commands, hooks, plugins and MCP servers per account, control more of the harness, and search sessions deeply? Method: web research plus the local `claude` CLI (2.1.299). Star counts come from third-party aggregators and drift. **[unverified]** marks single-source claims.

## Verdict

Yes. The extension ecosystem exploded in 2026 (skills.sh lists 600k+ skills, the official plugin marketplace grew from ~101 to ~238 plugins), and so did the attacks on it. Many tools author, list or scan extensions, but **none is account-aware**, none combines authoring + testing + safety + placement, and every scanner is a developer CLI. That gap fits Facet's existing promise (control with a diff, safety before install) and survives Anthropic shipping native multi-account.

## 1. Standards and official tools

- **Agent Skills open standard** (agentskills.io, Dec 2025): a skill is a folder with `SKILL.md`; only name and description load at startup. Adopted by Codex, Gemini CLI, Copilot, Cursor, VS Code; portability is uneven. https://www.unite.ai/anthropic-opens-agent-skills-standard-continuing-its-pattern-of-building-industry-infrastructure
- **skill-creator** (anthropics/skills): drafts skills, runs with/without-skill evals, and tunes the description against ~20 should/should-not-trigger queries for up to 5 rounds, via the Claude Code CLI. https://skills.sh/anthropics/skills/skill-creator
- **Built-in plugin system:** `/plugin` manager, `claude plugin validate`, `claude plugin eval` (scored JSON/HTML reports, compares against a no-plugin baseline). Official marketplaces auto-update by default; third-party and local ones do not. https://code.claude.com/docs/en/discover-plugins

## 2. Comparable tools

| Tool | What it does | Gap for Facet's users |
|---|---|---|
| opcode (ex-Claudia), Tauri, AGPL, ~20k★ | Sessions, checkpoints, custom agents, usage, MCP | Not account-aware; reportedly no commits since Oct 2025 |
| skills-manage (Tauri) | Central `~/.agents/skills` symlinked into 20+ agents | Stores GitHub tokens/API keys unencrypted in SQLite |
| Skills-Manager (Tauri 2), ~600★ | Syncs skills across Claude Code, Codex, Opencode | No safety, no accounts |
| claude-code-templates / aitmpl, MIT, ~28k★ | `npx` installer: 400+ agents, 225+ commands, 39+ hooks | Installs without preview |
| skills.sh (Vercel) | Directory, partner security audits before install | Web/CLI only, per machine |
| SkillsMP | Aggregates 185k–2M SKILL.md files | No review process |
| cc-switch | Provider/API-key switching | Handles API keys: not Facet's model |
| Nimbalyst (ex-Crystal) | Parallel sessions in worktrees, kanban | Session runner, not config |
| claude-code-history-viewer (Tauri, ~1.8k★) | Offline transcript browser and search | No accounts, no cost model |

There is no official settings sync (requests: anthropics/claude-code#36693, #22648); people use git or symlinked dotfiles.

## 3. What users complain about

1. **Skills do not trigger** (#9716, #9710, #21947, #20986). Description quality decides routing.
2. **Silent truncation:** descriptions cut at ~250 characters and a total budget trims routing text without warning **[unverified]**; a v2.1.177 bug dropped descriptions for half of user skills.
3. **YAML frontmatter mistakes** stop skills loading silently.
4. **Context bloat:** every skill's metadata and every MCP tool definition costs tokens at startup; `/context` shows it, nothing manages it.
5. **Hook debugging:** successful hooks are silent, matchers are case-sensitive, exit codes are confusing.
6. **Testing skills** is a real need (skill-creator evals, `plugin eval`, community /skill-doctor).
7. **Sharing across machines and accounts**, versioning.
8. **Silent plugin auto-sync** installs new marketplace plugins without consent (#37340).

## 4. Security: why the safety layer matters more now

- Check Point (Feb 2026): repo `.claude/settings.json` hooks ran code on open; CVE-2025-59536 auto-approved repo MCP servers; CVE-2026-21852 exfiltrated API keys via an `env` base-URL override. Patched. https://research.checkpoint.com/2026/rce-and-api-token-exfiltration-through-claude-code-project-files-cve-2025-59536/
- Snyk "ToxicSkills" (Feb 2026): 13.4% of 3,984 public skills had a critical issue. https://snyk.io/blog/toxicskills-malicious-ai-agent-skills-clawhub
- postmark-mcp (Sep 2025): first malicious MCP server in the wild, a rug-pull update on a trusted name. https://thehackernews.com/2025/09/first-malicious-mcp-server-found.html
- PromptArmor: a marketplace plugin's hook auto-approved commands. https://promptarmor.com/resources/hijacking-claude-code-via-injected-marketplace-plugins
- "Plugin4Shell" (Sep 2026): commit-hash-named branches defeat SHA pinning **[unverified]**.
- OWASP Agentic Skills Top 10 (AST01: malicious skills, typosquatting). https://owasp.org/www-project-agentic-skills-top-10/ast01
- Scanners: mcp-scan / Snyk Agent Scan (tool pinning by hash), Cisco skill-scanner (YARA + LLM judge, Apache-2.0), dotclaude-security. All CLIs; LLM modes need keys or network.

## 5. Deep search

Transcripts are deleted after **30 days by default** (`cleanupPeriodDays`). Existing search tools (agsearch, claude-find with FTS5 + Ollama embeddings, cchb, history viewers) are single-account. Recommended design: SQLite FTS5 with the trigram tokenizer (code, paths, Arabic), incremental by byte offset per JSONL file, facet columns (account, project, branch, model, tool, file, cost, date); optional local embeddings later (fastembed-rs + sqlite-vec, reciprocal-rank fusion), never a remote embedding API. Rule 9: message text indexed only after opt-in.

## 6. Ranked ideas (value × feasibility, all compliant)

1. Skill trigger lab (lint, description budget meter, user-started trigger tests via the official `claude -p` / `plugin eval`).
2. Per-account context-cost view (startup tokens of skills, MCP tools, CLAUDE.md) with toggles.
3. Safety preview v2 (bundled static rules, install hash, diff before update, untrusted-repo `.claude/` check).
4. Update gate for marketplaces (show what auto-update would change, approve, roll back).
5. Deep local session search across accounts, resume via `--resume`.
6. Hook workbench (matcher validation, dry-run with recorded input JSON).
7. Per-account library with placement (one library, copied or linked into chosen accounts/projects, drift diff).
8. Local marketplace authoring (scaffold, `claude plugin validate`).
9. Skill usage analytics (what actually ran, dead skills, cost per skill).
10. Transcript retention guard (`cleanupPeriodDays` per account, sealed archive).

Avoid: reading credentials, an LLM judge called from Facet, auto-switching accounts, driving the Claude UI. AI-assisted drafting is allowed only by launching the user's own official `claude` in the chosen account, at the user's request, with override variables scrubbed.
