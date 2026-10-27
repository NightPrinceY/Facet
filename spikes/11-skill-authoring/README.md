# Spike 11: authoring, linting and placing skills per account

**Status: done (parts A, B, C, 2026-10-26).**

**Question.** Can Facet's Studio (PRD M11, D-009) create, check and place skills per account using only documented files and the official CLI? Specifically:
1. Does a skill in `<config>/skills/` load only in sessions of that `CLAUDE_CONFIG_DIR`?
2. What does `claude plugin validate` check, and where does Facet need its own linter?
3. Does `claude plugin eval` run against the parent account's sign-in without Facet touching it, and what does a run cost?

## Method
- Part A (static, no prompts): `python3 -I lint_skills.py <dirs>` over this project, both accounts, and an installed plugin; compare with `claude plugin validate --json` on the same folders.
- Part B (needs approval; one short prompt per account): create a throwaway skill `facet-spike-11-probe` (description: "Use when the user says the exact phrase facet-probe-11") in `~/.claude-work/skills/` only, and only if it does not already exist. In each account run `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CONFIG_DIR=<config> claude -p "facet-probe-11" --output-format stream-json` and keep only skill names from `system/init` (the stream also carries reply text). Then remove the skill and log the removal.
- Part C (needs approval; spends quota): `claude plugin eval init` on a scratch plugin wrapping the probe skill, then `claude plugin eval --json --runs 1 --max-cost-usd 0.25 --ablation with-without`. Record the report fields and cost. **Stop condition:** if eval's fresh `CLAUDE_CONFIG_DIR` cannot sign in from the parent account without a token variable or copied sign-in files, stop and record that; do not run `claude setup-token` and do not copy `.credentials.json`. Same `env -u` scrubbing as Part B.

## Result

### Part A (2026-10-26, Claude Code 2.1.310): done

Commands:
```bash
python3 -I spikes/11-skill-authoring/lint_skills.py .claude ~/.claude-work ~/.claude
claude plugin validate .claude --json
claude plugin validate ~/.claude --json
claude plugin validate ~/.claude/skills --json
claude plugin validate ~/.claude/skills/huggingface-best --json
claude plugin validate ~/.claude/plugins/cache/claude-plugins-official/vercel/0.50.0 --json
```

Facet linter: 41 files (project commands and agents, 38 personal-account skills; the work account has no skills yet). 0 errors, 7 warnings:
- 3 skills with multi-line (block scalar) descriptions: `huggingface-best`, `vercel-composition-patterns`, `vercel-react-native-skills`.
- 4 skills with bodies over 500 lines (589 to 733): `huggingface-llm-trainer`, `huggingface-paper-publisher`, `huggingface-vision-trainer`, `transformers-js`.
- No shell injection, no frontmatter hooks, no unknown keys, no broken links.

Official validator:

| Target | Exit | Report |
|---|---|---|
| `.claude` (project) | 0 | `success: true`, `contents: []` |
| `~/.claude` (account folder) | 0 | `success: true`, `contents: []` |
| `~/.claude/skills` | 0 | `success: true`, `contents: []` |
| `~/.claude/skills/huggingface-best` | 1 | `No manifest found in directory. Expected .claude-plugin/marketplace.json or .claude-plugin/plugin.json` |
| vercel plugin 0.50.0 | 0 | marketplace manifest + 2 contents, warnings e.g. `CLAUDE.md at the plugin root is not loaded as project context` |

JSON shape: `success, strict, target, manifest{file,type,errors[],warnings[],notes[],gatingHooks[]}, contents[{file,type,errors,warnings,notes,gatingHooks}], advice[], directory`; each finding is `{path, message, code}`.

**Findings.**
1. `claude plugin validate` checks **plugins and marketplaces only**. On loose skills (an account's `skills/` or a project's `.claude/`) it returns success with zero items, which looks like a pass but checks nothing. **Facet needs its own linter for loose skills and agents**, and should use the official validator for plugin roots, merging both into one report.
2. A cheap path to official checks for loose skills: wrap them in a temporary plugin folder (`.claude-plugin/plugin.json` + `skills/`) in the scratch area and validate that (to try in Part B).
3. `gatingHooks` in the validator's output is a ready-made input for the safety preview.

### Part B (2026-10-26, Claude Code 2.1.310): done

**Answer: yes. A skill in `<config>/skills/` loads only for sessions of that `CLAUDE_CONFIG_DIR`.**

- Created `~/.claude-work/skills/facet-spike-11-probe/SKILL.md` at 01:45:58 (it did not exist before). Description: "Throwaway Facet test skill. Use only when the user's message is exactly the phrase facet-probe-11." Body: reply `PROBE-11-LOADED`.
- Each account ran `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u CLAUDE_CODE_OAUTH_TOKEN [CLAUDE_CONFIG_DIR=…] claude -p "facet-probe-11" --output-format stream-json --verbose`. Only skill names and tool names were read from the stream.

