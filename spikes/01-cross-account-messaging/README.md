# Spike 1: native messaging across accounts (also notes for tests 2 and 5)

**Question.** Claude Code v2.1.224+ lets sessions on one machine find each other (`/peers`, ListAgents) and send text (SendMessage). Does that reach a session running under a different `CLAUDE_CONFIG_DIR` signed into a different account?

**Why it matters.** Yes → Facet's Bridge is a thin UI over a native feature. No → at most a user-triggered handoff file. (Decision D-004.)

## Prerequisites
- Claude Code ≥ 2.1.224 (this machine: 2.1.299 on 2026-10-25).
- Two accounts signed in: personal (`~/.claude`) and work (`~/.claude-work`, sign in once with `claude-work`).

## Method
1. Terminal A: `cd /tmp && claude` (personal).
2. Terminal B: `cd /tmp && CLAUDE_CONFIG_DIR=~/.claude-work claude` (work).
3. In each, run `/peers`. Record what each sees.
4. In A, ask Claude to send a message to B's session if it is listed. Record delivery in B.
5. Repeat in the other direction.
6. Note where session discovery lives: list `$XDG_RUNTIME_DIR` and `/tmp` for Claude socket folders, and `ls <config>/sessions/` in each config dir (names only).
7. Control: open a second personal session (Terminal C, no `CLAUDE_CONFIG_DIR`) and confirm A and C see each other.

## Test 2 notes (Desktop-launched Code sessions)
Launch the work Desktop instance with `CLAUDE_CONFIG_DIR=~/.claude-work` in its environment, start a Code session in its Code tab, and check whether new transcripts appear under `~/.claude-work/projects/`.

### Test 2 result (2026-10-26): answered by inspection; live confirmation pending one user action

**Answer: the Code tab's *account* follows the Desktop it was started from, but its *config folder* follows `CLAUDE_CONFIG_DIR` in Desktop's environment, which defaults to the personal `~/.claude`.** Without the variable, a work Desktop's Code sessions would run as the work account while writing transcripts, settings, skills and the session registry into the personal folder. That would mix costs, leak personal skills into work sessions, and make work Code sessions discoverable by personal sessions.

Evidence (names only, no values read):
- Running processes: the personal Desktop (437708), the work Desktop (`--user-data-dir=~/.config/Claude-Work`, PID 447281) and the personal Desktop's Code session (440478) all run with `CLAUDE_CONFIG_DIR` unset. 440478 runs the bundled binary `~/.config/Claude/claude-code/2.1.308/…/claude` in SDK mode (`--output-format stream-json --input-format stream-json`).
- `tr '\0' '\n' < /proc/440478/environ | cut -d= -f1`: Desktop injects `CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CODE_OAUTH_SCOPES`, `CLAUDE_CODE_ACCOUNT_UUID`, `CLAUDE_CODE_USER_EMAIL`, `CLAUDE_CODE_ORGANIZATION_UUID`, `CLAUDE_CODE_SUBSCRIPTION_TYPE`, `CLAUDE_CODE_RATE_LIMIT_TIER`, `CLAUDE_CODE_SDK_HAS_HOST_AUTH_REFRESH`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_MESSAGING_SOCKET`/`_TOKEN`, `ANTHROPIC_BASE_URL`, `OTEL_RESOURCE_ATTRIBUTES`, `OTEL_SERVICE_NAME`, and more. So sign-in comes from the host Desktop, not from the config folder.
- Transcripts by `entrypoint`: `~/.claude` has 25 files from `claude-desktop` (plus 31 `cli`, 2 `claude-vscode`, 1 `sdk-cli`); `~/.claude-work` has none from `claude-desktop` (4 `cli`, 1 `sdk-cli`).
- 12 of the personal Desktop's `claude-code-sessions/` ids match transcript names in `~/.claude/projects/`. The work Desktop has downloaded its bundled Claude Code (2.1.288, 2.1.308) but has no Code session ids yet (only one `scheduled-tasks.json`).

**Method caution (compliance review, 2026-10-26):** reading a Claude Code process's environment, even names only, pulls the token value through a pipe inside an agent session; one typo would expose it. It was done once, names only, and must not be repeated. Facet's Health never reads the environment of Claude processes: for instances Facet starts it already knows what it passed; for others it looks for indirect evidence (the session registry file and transcript `entrypoint` in the expected folder). The account check uses `claude auth status --json`.

Change made: `export CLAUDE_CONFIG_DIR=/home/you/.claude-work` added to `~/.local/bin/claude-desktop-work` (backup `claude-desktop-work.bak-2026-10-26`) and to `reference/linux-setup/claude-desktop-work`. It takes effect at the next start of the work Desktop.

**To confirm live (needs the user):** quit the work Desktop, start it from its launcher, open its Code tab and send any short prompt. Then check that `~/.claude-work/sessions/` gains a `<pid>.json` with `entrypoint: claude-desktop` and that `~/.claude-work/projects/` gains a transcript with `entrypoint: claude-desktop`, while `~/.claude` gains none.

**Consequences for Facet:**
- Facet MUST start every Desktop instance with that account's `CLAUDE_CONFIG_DIR` (PRD M1).
- Rule 6 (scrub override variables) applies to what Facet passes to Desktop. Desktop itself then injects the token into its own Code children. That is the official app's behaviour, and Facet must not interfere with it.
- Telemetry: Desktop sets `OTEL_RESOURCE_ATTRIBUTES` and `OTEL_SERVICE_NAME` on its Code children. Test 3 must check whether a `facet.account` attribute from `settings.json` `env` survives for Desktop-launched sessions.

## Test 5 notes (sign-in links)
With personal and a second Desktop instance open, start Google sign-in in the second. Record which window receives the `claude://` link. Repeat with "Continue with email".

