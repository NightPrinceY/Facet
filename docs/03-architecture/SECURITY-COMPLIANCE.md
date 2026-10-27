# Security and compliance

*Research, not legal advice. Re-check the sources before launch.*

## What Anthropic's documents say (as of October 2026)
| Source | Relevant point |
|---|---|
| [Consumer Terms](https://www.anthropic.com/legal/consumer-terms) | No sharing credentials or making an account available to others. No automated or non-human access except via API key or where explicitly permitted. No bypassing protective measures. |
| [Usage Policy](https://www.anthropic.com/legal/aup) | No multiple accounts to avoid detection or evade a ban. No bypassing restrictions or guardrails. Page shows effective date Nov 12, 2026: re-check after. |
| [Claude Code legal and compliance](https://code.claude.com/docs/en/legal-and-compliance) | Third parties may not offer Claude.ai login, route requests through Free/Pro/Max credentials, or collect, store or intermediate credentials or tokens. An end user signing in to the unmodified Claude Code binary with their own subscription is fine. No Claude names or logos in product names or logos. |
| [Claude Code authentication](https://code.claude.com/docs/en/authentication) | Officially recommends one `CLAUDE_CONFIG_DIR` per account. API key and token env vars override sign-in. |
| GitHub #27359 (closed 2026-08-17 by the Claude Code lead) | Profiles: use `CLAUDE_CONFIG_DIR`; a `--profile` flag would be a thin wrapper. |

Enforcement so far (2026) targeted client spoofing, token reuse and capacity reselling (OpenCode, OpenClaw), not people separating their own accounts.

## Facet's design rules (enforced in code review)
1. Official binaries only; never modify, repackage or re-sign.
2. Never read, copy, back up, sync or log credentials, keychain items, cookies, session stores or tokens. Backups explicitly exclude `.credentials.json` and Electron storage folders.
3. No network calls to Anthropic. The only listener is the OTLP receiver on 127.0.0.1.
4. No account rotation, pooling or "more quota" features or copy.
5. No UI automation of Claude apps. Bridge uses documented Claude Code interfaces, user-triggered.
6. Scrub `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` from spawned processes.
7. Settings writes: schema-validated, diffed, backed up, atomic; never managed policy files; Desktop config only while stopped.
8. All accounts must belong to the user (onboarding states this).
9. Prompt and message content off by default; local only; excluded from crash reports.

## Threat model (short)
| Threat | Mitigation |
|---|---|
| Malicious skill or MCP server | Static safety preview, pinned-version warnings, per-account install, walls |
| Local malware reading Facet's DB | No secrets in DB; prompt capture off by default; OS-level disk encryption recommended |
| Another local process posting to the OTLP port | Bind 127.0.0.1; per-install random path token in the endpoint (verify Claude Code accepts a path) |
| Facet bug corrupting Claude config | Backup before write, atomic rename, one-click restore |
| Supply chain of Facet itself | Signed releases, reproducible builds, minimal dependencies, `cargo audit` and `npm audit` in CI |

## Release checklist
- [ ] `compliance-reviewer` agent passes on the release diff.
- [ ] No string "claude" in bundle id, package name, binary name.
- [ ] Network capture during a full test run shows no Anthropic endpoints contacted by Facet.
- [ ] Backups verified to exclude credential files.
- [ ] Network capture separates traffic of the official `claude` (for example `claude auth status`, user-triggered) from Facet's own process, which must have none.
- [ ] `env::command` lint test, xdotool allow-list test and official-build checks pass (D-013).
