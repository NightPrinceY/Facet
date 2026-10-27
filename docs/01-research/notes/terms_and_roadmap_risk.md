# Anthropic Terms, Policies, and Roadmap Risk for "Switchyard" (multi-account manager for Claude)

Research date: 2026-10-25. Not legal advice. All quotes come from pages fetched on that date, through a summarizing fetch tool. Wording may be lightly abridged, so the exact text should be checked at the URL before anyone relies on it.

## Q1. What do Anthropic's terms say about multiple accounts, account sharing, limits, automated access, and using subscriptions through third-party tools? What enforcement has happened?

### Takeaway
No Anthropic document found forbids one person from holding several accounts for legitimate reasons. Multiple accounts are prohibited only when they are used to evade detection or bans. What the terms do forbid clearly:
- sharing credentials or making an account available to someone else
- automated or non-human access outside the API, unless Anthropic explicitly permits it
- bypassing safeguards
- (enforced since January to April 2026) third parties offering Claude.ai login, or touching or routing subscription OAuth tokens

Anthropic has enforced the token and harness rule with technical blocks, some mistaken account bans, and a legal demand to OpenCode.

### Cited Findings
**Consumer Terms** (Free, Pro, Max). Effective Oct 8, 2025. [anthropic.com/legal/consumer-terms](https://www.anthropic.com/legal/consumer-terms)
- Section 2: "To access our Services, we may ask you to create an Account." Nothing in the page limits users to one account.
- Section 2: "You may not share your Account login information, Anthropic API key, or Account credentials with anyone else." Also: "You also may not make your Account available to anyone else." And: "You are responsible for all activity occurring under your Account."
- Section 3, item 7: "Except when you are accessing our Services via an Anthropic API Key or where we otherwise explicitly permit it, … to access the Services through automated or non-human means, whether through a bot, script, or otherwise."
- Section 3, item 3, bars "To decompile, reverse engineer, disassemble, or otherwise reduce our Services to human-readable form". Section 3 also bars "bypassing any of our systems or protective measures".
- Section 7: "Your use of any Third-Party Content, services, and integrations is at your own risk".
- Section 12, brand: "You may not, without our prior written permission, use our name, logos, or other trademarks".
- Section 12, termination: "We may suspend or terminate your access to the Services (including any Subscriptions) at any time without notice to you" if Anthropic believes the Terms were breached.
- The Consumer Terms do not mention usage limits or rate limits.

**Usage Policy (AUP).** The page displays an effective date of "Nov 12, 2026". That is after today's date, so it may be an announced upcoming version. Flag this and recheck. [anthropic.com/legal/aup](https://www.anthropic.com/legal/aup)
- Under "Do Not Abuse Our Platform":
  - "Coordinate malicious activity across multiple accounts, or create multiple accounts, to avoid detection"
  - "Circumvent a ban through the use of a different account, such as the creation of a new account"
  - "Intentionally bypass or interfere with capabilities, restrictions, or guardrails established within our products"
  - "Utilize automation in account creation or to engage in spammy behavior"
  - "Use stolen, fraudulent, or unauthorized payment methods, or otherwise evade payment for our services"
- There is no explicit clause on circumventing usage limits. The closest provisions are the guardrail-bypass and payment-evasion clauses above.

**Claude Code Legal and Compliance page.** No date shown; fetched 2026-10-25. [code.claude.com/docs/en/legal-and-compliance](https://code.claude.com/docs/en/legal-and-compliance)
- "Claude Code usage is subject to the Anthropic Usage Policy. Advertised usage limits for Pro and Max plans assume ordinary, individual usage of Claude Code and the Agent SDK."
- "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications."
- "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, or Max plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow."
- Explicit carve-out: the restriction does not "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription".
- For products that embed Claude Code: "The Claude Code binary must not be modified … customers may not remove, disable, or restrict any authentication method built into it". Each end user must use their own credentials.
- "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice."
- Licensing: Free, Pro, and Max users are governed by the Consumer Terms. Team, Enterprise, and API users are governed by the Commercial Terms.

**Enforcement timeline, 2026**
- **Jan 9, 2026.** Anthropic "tightened our safeguards against spoofing the Claude Code harness", according to Thariq Shihipar of Anthropic on X. Third-party harnesses that spoofed Claude Code's client identity were blocked; OpenCode is the named example. Abuse filters automatically banned some accounts, and Anthropic said it was reversing those bans. The stated reason was that unauthorized harnesses introduce bugs and usage patterns Anthropic cannot diagnose. — [VentureBeat](https://venturebeat.com/technology/anthropic-cracks-down-on-unauthorized-claude-usage-by-third-party-harnesses)
- **Feb 18–19, 2026.** Anthropic updated its docs to say Free, Pro, and Max OAuth tokens are for "use exclusively with Claude Code and Claude.ai". Using them in other products, including the Agent SDK, is "unauthorized and a violation of its consumer terms of service" (as reported). Shihipar also wrote: "We want to encourage local development and experimentation with the Agent SDK and claude -p" and "If you're building a business on top of the Agent SDK, you should use an API key instead". He apologized that the docs were confusing. — [GIGAZINE](https://gigazine.net/gsc_news/en/20260220-anthropic-third-party-block)
- **March 2026.** Anthropic reportedly sent legal demands to OpenCode. OpenCode removed Anthropic OAuth, the spoofed claude-code beta header, and its Anthropic provider references. **April 4, 2026:** the OpenClaw change took effect, and third-party usage now needs pay-as-you-go API billing. — Search-result summaries of [falcao.org](https://falcao.org/posts/anthropic-claude-access-crackdown-ecosystem-fallout/), [lilting.ch](https://lilting.ch/en/articles/anthropic-claude-code-openclaw-third-party-paygo) and [letsdatascience](https://letsdatascience.com/news/anthropic-revises-terms-to-bar-third-party-harnesses-b7377cc7). These pages were not fetched, so treat the details as secondary.

### Inferences
- The enforcement targets three things: spoofing the official client, taking or reusing OAuth tokens, and reselling subscription capacity. Switchyard's planned design avoids all three. It launches the unmodified official apps, the user signs in through Anthropic's own flow, and Switchyard never reads or holds tokens. On that basis it falls outside the conduct Anthropic has acted against so far.
- **Main ambiguous area: one person stacking several paid Pro or Max subscriptions to get more usage.** No clause found prohibits it. However, "ordinary, individual usage" and "bypass … restrictions" give Anthropic room to treat it as limit circumvention, especially if the tool rotates accounts automatically when limits are hit.
- Switchyard should not market itself as a way to "get around usage limits". It should not auto-rotate accounts when a limit is hit. Better framing: separating work, personal, and client accounts.
- **Message relay between sessions is the second-riskiest feature.** If Switchyard injects prompts into Claude Desktop sessions without a human (UI automation or a script), it may count as "automated or non-human means" under Consumer Terms Section 3(7).
  - Claude Code with `claude -p` or the Agent SDK, signed in with a subscription, was described as acceptable for local experimentation.
  - Driving the Desktop or claude.ai UI by script has no such permission.
  - Safer design: the user explicitly triggers each copy or paste, or the relay goes through Claude Code's documented interfaces (hooks, MCP, `claude -p`) and never automates the Desktop UI.
- Relaying content between accounts that belong to different people could breach "make your Account available to anyone else". Switchyard should state that all managed accounts must belong to the same user.

### Gaps
- I did not fetch the Commercial Terms directly. This matters for Team and Enterprise accounts, which an admin may restrict further. VentureBeat cites Section D.4 (no competing products).
- I found no Anthropic statement that addresses one person holding several consumer accounts. The relevant support.claude.com article was not found.
- I found no documented case of a ban for multi-accounting alone. The only bans found were the mistaken Jan 2026 bans tied to harnesses.
- The "Nov 12, 2026" effective date on the AUP is unexplained.

## Q2. Is launching official Claude Desktop with separate data directories compliant? What practices keep the tool clearly compliant?

### Takeaway
Anthropic staff point users to separate config directories for multiple Claude Code accounts. That makes Switchyard's Claude Code approach low-risk. For Claude Desktop, separate data directories (for example Electron's `--user-data-dir`) are community workarounds that Anthropic has neither endorsed nor forbidden.

### Cited Findings
- **Issue #27359, "Named account profiles…"** (opened Feb 21, 2026). Closed as completed on Aug 17, 2026 by @bcherny (Boris Cherny, an Anthropic collaborator), who wrote: "You can do this today with a separate config directory per account. Each directory keeps its own login, settings, and session history … alias claude-work='CLAUDE_CONFIG_DIR=~/.claude-work claude' … a built-in `--profile` flag would be a thin wrapper over the same mechanism." Commenters noted this does not cover Claude Desktop. One commenter described a `--user-data-dir` second instance for Desktop and said Windows SSO callbacks may return to the wrong window; that commenter sells a paid tool, so the claim is unverified. — [GitHub #27359](https://github.com/anthropics/claude-code/issues/27359)
- **Issue #25762** (opened Feb 14, 2026). @bcherny closed it on Aug 17, 2026: "This exists: set `CLAUDE_CONFIG_DIR` … All settings, session history, plugins, and (on Linux/Windows) credentials live under that path; on macOS credentials stay in the Keychain." The issue was locked on Oct 6, 2026. Users report `CLAUDE_CONFIG_DIR` is honored inconsistently: `~/.claude/CLAUDE.md` is still read, and `~/.claude.json` sits outside the directory. Issues #47056, #55065, #55249, #55456, #42217 and #30538 are cited. — [GitHub #25762](https://github.com/anthropics/claude-code/issues/25762)
- **Issue #18435** (Desktop multi-account) lists community workarounds: separate Desktop installs with separate data folders, plus third-party tools such as multi-claude, claude-account-switcher, lanes, cloak and giverny. — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)
- The Legal and Compliance page allows "an end user … signing in to the unmodified Claude Code binary with their own Claude subscription", and requires that "sign-in … complete through Anthropic's own flow". — [code.claude.com legal](https://code.claude.com/docs/en/legal-and-compliance)

### Inferences
Practices that keep Switchyard clearly on the right side:
1. Launch only unmodified, official binaries. Never patch, inject into, or repackage Claude Desktop or Claude Code.
2. Isolate accounts only through supported or OS-level means: `CLAUDE_CONFIG_DIR`, a separate HOME, Electron user-data directories, or separate OS users.
3. Never read, copy, export, sync or proxy OAuth tokens, session cookies or keychain entries, and never spoof client headers.
4. Make no network calls to Anthropic endpoints of Switchyard's own.
5. All accounts must belong to one user, and Switchyard must not pool or share accounts.
6. Do not automatically rotate accounts to avoid limits, and do not market the tool that way.
7. Relay only with a human in the loop, or through documented Claude Code interfaces.
8. Edit only documented settings keys in `settings.json`.

Because Anthropic "may [enforce] without prior notice", users carry the account risk. Switchyard should say so in its README.

### Gaps
- There is no official Anthropic statement on running several Claude Desktop instances with `--user-data-dir`, and no documentation that Desktop supports it.

## Q3. Trademark: can a third-party product use "Claude" in its name or logo, or say "works with Claude"?

### Takeaway
Do not put "Claude" in the product name or logo. Anthropic has actively enforced this (Clawdbot was renamed in January 2026). A plain-text factual statement of compatibility, together with a non-affiliation disclaimer, matches what Anthropic's Claude Code page allows. A "works with Claude" badge or logo lockup would need written permission.

### Cited Findings
- **Claude Code Legal page:** "You can accurately say, in plain text, that your product has Claude Code preinstalled or that it runs Claude Code. But you can't use the Claude Code or Anthropic names or logos as part of your own product, feature, or company name, in your own logo, or in a way that suggests Anthropic built, endorses, or is partnered with your product. Any other use … requires our written permission." — [code.claude.com legal](https://code.claude.com/docs/en/legal-and-compliance)
- **Trademark Guidelines** (effective Aug 1, 2024):
  - "may only use our trademarks as specifically permitted by us and only in materials we approve beforehand"
  - "No alterations of our trademarks"
  - "You may not use our trademarks in a manner that implies Anthropic's sponsorship or endorsement … or a relationship or affiliation with Anthropic, except as we expressly authorize"
  - The guidelines do not address domains or "works with" statements.
  - [anthropic.com/legal/trademark-guidelines](https://www.anthropic.com/legal/trademark-guidelines)
- **Consumer Terms Section 12:** brand use requires prior written permission. — [consumer terms](https://www.anthropic.com/legal/consumer-terms)
- **Clawdbot precedent.** Clawdbot became Moltbot on Jan 27, 2026 after Anthropic asked for a name change. Creator Peter Steinberger: "Anthropic asked us to change our name (trademark stuff)". Reports disagree on whether this was a polite email or a lawyer's letter. The project was renamed OpenClaw on Jan 30. — [Laravel News](https://laravel-news.com/index.php/clawdbot-rebrands-to-moltbot-after-trademark-request-from-anthropic), [Morphllm](https://www.morphllm.com/clawdbot), [AlternativeTo](https://alternativeto.net/news/2026/1/trending-open-source-ai-agent-clawdbot-rebrands-to-moltbot-after-pressure-from-anthropic/)
- **Claudia to opcode.** Claudia was a GUI for Claude Code from Asterisk (YC S24). A "rename claudia to opcode" commit is dated 2025-10-10. opcode's README carries a non-affiliation disclaimer and says "Claude is a trademark of Anthropic, PBC". No source gives a reason for the rename. — [git mirror](https://git.joshthomas.dev/mirrors/claudia), [itsfoss](https://itsfoss.com/news/claudia/)

### Inferences
- "Switchyard" as a name is fine.
- Use a plain-text description, for example "an unofficial manager for Claude Desktop and Claude Code accounts", plus a disclaimer: "not affiliated with or endorsed by Anthropic; Claude is a trademark of Anthropic, PBC".
- Avoid "works with Claude" styled as a badge, Anthropic or Claude logos, the Claude starburst or similar shapes, and "Claude" in the domain, bundle ID or package name.

### Gaps
- The reason for the Claudia rename is unconfirmed. Treating it as trademark-driven is only an inference.
- I found no Anthropic "works with Claude" partner or badge program.

## Q4. Platform and roadmap risk: native multi-account support, profiles, dashboards, and Anthropic shipping features that replace third-party tools

### Takeaway
The risk that Anthropic ships something that makes Switchyard redundant is high and already partly real:
- Mobile multi-account switching appears to have shipped around Sep 11–15, 2026 (community reports, up to 3 accounts).
- Anthropic says Claude Code profiles are "a thin wrapper" over `CLAUDE_CONFIG_DIR`.
- Desktop multi-account (#18435) is the most-requested gap and still has no staff reply.

The likely next steps are Desktop multi-account, then usage, plugin and cross-session features. Switchyard's differentiation is narrow.

### Cited Findings
**Issue #18435** (Desktop multi-account). Opened Jan 15, 2026 and still open. Labels: area:auth, area:desktop, area:ide, enhancement. No Anthropic staff responses. Last activity Oct 24, 2026, when a user tagged @sarahdeaton asking for news, with no reply. Many +1 comments from consultants and enterprise IT, and top comments have about 100 thumbs-up. — [GitHub #18435](https://github.com/anthropics/claude-code/issues/18435)

**Issue #36151** (mobile multi-account without a shared email). Opened Mar 19, 2026 and still open; a bot labeled it `invalid`. Community reports:
- Sep 13, 2026: "There is now an 'add account' button to add multiple accounts with the ability to switch between them."
- Works on Android and iOS on app version 1.260911.19, apparently with a gradual rollout.
- One user reported a limit of 3 accounts.
- Anthropic has made no official announcement in the thread.
- macOS Desktop is not yet supported, according to Sep 16 comments.

Source: [GitHub #36151](https://github.com/anthropics/claude-code/issues/36151)

**Existing official switching.** It covers only an individual account plus a Team or Enterprise account under the same email, through the initials menu. — [support.claude.com 13189465](https://support.claude.com/en/articles/13189465-log-in-to-your-claude-account)

**Profiles in Claude Code.** Anthropic's position (#27359, Aug 17, 2026) is that `CLAUDE_CONFIG_DIR` covers the use case and a `--profile` flag would be "a thin wrapper". That makes a native flag cheap to ship. — [GitHub #27359](https://github.com/anthropics/claude-code/issues/27359)

**Claude Desktop on Linux.** An official beta for Ubuntu 22.04+ and Debian 12+ (x86_64 and arm64) shipped on June 30, 2026, without Computer Use. No date for Linux Computer Use was found. — [IT Brief](https://itbrief.co.uk/story/anthropic-launches-claude-desktop-beta-for-linux-users), [pasqualepillitteri.it](https://pasqualepillitteri.it/en/news/6967/claude-desktop-linux-beta-ubuntu-debian-en)

**Claude Code feature cadence** (community changelogs; not checked against the official CHANGELOG.md):
- v2.1.157 (May 29, 2026): plugins auto-loaded from `.claude/skills`.
- v2.1.172 (Jun 10): search bar for the `/plugin` marketplace; nested subagents.
- v2.1.178 (Jun 15): agent teams simplified, with an implicit team per session.
- v2.1.198 (Jul 1): Chrome integration out of beta.

Sources: [claudemarket.ai](https://claudemarket.ai/blog/claude-code-changelog), [havoptic](https://www.havoptic.com/tools/claude-code)

**Desktop multi-session management.** Ben's Bites headlined that Anthropic redesigned Claude Code on desktop "adding a sidebar for managing multiple sessions, a drag-and-drop layout, an integrated terminal and a file editor". The page could not be fetched (HTTP 522), so the date is unknown. — [Ben's Bites](https://news.bensbites.co/posts/63975-anthropic-redesigns-claude-code-on-desktop-adding-a-sidebar-for-managing-multiple-sessions-a-drag-and-drop-layout-an-integrated-terminal-and-a-file-editor)

**Precedent of replacing third-party tools:**
- Anthropic made the OpenClaw and OpenCode subscription path non-viable through policy (see Q1).
- Desktop session management and the plugin marketplace overlap with what GUIs such as Claudia/opcode offered.
- Mobile account switching was shipped about six months after the community request.

### Inferences
- The feature most likely to make Switchyard redundant is native Desktop multi-account. Mobile already has it, and the issue thread shows strong demand.
- Plan for Anthropic to ship it within months.
- Durable differentiators would be cross-vendor or cross-tool orchestration, usage analytics spanning accounts, and Linux support, where Desktop is still in beta.
- Cross-session relay inside one account is already partly covered by agent teams and subagents. Relay across accounts is unlikely to be shipped officially, but it is also the most policy-sensitive feature (see Q1).

### Gaps
- I could not verify official dates for these earlier launches: Claude Code in Desktop, Claude Code on web, the `/usage` command, the original plugin marketplace launch, and agent teams. The official CHANGELOG.md was not fetched.
- I found no official Anthropic usage-dashboard announcement for consumers.
- No staff comment on Desktop multi-account was found.
- The 3-account mobile limit comes from a single user report.

## Q5. Data and privacy: what to do with local session logs, and what never to touch (OAuth tokens, keychain)

### Takeaway
Treat credentials as untouchable. Keep log analytics local-only, read-only and opt-in. The documents fetched set no rule on reading your own local logs. The risk is in handling tokens and in sending conversation content anywhere.

### Cited Findings
- On macOS, Claude Code credentials stay in the Keychain. On Linux and Windows they live under the config directory, alongside settings, session history and plugins. — [GitHub #25762](https://github.com/anthropics/claude-code/issues/25762)
- "developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow." — [code.claude.com legal](https://code.claude.com/docs/en/legal-and-compliance)
- The Consumer Terms forbid reverse engineering the Services and bypassing protective measures. — [consumer terms](https://www.anthropic.com/legal/consumer-terms)

### Inferences
**Never touch:**
- the macOS Keychain entries for Claude
- the credentials file under the config directory on Linux and Windows (exclude it from any copy, backup or sync)
- Electron cookie and session stores in Desktop user-data directories
- any token in memory or on the network

**When creating profiles,** clone only non-secret settings and never the credentials. Have each profile sign in fresh through Anthropic's flow.

**For session logs (JSONL transcripts):**
- parse locally, read-only
- extract metadata only, such as token counts and timestamps
- send no telemetry containing prompt content
- let users choose which directories are scanned
- note that the format is undocumented and may change, so this carries platform risk

**For `settings.json` edits,** make a backup first, change only documented keys, and never edit managed or enterprise policy files.

### Gaps
- The fetched sources contain no Anthropic statement on whether third parties may read local Claude Code transcripts. Treat it as permitted by default but undocumented.
- The exact credential filenames for Claude Code were not verified in the sources.