## Result

**Status: done (2026-10-26, Claude Code 2.1.310).**

**Answer: discovery does not cross accounts, but delivery does.**
- `ListAgents` / `/peers` and send-by-name only see sessions under the **same `CLAUDE_CONFIG_DIR`** (both directions, CLI and Desktop-launched sessions alike).
- A message sent to a session's raw socket address (`uds:/run/user/1000/cc-socks/<pid>.sock`) **is delivered across accounts, both ways**, and the receiver can reply the same way. The `uds:` form is the documented reply address (`from=` attribute of an incoming message).
- Same-machine messages stay local (docs); inbound can be limited per account with the documented setting `crossSessionInbound` (`accept|hold|refuse`).

### Run 2: headless sessions in both accounts (2026-10-26)

All sessions launched with `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN -u CLAUDE_CODE_OAUTH_TOKEN`, `--model haiku`, `--output-format stream-json`, prompts written for the test (no user content). Total cost reported: $0.0154.

| Step | From → to | Method | Result (verbatim excerpt) |
|---|---|---|---|
| Control | work headless `t1-a9` (`CLAUDE_CONFIG_DIR=~/.claude-work`) → this session `facet-fa` (work) | ListAgents, then SendMessage by name | Listed `facet-fa [aa194f] · interactive · busy`. Send `success: true`; ping arrived here as `<cross-session-message from="uds:/run/user/1000/cc-socks/954943.sock" from-name="t1-a9">` |
| Discovery personal → work | personal headless `t1-ce` (no `CLAUDE_CONFIG_DIR`, `~/.claude`) | ListAgents (twice) | Only `Dual Claude Desktop instances [7077cf] · interactive · idle · Claude Desktop session`. `facet-fa` not listed |
| Send by name personal → work | `t1-ce` → `facet-fa` | SendMessage `to: "facet-fa"` | `{"success":false,"message":"No agent named 'facet-fa' is reachable."}` |
| Discovery work → personal | `facet-fa` while `t1-ce` (PID 961559) was running and registered in `~/.claude/sessions/961559.json` | ListAgents | `No reachable agents` |
| Send by socket work → personal | `facet-fa` → `uds:/run/user/1000/cc-socks/961559.sock` | SendMessage | `success: true`; `t1-ce` received it |
| Reply personal → work | `t1-ce` → `uds:/run/user/1000/cc-socks/816450.sock` | SendMessage | `success: true`; arrived here: `from-name="t1-ce"`: "Ack from personal-headless (t1-ce): received your facet spike 1 ping." |

