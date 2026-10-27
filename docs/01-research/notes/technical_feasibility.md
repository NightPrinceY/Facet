# Technical Feasibility and Fragility: Cross-Platform Multi-Account Manager for Claude Desktop + Claude Code (as of late Oct 2026)

Scope note: research done 2026-10-25. Baseline already proven by the user on Ubuntu 26 GNOME Wayland, Claude Desktop 3.6556.2 (.deb): `--user-data-dir=<dir>` gives an isolated instance and login; Wayland app_id is hardcoded `com.anthropic.Claude` (ignores `--class`); separate dock icons needed `--ozone-platform=x11` plus WM_CLASS relabel via xdotool; Claude Code isolates via `CLAUDE_CONFIG_DIR`. Official Anthropic docs say nothing about multi-instance Claude Desktop. Everything about Desktop multi-instance below comes from community tools and guides, not Anthropic.

## Q1. Multi-instance Claude Desktop on Windows and macOS (`--user-data-dir`, existing tools, single-instance lock, updater, MSIX)

### Takeaway
`--user-data-dir` works on all three OSes because Claude Desktop is Electron and the single-instance lock is tied to the data directory. The method is unofficial. On macOS you launch with `open -n -a Claude --args --user-data-dir=...`. On Windows the main obstacle is MSIX packaging: you can't launch the .exe directly from `WindowsApps`. Tools get around this by copying the app out of the package or by using the Store's `claude-desktop.exe` execution alias. Auto-update is the main ongoing source of breakage, mostly for approaches that copy or tint the app.

