# Demand and User Segments for a Claude Multi-Account Manager ("Switchyard")

Research date: 2026-10-25. Every GitHub count below was pulled live through the GitHub API on 2026-10-25. Hacker News figures come from the hn.algolia.com API on the same day. "r" means total reactions and "c" means comments.

## 1. What demand signals exist?

### Takeaway
Demand is real, visible, and still unmet for the **Desktop app** (and mobile). The main Desktop issue (#18435) has about 1,186 reactions and 217 comments, with no Anthropic staff reply in 9 months. Its mobile counterpart has 1,067 reactions. For the **CLI**, Anthropic's official position (Aug 2026) is that `CLAUDE_CONFIG_DIR` already solves the problem, and it closed the CLI profile requests as "completed". The result is a crowded field of small free CLI switchers, plus a thin, fragmented set of Desktop side-by-side tools (most with fewer than 50 stars, mostly macOS-only).

### Cited Findings
**GitHub, anthropics/claude-code (figures as of 2026-10-25)**
- #18435 "Add the ability to manage multiple Claude accounts within the Claude Desktop app": opened 2026-01-15, still **open**. It has **1,186 reactions (951 thumbs-up) and 217 comments**, and the latest comment is dated 2026-10-24. Labels: enhancement, area:auth, area:ide, area:desktop. **No comments from Anthropic members or collaborators** among the 217. The body asks for separate history, settings, and MCP configs per account and a visible indicator of the active account, "similar to how Slack handles workspace switching or how Chrome manages multiple user profiles". — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)
- #36151 "Multi-account switching in Claude Mobile app without shared email": opened 2026-03-19, open, **1,067 reactions and 188 comments**, no staff reply. The top comment (66 reactions) gives the use case: Claude Code runs on a Team plan on the work machine while the personal Pro plan lives on iOS. Remote Control/Dispatch makes switching necessary. — [GitHub #36151](https://github.com/anthropics/claude-code/issues/36151)
- #27302 "Support multiple Connector accounts (same connector, different accounts) in Claude and Claude Code": opened 2026-02-21, open, **617 reactions and 282 comments**. This is an adjacent identity-separation pain. — [GitHub #27302](https://github.com/anthropics/claude-code/issues/27302)
- #27359 "Named account profiles for quick switching": opened 2026-02-21, 39 reactions, 9 comments. **Closed "completed" on 2026-08-17** by collaborator bcherny (Boris Cherny, Claude Code lead). His answer: "You can do this today with a separate config directory per account… `alias claude-work='CLAUDE_CONFIG_DIR=~/.claude-work claude'`". A later user (2026-09-08) reports that in Desktop, Code-tab history for a Team seat and a personal plan on the same email gets mixed up. That user built a macOS tool to fix it. — [GitHub #27359](https://github.com/anthropics/claude-code/issues/27359)
- #25762 "Add environment variable to configure .claude config directory location": opened 2026-02-14, 43 reactions, 13 comments. Closed "completed" on 2026-08-17 by bcherny with the note "This exists: set `CLAUDE_CONFIG_DIR`… on macOS credentials stay in the Keychain." Note: this reply is about the CLI. The Desktop app honoring CLAUDE_CONFIG_DIR was not addressed. — [GitHub #25762](https://github.com/anthropics/claude-code/issues/25762)
- #36693 "No native way to sync Claude Code config and skills across machines": opened 2026-03-20, **2 reactions, 3 comments**. Auto-closed as stale/"not_planned" on 2026-04-30. This is a weak signal. — [GitHub #36693](https://github.com/anthropics/claude-code/issues/36693)
- Other multi-account issues:
  - #20131 "Multi-Account Profile Support": 115 r / 44 c, closed completed. [link](https://github.com/anthropics/claude-code/issues/20131)
  - #30031 "multiple account login and switching (like `gh auth switch`)": 94 r / 18 c, open. [link](https://github.com/anthropics/claude-code/issues/30031)
  - #55621 "Multi-account support in VSCode extension": 82 r / 17 c, open. [link](https://github.com/anthropics/claude-code/issues/55621)
  - #261 (Mar 2025, earliest): 40 r. [link](https://github.com/anthropics/claude-code/issues/261)
  - #3835 "switch from Max to API and back w/o logout": 19 r. [link](https://github.com/anthropics/claude-code/issues/3835)
  - Many Desktop duplicates were closed as duplicate or not_planned: #30565 (Mar 2026), #36821 (Mar 2026), #60607 (May 2026), #72967 (Jul 2026). — [#30565](https://github.com/anthropics/claude-code/issues/30565), [#36821](https://github.com/anthropics/claude-code/issues/36821), [#60607](https://github.com/anthropics/claude-code/issues/60607), [#72967](https://github.com/anthropics/claude-code/issues/72967)
- Title search counts (GitHub search API, 2026-10-25): 38 issues with "multi-account" in the title, 29 with "multiple accounts", and 103 with "switch" plus "account". — [GitHub search](https://github.com/anthropics/claude-code/issues?q=is%3Aissue+in%3Atitle+multi-account)
- For scale against the repo's top issues: #18435 (1,186 r) is the top dedicated feature request in the multi-account area. It sits well below the biggest repo issues: AGENTS.md support has 6,688 r and "Claude Code is unusable… Feb updates" has 3,286 r. It is comparable to "Support for OpenCode and Max plan" at 1,412 r. — [#6235](https://github.com/anthropics/claude-code/issues/6235), [#42796](https://github.com/anthropics/claude-code/issues/42796), [#17118](https://github.com/anthropics/claude-code/issues/17118)

**Community tools (GitHub stars as of 2026-10-25) as revealed demand**
- About 20 distinct third-party tools are linked inside #18435's comments. Examples: odahcam/claude-desktop-profiles, thomasog/multi-claude, synth1s/cloak, vitaliyhayda/claude-transplant, synthetixis/lanes, erorplex/claude-desktop-profiles-macos, GiovanniTrevisan/claude-multi-account (Windows), reddeer-tech/claude-account, yarikleto/claude-profile, y0av/giverny. — [GitHub #18435 comments](https://github.com/anthropics/claude-code/issues/18435)
- **CLI switchers with rate-limit rotation lead on stars:**
  - realiti4/claude-swap: **4,016 stars** (created 2026-01-11), described as "Switch between multiple Claude Code accounts, with automatic rate-limit rotation". — [GitHub](https://github.com/realiti4/claude-swap)
  - kaitranntt/ccs: 2,892 stars, switches between Claude accounts, Gemini, Copilot, and OpenRouter. — [GitHub](https://github.com/kaitranntt/ccs)
  - edison7009/EchoBird: 3,438 stars, "Manage ChatGPT, Codex CLI, and Claude Code accounts and track quota in one place". — [GitHub](https://github.com/edison7009/EchoBird)
  - KarpelesLab/teamclaude: 446 stars, a "multi-account proxy… pools Claude Max". — [GitHub](https://github.com/KarpelesLab/teamclaude)
  - ming86/cc-account-switcher: 452 stars. [GitHub](https://github.com/ming86/cc-account-switcher)
  - uwuclxdy/clauth: 258 stars, a multi-account manager plus usage monitor. [GitHub](https://github.com/uwuclxdy/clauth)
  - Dicklesworthstone/coding_agent_account_manager: 210 stars. [GitHub](https://github.com/Dicklesworthstone/coding_agent_account_manager)
  - XueshiQiao/CCSwitcher: 197 stars. [GitHub](https://github.com/XueshiQiao/CCSwitcher)
  - burakdede/aisw: 135 stars. [GitHub](https://github.com/burakdede/aisw)
  - Licoy/Quosparr: 300 stars, a cross-platform tray app tracking Codex and Claude Code quotas. [GitHub](https://github.com/Licoy/Quosparr)
- **Desktop side-by-side tools (closest to Switchyard) are small:**
  - jmdarre-v/claude-multiprofile: 133 stars, "Run multiple Claude Desktop and Claude Code accounts side by side on macOS". — [GitHub](https://github.com/jmdarre-v/claude-multiprofile)
  - vitaliyhayda/claude-transplant: 48 stars. Fixes the empty Code sidebar after switching accounts. [GitHub](https://github.com/vitaliyhayda/claude-transplant)
  - synth1s/cloak: 42 stars. [GitHub](https://github.com/synth1s/cloak)
  - yarikleto/claude-profile: 12. [GitHub](https://github.com/yarikleto/claude-profile)
  - synthetixis/lanes: 12. [GitHub](https://github.com/synthetixis/lanes)
  - thomasog/multi-claude: 11. [GitHub](https://github.com/thomasog/multi-claude)
  - odahcam/claude-desktop-profiles: 9. [GitHub](https://github.com/odahcam/claude-desktop-profiles)
  - GiovanniTrevisan/claude-multi-account: 6, Windows. [GitHub](https://github.com/GiovanniTrevisan/claude-multi-account)
  - erorplex/claude-desktop-profiles-macos: 5. [GitHub](https://github.com/erorplex/claude-desktop-profiles-macos)
  - brunorodmoreira/claude-desktop-multi-account: 2. [GitHub](https://github.com/brunorodmoreira/claude-desktop-multi-account)
- Commercial "Multi-Claude" macOS app: launched on Product Hunt 2026-05-20 with 83 upvotes and 11 comments, #19 that day (per the search snippet; not independently verified). — [hunted.space](https://hunted.space/product/multi-claude); [toolradar](https://toolradar.com/tools/multi-claude)
- AgentsRoom sells "Claude Multi-Account" as a feature: one account pinned per project. — [agentsroom.dev](https://agentsroom.dev/features/claude-multi-account)
- Adjacent tools with very large star counts:
  - manaflow-ai/cmux: 28,438 stars, a terminal for AI coding agents, created 2026-01-28. [GitHub](https://github.com/manaflow-ai/cmux)
  - steipete/CodexBar: 22,793 stars, Claude Code/Codex usage stats. [GitHub](https://github.com/steipete/CodexBar)
  - ccusage: 19,174 stars. [GitHub](https://github.com/ccusage/ccusage)
  - Claude-Code-Usage-Monitor: 8,764 stars. [GitHub](https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor)
  - claude-squad: 8,666 stars. [GitHub](https://github.com/smtg-ai/claude-squad)

**Hacker News (via hn.algolia.com)**
- "Show HN: Claude-account – switch Claude Code accounts without logging in again" (2026-07-30): **54 points, 31 comments**. Motive stated by the author: separate work and personal accounts. Several commenters said they already handle it with 2-line `CLAUDE_CONFIG_DIR` aliases or mise per-directory env. Repeated ban worries came up ("don't people get banned for doing this kind of thing?"). One user runs two Claude Max accounts through two Desktop launchers with `--user-data-dir`. — [HN 49111019](https://news.ycombinator.com/item?id=49111019)
- Many low-traction Show HNs, each 1–5 points with 0–3 comments:
  - Claude-multiprofile (2026-04-28). [HN](https://news.ycombinator.com/item?id=47942071)
  - "Run multiple Claude desktop accounts in isolated windows (Mac/Windows)" (2026-08-02, deckspace.dev). [HN](https://news.ycombinator.com/item?id=49141736)
  - Clausona (2026-03-09). [HN](https://news.ycombinator.com/item?id=47308676)
  - claude-env (2026-02-25). [HN](https://news.ycombinator.com/item?id=47148222)
  - CCS (2025-12-17). [HN](https://news.ycombinator.com/item?id=46306444)
  - Claude-Nonstop, auto account switching (2026-02-20). [HN](https://news.ycombinator.com/item?id=47082232)
  - AgentCodeGUI multi-account (2026-08-03). [HN](https://news.ycombinator.com/item?id=49154498)
  - Hatswap, "one Claude Code account per client" (2026-10-13). [HN](https://news.ycombinator.com/item?id=50075273)
  - Teamclaude Rust rewrite (2026-07-18). [HN](https://news.ycombinator.com/item?id=48955755)
- The highest-engagement HN thread in the area is about limits rather than switching: "Claude Code weekly rate limits" (2025-07-28) with **609 points and 705 comments**. Older data. — [HN 44713757](https://news.ycombinator.com/item?id=44713757)

**Blogs and guides**
- Multiple how-to guides exist for running two accounts. — [Codeminer42 "AI TIP: Running Two Claude Code Accounts"](https://blog.codeminer42.com/ai-tip-running-two-claude-code-accounts/); [madewithlove "Running multiple Claude accounts without logging out"](https://madewithlove.com/blog/running-multiple-claude-accounts-without-logging-out.md); [Medium "Running Two Claude Code Subscriptions Side by Side"](https://medium.com/front-end-weekly/running-two-claude-code-subscriptions-side-by-side-65d1f10a9dd4)
- Reddit (r/ClaudeAI, April 2026, via a mirror): Claude Code hangs on a personal plan when a work account is also signed in on the same machine. "Claude code gets confused when two accounts are authenticated." — [reddit snapshot mirror](https://reddit.sentinel-team.org/posts/1sdvzuo/snapshots/2026-04-09T16%3A41%3A00.47446Z)

### Inferences
- Combined reactions on the Desktop and mobile multi-account issues (about 2,250) plus the connector multi-account issue (617) make this one of the larger unmet feature clusters in the repo. Reactions are a lower bound on affected users: most affected users never visit GitHub.
- Anthropic's Aug 2026 "CLAUDE_CONFIG_DIR solves it" closures mean the CLI side is officially "solved". Because the workaround is so simple, CLI-only switching is a commodity. Many free tools exist and HN commenters dismiss it as two lines of shell. The defensible gap is **Desktop** (Chat/Cowork/Code tabs), where there is no official path, history gets mixed up, and existing tools are tiny and mostly macOS-only.
- The long silence on #18435 (9 months, no staff reply) suggests Anthropic may not ship native Desktop multi-account soon. It also leaves Switchyard exposed: if Anthropic ships a Slack-style switcher, the core value disappears.

### Gaps
- Could not access Reddit directly. No upvote or comment counts for r/ClaudeAI or r/ClaudeCode threads on multi-account were found.
- No X/Twitter post metrics were collected.
- Did not verify the Multi-Claude Product Hunt numbers at the primary source, or any paid-tool revenue.

## 2. Why do people need multiple accounts? Legitimate separation vs limit-evasion

### Takeaway
On the Desktop and mobile issues, the stated motive is overwhelmingly **work (employer Team/Enterprise seat) plus personal plan separation**, followed by consultants and freelancers with client accounts and privacy/compliance separation. **Limit-evasion** (stacking several Max subscriptions and rotating when the limit hits) is a minority motive in issue comments. It does, however, drive the **most-starred CLI tools** (claude-swap at 4k stars, ccs, teamclaude pooling), so it is a large hidden segment carrying ToS and ban risk.

### Cited Findings
- Keyword coding of all 217 comments on #18435, done by me with a regex pass (an approximation, not manual coding). Mentions:
  - work/team/enterprise/company: **110**
  - "personal": **81**
  - privacy/separation/compliance/data: **70**
  - client/freelance/agency/consulting: **23**
  - limit/usage/quota/rate: **18**
  - "two/multiple Max/Pro subscriptions": **6**
  — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)
- The same pass on #36151 (188 comments): work 53, personal 50, privacy 11, client 3, limits 3. — [GitHub #36151](https://github.com/anthropics/claude-code/issues/36151)
- Representative highly-reacted comments on #18435:
  - An IT administrator with a corporate-managed account plus a personal Max plan on different emails (35 reactions).
  - An org IT worker who also runs a consulting business (40 reactions).
  - A user with "5 separate API workspaces (work org + personal + consulting)… separate cost tracking", calling it "a significant adoption blocker" (49 reactions).
  - A user running parallel projects across two GitHub accounts, who notes Connectors allow only one GitHub account (34 reactions).
  — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)
- From #36151: "Lack of this feature currently really lowers the value proposition of the company account for me" (12 reactions). Another user asks for a different background colour per account "to prevent accidentally prompting the business account for personal". — [GitHub #36151](https://github.com/anthropics/claude-code/issues/36151)
- A March 2026 Desktop request says one paid plan sits idle because switching is painful. — [GitHub #30565 / claudeissues mirror](https://claudeissues.com/issue/30565-feature-multi-account-support-in-claude-desktop-app-with-account-switching)
- Limit-evasion tools in the space:
  - claude-swap, "automatic rate-limit rotation", 4,016 stars. — [GitHub](https://github.com/realiti4/claude-swap)
  - teamclaude, "pools Claude Max", 446 stars. — [GitHub](https://github.com/KarpelesLab/teamclaude)
  - Claude-Nonstop, auto account switching. — [HN](https://news.ycombinator.com/item?id=47082232)
  - On HN, a commenter promotes omp, which "uses the next account" when a limit is reached. — [HN 49111019](https://news.ycombinator.com/item?id=49111019)
  - The maxpool proxy README warns that multi-account proxying is in a contested gray area of Anthropic's terms and may put your Claude accounts at risk. — [maxpool README](https://cdn.jsdelivr.net/npm/maxpool@1.11.0/README.md)
- ToS and ban context:
  - Anthropic (2025-07-28): "a small number of users are violating our usage policies by sharing and reselling accounts". It introduced weekly limits in response. — [X @AnthropicAI](https://x.com/AnthropicAI/status/1949898513019466140)
  - Reports of bans on users holding multiple Max subscriptions began in mid-Feb 2026. A Claude Code team member, Thariq Shihipar, is quoted: "It's not against terms of service to have multiple MAX accounts" (secondary source; no direct link given). — [MetricNexus](https://metricnexus.ai/blog/anthropic-banning-multiple-claude-accounts)
  - HN commenters express fear of bans and a "draconian" appeals process even for work/personal separation. — [HN 49111019](https://news.ycombinator.com/item?id=49111019)
  - HN thread "Claude Account Suspended Seconds After Purchase?" (2026-05-14), 70 points, 45 comments. — [HN 48134808](https://news.ycombinator.com/item?id=48134808)
- Data-use separation motive: the HN thread "If you have a Claude account, they're going to train on your data moving forward" (2025-08-29) drew 442 points and 229 comments. Older data. It is relevant because consumer accounts and commercial Team/Enterprise seats have different data terms. — [HN 45062738](https://news.ycombinator.com/item?id=45062738)

### Inferences
- In public feature-request discourse, **work+personal is the dominant need**, by roughly 5–6x over limit motives in the keyword counts. Freelancers, consultants and agencies with client accounts are a meaningful second group (about 10% of #18435 comments).
- Limit-rotation demand is real and possibly larger in raw users, judging by stars. But it is the segment most likely to draw Anthropic enforcement. Positioning Switchyard around rotation or pooling would add ban and ToS risk for its users and probably platform hostility. The safer positioning is "separation and control", not "more quota".
- Accidentally prompting the wrong account (data leaking across work and personal) is a concrete pain that clear visual separation (colours and icons per account) addresses.

### Gaps
- No survey data on how many Claude users hold two or more accounts.
- No official Anthropic ToS clause found that explicitly addresses one person holding several accounts. The official statements found are secondary.

## 3. How large is the potential user base?

### Takeaway
Anthropic is very large and growing fast: about $65B annualized run-rate by end of July 2026 (reported), 300k+ business customers, and Claude Code above $2.5B run-rate in Feb 2026. There is **no official count of Claude Code developers, consumer subscribers, or Max/Team seats**, so sizing the multi-account segment must be estimated bottom-up.

### Cited Findings
- Run-rate timeline:
  - ~$9B at end of 2025
  - $14B in Feb 2026
  - $19B in Mar 2026
  - $30B in Apr 2026
  - Dario Amodei cited about 80x annualized growth in Q1 2026 at Code with Claude (May 2026)
  - OpenAI has argued the $30B figure is overstated by about $8B
  — [VentureBeat](https://venturebeat.com/technology/anthropic-says-it-hit-a-30-billion-revenue-run-rate-after-crazy-80x-growth)
- Run rate of **about $65B by end of July 2026**, shared with investors and reported mid-Aug 2026 by Bloomberg and Reuters (unofficial). $47B was reported in May. Valuation was $965B after a $65B Series H in May 2026, with a confidential IPO filing. The FT reports investors expect $100–120B of 2026 revenue. — [Bloomberg Gov](https://news.bgov.com/financial-accounting/anthropic-revenue-run-rate-surpasses-65-billion-ahead-of-ipo); [AI Magazine](https://aimagazine.com/news/how-anthropics-revenue-run-rate-topped-us-65bn)
- Claude Code as of Feb 12, 2026:
  - run-rate above **$2.5B**, doubled since Jan 1
  - weekly active users **doubled** since Jan 1 (no absolute number)
  - business subscriptions **quadrupled** since the start of 2026
  - 500+ customers spending over $1M a year
  — [Constellation Research](https://www.constellationr.com/insights/news/anthropics-claude-code-revenue-doubled-jan-1)
- By about April–May 2026, 1,000+ enterprise customers were spending over $1M a year, doubled since Feb. An average developer uses Claude Code about 20 hours a week (undated). — [VentureBeat](https://venturebeat.com/technology/anthropic-says-it-hit-a-30-billion-revenue-run-rate-after-crazy-80x-growth)
- Anthropic says it serves more than 300,000 business customers (per secondary aggregators; date unclear, likely late 2025). — [getpanto](https://www.getpanto.ai/blog/anthropic-ai-statistics)
- Consumer MAU estimates vary widely, from about 19M (third-party, around Feb 2026) to about 245M (attributed to Sensor Tower; uncorroborated). There is no official figure. — [getpanto](https://www.getpanto.ai/blog/claude-ai-statistics); [Skillademia](https://skillademia.com/statistics/claude-statistics/)
- Claude Desktop for Linux launched in **beta about 2026-06-30**. It supports Ubuntu 22.04 and Debian 12 (x86_64/arm64) through an apt repo, includes Chat/Cowork/Code tabs, and works on paid plans. No adoption figures are published. — [IT Brief UK](https://itbrief.co.uk/story/anthropic-launches-claude-desktop-beta-for-linux-users); [pasqualepillitteri.it](https://pasqualepillitteri.it/en/news/6967/claude-desktop-linux-beta-ubuntu-debian-en)
- Proxy for the developer-tools market: ccusage, a usage CLI for Claude Code, has 19,174 GitHub stars, and CodexBar has 22,793. — [ccusage](https://github.com/ccusage/ccusage); [CodexBar](https://github.com/steipete/CodexBar)

### Inferences
- If Claude Code developers number in the low millions (unverified), and Team/Enterprise seat growth (business subscriptions up 4x in about 6 weeks of 2026) keeps adding employees who also hold personal Pro or Max plans, then the work+personal dual-account population plausibly reaches hundreds of thousands. This is speculative and should be framed as an assumption, not a finding.
- About 2,250 reactions on two issues, tool stars in the low thousands, and roughly 20 indie tools are consistent with tens of thousands of actively frustrated power users visible online.

### Gaps
- No official Claude Code user count, Max subscriber count, Team/Enterprise seat count, or Linux beta install numbers were found.
- The 300k business-customer figure is from an aggregator, not verified at an Anthropic primary source.

## 4. Adjacent pains with demand signals

### Takeaway
Usage and limit visibility is the strongest adjacent pain, with very high engagement and five-figure-star tools. Session and agent orchestration is also huge (cmux has 28k stars). MCP, skills and config sync have moderate signals: the Skills-sync issue has 171 reactions, but cross-machine config sync is weak (2 reactions). I found little demand evidence for cross-session or cross-account messaging.

### Cited Findings
- Usage and limits:
  - #16157 "Instantly hitting usage limits with Max subscription": **726 reactions, 1,499 comments**, open since 2026-01-03. — [GitHub #16157](https://github.com/anthropics/claude-code/issues/16157)
  - #38335 "Max plan session limits exhausted abnormally fast since March 23, 2026": 544 r, 879 c. — [GitHub #38335](https://github.com/anthropics/claude-code/issues/38335)
  - #20636 "Expose rate limit usage (session %, weekly %) to statusLine": 59 r, closed Jan 2026. — [GitHub #20636](https://github.com/anthropics/claude-code/issues/20636)
  - #76272 "control over weekly limit reset timing": 37 r, open. — [GitHub #76272](https://github.com/anthropics/claude-code/issues/76272)
  - Tools: ccusage at 19.2k stars, CodexBar at 22.8k, Claude-Code-Usage-Monitor at 8.8k, Quosparr at 300. Newer entrants include "Peras", which pauses agents before they burn the weekly limit (HN, 2026-10-21), and "Limits", an iOS usage tracker (HN, 2026-07-15). — [ccusage](https://github.com/ccusage/ccusage), [CodexBar](https://github.com/steipete/CodexBar), [Usage-Monitor](https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor), [HN Peras](https://news.ycombinator.com/item?id=50180215), [HN Limits](https://news.ycombinator.com/item?id=48925453)
- Skills and config sync:
  - #20697 "Sync Skills between Claude Desktop and Claude Code CLI": 171 r, 51 c, open. — [GitHub #20697](https://github.com/anthropics/claude-code/issues/20697)
  - #36693 cross-machine config/skills sync: 2 r, auto-closed. — [GitHub #36693](https://github.com/anthropics/claude-code/issues/36693)
  - #63242 "Cloud-synced memory and settings across machines": 0 r. — [GitHub #63242](https://github.com/anthropics/claude-code/issues/63242)
- MCP config management:
  - #26073 Windows MSIX "Edit Config" opens the wrong claude_desktop_config.json: 33 r. — [GitHub #26073](https://github.com/anthropics/claude-code/issues/26073)
  - #24057 auto-reload MCP/hooks/plugins on config change: 21 r. — [GitHub #24057](https://github.com/anthropics/claude-code/issues/24057)
  - #23669 per-teammate MCP configs: 32 r. — [GitHub #23669](https://github.com/anthropics/claude-code/issues/23669)
  - #27302 multiple connector accounts: 617 r. — [GitHub #27302](https://github.com/anthropics/claude-code/issues/27302)
- Session management:
  - cmux (agent terminal): 28.4k stars. [GitHub](https://github.com/manaflow-ai/cmux)
  - claude-squad: 8.7k stars. [GitHub](https://github.com/smtg-ai/claude-squad)
  - #31992 cross-machine session resume: 21 r. [GitHub #31992](https://github.com/anthropics/claude-code/issues/31992)
  - #16901 list/delete session commands: 59 r. [GitHub #16901](https://github.com/anthropics/claude-code/issues/16901)
  - Account switching destroys visible Desktop Code-tab history, which motivated claude-transplant (48 stars). [GitHub](https://github.com/vitaliyhayda/claude-transplant)
  - HN: "Anyone switch accounts for Claude Code, did you lose everything?" [HN](https://news.ycombinator.com/item?id=47636410)
- Per-account cost tracking is explicitly asked for in #18435: "separate cost tracking". — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)

### Inferences
- Combining multi-account with **per-account usage and limit dashboards** pairs the top unmet Desktop request with the most proven adjacent category (tools with 20k+ stars). Usage analytics is crowded and free, though, so it works as a retention feature rather than a wedge.
- "Connect sessions across accounts" and cross-session messaging lack direct demand evidence. Treat them as speculative differentiators.

### Gaps
- GitHub issue searches with OR terms are fuzzy, so search "total_count" values are not reliable measures and were not used.
- I found no specific demand signal for cross-account session messaging.

## 5. Which segment is most willing to pay and most underserved?

### Takeaway
The most underserved segment is **Desktop-app users with an employer Team/Enterprise seat plus a personal Pro/Max plan, especially on Windows and Linux**. There is no official path, and existing tools are mostly macOS-only with few stars. The segment most willing to pay is **consultants, freelancers and agencies juggling several client or company accounts**, who also want per-account cost attribution. Limit-rotators are numerous but carry ban risk and gravitate to free CLI tools.

### Cited Findings
- Desktop multi-account tools are nearly all macOS. Windows has GiovanniTrevisan/claude-multi-account (6 stars) and deckspace.dev (Mac/Windows). The closest analogue, claude-multiprofile (133 stars), is macOS-only. I found no Linux Desktop multi-account tool. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile); [GiovanniTrevisan](https://github.com/GiovanniTrevisan/claude-multi-account); [HN deckspace](https://news.ycombinator.com/item?id=49141736)
- Consultants appear explicitly in top comments: "work org + personal + consulting… separate cost tracking" and "IT for an organization and also run my own consulting business". In Oct 2026, the Hatswap tool targets "one Claude Code account per client". — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435); [HN Hatswap](https://news.ycombinator.com/item?id=50075273)
- CLI users are well served by free options and official guidance (`CLAUDE_CONFIG_DIR`, per bcherny, 2026-08-17). HN commenters call a dedicated tool unnecessary: "you can do this with 2 lines of shell code". — [GitHub #27359](https://github.com/anthropics/claude-code/issues/27359); [HN 49111019](https://news.ycombinator.com/item?id=49111019)
- Enterprise users describe the missing feature as lowering the value of the company account, which implies employers have a stake too. — [GitHub #36151](https://github.com/anthropics/claude-code/issues/36151)

### Inferences
- Willingness to pay is likely highest among professionals who already pay $100–200 a month for Max plans and bill clients. A $5–15/month or one-time-license tool is small next to that spend. This is inference; no pricing data was found.
- The main risks:
  1. Anthropic ships a native Desktop switcher (#18435 is its most-reacted Desktop request).
  2. ToS and ban fears hurt adoption of any tool that manipulates credentials.
  3. Commoditization by free OSS tools.
  Cross-platform support (Windows and Linux) plus Desktop coverage is the clearest gap.

### Gaps
- No direct evidence of payment: no revenue figures for Multi-Claude, AgentsRoom or similar paid tools, and no pricing surveys.
- No data on how employers' IT policies treat personal accounts on work machines.