Also seen: headless `-p` sessions register in `<config>/sessions/` and are discoverable like interactive ones; a Desktop-launched Code session registers in its account's folder with `entrypoint: claude-desktop` and is listed as "Claude Desktop session". Messages arrived without approval because sender and receiver ran in compatible permission modes (`from-mode="prompting"`).

Not examined (on purpose): the `<pid>.*.key` files and how the socket authenticates the sender. Open question: whether delivery to a raw `uds:` address of another account is intended or will be closed; it is not described in the docs. A draft question for Anthropic is in `docs/04-plan/ANTHROPIC-QUESTIONS.md`. Registry fields were only partly listed; a Facet parser must use an allow-list.

### What this means for Facet
(Revised after compliance review: see D-010. The handoff file is the default; the native path below stays Lab-only until Anthropic documents it.)
- The Bridge *could* be a **thin UI over native messaging**: Facet lists live sessions of every account from each `<config>/sessions/<pid>.json` (name, cwd, status, `messagingSocketPath`, never the `.key`), and the user asks a session to message another by its `uds:` address. Facet never writes to the sockets itself (rule 5).
- Walls are enforceable with documented settings: walled accounts get `crossSessionInbound: "refuse"`.
- Because any same-user session can reach any other session's socket, Facet should show which accounts accept inbound messages and default sensitive (client) accounts to `hold`.
- Risk: relies on an undocumented cross-account behaviour; treat as Lab until Anthropic documents it.

### Run 1 (2026-10-25)


Setup found on the machine (not created for the test):

| Session | PID | Config dir | Version | `entrypoint` | Name |
|---|---|---|---|---|---|
| A (personal) | 440478 | `~/.claude` (`CLAUDE_CONFIG_DIR` unset) | 2.1.308 (bundled with Claude Desktop) | `claude-desktop` | "Dual Claude Desktop instances" |
| B (work, this session) | 816450 | `~/.claude-work` | 2.1.299 (`claude --version`) | `cli` | `facet-fa` |

Commands run (in B):
```bash
claude --version                                   # 2.1.299 (Claude Code)
ls ~/.claude/sessions ~/.claude-work/sessions       # names only
tr '\0' '\n' < /proc/<pid>/environ | grep ^CLAUDE_CONFIG_DIR=
# key names and peer fields only from <config>/sessions/<pid>.json (no values of ids or paths beyond the socket folder)
```
Then the ListAgents tool in B.

Raw evidence:
- ListAgents in B: `No reachable agents — no other Claude session is running on this machine right now (peer messaging itself is available ...)`. A was running (elapsed 05:52:06) and was not listed.
- Session registry is **per config dir**: `~/.claude/sessions/440478.json` and `~/.claude-work/sessions/816450.json`. Each holds `pid, sessionId, cwd, version, peerProtocol, peerFeatures, kind, entrypoint, pidDomain, messagingSocketPath, name, status, ...` (A also has `hostSessionId`, `bridgeSessionId`).
- Both advertise `peerProtocol: 1`, `peerFeatures: ['notify_idle', 'reply_across_default_dirs', 'artifact_yield']`, the same `pidDomain`, and both sockets sit in the **same shared folder** `/run/user/1000/cc-socks/`.
- Each folder also has a `<pid>.<hash>.key` file. Not opened (treated as secret).

Reading so far: the messaging transport (sockets in `$XDG_RUNTIME_DIR/cc-socks`) is shared by user, but **discovery reads `<CLAUDE_CONFIG_DIR>/sessions/`**, so a session under another config dir is not listed. The feature name `reply_across_default_dirs` hints that replies may cross config dirs in some cases (unverified).

Caveat: A is a Desktop-spawned session on a newer version (2.1.308, stream-json). A clean negative needs the control below.
