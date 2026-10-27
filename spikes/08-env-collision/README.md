# Spike 8: override variables collapse accounts

**Question.** If `ANTHROPIC_API_KEY` (or `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`) is set in the environment, does it override a profile's signed-in account? Anthropic's authentication docs say it takes precedence; this confirms it on our version.

## Method (needs the user's go-ahead: it makes Claude Code send requests to Anthropic with fake credential values)
```bash
ANTHROPIC_API_KEY=sk-ant-invalid-facet-test claude -p "say ok" ; echo "exit=$?"
claude -p "say ok" ; echo "exit=$?"
```
Expected: the first fails with an invalid-key error even though the profile is signed in; the second succeeds. Never use a real key.

Also check the user's shell files for these variables (names only, so values never pass through the pipe): `grep -noE '\b(ANTHROPIC_API_KEY|ANTHROPIC_AUTH_TOKEN|CLAUDE_CODE_OAUTH_TOKEN|ANTHROPIC_BASE_URL)\b' ~/.bashrc ~/.profile ~/.bash_profile 2>/dev/null`

## Result (2026-10-26, Claude Code 2.1.310): done

**Answer: yes, all three variables override a signed-in profile, and none of them falls back to the sign-in.** That confirms the scrubbing rule (rule 6) and the Health check for it.

Requests sent: three with fake credential values and one control with the real sign-in, all by the official Claude Code during development (approved by the user: "keep going yourself", then "I give you all the authorization"). Facet sent nothing. This is a one-time developer test: never repeat it in Health, CI or a loop, because repeated invalid sign-ins look like credential abuse. Future runs should use a neutral fake value such as `facet-invalid-test` without a real key prefix, so secret scanners stay quiet.

Commands (personal profile, signed in; fake values only; each run started from a scrubbed environment and then given a single variable):
```bash
U="-u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u CLAUDE_CODE_OAUTH_TOKEN -u CLAUDE_CONFIG_DIR"
env $U ANTHROPIC_API_KEY=sk-ant-invalid-facet-test claude -p "say ok" --output-format json
env $U ANTHROPIC_AUTH_TOKEN=facet-invalid-test       claude -p "say ok" --output-format json
env $U CLAUDE_CODE_OAUTH_TOKEN=facet-invalid-test    claude -p "say ok" --output-format json
env $U                                               claude -p "say ok" --output-format json
```

| Run | Exit | `is_error` | `api_error_status` | `result` | stderr |
|---|---|---|---|---|---|
| fake `ANTHROPIC_API_KEY` | 1 | true | 401 | `Invalid API key · Fix external API key` | `⚠ claude.ai connectors are disabled because ANTHROPIC_API_KEY or another auth source is set and takes precedence over your claude.ai login` |
| fake `ANTHROPIC_AUTH_TOKEN` | 1 | true | 401 | `Failed to authenticate. API Error: 401 Invalid bearer token` | same warning |
| fake `CLAUDE_CODE_OAUTH_TOKEN` | 1 | true | 401 | `Failed to authenticate. API Error: 401 Invalid bearer token` | (none) |
| control, scrubbed | 0 | false | — | `ok` | (none) |

Shell files: a names-only search (`grep -nE '<the three names>|ANTHROPIC_BASE_URL' … | cut -d= -f1` over `~/.bashrc`, `~/.profile`, `~/.bash_profile`, `~/.zshrc`, `~/.config/environment.d/*.conf`, `/etc/environment`) found nothing on this machine. Use `grep -noE` next time: `cut` still lets whole lines pass through the pipe.

**Consequences:**
- Health should flag any of the three variables in Facet's own environment (presence only), the login environment or shell files (names only), and flag `ANTHROPIC_BASE_URL` there too (the vector in CVE-2026-21852). Health must not inspect Claude's own processes: Desktop sets `ANTHROPIC_BASE_URL` and the token on its Code children by design.
- The stderr warning is a cheap signal: Facet can show it when a user-started session reports it.
- Claude Desktop legitimately sets `CLAUDE_CODE_OAUTH_TOKEN` for its own Code children (spike 1). Rule 6 covers only what Facet starts directly, so Facet must not try to change that.
