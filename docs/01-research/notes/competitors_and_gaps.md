# Competitive Landscape and OSS Ecosystem for Managing Claude Desktop + Claude Code (as of 2026-10-25)

Method note: GitHub star counts, last-push dates and licenses below were pulled live via the GitHub API (`gh api repos/...`) on 2026-10-25 and are exact as of that date. Product-page facts come from WebFetch of vendor pages on the same date. Where I relied on search-engine summaries rather than a fetched page, that is flagged. Several roundup/comparison articles are written by competitors (notably Nimbalyst/Stravu) and should be treated as marketing.

## 1. Multi-account tools (Claude Desktop and Claude Code)

### Takeaway
Multi-account for Claude Desktop is a crowded field of small, mostly single-platform paid utilities ($10-$30 one-time) plus dozens of tiny OSS scripts, all using the same trick (`--user-data-dir` for Desktop, `CLAUDE_CONFIG_DIR`/keychain swap for Code). Only one paid product (DeckSpace "Claude Accounts", $14) claims macOS + Windows + Linux (Linux beta); the dominant CLI switcher is claude-swap (~4.0k stars) with rate-limit auto-rotation. None combines multi-account with deep analytics, cross-account bridging, and an ecosystem manager.

### Cited Findings

Desktop (GUI) multi-account products
- **Claude Desktop Manager (claudeprofiles.com)**: Windows 10 (19041+)/11 only, x64 + ARM64; no Mac version. Tiers: Free (unlimited profiles but parallel instances limited to 15 min), Standard $10 one-time (2 simultaneous instances, no time limit), Pro $30 one-time (unlimited instances, per-profile QuickPrompt, global QuickAsk hotkey, daily backup of Claude Code and Cowork work, colored window borders). No subscription. — [claudeprofiles.com](https://claudeprofiles.com/)
- Claude Desktop Manager features: per-profile icons/colors, hotkeys and global switcher, auto-restart, tray, JSON import/export, orphan-window adoption, per-profile MCP servers with validated JSON editor, isolated Claude Code environment per profile with one-click terminal, a "sessions board" (working/waiting/done/stalled), completion alerts, "seed profiles" (copy MCP setup without login), "no telemetry" (only Gumroad license check). Traction: Gumroad 5.0/5 from only 5 ratings; download counter "since June 2026" but no number shown; page internally inconsistent on which tier gets QuickPrompt/QuickAsk/Backup. — [claudeprofiles.com](https://claudeprofiles.com/)
- **Multi-Claude (multiclaude.app)**: macOS 12+ only (Windows "on the roadmap"), $20 one-time, license covers up to 5 Macs, 30-day refund; unlimited profiles each with own Dock icon/process/data dir; migration of existing history; local only. Claude Code support not mentioned. Maker Bryan Parker (independent). Product Hunt "featured" badge; claims most users run 2-3 profiles. No user numbers. — [multiclaude.app](https://multiclaude.app/)
- **Claude Accounts by DeckSpace (deckspace.dev)**: $14 one-time single license (unlimited accounts), Team 5-pack $49 (page also shows $70 — inconsistent), bundle with DeckVoice $29; macOS 13+ (signed/notarized), Windows 10/11 (incl. Microsoft Store Claude since v3.0), Linux beta (Ubuntu 22.04+/Debian 12+, sign-in one account at a time). SSO work accounts (Okta/Microsoft/Google) native on macOS, beta on Windows. Claude Code per-account terminal since v2.8 (VS Code extension not covered). Source code included in download. Current version v3.8. Maker: DeckSpace / Opxera / Samic Ventures LLC. No traction figures. — [deckspace.dev/claude-accounts](https://deckspace.dev/claude-accounts/)
- **Multi Instance for Claude Desktop**: free Windows app by Carlos De La Torre announced via a feature-request post; uses `--user-data-dir` per instance with per-instance profile/color; macOS planned; promised open-source repo not found. (Search-summary only; not independently verified on a store page.) — [claudeissues.com issue 70665](https://claudeissues.com/issue/70665-feature-i-built-a-free-app-for-this-multi-instance-for-claude-desktop-run-isolat)
- **jmdarre-v/claude-multiprofile**: MIT, 133 stars, last push 2026-10-23 (GitHub API). Desktop profiles macOS-only (generated .app launchers with Electron `--user-data-dir`); Code profiles on macOS + Linux via `CLAUDE_CONFIG_DIR` (keychain login keyed to config dir); each profile own login, chats, settings, MCP, plugins, skills, Dock icon, shell alias (`claude-work`). Writes `permissions.deny` rules so a Code profile can't read other profiles' dirs; `doctor` detects two profiles signed into the same account; `extensions` copies Desktop extensions/MCP connectors between profiles; chats/Projects cannot move between profiles. Highest version mentioned v0.1.34. No telemetry. Windows not mentioned. — [GitHub](https://github.com/jmdarre-v/claude-multiprofile)
- Long tail of tiny Desktop OSS launchers (all <30 stars): SnlperStripes/claude-account-switcher (27★, Apache-2.0, desktop switch "without the email code"), karthiknl0/claude-account-switcher (7★, Windows), GiovanniTrevisan/claude-multi-account (6★, Windows), NeezerGu/claude-profile-switcher (6★, swaps Electron session files), dzineer/claude-profile-switcher (native macOS), DiogoDuart3/claude-multi-account (macOS menu bar), claude-desktop-clone, dual-claude-launcher, TrackMyUsage (macOS). — GitHub search via `gh search repos` 2026-10-25; [claude-desktop-clone](https://github.com/vodongha/claude-desktop-clone); [dual-claude-launcher](https://github.com/ajc3xc/dual-claude-launcher); [trackmyusage](https://github.com/frogbark/trackmyusage)
- Underlying free workaround: Claude Desktop keeps one login per data folder and is single-instance; every tool automates `--user-data-dir`. A Japanese guide documents browser/app/CLI switching on Mac. — [claudeprofiles blog](https://claudeprofiles.com/blog/multiple-claude-desktop-accounts/); [zenn.dev guide](https://zenn.dev/5alt_nacl/articles/claude-account-switching-guide-mac?locale=en)

Claude Code CLI switchers
- **realiti4/claude-swap (`cswap`)**: MIT, 4,016 stars, last push 2026-10-22 (GitHub API); ~610 commits. Switch by slot/email/alias without logout; `cswap auto` rotates accounts at a usage threshold (default 90%, weekly thresholds, per-account thresholds, consume-first strategy, hysteresis); `cswap list` shows each account's 5-hour and 7-day usage and reset times, per-model weekly windows; TUI, macOS menu bar app (5h/7d/spend), JSON output; `cswap run N` for parallel sessions per account; `cswap map` binds directories to accounts; optional shared history. Platforms macOS/Linux/Windows/WSL; Python 3.12+. Works with CLI and VS Code extension; Claude Desktop not supported. — [GitHub](https://github.com/realiti4/claude-swap); [PyPI](https://pypi.org/project/claude-swap/0.22.0/)
- **ming86/cc-account-switcher**: MIT, 452 stars, ARCHIVED, last push 2025-07-01. — GitHub API ([repo](https://github.com/ming86/cc-account-switcher))
- **Symbioose/claude-account-switcher**: 87★, macOS menu bar, switches Claude Code and Codex CLI accounts with live usage/rate-limit view, pushed 2026-10-11. Others: loekj/claude-acct-switcher (14★, for multiple Max 20x accounts), rvnikita alfred workflow (22★), ftery0/claude-account-switch (5★), guibes/claude-profile-switch (6★), ghackk/claude-multi-account (5★, shared settings + MCP sync + backup, cross-platform), CarlosTheory/claude-multi-account (2★), ramniel/claude-multi-account (1★, concurrent accounts in different terminals/VS Code windows), Savalinn/css Go "Claude Subscription Switcher" snapshotting `~/.claude/`, LeZuse cc-switch Keychain gist. — `gh search repos` 2026-10-25; [css on pkg.go.dev](https://pkg.go.dev/github.com/Savalinn/css); [LeZuse gist](https://gist.github.com/LeZuse/e92877e2e647eff253076d7aa10931a2)
- I could not locate repos named exactly "claude-profile-switch/cps", "claude-switch" or "claude-account" with meaningful traction; the name-space is fragmented with dozens of 0-30 star projects.
- **Official status**: open feature requests in anthropics/claude-code for built-in profiles (`claude profile add/switch`, #20131) and `/accounts` / `--profile` (#42775); no evidence they have shipped. — [issue 20131](https://github.com/anthropics/claude-code/issues/20131); [issue 42775](https://github.com/anthropics/claude-code/issues/42775)

Adjacent "switchers" (providers, not accounts)
- **farion1231/cc-switch**: MIT, 148,554 stars, pushed 2026-10-25 — by far the most popular desktop manager in the ecosystem. Tauri 2 + Rust + React + SQLite; Windows/macOS/Linux. Manages API providers (90+ presets, routing/failover, aggregation), unified MCP panel synced across 12 tools, skills install (skills.sh/GitHub/ZIP) and sync, prompt library, usage dashboard (tokens/requests/spend by provider and model), session reading/search, WebDAV/S3 cloud sync. Multiple Claude subscription OAuth accounts are NOT supported (Claude used via official `/login`; "Claude official subscriptions are currently unsupported in Aggregation"); multi-account beta covers Copilot/ChatGPT/xAI/Kimi. Does not manage Claude Desktop. — [GitHub](https://github.com/farion1231/cc-switch)
- Wei-Shaw/claude-relay-service (12,734★, MIT): self-hosted relay to pool/share Claude/OpenAI/Gemini subscriptions ("carpooling") — a different, ToS-riskier approach to multi-account. ThinkWatch-Lite (278★) local gateway; Folkbench (3,635★) relay benchmark/switcher. — `gh search repos` 2026-10-25; [claude-relay-service](https://github.com/Wei-Shaw/claude-relay-service)

### Inferences
- Desktop multi-account is commoditizing toward free (OSS launchers, free Multi Instance app, free tier of Claude Desktop Manager); price ceiling so far is $10-$30 one-time with tiny traction (5 Gumroad ratings). This suggests multi-account alone is not a sustainable paid wedge.
- No product spans Desktop + Code on all three OSes with first-class Linux; DeckSpace is closest (Linux beta), claude-multiprofile has no Windows, Claude Desktop Manager is Windows-only, Multi-Claude macOS-only.
- cc-switch's 148k stars show huge demand for a cross-platform "control panel" for Claude Code config (MCP/skills/prompts/providers) — but it is provider-centric, not account-centric. It is the most dangerous adjacent competitor if it adds multi-OAuth-account Claude support.
- claude-swap shows rate-limit rotation across multiple Max subscriptions is the main power-user motive; Anthropic may view auto-rotation unfavorably (ToS risk — not verified here).

### Gaps
- No sales/download numbers for any paid tool; star counts are the only traction proxy for OSS.
- Could not verify Multi Instance for Claude Desktop on a store page or find its repo.
- Did not verify Anthropic's current Consumer Terms position on multiple accounts / automated rotation (Multi-Claude vendor asserts multiple legitimate accounts are permitted — unverified).

## 2. Claude Code GUIs and session managers — do any support multiple accounts?

### Takeaway
The Claude Code GUI category is large and mostly free/OSS (happy 24k, vibe-kanban 28k, opcode 22k, CloudCLI 14k, claude-squad 8.7k, Nimbalyst 2k) and increasingly multi-agent (Codex, OpenCode, Grok), but none found treats multiple Claude accounts as a first-class concept; they assume one logged-in Claude Code. Several leaders are stalling (opcode last push Sept 2026, vibe-kanban reportedly sunsetting, Crystal deprecated).

### Cited Findings
- **opcode (winfunc/opcode, formerly Claudia, getAsterisk/claudia redirects to it)**: AGPL-3.0, 22,420 stars, last push 2026-09-18 (GitHub API). GUI/toolkit: custom agents, interactive session management, background agents. A (Nimbalyst-authored) guide says users switch away because it is not actively maintained. — [GitHub](https://github.com/winfunc/opcode); [dev.to opcode alternatives](https://dev.to/stravukarl/best-opcode-alternatives-in-2026-23oa)
- **Nimbalyst (nimbalyst/nimbalyst)**: MIT, 1,973 stars, pushed 2026-10-25. Visual workspace for Claude Code, Codex, OpenCode; parallel agents, markdown/mockup/diagram editing, task tracking; macOS/Windows/Linux + iOS/Android companion; Electron; open-sourced MIT on 2026-04-29; official successor to Crystal (Stravu). — [GitHub](https://github.com/nimbalyst/nimbalyst); [rywalker research](https://rywalker.com/research/nimbalyst); [implicator.ai](https://www.implicator.ai/nimbalyst-claude-code-codex-mac-windows-linux/)
- **Crystal (stravu/crystal)**: MIT, 3,132 stars, last push 2026-02-26; "Crystal is now Nimbalyst" (deprecated Feb 2026). — [GitHub](https://github.com/stravu/crystal)
- **CloudCLI / claudecodeui (siteboon/claudecodeui)**: AGPL-3.0, 14,281 stars, pushed 2026-10-24; web/mobile UI for Claude Code, OpenCode, Cursor CLI, Codex; remote session/project management. — [GitHub](https://github.com/siteboon/claudecodeui)
- **happy (slopus/happy)**: MIT, 24,304 stars, pushed 2026-10-25; "Orchestrate Claude, Codex and Grok in one app... reuses the subscriptions you already pay for" (mobile-first). — [GitHub](https://github.com/slopus/happy)
- **claude-squad (smtg-ai/claude-squad)**: AGPL-3.0, 8,666 stars, last push 2026-08-20; TUI managing multiple terminal agents (Claude Code, Codex, OpenCode, Amp). — [GitHub](https://github.com/smtg-ai/claude-squad)
- **vibe-kanban (BloopAI/vibe-kanban)**: Apache-2.0, 28,446 stars, last push 2026-09-19; supported 10+ agent CLIs; repo reportedly said product was sunsetting (search summary of an Aug 2026 capture). — [GitHub](https://github.com/BloopAI/vibe-kanban); [amux.io roundup](https://amux.io/guides/best-claude-code-session-managers-2026/); [mindstudio](https://www.mindstudio.ai/blog/vibe-kanban-vs-paperclip-vs-claude-code-dispatch-2)
- **Conductor (conductor.build)**: macOS only (Windows waitlist); parallel Claude Code + Codex agents in isolated git worktrees; core app free with user's own subscription; one directory lists Pro $50/mo, Teams $60/user/mo (conflicting/unverified vs. vendor). Agents don't coordinate; each draws from the same Max allocation. — [conductor.build](https://www.conductor.build); [codepick.dev](https://codepick.dev/en/guides/conductor-build-intro/); [tooldirectory.ai](https://tooldirectory.ai/tools/conductor); [morphllm](https://www.morphllm.com/conductor-ai-coding)
- **Official Claude Code features** relevant to session management: background sessions/agent view, Remote Control, cloud sessions, agent teams, and cross-session messaging (see section 4). — [code.claude.com cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging)
- Multi-account support: none of the fetched GUI READMEs/descriptions advertise multiple Claude accounts; claude-swap's `cswap run N` / `cswap map` is the only tool found that launches parallel sessions under different accounts. — [claude-swap](https://github.com/realiti4/claude-swap)

### Inferences
- The GUI category is commoditized and free; Switchyard should not compete as "yet another Claude Code GUI" but can integrate (or co-exist with) these, since they are account-agnostic.
- Maintenance decay of large OSS GUIs (opcode, vibe-kanban, Crystal) indicates the category is hard to sustain without a business model and official desktop features eroding it.

### Gaps
- Did not verify the official Claude Code desktop app's account switching behavior (search returned nothing); assume single account per OS user/config dir.
- Did not fetch each GUI README in full to confirm absence of multi-account; conclusion based on descriptions + roundups.

## 3. Usage / analytics tools — depth of analytics

### Takeaway
Token/cost analytics for Claude Code is heavily commoditized: ccusage (19k stars) covers daily/weekly/monthly/session/5-hour-block/per-project/per-model across ~18 agent CLIs; ccstatusline (13k) and many menu-bar/extension trackers cover live quota. Gaps: none found aggregate across multiple Claude accounts (except claude-swap's quota-only view), none join Claude Desktop/claude.ai chat usage with Claude Code usage, and per-tool-call cost attribution is mostly left to DIY OpenTelemetry.

### Cited Findings
- **ccusage (ccusage/ccusage, formerly ryoppippi/ccusage)**: 19,174 stars, pushed 2026-10-25; README says MIT (GitHub API reports NOASSERTION). Reports: daily, weekly, monthly, session, `blocks` (5-hour billing windows); `--instances`/`--project` per-project; per-model `--breakdown`; `--by-agent`; statusline (beta). Supports Claude Code, Codex, OpenCode, Amp, Droid, Gemini CLI, Copilot CLI, Goose, Kimi, Qwen, Grok Build and more. No documented multi-account support. — [GitHub](https://github.com/ccusage/ccusage)
- **ccstatusline (sirmalloc/ccstatusline)**: MIT, 13,404 stars, pushed 2026-10-19; customizable statusline (powerline, themes). — [GitHub](https://github.com/sirmalloc/ccstatusline)
- **claude-code-templates (davila7)**: MIT, 32,887 stars, pushed 2026-10-25; "CLI tool for configuring and monitoring Claude Code" (templates + monitoring dashboard). — [GitHub](https://github.com/davila7/claude-code-templates)
- **Claude-Code-Usage-Monitor (Maciek-roboblog)**: MIT, 8,764 stars, last push 2026-07-05; real-time usage with predictions/warnings. — [GitHub](https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor)
- **token-monitor (Javis603)**: MIT, 3,348 stars, pushed 2026-10-25; local-first desktop widget tracking tokens, costs, limits and session activity across 47+ AI coding tools. — `gh search repos` 2026-10-25 ([repo](https://github.com/Javis603/token-monitor))
- **Quosparr (Licoy)**: AGPL-3.0, 300★; menu bar/tray app (macOS/Windows/Linux) tracking Codex, Claude Code, Grok quotas and resets. — `gh search repos` ([repo](https://github.com/Licoy/Quosparr))
- **SessionWatcher (sessionwatcher.com)**: paid Mac app with iPhone/Watch/widget/Live Activity companions for Claude Pro/Max usage; Windows installer repo exists (Soren-Starck/sessionwatcher-windows, pushed 2026-10-21). — [sessionwatcher.com](https://sessionwatcher.com/claude-usage); [Windows repo](https://github.com/Soren-Starck/sessionwatcher-windows)
- Browser-extension trackers for claude.ai: lugia19 Claude Usage Tracker (free/OSS Chrome+Firefox, Firebase sync), Claude Token Tracker (Chrome), Limitra, TokenWatch (macOS menu bar; session %, burn-rate ETA, weekly model limits). — [Firefox add-on](https://addons.mozilla.org/vi/firefox/addon/claude-usage-tracker/); [Chrome: Claude Token Tracker](https://chromewebstore.google.com/detail/hijgdcmendlgjcmphabnhinkdnnockbo); [Limitra](https://chromewebstore.google.com/detail/hmmjnhldopmcepchhmlfbbkembkjmbpe); [TokenWatch PH](https://www.producthunt.com/p/tokenwatch-ai-usage-tracker/tokenwatch-2)
- "tokenkarma" and "Claude Usage Meter": no results found by name.
- **OpenTelemetry**: Claude Code exports OTel metrics; community setups exist (centminmod/claude-code-opentelemetry-setup 126★, SigNoz, Greptime/Perses dashboards, cotel); official dashboard points users to OTel for per-user token counts and cost. — `gh search repos`; [Claude Code analytics docs](https://code.claude.com/docs/en/analytics); [LangWatch usage tracking](https://langwatch.ai/blog/claude-code-usage-tracking)
- **Official Team/Enterprise analytics**: dashboard at claude.ai/analytics/claude-code — lines of code accepted, suggestion accept rate, DAU/sessions, contributor leaderboard, CSV export; Analytics API returns per-user per-day sessions, LOC added/removed, commits/PRs (read:analytics key); spend report per user and per model (daily, CSV); contribution metrics (public beta) need a GitHub app and are unavailable with ZDR. — [code.claude.com/docs/en/analytics](https://code.claude.com/docs/en/analytics); [support: Team/Enterprise analytics](https://support.claude.com/en/articles/12883420-view-usage-analytics-for-team-and-enterprise-plans); [support: Claude Code usage analytics](https://support.claude.com/en/articles/12157520-claude-code-usage-analytics)
- cc-switch also has a tokens/requests/spend dashboard by provider and model. — [GitHub](https://github.com/farion1231/cc-switch)
- Micro-projects for dashboards exist but have 0-4 stars (claudescope, cc-analytics, claumon-peek). — `gh search repos` 2026-10-25

### Inferences
- account→project→session aggregation is commodity for a single account (ccusage). Scarce: (a) cross-account rollups for individuals with multiple Pro/Max seats, (b) tool-call-level attribution (which MCP server/tool/skill/hook burned tokens), (c) "API-equivalent value vs. subscription price" per account, (d) Desktop chat usage joined with Code usage. Official analytics serve org admins on Team/Enterprise, not individuals juggling personal + client accounts.

### Gaps
- Did not confirm whether any tool computes per-tool-call token attribution from JSONL transcripts; ccusage docs don't mention it.
- SessionWatcher pricing not captured.

## 4. Config / MCP / skills managers and cross-session messaging

### Takeaway
Ecosystem content is massive and free (Superpowers ~305k, ECC ~289k, gstack ~138k, official plugin directory ~44k stars); config/MCP/skills managers exist (cc-switch, skills-manager 6.8k, claude-code-templates) but are single-account and lack a "safety preview" (permission/hook/network diff before install). Native cross-session messaging (Aug 2026) now covers same-machine bridging, which kills most third-party bridges — but it appears scoped to one OS user/config and text-only, leaving cross-account handoffs unaddressed.

### Cited Findings
Skill packs / marketplaces (GitHub API, 2026-10-25)
- obra/superpowers: 304,898★, MIT, pushed 2026-10-23. — [GitHub](https://github.com/obra/superpowers)
- affaan-m/everything-claude-code (now "ECC"): 288,565★, MIT, pushed 2026-10-20; skills/memory/security for Claude Code, Codex, OpenCode, Cursor. — [GitHub](https://github.com/affaan-m/everything-claude-code)
- garrytan/gstack: 137,647★, MIT, pushed 2026-10-25; 23 opinionated tools/roles. — [GitHub](https://github.com/garrytan/gstack)
- anthropics/claude-plugins-official: 44,486★, Apache-2.0; "Official, Anthropic-managed directory of high quality Claude Code Plugins." — [GitHub](https://github.com/anthropics/claude-plugins-official)
- Awesome lists: hesreallyhim/awesome-claude-code 56,498★; rohitg00/awesome-claude-code-toolkit 2,749★ (last push 2026-05-12); jqueryscript/awesome-claude-code 536★. — [hesreallyhim](https://github.com/hesreallyhim/awesome-claude-code); [rohitg00](https://github.com/rohitg00/awesome-claude-code-toolkit); [jqueryscript](https://github.com/jqueryscript/awesome-claude-code)

Config / MCP / skills managers
- cc-switch: unified MCP panel, skills install/sync, prompts, cloud sync across 12 tools (see §1). — [GitHub](https://github.com/farion1231/cc-switch)
- xingkongliang/skills-manager: 6,822★, MIT, desktop app to manage/sync skills across 50+ coding tools. — `gh search repos` ([repo](https://github.com/xingkongliang/skills-manager))
- Claude Desktop Manager: per-profile MCP config with validated JSON editor; seed profiles. — [claudeprofiles.com](https://claudeprofiles.com/)
- claude-multiprofile: copies Desktop extensions/MCP connectors between profiles. — [GitHub](https://github.com/jmdarre-v/claude-multiprofile)
- "claude mcp manager" repos are tiny (≤6★, mostly 2025, shell/TUI). — `gh search repos` 2026-10-25 (e.g., [zongmin-yu/claude-mcp-manager](https://github.com/zongmin-yu/claude-mcp-manager), [DonMecca/claudecode-mcp-manager](https://github.com/DonMecca/claudecode-mcp-manager))
- No standalone "claude-sync" config-sync project of note found; cc-switch (WebDAV/S3) and ghackk/claude-multi-account (MCP sync, backup/restore) cover it. — [cc-switch](https://github.com/farion1231/cc-switch); [ghackk](https://github.com/ghackk/claude-multi-account)

Cross-session messaging
- **Native (official)**: Claude Code v2.1.224+ (macOS/Linux/WSL2), v2.1.234+ native Windows; `ListAgents` + `SendMessage`, `/list-agents` (`/peers`), `@session` mentions (v2.1.232+), `notify_when_idle` (v2.1.236+). Reaches subagents, teammates, other local sessions (per-session Unix socket / named pipe restricted to the OS user, never via Anthropic servers), and cloud/other-machine sessions via Remote Control (through Anthropic servers; requires claude.ai sign-in). Plain text only (~1M char cap), not history or files. Controls: `crossSessionInbound` accept/hold/refuse, `isolatePeerMachines`, `dialogExpiry`; incoming messages cannot approve prompts or change config. Exposes `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN` so hooks/scripts can post into a session. Container↔host and WSL↔native Windows cannot reach each other. Desktop app/VS Code sessions can't show the hold dialog. — [code.claude.com cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging)
- Release date reported as Aug 6 or Aug 7, 2026 by secondary sources. — [letsdatascience](https://letsdatascience.com/news/claude-code-adds-cross-session-messaging-7cf1c3a5); [pasqualepillitteri.it](https://pasqualepillitteri.it/en/news/10215/claude-code-cross-session-messaging); [claudelog](https://claudelog.com/faqs/what-is-cross-session-sendmessage-in-claude-code/)
- Third-party bridges are tiny and being obsoleted: non4me/cc2cc (14★, file-based JSON messaging + MCP), paulrobello/cc2cc (4★, LAN hub), maqitong/hist-bridge (2★, cross-tool session bridge Claude Code/Codex/Cursor), nnemirovsky/peerbus (1★, agent-agnostic durable bus); claude-intercom npm (Channels API) notes native messaging now exists. "claude-relay" mostly returns subscription-relay services, not session bridges. No "session-bridge" repo of note found. — `gh search repos` 2026-10-25; [claude-intercom on socket.dev](https://socket.dev/npm/package/claude-intercom/overview/2.0.0)

### Inferences
- Same-account, same-machine session bridging is now a platform feature — Switchyard should not build its own transport for that, but can ride on it (the documented socket + token lets an external app inject messages).
- Cross-ACCOUNT bridging remains open: the docs speak of "your" sessions and per-OS-user sockets; whether sessions under different `CLAUDE_CONFIG_DIR`/accounts discover each other is undocumented. Cross-account remote (Remote Control) is tied to one claude.ai sign-in, so cross-account cloud handoffs are definitely not native. Rich handoffs (context packs with files/diffs/summaries, not just text) are also unaddressed natively ("to move a whole conversation, resume the session").
- Safety preview for installing skills/plugins/MCP (diff of hooks, permissions, network/env access) is absent from all managers found — notable given ecosystem packs with 100k-300k stars that install hooks and commands.

### Gaps
- Not verified whether native cross-session messaging works across different CLAUDE_CONFIG_DIR profiles on the same OS user (needs hands-on test).
- Did not review the official plugin marketplace install UX for any built-in permission/safety preview.

## 5. Analogous multi-account managers in other ecosystems

### Takeaway
Analogues (Rambox, Shift, Wavebox, Ferdium, Station, Franz) show that "many accounts of web apps in one window" supports small subscription businesses ($5-$17/mo) but repeatedly gets commoditized by free OSS forks (Franz→Ferdi→Ferdium) and by platform-native features (Chrome profiles, Firefox containers, Slack multi-workspace). Evidence collected here is thin and pricing sources conflict.

### Cited Findings
- Rambox: Pro ~$5/user and Enterprise ~$10/user (TrustRadius); Capterra lists from US$5.83 with free Basic plan — sources conflict. — [TrustRadius Rambox](https://www.trustradius.com/products/rambox/pricing); [Capterra Rambox](https://www.capterra.ca/reviews/1012244/rambox); [G2 Rambox](https://g2.com/products/rambox/pricing)
- Shift: free tier limited to 2 email accounts + 1 app; Advanced $199/yr (TrustRadius). — [TrustRadius Shift vs Wavebox](https://www.trustradius.com/compare-products/shift-desktop-app-vs-wavebox-pro); [G2 Station/Shift](https://www.g2.com/products/station/pricing)
- Wavebox Pro ~$8.32/mo billed annually; multi-account via sandboxes/containers/cookie profiles. — [TrustRadius](https://www.trustradius.com/compare-products/shift-desktop-app-vs-wavebox-pro)
- Ferdium is free/OSS; Ferdi (predecessor fork of Franz) discontinued; Station described as open-source smart browser. — [AlternativeTo web app browsers](https://alternativeto.net/lists/12084/web-app-browsers); [SaaSHub](https://www.saashub.com/best-customisable-web-browser-software/c/digital-workspace-organizer)

### Inferences
- Pricing analogues suggest a $5-$10/mo or $30-$80 one-time band is plausible for prosumers, but the pattern of OSS forks and native features eroding paid multi-account apps is strong; Switchyard's defensibility must come from analytics/bridging/ecosystem safety, not account isolation.

### Gaps
- Did not verify Station's shutdown history (Station was reportedly discontinued ~2021-2022 in my training memory, but no source found in this session), Franz pricing, or revenue/user figures for any analogue.
- Chrome profiles / Firefox Multi-Account Containers / Slack multi-workspace not researched in this session (free, native — treat as known context, unsourced here).

## 6. Synthesis — commoditized vs. scarce; white space

### Takeaway
Commoditized (free, high-star OSS or official): single-account Claude Code GUIs, token/cost reports for one account, live quota meters, statuslines, skill/plugin content, MCP/skills sync across tools (cc-switch), same-account cross-session messaging (native), basic Desktop multi-instance launching. Scarce/white space: one cross-platform app that unifies multiple isolated Claude Desktop + Claude Code accounts with (1) cross-account analytics down to tool-call level, (2) cross-account handoffs with rich context, (3) per-account settings/permissions/hooks editing with diff, and (4) safety-previewed ecosystem installs.

### Cited Findings (feature → who covers it)
- Desktop multi-instance: Claude Desktop Manager (Win, $10/$30), Multi-Claude (Mac, $20), DeckSpace Claude Accounts (Mac/Win/Linux-beta, $14), claude-multiprofile (Mac, MIT, free), free Multi Instance app (Win). — [claudeprofiles](https://claudeprofiles.com/); [multiclaude](https://multiclaude.app/); [deckspace](https://deckspace.dev/claude-accounts/); [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- Claude Code multi-account + quota rotation: claude-swap (4.0k★, free). — [GitHub](https://github.com/realiti4/claude-swap)
- Config/MCP/skills/provider control panel: cc-switch (148.6k★, free, cross-platform, no multi-Claude-account). — [GitHub](https://github.com/farion1231/cc-switch)
- Usage analytics: ccusage (19.2k★), token-monitor (3.3k★), Usage Monitor (8.8k★), official org analytics (Team/Enterprise). — [ccusage](https://github.com/ccusage/ccusage); [docs](https://code.claude.com/docs/en/analytics)
- Session bridging: native ListAgents/SendMessage. — [docs](https://code.claude.com/docs/en/cross-session-messaging)

### Inferences (white space)
1. **Unified multi-account across Desktop + Code on Linux/Windows/macOS** — no single product; DeckSpace closest but Linux beta and no analytics/config depth.
2. **Cross-account analytics rollup** (account→project→session→tool call, plus subscription value/ROI per account and 5h/7d quota forecasting across accounts) — ccusage is single-account; claude-swap is quota-only; official analytics are org-admin only.
3. **Cross-account handoffs** — native messaging is per-user/per-sign-in and text-only; a broker that packages context (summary + files + git diff) from account A to a session in account B is unoccupied.
4. **Per-account Claude settings GUI** (model, permission rules, hooks, env, plugins) with diffs/templates/seeding — partially in Claude Desktop Manager (MCP only) and claude-multiprofile (copy extensions); no full settings editor per account found.
5. **Ecosystem manager with safety preview** — cc-switch and skills-manager install/sync but no evidence of permission/hook/network-risk preview; given 100k-300k-star packs that install hooks, this is a differentiator.
6. **Risks**: Anthropic could ship native profiles (open issues #20131/#42775); cc-switch could add multi-OAuth Claude accounts; paid tools in this space show very weak traction (5 Gumroad ratings), suggesting willingness-to-pay is low for isolation alone.

### Gaps
- No market-size or revenue data for any competitor.
- ToS implications of multi-account and auto-rotation not verified.
- Hands-on validation needed for cross-config-dir native messaging and for whether official Claude Code desktop supports account switching.
