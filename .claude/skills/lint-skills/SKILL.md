---
name: lint-skills
description: Lint Claude Code skills, commands and subagents (frontmatter, description budget, body length, shell injection, broken links) and run the official claude plugin validate on plugin roots. Use after creating or editing a skill, agent or plugin, or to audit an account's extensions.
argument-hint: "[dir ...] (default: .claude)"
allowed-tools: Bash(python3 -I spikes/11-skill-authoring/lint_skills.py:*), Bash(claude plugin validate:*)
---

Targets: $ARGUMENTS (if empty, use `.claude`).

1. Run `python3 -I spikes/11-skill-authoring/lint_skills.py <targets>`. It reads frontmatter and prints findings only, never skill bodies.
2. For each target that contains `.claude-plugin/plugin.json` or `.claude-plugin/marketplace.json`, also run `claude plugin validate <target> --json` and summarise errors, warnings and `gatingHooks`. On a loose skills folder the validator returns success with zero items; say that it checked nothing there.
3. Report one table: file, severity (error, risk, warn, info), finding, fix. Lead with errors and risks.
4. Offer fixes, but edit skills in another account's config folder only after the user says yes, and back up the file first (rule 7).
