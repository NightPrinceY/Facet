---
name: surface-check
description: Re-check docs/03-architecture/EXTENSION-SURFACE.md against the installed Claude Code version and the official docs; record what changed. Use when claude --version moves to a new minor release or before building Studio, Control or Ecosystem parsers.
disable-model-invocation: true
---

1. Run `claude --version` and compare with the version at the top of `docs/03-architecture/EXTENSION-SURFACE.md`.
2. Run `claude --help`, `claude plugin --help`, `claude plugin eval --help`, `claude mcp --help` and diff the subcommands and flags against the doc.
3. Ask a `claude-code-guide` agent what changed in skills frontmatter, subagent fields, hook events, plugin and marketplace manifests, settings keys and cross-session messaging since the doc's date, with URLs.
4. Update the doc in place: new version and date at the top, changed items edited, removed items struck with the version that removed them. Keep `[unverified]` marks honest.
5. If a change affects a rule in `CLAUDE.md` or a module in the PRD, record it with `/decide`.