### Cited Findings
- Community consensus: Claude Desktop accepts `--user-data-dir`, and a separate folder per instance gives an isolated copy with its own login. macOS uses `open -n -a "Claude" --args --user-data-dir=...`. None of this is official Anthropic documentation. — [philippstracker.com guide](https://philippstracker.com/multiple-claude-instances/); [deckspace.dev guide](https://deckspace.dev/claude-accounts/guide/); [multi-claude-app (macOS/Linux/Windows)](https://github.com/IbrahimHosseini/multi-claude-app)
- One guide says the single-instance lock is tied to the data directory, so a different `--user-data-dir` gives a different lock and a second instance can run. Another warns that relaunching the same `--user-data-dir` can spawn a duplicate instance and risk corrupting the profile. (Secondary community sources, summarized via search.) — [deckspace.dev](https://deckspace.dev/claude-accounts/guide/); [philippstracker.com](https://philippstracker.com/multiple-claude-instances/)
- Contrary detail: a DEV article says Chromium's single-instance logic checks for a lock file in the *default* data folder, so the default window always receives sign-in callbacks. — [DEV: why the obvious hacks break](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb). This conflicts with the user's own Linux result (separate instances worked). It is more likely a description of deep-link routing than of the lock itself. Treat it as unverified.
- **Windows / MSIX (official):** Anthropic offers MSIX packages for Windows deployment via Intune, SCCM, Group Policy, or PowerShell (`Add-AppxPackage` / `Add-AppxProvisionedPackage`), for x64 and arm64. Cowork requires admin rights and the Virtual Machine Platform feature. — [support.claude.com: Deploy Claude Desktop for Windows](https://support.claude.com/en/articles/12622703-deploy-claude-desktop-for-windows)
- Anthropic's Intune README: "Claude updates itself in place (within the MSIX framework — the package identity never changes, only the version)". A `disableAutoUpdates` policy exists. — [Claude Intune README](https://downloads.claude.ai/releases/enterprise/intune/Claude-Intune-README.md)
- **Windows tool (claude-desktop-multi):** Claude installs as MSIX under `C:\Program Files\WindowsApps`. Launching the .exe from there fails with "Windows cannot access the specified device, path, or file". The tool uses `Get-AppxPackage` to locate the install, `robocopy /MIR` to copy it to `C:\ClaudePortable`, and shortcuts that run `Claude.exe --user-data-dir="%APPDATA%\Claude-InstanceN"`. The portable copy doesn't auto-update; re-running the installer re-copies it. — [sypnose-cloud/claude-desktop-multi](https://github.com/sypnose-cloud/claude-desktop-multi/tree/main)
- Another Windows tool, claude-desktop-clone, uses the same Chromium `--user-data-dir` mechanism, one instance per account. — [vodongha/claude-desktop-clone](https://github.com/vodongha/claude-desktop-clone)
- Alternative Windows route: the Store install exposes a `claude-desktop.exe` alias in `WindowsApps` that can be launched with `--user-data-dir`. The Store version redirects data to `%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Roaming\Claude`. — [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)
- One guide claims its Windows method resolves the install path at runtime and needs "no patching, no copying the app, no admin rights". — [deckspace.dev](https://deckspace.dev/claude-accounts/guide/)
- **macOS tool (claude-multiprofile):** Builds per-profile AppleScript launcher apps (via `osacompile`) in `~/Applications`. Each runs `open -n -a 'Claude' --args --user-data-dir='...'` with data in `~/Library/Application Support/Claude-NAME/`. Since v0.1.12 the launcher also sets `CLAUDE_CONFIG_DIR` so Code sessions started from Desktop use the profile's config folder. — [jmdarre-v/claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- **Update breakages documented by claude-multiprofile:** Before v0.1.29, Claude's self-updater ran inside each tinted profile copy. Each update removed the tint, relaunched the copy on the default account, and shared update state with the main app. The fix was to block the self-updater per copy via the `disableAutoUpdates` policy and rebuild the copy when Claude.app changes. On macOS 27, launchers could be cut off before the helper finished (fixed v0.1.34). A stale LaunchServices cache can break Dock launches. — [claude-multiprofile README](https://github.com/jmdarre-v/claude-multiprofile)
- The DEV article says Claude updates itself, so copied or re-signed apps must be rebuilt after each release. A commenter (the Parall developer) says re-signing replaces Anthropic's signature. — [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)
- A new `--user-data-dir` starts empty. Desktop keeps cookies, settings, connectors, MCP config, and the Claude Code session list in the user data directory. — [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)
- Each instance is a full Electron app with its own VM setup, which costs substantial RAM and disk. — [deckspace.dev](https://deckspace.dev/claude-accounts/guide/) (secondary)

### Inferences
- The most durable design launches the official, unmodified binary with `--user-data-dir`: on macOS via `open -n`, on Windows via the Store or MSIX execution alias if it accepts args. Copying the app (Windows portable copy, macOS tinted clones) adds update drift and signature or integrity problems.
- MSIX does not stop argument passing as such. The problem is the ACL on `WindowsApps`. A Tauri manager could start `claude-desktop.exe` (the alias) with args, or use `IApplicationActivationManager::ActivateApplication` with an arguments string. This needs verification on a real machine.
- Anthropic could break the approach by ignoring `--user-data-dir` (unlikely for Electron), by adding a global (non-userData) single-instance lock, or by tying VM/Cowork services to one profile. The Windows Cowork service is a system-level component, which raises the risk of cross-instance conflicts.

### Gaps
- No official Anthropic statement on supporting or blocking multiple instances.
- No dated report of a specific Claude Desktop version breaking `--user-data-dir` itself. Documented breakages concern the self-updater and copies.
- I didn't verify whether the Windows `claude-desktop.exe` alias forwards `--user-data-dir` reliably on current builds. That claim is single-source (the DEV article).
- I didn't check how Cowork VM / Virtual Machine Platform behaves with two simultaneous instances.
- I didn't fetch the "Multi Instance for Claude Desktop" tool specifically.

## Q2. Separate taskbar/dock identity (Windows AUMID, macOS bundle IDs, Linux app_id)

### Takeaway
On macOS, every instance belongs to bundle `com.anthropic.claudefordesktop`, so all windows share one Dock tile. Launcher apps with distinct bundle IDs give separate *launchers*, but the running window still groups under Claude unless you clone the app (which breaks the code signature). On Windows, a window-level AppUserModelID overrides the process-level one. An external manager cannot easily set it on Electron windows it doesn't own, so per-instance taskbar separation on Windows is unproven. On Linux X11 a WM_CLASS relabel works (user-proven). Wayland app_id can't be changed externally.

### Cited Findings
- On macOS, instances share bundle ID `com.anthropic.claudefordesktop`, so they share one Dock icon, name, and ⌘-Tab entry. A Script Editor launcher is a separate app, but the window it opens still belongs to Claude. — [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)
- claude-multiprofile: launchers built before v0.1.9 shared the default AppleScript bundle ID, which confused macOS. Users are told to pin the launcher, not the running window's tile, because the pinned window tile opens the default account. The window title is always "Claude" because `CFBundleName` can't be edited without breaking code signing. Optional tinted copies (`cp -Rc` APFS clone) give a coloured Dock tile but fail `codesign --verify --deep --strict` while still reporting Anthropic's bundle ID and Team ID. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- Windows: Microsoft says to set a process AppUserModelID with `SetCurrentProcessExplicitAppUserModelID` during startup before any UI. A window-level AppUserModelID set through `SHGetPropertyStoreForWindow` / `System.AppUserModel.ID` overrides the process-level one, and the taskbar refreshes when the property is set. — [Microsoft: AppUserModelIDs](https://learn.microsoft.com/sk-sk/windows/win32/shell/appids); [Microsoft sample: AppUserModelID window property](https://learn.microsoft.com/en-au/windows/win32/shell/samples-appusermodelidwindowproperty); [System.AppUserModel.ID](https://learn.microsoft.com/windows/win32/properties/props-system-appusermodel-id)
- The Windows multi-instance tool (claude-desktop-multi) does not cover AUMID or taskbar grouping. — [claude-desktop-multi](https://github.com/sypnose-cloud/claude-desktop-multi/tree/main)

### Inferences
- Windows: MSIX apps get a package-derived AUMID. All instances will likely group under one taskbar button. An external manager could in theory call `SHGetPropertyStoreForWindow` on another process's HWND and set `System.AppUserModel.ID` per window. This works for many Win32 apps but is undocumented for this use, may be reset by Electron, and needs prototyping. A shortcut's AUMID only matters if the process adopts it. Expect separation on Windows to be the weakest part of the product.
- macOS: separate Dock identity for the *running* window realistically requires a copied and re-signed bundle with a distinct `CFBundleIdentifier` (ad-hoc signing). That breaks Anthropic's signature and Keychain "Safe Storage" access, and update drift follows. A safer product position is a separate launcher plus window badges or titles.
- Linux: X11 via `--ozone-platform=x11` plus WM_CLASS rewrite works (user-proven), but loses native Wayland (fractional scaling and similar). If Anthropic drops X11/XWayland support or enforces Wayland, this fails.

### Gaps
- I didn't fetch Electron's own docs for `app.setAppUserModelId` / per-window `setAppDetails` to confirm whether Claude sets an AUMID explicitly.
- I found no report of anyone achieving per-instance taskbar separation for Claude Desktop on Windows.

## Q3. `claude://` deep-link routing to the correct instance

### Takeaway
Deep links are the biggest functional fragility. The OS sends `claude://` to whichever app is registered for the scheme (macOS: by bundle ID; Windows/Linux: last registrant), so browser-based sign-in (Google SSO) can land in the wrong instance. Every existing tool works around this with "log in with other instances closed" or email-code login. A manager could intercept the scheme itself and forward it, but that hijacks Anthropic's scheme registration and is fragile.

### Cited Findings
- "claude:// deep links route to whichever Claude instance registered the URL scheme last." — community guides via search ([deckspace.dev](https://deckspace.dev/claude-accounts/guide/); [philippstracker.com](https://philippstracker.com/multiple-claude-instances/))
- Windows tool: login returns through `claude://`, which opens in whichever instance is running. With several open, accounts can get crossed. The workaround is to log in with the others closed. — [claude-desktop-multi](https://github.com/sypnose-cloud/claude-desktop-multi/tree/main)
- macOS tool: browser sign-ins such as Google return via `claude://`. Because profiles share a bundle ID, macOS may deliver the link to the wrong running Claude. Workarounds: quit other instances before first sign-in, or use email-code sign-in by typing the code rather than clicking the link. Two profiles can end up on the same account, and `doctor` compares account IDs. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- Tauri 2 deep-link plugin (official docs): on Linux and Windows, deep links are delivered as a command-line argument to a new app process. Schemes can be registered at runtime with `app.deep_link().register("scheme")` / `register_all`, on Linux and Windows only. "Registering deep links at runtime is not possible on macOS"; there deep links work only in the bundled app installed in `/Applications`. To route to an existing instance, enable the `deep-link` feature on `tauri-plugin-single-instance` and register single-instance first. Schemes registered at runtime must be checked manually in `argv`. AppImage deep links break if the file moves (absolute path). — [Tauri docs: deep-linking.mdx](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/deep-linking.mdx) (via Context7 `/tauri-apps/tauri-docs`)

### Inferences
- A manager could register itself as the `claude://` handler on Windows and Linux (runtime `register`), receive the URL in argv via single-instance, and forward it to the right Claude instance by launching `Claude.exe --user-data-dir=<pending-login-profile> "claude://..."`. Chromium/Electron would hand it to the running instance holding that userData lock. The manager would track which profile started a login. This is plausible but unverified. It also contends with Claude re-registering the scheme on every launch or update, which would silently undo the hijack.
- On macOS, scheme ownership is decided by LaunchServices by bundle. A Tauri app declaring `claude` in Info.plist would compete with Claude.app. The outcome is non-deterministic and hostile to Anthropic's app. Not recommended.
- Safest product approach: serialize logins (a guided "sign in this profile" flow that temporarily quits other instances), prefer email-code login, and verify account identity after login.

### Gaps
- I didn't confirm how Claude Desktop itself handles `open-url` / `second-instance` (official Electron docs not fetched in this session).
- I didn't verify whether Claude re-registers the `claude://` handler on every start on Windows and Linux.

## Q4. Claude Code: `CLAUDE_CONFIG_DIR`, credentials/Keychain, settings schema, cross-session messaging, OTel, JSONL logs

### Takeaway
`CLAUDE_CONFIG_DIR` is now the **officially documented** multi-account mechanism. Credentials are per-directory, including the macOS Keychain entry, which docs say is "keyed to that directory". Exception: Console keyless-profile sign-ins are stored outside the config dir. Settings, OTel (with account UUID and email attributes, plus estimated cost), and JSONL transcripts are well documented. Cross-session messaging works per OS user over local sockets. Whether it spans different config dirs is not documented.

### Cited Findings
- Official multi-account guidance: "give each account its own configuration directory… set the `CLAUDE_CONFIG_DIR` environment variable… Each directory has its own settings, session history, and claude.ai login or API key", e.g. `alias claude-work='CLAUDE_CONFIG_DIR=~/.claude-work claude'`. Caveat: separate directories "don't keep two Claude Console sign-ins without an API key apart" because that sign-in is stored outside the config dir (Anthropic profiles in `~/.config/anthropic` or `%APPDATA%\Anthropic`). — [code.claude.com: Authentication](https://code.claude.com/docs/en/authentication)
- Credential storage: on macOS, the encrypted Keychain, falling back to `~/.claude/.credentials.json` (0600) if the Keychain rejects the write. On Linux, `~/.claude/.credentials.json` (0600). On Windows, `%USERPROFILE%\.claude\.credentials.json`. With `CLAUDE_CONFIG_DIR` set, Claude Code "keeps the `.credentials.json` file under that directory… and keys the macOS Keychain entry to that directory too, so a session with a different `CLAUDE_CONFIG_DIR` reads a different entry." — [Authentication](https://code.claude.com/docs/en/authentication)
- claude-multiprofile confirms a Keychain key derived from the config dir, searchable as "Claude Code-credentials". Renaming a dir therefore signs the profile out, and deleting a dir leaves a Keychain entry behind. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- Historical conflicting reports in GitHub issues: some users said both instances used one fixed Keychain key, or that Linux/WSL second profiles weren't picked up. #22872 reports incomplete isolation (links #20553). The VS Code extension does not read `CLAUDE_CONFIG_DIR` (#55621), and Desktop does not honor it (#27359 → #25762). — [#30031](https://github.com/anthropics/claude-code/issues/30031); [#24963](https://github.com/anthropics/claude-code/issues/24963); [#22872](https://github.com/anthropics/claude-code/issues/22872); [#55621](https://github.com/anthropics/claude-code/issues/55621); [#27359](https://github.com/anthropics/claude-code/issues/27359). Current docs (above) supersede the Keychain complaint, but the other surfaces' gaps may remain. I didn't verify the current status of each issue.
- A third-party usage tool hit the problem that only the account matching the *process's* `CLAUDE_CONFIG_DIR` could be read from the Keychain. This is a pitfall for any manager reading tokens. — [openusage #1214](https://github.com/robinebers/openusage/issues/1214)
- Auth precedence: cloud provider vars, then `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `apiKeyHelper`, `CLAUDE_CODE_OAUTH_TOKEN` (one-year token from `claude setup-token`), Anthropic profile/federation, and `/login` OAuth last. A stray `ANTHROPIC_API_KEY` in the environment would override every profile. Claude Desktop does not read these env vars or `apiKeyHelper` and uses OAuth. — [Authentication](https://code.claude.com/docs/en/authentication)
- `CLAUDE_CONFIG_DIR` relocates every `~/.claude` path (settings, transcripts, plugins). `~/.claude.json` holds account details, MCP servers, and per-project trust. — [Settings](https://code.claude.com/docs/en/settings); [.claude directory](https://code.claude.com/docs/en/claude-directory)
- Settings: scopes are user (`~/.claude/settings.json`), project (`.claude/settings.json`), local (`.claude/settings.local.json`), and managed. Precedence runs managed > `--settings` CLI > local > project > user. List keys such as `permissions.allow` merge across scopes. Keys include `permissions` (allow/ask/deny, `defaultMode`), `hooks`, `env`, `model`, `apiKeyHelper`, `enabledPlugins`, `extraKnownMarketplaces`, `cleanupPeriodDays`, and `crossSessionInbound`. JSON schema is at `https://json.schemastore.org/claude-code-settings.json` ("can lag behind the newest CLI releases"). Files are hot-reloaded, except `model`/`effortLevel`, which load at start. — [Settings](https://code.claude.com/docs/en/settings)
- `permissions.defaultMode` values `auto`/`bypassPermissions` don't apply from project or local files. Telemetry export vars in a project `env` block are ignored. — [Settings](https://code.claude.com/docs/en/settings)
- **Cross-session messaging:** requires v2.1.224+ on macOS/Linux/WSL2 and v2.1.234+ on native Windows, on by default. Claude uses `ListAgents` and `SendMessage`; `/list-agents` (`/peers`) shows reachable sessions. Same-machine transport is a per-session Unix socket or named pipe, "never through Anthropic servers", restricted to the OS user (fallback dir `/tmp/cc-socks-<uid>`). `CLAUDE_CODE_MESSAGING_SOCKET` and `CLAUDE_CODE_MESSAGING_TOKEN` are exported to hooks and Bash, and scripts can post JSON lines (auth line required on Windows). Cross-machine messaging goes through Anthropic via Remote Control and needs a claude.ai login. `crossSessionInbound` takes accept/hold/refuse; `isolatePeerMachines` exists. Messages are plain text, about 1M char cap. Container↔host and WSL2↔native Windows sessions can't reach each other. — [code.claude.com: Cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging.md)
- **OpenTelemetry:** enabled with `CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER`/`OTEL_LOGS_EXPORTER` (otlp|prometheus|console|none), `OTEL_EXPORTER_OTLP_PROTOCOL` (no default, must be set), and `OTEL_EXPORTER_OTLP_ENDPOINT`. Metrics: `claude_code.session.count`, `cost.usage` (USD), `token.usage` (input/output/cacheRead/cacheCreation), `lines_of_code.count`, `commit.count`, `pull_request.count`, `code_edit_tool.decision`, `active_time.total`. The `claude_code.api_request` event carries `model`, `cost_usd` ("estimated"), `cost_usd_micros`, `duration_ms`, and the token counts. Attributes include `session.id`, `organization.id`, `user.account_uuid`, `user.account_id`, `user.email`, and `user.id`. — [code.claude.com: Monitoring](https://code.claude.com/docs/en/monitoring-usage)
- **Local data:** transcripts at `<config>/projects/<project>/<session>.jsonl` ("every message, tool call, and tool result"), subagent transcripts under `<session>/subagents/`, `history.jsonl`, `stats-cache.json` ("aggregated token and cost counts shown by `/usage`"), and `sessions/` (one file per running session). Default retention `cleanupPeriodDays` is 30. Transcripts are plaintext. — [.claude directory](https://code.claude.com/docs/en/claude-directory)

### Inferences
- Per-account analytics are best done via OTel. Set `CLAUDE_CODE_ENABLE_TELEMETRY` plus a local OTLP endpoint in each profile's `env` (user-scope settings inside each config dir) pointing at a collector embedded in the manager. `user.account_uuid` and `user.email` give account attribution for free. JSONL parsing is a fallback. Its schema is undocumented as a stable contract, so a versioned parser is needed.
- Since the inbox socket is per OS user and sessions are discovered by local sockets, sessions under different `CLAUDE_CONFIG_DIR`s on the same OS user can probably reach each other. Docs don't confirm this, and discovery may be scoped through the config dir's `sessions/` registry. Test before building features on it.
- The manager must scrub `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN` / `CLAUDE_CODE_OAUTH_TOKEN` from spawned environments, or every profile collapses onto one credential.
- For Desktop-launched Code sessions, setting `CLAUDE_CONFIG_DIR` in the Desktop launch environment (as claude-multiprofile does) is the known approach. Desktop reportedly doesn't honor it officially (#27359), so this is fragile.

### Gaps
- Not documented: whether `ListAgents` discovery spans different `CLAUDE_CONFIG_DIR`s or different accounts.
- No official JSONL field-level schema found; I didn't confirm whether per-message `usage` token fields exist in transcripts.
- I didn't verify the current open/closed state of #22872, #20553, #27359, #55621.

## Q5. Claude Desktop config (`claude_desktop_config.json`)

### Takeaway
The file lives in the Electron userData directory (`~/Library/Application Support/Claude/` on macOS, `%APPDATA%\Claude\` on Windows, redirected under `%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Roaming\Claude` for Store/MSIX installs). With `--user-data-dir`, each profile gets its own copy. Changes need a full restart.

### Cited Findings
- Paths: macOS `~/Library/Application Support/Claude/claude_desktop_config.json`, Windows `%APPDATA%\Claude\claude_desktop_config.json`. After editing, "completely quit Claude Desktop and restart it". Logs are at `~/Library/Logs/Claude` and `%APPDATA%\Claude\logs` (`mcp.log`, `mcp-server-NAME.log`). — [MCP docs: Connect to local MCP servers](https://modelcontextprotocol.io/docs/develop/connect-local-servers)
- Store install redirects data to `%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Roaming\Claude`. — [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)
- `--user-data-dir` moves MCP config, connectors, and auth into the profile folder. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile); [DEV article](https://dev.to/sebastian_balog_c88dc2913/how-to-run-multiple-claude-accounts-at-once-desktop-claude-code-and-why-the-obvious-hacks-break-19jb)

### Inferences
- Linux path is probably `~/.config/Claude/claude_desktop_config.json` (the user's machine has `~/.config/Claude/`); with `--user-data-dir`, `<dir>/claude_desktop_config.json`.
- The manager should edit config only while that instance is stopped, do atomic writes, and keep backups, because the app also writes this file (for example the in-app connector/extension UI) and may overwrite edits made while it runs.

### Gaps
- No authoritative source found on whether Desktop rewrites `claude_desktop_config.json` while running. The Linux path isn't officially documented.

## Q6. Tauri 2 suitability (tray, global shortcut, autostart, updater, process spawning, signing costs)

### Takeaway
Tauri 2 covers the needs. Official plugins exist for deep-link + single-instance (with the macOS runtime-registration caveat), autostart (macOS LaunchAgent), and a signed updater. Global shortcuts are the weak spot on Wayland: the plugin is X11-centric, and Wayland needs the XDG GlobalShortcuts portal. Signing costs are Apple's $99/yr (from background knowledge, not fetched) and Azure Artifact (Trusted) Signing for Windows (Basic tier: 5,000 signatures/month; price not rendered on the page I reached).

### Cited Findings
- Deep-link and single-instance behaviour and limits are as in Q3. — [Tauri docs deep-linking](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/deep-linking.mdx)
- Autostart plugin: `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(args))`, and it can pass launch args. — [Tauri docs autostart.mdx](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/autostart.mdx)
- Updater: needs a signing keypair (`TAURI_SIGNING_PRIVATE_KEY`; .env files don't work) and a static JSON manifest with per-platform `url` + `signature`. — [Tauri docs updater.mdx](https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/updater.mdx)
- Global shortcuts on Wayland (community sources, anecdotal): one developer found Tauri's Global Shortcut plugin "only natively supports X11" and patched in a portal-based PR. Wayland blocks global key listening, and the fix is the GlobalShortcuts portal. A Tauri discussion attributes unregistered shortcuts on WSL to Wayland. — [Hack Club devlog](https://stardance.hackclub.com/projects/16334/devlogs/13944); [tauri discussion #10017](https://github.com/tauri-apps/tauri/discussions/10017)
- Azure Artifact Signing (formerly Trusted Signing): Basic has 5,000 signatures/month vs 100,000 for Premium. Billing starts on account creation. The dollar price was rendered as a placeholder in fetched results. — [Azure Artifact Signing pricing](https://azure.microsoft.com/pricing/details/artifact-signing/)

### Inferences
- Tray, process spawning (`std::process::Command` / shell plugin), and window management are standard Tauri 2 capabilities and low risk.
- On GNOME Wayland, the manager should use the XDG GlobalShortcuts portal (GNOME 48+ supports it) or document "set a GNOME custom shortcut that runs `switchyard --open <profile>`" as a fallback. The CLI fallback is robust.
- Apple Developer Program costs $99/yr. This is background knowledge, not fetched this session; verify at https://developer.apple.com/programs/. It's required for Developer ID signing and notarization so Gatekeeper doesn't block the app. Without it, macOS users must bypass Gatekeeper manually.
- Windows: Azure Artifact Signing has historically been about $9.99/month for Basic (background knowledge, unverified here), with eligibility restrictions on certain individual and organization regions. The alternative is an OV/EV certificate from a CA (hundreds of USD/yr). Unsigned binaries trigger SmartScreen warnings.

### Gaps
- Didn't fetch the current global-shortcut plugin page for its official platform table, or check whether portal support has merged.
- Didn't confirm the current Azure Artifact Signing dollar price or eligibility.
- Apple $99/yr not verified from Apple's site this session.

## Q7. Overall fragility assessment and mitigation

### Takeaway
Claude Code multi-account is **solid and officially supported** (`CLAUDE_CONFIG_DIR`). Claude Desktop multi-instance via `--user-data-dir` is **proven but unofficial** on all three OSes. The fragile parts, in order: (1) `claude://` deep-link sign-in routing, (2) per-instance taskbar/dock identity on Windows and macOS (and Wayland), (3) Windows MSIX launch path and Cowork/VM service interaction, (4) auto-update interference with copied bundles, (5) undocumented JSONL format.

### Cited Findings
- Tool authors call their approaches unofficial and say they may need updates if Claude changes how profiles work. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)
- Real breakages so far came from the self-updater (profile copies reset to the default account), macOS OS updates (launcher timing on macOS 27), LaunchServices cache, and deep-link misrouting. — [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile); [claude-desktop-multi](https://github.com/sypnose-cloud/claude-desktop-multi/tree/main)
- `disableAutoUpdates` is an available Desktop policy for controlling updates in copies. — [Claude Intune README](https://downloads.claude.ai/releases/enterprise/intune/Claude-Intune-README.md); [claude-multiprofile](https://github.com/jmdarre-v/claude-multiprofile)

### Inferences
Per-OS fragility matrix (inferred from the findings above):

| Capability | Linux | Windows | macOS |
|---|---|---|---|
| Isolated Desktop instance (`--user-data-dir`) | Proven (user) | Works via portable copy or alias; MSIX ACL complicates it; medium risk | Proven (`open -n`); low-medium risk |
| Separate dock/taskbar identity | X11 + WM_CLASS proven; Wayland impossible externally | Unproven (AUMID override hack); high risk | Launcher yes; running-window separation needs re-signed clone; high risk |
| Login deep-link routing | Last-registrant; serialize logins | Same | Bundle-ID based; serialize logins |
| Claude Code isolation | Official | Official | Official (Keychain keyed per dir) |
| MCP config per profile | Per userData | Per userData (Store redirect path) | Per userData |
| Analytics | OTel (official) + JSONL (unofficial) | Same | Same |

Mitigations:
- **Versioned adapters:** an adapter per (OS, Claude Desktop version range, install type: deb/AppImage/MSIX/Store/.dmg) covering binary discovery (`Get-AppxPackage`, `/Applications/Claude.app`, `/usr/lib/claude-desktop`), data-dir layout, and launch method.
- **Detection at startup:** read the Claude Desktop version and Claude Code `--version`. Warn when a version falls outside the tested matrix, and degrade features (for example disable taskbar relabel) rather than fail.
- **Never modify Anthropic's binaries by default.** Make clones or re-signing opt-in only, and block updates in clones via `disableAutoUpdates`.
- **Login flow guard:** quit other instances during a profile's first sign-in. Afterwards verify account identity (Desktop: compare account email/ID; Code: `/status` or the `.claude.json` account fields) and alert on duplicate accounts across profiles.
- **Env hygiene:** strip `ANTHROPIC_API_KEY`/`AUTH_TOKEN`/`CLAUDE_CODE_OAUTH_TOKEN` and set `CLAUDE_CONFIG_DIR` explicitly for each spawned process.
- **Config writes:** only while the instance is stopped, atomic, with backups.
- **Automated smoke tests:** per OS in CI VMs (launch two instances, check distinct PIDs and lock files, check window class/AUMID, run Code `claude --version` under each config dir), re-run on each new Claude release (watch release feeds).
- **Analytics:** prefer OTel. Keep the JSONL parser tolerant and schema-versioned.

### Gaps
- No Anthropic policy statement found on whether running multiple accounts concurrently is allowed under the consumer terms. Product and legal risk is outside this technical scope but worth checking.
- No telemetry on how often Claude Desktop releases (cadence) break third-party launchers.