| Account | Skills in `system/init` | Probe listed | What happened | Cost |
|---|---|---|---|---|
| work | 80 | **yes** | `Skill {"skill": "facet-spike-11-probe"}`, result `PROBE-11-LOADED` | $0.165 |
| personal | 101 | **no** | ran `ls -la`, then said it could not tell what the user wanted | $0.249 |

- Removed the probe at 01:48:26.
- **Side finding:** Claude Code itself created `~/.claude-work/skills/synced/<bucket>/` at 01:36, during spike 1's first headless work session. It holds the account's claude.ai skills (`built-in-browser`, `xlsx`, `pdf`, `skill-creator`, `setup-writing-style`, `google-workspace`, …) plus `.bucket-*` and `.last-complete-round` markers. `~/.claude/skills/synced/` exists too. The bucket name uses the same two ids as the work Desktop's `claude-code-sessions/<id>/<id>/` folder. **Studio must treat `skills/synced/` as owned by Claude Code:** show it read-only as "synced from claude.ai", never place into it, never lint it as the user's own. Left untouched.

### Part C (2026-10-26, Claude Code 2.1.310): done

**Answer: `claude plugin eval` works per account with no help from Facet on sign-in, and it gives exactly the trigger lab that Studio needs.** The stop condition was not hit: no token variable was passed and no files were copied. The help text says each run is "a full claude child on your own credential", so Claude Code passes the credential internally.

Setup: a scratch plugin (`.claude-plugin/plugin.json` + `skills/facet-spike-11-probe/`) in the scratchpad; `claude plugin eval init --bare triggers` gave the template (`prompt.md` with `max_turns`, `allowed_tools`; `graders/criteria.md` with `type: llm`, `weight`). Two cases:
- `triggers` (tag `should-trigger`): prompt `facet-probe-11`; graders `type: regex` (`pattern: "PROBE-11-LOADED"`) and `type: tool_used` (`tool: Skill`).
- `no-trigger` (tag `should-not-trigger`): "What is the capital of France? Answer in one word."; grader `type: llm` ("Paris" and no marker).

Command:
```bash
env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CONFIG_DIR=~/.claude-work \
  claude plugin eval . --no-publish --trust-plugin --max-cost-usd 3 --ablation with-without --json <file>
```
`claude plugin validate .` passed first (one warning: no author).

Result: exit 0, 44 s, `costUsd` 0.367, `partial: false`.

Change from the planned method: the plan said `--runs 1 --max-cost-usd 0.25`. The run used the cases' own `runs: 2` and a $3 ceiling, after the user said "don't think about the cost, think about the best thing" and gave full authorisation. Two runs per arm give a less noisy delta. Facet itself must still show the ceiling and get a click per run.

| Case | With plugin | Without plugin | Delta |
|---|---|---|---|
| triggers | score 1, pass rate 1 (marker ✓, skill-fired ✓, `withOnly: true`) | score 0 (marker ✗) | +1 |
| no-trigger | score 1 | score 1 | 0 |

Overall: `casesPassed 2/2`, `overallScore 1`, `meanDelta 0.5`. Per-run cost about $0.04–0.05; judge model haiku by default.

JSON shape (schemaVersion 1): `claudeVersion, startedAt, durationSeconds, costUsd, partial, suite{root, ablation, threshold, concurrency, plugins[{name, version, path}]}, cases[{name, dir, source, promptMarkdown, runsPerCase, timeoutSeconds, maxTurns, graders[], arms{with[], without[]}, aggregates{score, passRate, scoreWithout, passRateWithout, delta}}], aggregates{casesTotal, casesPassed, overallScore, overallPassRate, meanDelta}`. Each run: `costUsd, judgeCostUsd, durationSeconds, error, graders[{name, pass, withOnly}], passed, score, skippedPaidGraders, startedAt, tracePath, turns`. An HTML report is written to `evals/results/<timestamp>/report.html`.

**Findings for Studio (M11):**
1. **Eval publishes its HTML report (with prompts) to claude.ai by default** ("already the default when your account supports it"). Facet must always pass `--no-publish` and say so in the UI (rule 9).
2. Loose skills can be evaluated by wrapping them in a temporary plugin folder. The help also says `skills-dir` plugins (`<name>@skills-dir`) resolve as targets, which is not yet tried.
3. `--trust-plugin` is needed for non-interactive runs. Facet may pass it only for items the user authored in Studio or approved in the safety preview.
4. `--max-cost-usd` and per-run `costUsd` let Facet show a ceiling before the run and the real cost after it. The trigger lab's "should / should not trigger" pairs are just tags on cases.
5. The `tool_used: Skill` grader is automatically "with-only", so the delta measures whether the skill fired.
