# Session history

How Facet started and what has been decided, so any Claude account opening this project starts with the full picture. Newest decisions are in `docs/04-plan/DECISIONS.md`.

## 2026-10-25: from a personal setup to a product idea

### 1. The original problem
The developer runs Ubuntu 26 (GNOME, Wayland) and wanted a personal Claude account and a work Claude account open side by side in Claude Desktop.

### 2. What was built by hand (works today)
- **Install found:** Claude Desktop `.deb` (`/usr/lib/claude-desktop/claude-desktop`, icon `claude-desktop`, Wayland app id `com.anthropic.Claude`). Personal profile: `~/.config/Claude` (never touched).
- **Second instance:** `claude-desktop --user-data-dir=/home/you/.config/Claude-Work` opens a separate instance with its own sign-in. Electron's single-instance lock is per data folder.
- **Separate dock icon:** Claude hardcodes its Wayland app id and ignores `--class`. Fix: run the work instance with `--ozone-platform=x11` and relabel its X11 `WM_CLASS` to `claude-work` with `xdotool`, matching `StartupWMClass=claude-work` in the launcher. The script is `reference/linux-setup/claude-desktop-work` (installed at `~/.local/bin/claude-desktop-work`); the launcher is `reference/linux-setup/claude-work.desktop` (installed at `~/.local/share/applications/`).
- **Gotcha found:** Electron rewrites `/proc/PID/cmdline` into one space-separated string, so match substrings. Never `pgrep -f` a pattern that also appears in your own command line.
- **Claude Code CLI:** `alias claude-work='CLAUDE_CONFIG_DIR=/home/you/.claude-work claude'` added to `~/.bashrc`. `~/.claude-work` is created on first sign-in.
- **Updates:** Anthropic's apt repository added (`/etc/apt/sources.list.d/claude-desktop.list`, key `/usr/share/keyrings/claude-desktop-archive-keyring.asc`). Upgraded 2.19675.1 → 3.6556.2. Updates now come through Software Updater; unattended upgrades for this source were left off on purpose.
- **Computer use:** not available in the Linux build as of 3.6556.2. The app code mentions Linux needing an Xorg session; the feature is gated on Anthropic's side.

### 3. The idea grew
Two more Claude accounts made the hand-made setup feel limited. The idea became an app: many isolated accounts, sessions that can talk across accounts, full control of Claude settings, deep usage analytics, and an ecosystem manager. First name "Switchyard", renamed **Facet** ("Lanes" and "Tessera" were already taken by AI coding tools).

### 4. Research (five tracks, see `docs/01-research/`)
Verdict: **build it, but narrower than first imagined.**
- Demand is real (GitHub #18435: about 1,186 reactions, 217 comments, no Anthropic reply in nine months). Most people want to separate work and personal, not stretch limits.
- Running several accounts alone is a $10–30 commodity and Anthropic will likely ship it natively (mobile multi-account appeared around September 2026).
- What is defensible: cross-account costs down to the tool call, client budgets, per-account settings control, a safety preview for the ecosystem, and good Linux support.
- Avoid: auto-rotation, UI automation, touching tokens, "Claude" in the name.
- Business: open core with Pro around $9 a month or $89 a year, Team later.
- The cross-account Bridge stays experimental until test 1 shows whether native messaging crosses accounts.

### 5. Prototype
Three versions; the latest is `prototype/facet.html`. Version 4 adds the Timeline (down to the second, replay, most-expensive conversations, tasks and tool runs).

### 6. Product direction
- Research before building: what, why and how come before code.
- "Full control" means full control over Claude settings per account.

## Session 2, 2026-10-25 to 26: research, Phase 0 tests, Phase 1 skeleton

### 1. Research on extensions and harness control
The user asked for more abilities: creating skills and local skills, deeper harness control, deep search. Research (`docs/01-research/notes/extensibility_and_studio.md`, `docs/03-architecture/EXTENSION-SURFACE.md`) led to PRD module M11 Studio, whole-harness Control, deep search and test 11 (D-009). Four project skills (`/research`, `/new-spike`, `/lint-skills`, `/surface-check`) and the `guard-secrets` PreToolUse hook were added; the user enabled the hook.

### 2. Phase 0 results (run autonomously with the user's permission)
- Test 1: discovery is per account; delivery to a raw `uds:` socket crosses accounts. Bridge stays handoff-first; native path waits on Anthropic (D-010, `docs/04-plan/ANTHROPIC-QUESTIONS.md`).
- Test 2 (inspection): Desktop's Code tab signs in with Desktop's account but uses `CLAUDE_CONFIG_DIR`, defaulting to `~/.claude`. The hand-made work launcher was fixed (backups kept) (D-012).
- Test 3: telemetry per account with exact cost, content redacted; attribute by account uuid (D-011).
- Test 8: all three override variables beat a sign-in.
- Test 11: account-level skills stay in their account; `plugin validate` checks plugin roots only; `plugin eval` gives trigger deltas (and publishes reports unless `--no-publish`).
- Test 7 facts: GlobalShortcuts portal present on GNOME 50, all monitors at scale 1.0.

### 3. Phase 1 skeleton
Built in `app/` with Rust, React and tests; reviewed by the compliance and design reviewers, with all findings fixed (D-013, D-014). Packages (.deb, AppImage) build and start. Real-machine check: both running Desktop instances and live Code sessions are detected per account; both accounts use the default 30-day transcript clean-up (Health warns).
