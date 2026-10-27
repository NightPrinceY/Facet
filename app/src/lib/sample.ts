// Sample backend for the browser preview and tests. Everything here is
// invented and the UI labels it "sample data". It never touches the disk.

import type {
  Account,
  AccountInput,
  AccountState,
  Api,
  FolderRule,
  AuthStatus,
  DetectedSetup,
  HealthCheck,
} from "./api";
import { defaultDirs, firstUnusedColor, slugify } from "./accounts";
import { sampleEvents, sampleUsage } from "./sampleUsage";
import { SAMPLE_HIGH, sampleInventory } from "./sampleEcosystem";
import { HIDDEN_MARKER, REFUSED_ENV, applyOps, diffLines, isRefusedEnv, type Settings, type SettingsBackup } from "./control";
import { DESC_BUDGET, relPath, validName, type AccountItem, type Finding, type ItemKind, type Lint } from "./studio";

const HOME = "/home/you";

export function createSampleApi(): Api {
  let accounts: Account[] = [
    {
      id: "personal",
      name: "Personal",
      color: 1,
      plan: "Max",
      desktopDir: `${HOME}/.config/Claude`,
      codeDir: `${HOME}/.claude`,
      expectedEmail: "you@example.com",
      tint: true,
      x11Identity: false,
      createdAt: Date.UTC(2026, 9, 25),
      launcherIcon: "claude-badge",
    },
    {
      id: "work",
      name: "Work",
      color: 2,
      plan: "Team",
      desktopDir: `${HOME}/.config/Claude-Work`,
      codeDir: `${HOME}/.claude-work`,
      expectedEmail: "you@company.example",
      tint: true,
      x11Identity: true,
      createdAt: Date.UTC(2026, 9, 25),
      launcherIcon: "claude-badge",
      badge: "W",
      notes: "Acme client work. Bill monthly.",
    },
  ];
  const running = new Map<string, number>([["personal", 4101]]);
  let rules: FolderRule[] = [{ folder: `${HOME}/work`, accountId: "work" }];
  let autoInstalled = false;
  const settings = new Map<string, Settings | null>([
    ["personal", { model: "opus", permissions: { allow: ["Bash(git status)"] }, env: { GITHUB_TOKEN: "sample-not-a-real-token", OTEL_METRICS_EXPORTER: "otlp" } }],
    ["work", null],
  ]);
  const backups = new Map<string, { b: SettingsBackup; text: string }[]>();
  const hideSecrets = (st: Settings | null): Settings | null => {
    if (!st?.env || typeof st.env !== "object" || Array.isArray(st.env)) return st;
    const env = Object.fromEntries(Object.entries(st.env as Settings).map(([k, v]) => [k, /KEY|TOKEN|SECRET|PASSWORD|HEADERS|AUTH/i.test(k) ? HIDDEN_MARKER : v]));
    return { ...st, env };
  };
  let tick = 0;
  const pluginEnabled = new Map<string, boolean>();
  const handoffs = new Map<string, { text: string; at: number }>();
  const addedMcp = new Map<string, import("./ecosystem").McpInfo[]>();
  const removedMcp = new Set<string>();
  const links: import("./api").LinkSettings = { enabled: false, fallback: null, previousDefault: null, accounts: {} };
  const sampleBrowsers: import("./api").Browser[] = [
    {
      id: "chrome",
      name: "Google Chrome",
      exe: "/usr/bin/google-chrome-stable",
      profiles: [
        { dir: "Default", name: "Personal", emails: ["you@example.com"] },
        { dir: "Profile 1", name: "Work", emails: ["you@company.example"] },
      ],
    },
    { id: "firefox", name: "Firefox", exe: "/usr/bin/firefox", profiles: [{ dir: "default-release", name: "default-release", emails: [] }] },
  ];
  const pluginReviewed = new Map<string, string>();
  // Studio sample: a library and the items placed in each account.
  const library = new Map<string, { kind: ItemKind; name: string; text: string }>([
    [
      "skill:review-pr",
      {
        kind: "skill",
        name: "review-pr",
        text: '---\nname: review-pr\ndescription: "Review a pull request for bugs, missing tests and unclear names. Use when the user asks to review a PR or a diff."\n---\n\n1. Read the diff.\n2. List bugs first, then tests, then naming.\n',
      },
    ],
  ]);
  const placed = new Map<string, string>([["work:skill:review-pr", "older text"]]);
  const accountOnly = new Map<string, { kind: ItemKind; name: string; text: string }[]>([
    ["personal", [{ kind: "command", name: "standup", text: '---\ndescription: "Write my standup from today\'s commits"\n---\nSummarise.\n' }]],
  ]);
  const sampleLint = (kind: ItemKind, text: string): Lint => {
    const findings: Finding[] = [];
    const m = /^---\n([\s\S]*?)\n---/.exec(text);
    if (!m) {
      if (kind !== "command") findings.push({ severity: "error", message: "No frontmatter: the file must start with a --- block holding name and description." });
      return { findings, descriptionChars: 0, description: "" };
    }
    const d = /^description:\s*(.*)$/m.exec(m[1]!)?.[1]?.replace(/^"|"$/g, "") ?? "";
    if (!d && kind !== "command") findings.push({ severity: "error", message: "Missing description. Claude decides when to use this from the description." });
    if (d.length > DESC_BUDGET) findings.push({ severity: "warn", message: `Description and when_to_use are ${d.length} characters; Claude Code cuts them at ${DESC_BUDGET}. Put the trigger words first.` });
    else if (d && d.length < 40) findings.push({ severity: "warn", message: "The description is very short. Say what it does and when to use it." });
    if (text.includes("!`")) findings.push({ severity: "risk", message: "Runs shell commands when it loads (!`command`)." });
    return { findings, descriptionChars: d.length, description: d };
  };
  const toItem = (kind: ItemKind, name: string, text: string): AccountItem => {
    const l = sampleLint(kind, text);
    return { kind, name, path: `${HOME}/.local/share/facet/library/${relPath(kind, name)}`, description: l.description, hash: String(text.length), lint: l.findings };
  };
  const libGet = (kind: ItemKind, name: string) => {
    const it = library.get(`${kind}:${name}`);
    if (!it) throw `Not found: ${name} in the library`;
    return it;
  };
  const pretty = (s: Settings | null) => (s ? JSON.stringify(s, null, 2) + "\n" : "");
  const hashOf = (s: Settings | null) => String(pretty(s).length) + ":" + pretty(s).slice(0, 40);
  const checkOps = (ops: { path: string[]; value: unknown }[]) => {
    for (const op of ops) {
      if (op.path[0] === "env" && op.path[1] && isRefusedEnv(op.path[1]))
        throw `Facet does not set ${op.path[1]}: it would replace the account's sign-in, redirect its traffic or weaken TLS.`;
      if (op.path[0] === "cleanupPeriodDays" && op.value !== null && !(Number(op.value) >= 1)) throw "cleanupPeriodDays must be a whole number of at least 1.";
    }
  };
  const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
  const listeners = new Set<(s: AccountState[]) => void>();

  const states = (): AccountState[] =>
    accounts.map((a) => ({
      id: a.id,
      desktopPid: running.get(a.id) ?? null,
      launchedByFacet: a.id !== "personal" && running.has(a.id),
      liveSessions: a.id === "work" ? 2 : 1,
      waitingSessions: a.id === "work" ? 1 : 0,
      shimInstalled: a.id === "work",
      launcherInstalled: a.id === "work",
    }));
  const emit = () => listeners.forEach((cb) => cb(states()));
  const find = (id: string) => {
    const a = accounts.find((x) => x.id === id);
    if (!a) throw `Not found: account ${id}`;
    return a;
  };

  return {
    isSample: true,
    async appInfo() {
      return {
        version: "0.1.0",
        platform: "linux",
        desktop: { path: "/usr/lib/claude-desktop/claude-desktop", version: "sample", channel: "deb", official: true },
        code: { path: `${HOME}/.local/bin/claude`, version: "sample", channel: "path", official: true },
        shortcut: "portal",
        storePath: `${HOME}/.config/facet/profiles.toml`,
        home: HOME,
      };
    },
    async listAccounts() {
      return accounts.map((a) => ({ ...a }));
    },
    async accountStates() {
      return states();
    },
    async saveAccount(input: AccountInput) {
      const name = input.name.trim();
      if (!name) throw "Give the account a name.";
      const id = input.id ?? slugify(name);
      if (!id) throw `Account id "${id}" must be 1–32 lowercase letters, digits or dashes.`;
      const existing = accounts.find((a) => a.id === id);
      if (!input.id && existing) throw `An account called "${id}" already exists.`;
      const first = accounts.length === 0 && !existing;
      const defs = defaultDirs(HOME, id, first);
      const codeDir = input.codeDir || defs.codeDir;
      const clash = accounts.find((a) => a.id !== id && a.codeDir === codeDir);
      if (clash) throw `${clash.name} already uses ${codeDir}. Each account needs its own Claude Code folder.`;
      if (input.expectedEmail && !input.expectedEmail.includes("@"))
        throw `"${input.expectedEmail}" does not look like an email address.`;
      const account: Account = {
        id,
        name,
        color: input.color ?? firstUnusedColor(accounts),
        plan: input.plan?.trim() || undefined,
        desktopDir: input.withDesktop ? input.desktopDir || defs.desktopDir : undefined,
        codeDir,
        expectedEmail: input.expectedEmail?.trim() || undefined,
        tint: input.tint ?? true,
        x11Identity: input.x11Identity ?? !first,
        createdAt: existing?.createdAt ?? Date.now(),
        badge: input.badge?.trim() || undefined,
        notes: input.notes?.trim() || undefined,
        launcherIcon: input.launcherIcon ?? existing?.launcherIcon ?? "claude-badge",
        terminalDir: input.terminalDir?.trim() || undefined,
        launcherName: input.launcherName?.trim() || undefined,
        imageUrl: existing?.imageUrl ?? null,
      };
      accounts = existing ? accounts.map((a) => (a.id === id ? account : a)) : [...accounts, account];
      emit();
      return account;
    },
    async setAccountImage(id, data) {
      const a = find(id);
      if (data !== null && !/^data:image\/(png|jpeg|webp|gif|svg\+xml);base64,/.test(data)) throw "Facet takes PNG, JPEG, WebP, GIF or SVG pictures.";
      const next = { ...a, imageUrl: data };
      accounts = accounts.map((x) => (x.id === id ? next : x));
      emit();
      return next;
    },
    async reorderAccounts(ids) {
      const pos = (a: Account) => {
        const i = ids.indexOf(a.id);
        return i < 0 ? Number.MAX_SAFE_INTEGER : i;
      };
      accounts = [...accounts].sort((x, y) => pos(x) - pos(y));
      emit();
      return accounts.map((a) => ({ ...a }));
    },
    async renameAccount(id, newId) {
      const a = find(id);
      if (newId === id) return a;
      if (!/^[a-z0-9][a-z0-9-]{0,31}$/.test(newId)) throw `Account id "${newId}" must be 1–32 lowercase letters, digits or dashes.`;
      if (["desktop", "code", "auto", "cli", "app", "facet"].includes(newId)) throw `"${newId}" is reserved because claude-${newId} could hide another command. Pick another name.`;
      if (accounts.some((x) => x.id === newId)) throw `An account called "${newId}" already exists.`;
      if (running.has(id)) throw `Quit ${a.name}'s Desktop app first: its windows are named after the old short name.`;
      const next = { ...a, id: newId };
      accounts = accounts.map((x) => (x.id === id ? next : x));
      rules = rules.map((r) => (r.accountId === id ? { ...r, accountId: newId } : r));
      emit();
      return next;
    },
    async usageReport(days) {
      await sleep(50);
      return sampleUsage(accounts.map((a) => a.id), days);
    },
    async resumeSession(id, sessionId) {
      find(id);
      if (!/^[A-Za-z0-9-]{1,64}$/.test(sessionId)) throw "That does not look like a session id.";
      return 6001;
    },
    async timelineEvents(since, until) {
      await sleep(20);
      return sampleEvents(accounts.map((a) => a.id), since, until);
    },
    async bridgeDraft(fromId, sessionId, toId, note, includeGit) {
      const from = find(fromId);
      const to = find(toId);
      if (fromId === toId) throw "Pick a different account to hand off to.";
      const s = sampleUsage([fromId], 365).accounts[0]!.sessions.find((x) => x.sessionId === sessionId);
      if (!s) throw `Not found: session ${sessionId}`;
      const text = [`# Handoff from ${from.name} to ${to.name}`, "", "## Where the work is", "", `- Folder: \`${s.project}\``, `- Branch: \`${s.branch}\``, `- Session in ${from.name}: \`${s.sessionId}\``, "", "## Note", "", note.trim() || "(No note.)", "", ...(includeGit ? ["## Changed files", "", "```", " M src/app.ts", "```", ""] : []), "## For the session that reads this", "", "Read the note and the changes above, then continue the work."].join("\n");
      return { name: `20261026T120000-${fromId}-to-${toId}.md`, path: "", text };
    },
    async bridgeSave(name, text) {
      if (/ghp_[A-Za-z0-9]{20,}|sk-ant-/.test(text)) throw "The handoff looks like it holds a secret (a GitHub token). Remove it, then save.";
      handoffs.set(name, { text, at: Date.now() });
      return { name, path: `${HOME}/.local/share/facet/handoffs/${name}`, text };
    },
    async bridgeList() {
      return [...handoffs.entries()].map(([name, h]) => ({ name, path: `${HOME}/.local/share/facet/handoffs/${name}`, bytes: h.text.length, modifiedMs: h.at, title: h.text.split("\n")[0]!.replace(/^# /, "") })).sort((a, b) => b.modifiedMs - a.modifiedMs);
    },
    async bridgeRemove(name) {
      if (!handoffs.delete(name)) throw `Not found: ${name}`;
    },
    async bridgeOpen(toId, name) {
      find(toId);
      if (!handoffs.has(name)) throw `Not found: ${name}`;
      return 6002;
    },
    async linksState() {
      const suggestions: Record<string, import("./api").LinkTarget> = {};
      for (const a of accounts) {
        for (const b of sampleBrowsers) {
          const p = b.profiles.find((x) => a.expectedEmail && x.emails.includes(a.expectedEmail));
          if (p && !suggestions[a.id]) suggestions[a.id] = { browser: b.id, profile: p.dir };
        }
      }
      return { settings: structuredClone(links), browsers: sampleBrowsers, suggestions, currentDefault: links.enabled ? "facet-links.desktop" : "com.google.Chrome.desktop", recent: links.enabled ? [{ ms: Date.now() - 60_000, host: "claude.ai", accountId: "work", browser: "chrome", profile: "Profile 1" }] : [] };
    },
    async linksSetAccount(id, target) {
      find(id);
      if (target) links.accounts[id] = target;
      else delete links.accounts[id];
      return structuredClone(links);
    },
    async linksSetFallback(target) {
      links.fallback = target;
      return structuredClone(links);
    },
    async linksEnable() {
      links.enabled = true;
      links.previousDefault = "com.google.Chrome.desktop";
      return structuredClone(links);
    },
    async linksDisable() {
      links.enabled = false;
      return structuredClone(links);
    },
    async linksTest() {},
    async mcpAdd(id, input) {
      find(id);
      if (!/^[A-Za-z0-9_][A-Za-z0-9_-]{0,63}$/.test(input.name)) throw "Name the server with letters, digits, - or _ (up to 64).";
      if (input.env.some(([k]) => isRefusedEnv(k))) throw "Facet does not pass that variable to an MCP server: it would carry the account's sign-in or redirect its traffic.";
      if (input.transport !== "stdio" && !/^https?:\/\//.test(input.commandOrUrl)) throw "The address must start with https://.";
      const list = addedMcp.get(id) ?? [];
      list.push({ name: input.name, transport: input.transport, command: input.transport === "stdio" ? input.commandOrUrl : null, args: input.args, url: input.transport === "stdio" ? null : input.commandOrUrl, envNames: input.env.map(([k]) => k), headerNames: input.headers.map(([k]) => k), scope: input.scope, project: input.folder, notes: [] });
      addedMcp.set(id, list);
      removedMcp.delete(`${id}:${input.name}`);
      return `Added ${input.transport} MCP server ${input.name} to ${input.scope} config`;
    },
    async mcpRemove(id, name) {
      find(id);
      addedMcp.set(id, (addedMcp.get(id) ?? []).filter((m) => m.name !== name));
      removedMcp.add(`${id}:${name}`);
      return `Removed MCP server ${name}`;
    },
    async ecosystemInventory(id) {
      find(id);
      await sleep(30);
      const inv = sampleInventory(id, pluginEnabled, pluginReviewed);
      inv.mcp = [...inv.mcp.filter((m) => !(removedMcp.has(`${id}:${m.name}`))), ...(addedMcp.get(id) ?? [])];
      // Plugin switches made through Control show up here.
      const st = settings.get(id) as { enabledPlugins?: Record<string, boolean> } | null | undefined;
      for (const p of inv.plugins) if (st?.enabledPlugins && p.id in st.enabledPlugins) p.enabled = st.enabledPlugins[p.id]!;
      return inv;
    },
    async ecosystemScan(path) {
      if (!path.startsWith("/")) throw "Use a full folder path, for example ~/Downloads/plugin.";
      await sleep(30);
      return { ...SAMPLE_HIGH, root: path };
    },
    async ecosystemMarkReviewed(id, pluginId, hash) {
      find(id);
      if (hash) pluginReviewed.set(`${id}:${pluginId}`, hash);
      else pluginReviewed.delete(`${id}:${pluginId}`);
    },
    async exportText(name) {
      return `${HOME}/Downloads/${name}`;
    },
    async launcherPreview(id, o) {
      const a = { ...find(id), ...(o?.name ? { name: o.name } : {}) };
      const ch = (o?.badge ?? a.badge ?? a.name.match(/[\p{L}\p{N}]/u)?.[0] ?? "?").toUpperCase();
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect x="8" y="8" width="112" height="112" rx="30" fill="gray"/><text x="64" y="84" font-size="60" text-anchor="middle" fill="white">${ch}</text></svg>`;
      return `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
    },
    async deleteAccount(id) {
      find(id);
      accounts = accounts.filter((a) => a.id !== id);
      running.delete(id);
      emit();
    },
    async detectExisting(): Promise<DetectedSetup[]> {
      const sample: DetectedSetup[] = [
        { suggestedId: "client-acme", suggestedName: "Client Acme", desktopDir: null, codeDir: `${HOME}/.claude-client-acme` },
      ];
      return sample.filter((d) => !accounts.some((a) => a.codeDir === d.codeDir));
    },
    async launchDesktop(id) {
      const a = find(id);
      if (!a.desktopDir) throw `${a.name} has no Desktop app set up.`;
      await sleep(400);
      const pid = running.get(id) ?? 5000 + Math.floor(Math.random() * 999);
      running.set(id, pid);
      emit();
      return pid;
    },
    async quitDesktop(id) {
      const a = find(id);
      if (!running.has(id)) throw `Not found: a running Desktop app for ${a.name}`;
      await sleep(200);
      running.delete(id);
      emit();
    },
    async openTerminal(id) {
      find(id);
      return 6000;
    },
    async installShim(id) {
      return `${HOME}/.local/bin/claude-${find(id).id}`;
    },
    async installLauncher(id) {
      return `${HOME}/.local/share/applications/facet-${find(id).id}.desktop`;
    },
    async checkSignIn(id): Promise<AuthStatus> {
      const a = find(id);
      await sleep(300);
      return {
        loggedIn: true,
        authMethod: "claude.ai",
        email: a.expectedEmail ?? "you@example.com",
        orgName: a.id === "work" ? "Company (sample)" : null,
        subscriptionType: a.plan?.toLowerCase() ?? "pro",
        configDirectory: a.codeDir,
      };
    },
    async runHealth(checkSignIn): Promise<HealthCheck[]> {
      await sleep(300);
      const checks: HealthCheck[] = [
        {
          id: "retention:personal",
          status: "warn",
          title: "Claude Code deletes Personal's transcripts after 30 days",
          detail: "Older sessions disappear from Claude Code, and from Facet's Timeline and Costs.",
          fix: "Raise cleanupPeriodDays in this account's settings.json (Facet's Control view will do it with a backup in Phase 2).",
          accountId: "personal",
        },
        { id: "desktop-install", status: "ok", title: "Claude Desktop is installed", detail: "/usr/lib/claude-desktop/claude-desktop · sample" },
        { id: "code-install", status: "ok", title: "Claude Code is installed", detail: `${HOME}/.local/bin/claude · sample` },
        { id: "env-facet", status: "ok", title: "No override variables where Facet runs", detail: "None of the sign-in override variables are set in Facet's environment." },
        { id: "shortcut", status: "ok", title: "Global shortcut available", detail: "The desktop offers the GlobalShortcuts portal." },
      ];
      if (checkSignIn) {
        for (const a of accounts) {
          checks.push({ id: `auth:${a.id}`, status: "ok", title: `${a.name} is signed in`, detail: `${a.expectedEmail ?? "you@example.com"} · sample`, accountId: a.id });
        }
      }
      return checks;
    },
    async onAccountStates(cb) {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    async listRules() {
      return rules.map((r) => ({ ...r }));
    },
    async saveRules(next) {
      const seen = new Set<string>();
      const clean: FolderRule[] = [];
      for (const r of next) {
        let folder = r.folder.trim().replace(/\/+$/, "");
        if (folder === "~" || folder.startsWith("~/")) folder = HOME + folder.slice(1);
        if (!folder.startsWith("/")) throw `Use a full folder path, for example ~/work (got "${r.folder}").`;
        if (folder === "") throw "A rule for / would cover everything; pick a narrower folder.";
        if (!accounts.some((a) => a.id === r.accountId)) throw `No account called "${r.accountId}".`;
        if (seen.has(folder)) throw `${folder} has two rules. Keep one.`;
        seen.add(folder);
        clean.push({ folder, accountId: r.accountId });
      }
      rules = clean.sort((a, b) => a.folder.localeCompare(b.folder));
      return { rules: rules.map((r) => ({ ...r })), commandUpdated: autoInstalled ? `${HOME}/.local/bin/claude-auto` : null };
    },
    async installAutoCommand() {
      autoInstalled = true;
      return `${HOME}/.local/bin/claude-auto`;
    },
    async autoCommandInfo() {
      return { path: `${HOME}/.local/bin/claude-auto`, installed: autoInstalled };
    },
    async takeSwitcherRequest() {
      return false;
    },
    async onOpenSwitcher() {
      return () => {};
    },
    async onNotice() {
      return () => {};
    },
    async setTitle(title) {
      document.title = title;
    },
    async controlRead(id) {
      find(id);
      const s = settings.get(id) ?? null;
      return { current: { path: `${find(id).codeDir}/settings.json`, exists: s !== null, settings: hideSecrets(s), hash: hashOf(s), blocked: null }, managedPath: null };
    },
    async controlPreview(id, ops) {
      const a = find(id);
      checkOps(ops);
      const before = settings.get(id) ?? null;
      const after = applyOps(before, ops);
      const diff = diffLines(pretty(hideSecrets(before)), pretty(hideSecrets(after)));
      const warnings: string[] = [];
      if ((after.permissions as Settings | undefined)?.defaultMode === "bypassPermissions") warnings.push("bypassPermissions lets Claude Code run tools without asking. Use it only in a sandbox.");
      return { path: `${a.codeDir}/settings.json`, baseHash: hashOf(before), diff, changed: diff.some((l) => l.kind !== "same"), formatOnly: false, warnings };
    },
    async controlApply(id, ops, baseHash) {
      const a = find(id);
      checkOps(ops);
      const before = settings.get(id) ?? null;
      if (hashOf(before) !== baseHash) throw "settings.json changed since you reviewed it (maybe Claude Code or an editor wrote it). Review the change again.";
      let backup: string | null = null;
      if (before) {
        tick += 1;
        const name = `2026102${tick}T000000Z-000-settings.json`;
        backup = `${HOME}/.local/share/facet/backups/${id}/${name}`;
        backups.set(id, [{ b: { name, bytes: pretty(before).length }, text: pretty(before) }, ...(backups.get(id) ?? [])]);
      }
      settings.set(id, applyOps(before, ops));
      return { path: `${a.codeDir}/settings.json`, backup };
    },
    async controlBackups(id) {
      return (backups.get(id) ?? []).map((x) => x.b);
    },
    async studioList() {
      return [...library.values()].map((it) => ({
        ...toItem(it.kind, it.name, it.text),
        placements: accounts.map((a) => {
          const p = placed.get(`${a.id}:${it.kind}:${it.name}`);
          return { accountId: a.id, state: p === undefined ? ("absent" as const) : p === it.text ? ("in-sync" as const) : ("drifted" as const) };
        }),
      }));
    },
    async studioRead(kind, name) {
      return libGet(kind, name).text;
    },
    async studioLint(kind, _name, text) {
      return sampleLint(kind, text);
    },
    async studioTemplate(kind, name, description, body) {
      if (!validName(name)) throw `"${name}" must be 1–64 lowercase letters, digits or dashes, for example review-pr.`;
      const d = JSON.stringify(description.split(/\s+/).filter(Boolean).join(" "));
      const b = body.trim() ? body.trimEnd() + "\n" : "Steps Claude should follow:\n\n1. \n";
      return kind === "command" ? `---\ndescription: ${d}\n---\n\n${b}` : `---\nname: ${name}\ndescription: ${d}\n---\n\n${b}`;
    },
    async studioSave(kind, name, text) {
      if (!validName(name)) throw `"${name}" must be 1–64 lowercase letters, digits or dashes, for example review-pr.`;
      if (/sk-ant-|ghp_|github_pat_|-----BEGIN/.test(text)) throw "This looks like it contains a secret. The library never stores secrets; use an environment variable reference instead.";
      library.set(`${kind}:${name}`, { kind, name, text });
      return toItem(kind, name, text);
    },
    async studioRemove(kind, name) {
      libGet(kind, name);
      library.delete(`${kind}:${name}`);
    },
    async studioPlacePreview(kind, name, accountId) {
      const a = find(accountId);
      const it = libGet(kind, name);
      const cur = placed.get(`${accountId}:${kind}:${name}`) ?? "";
      const l = sampleLint(kind, it.text);
      return {
        target: `${a.codeDir}/${relPath(kind, name)}`,
        state: cur === "" ? "absent" : cur === it.text ? "in-sync" : "drifted",
        baseHash: String(cur.length),
        diff: diffLines(cur, it.text),
        risks: l.findings.filter((f) => f.severity === "risk"),
        blockers: l.findings.filter((f) => f.severity === "error"),
        liveSessions: accountId === "work" ? 2 : 0,
      };
    },
    async studioPlace(kind, name, accountId, baseHash, riskAck) {
      const a = find(accountId);
      const it = libGet(kind, name);
      const l = sampleLint(kind, it.text);
      if (l.findings.some((f) => f.severity === "error")) throw "Fix this first: " + l.findings.find((f) => f.severity === "error")!.message;
      if (l.findings.some((f) => f.severity === "risk") && !riskAck) throw "This item runs commands on this computer. Confirm that you understand before placing it.";
      const key = `${accountId}:${kind}:${name}`;
      const cur = placed.get(key) ?? "";
      if (String(cur.length) !== baseHash) throw "The account's copy changed since you reviewed it. Review again.";
      placed.set(key, it.text);
      return cur && cur !== it.text ? `${HOME}/.local/share/facet/backups/${a.id}/studio/sample-${kind}-${name}.md` : null;
    },
    async studioUnplace(kind, name, accountId) {
      const key = `${accountId}:${kind}:${name}`;
      if (!placed.has(key)) throw `Not found: ${name} in this account`;
      placed.delete(key);
      return `${HOME}/.local/share/facet/backups/${accountId}/studio/sample-${kind}-${name}.md`;
    },
    async studioAccountItems(accountId) {
      find(accountId);
      const own = (accountOnly.get(accountId) ?? []).map((x) => toItem(x.kind, x.name, x.text));
      const fromLib = [...placed.keys()]
        .filter((k) => k.startsWith(accountId + ":"))
        .map((k) => {
          const [, kind, name] = k.split(":") as [string, ItemKind, string];
          return toItem(kind, name, placed.get(k)!);
        });
      return [...own, ...fromLib];
    },
    async studioImport(accountId, kind, name) {
      const src = (accountOnly.get(accountId) ?? []).find((x) => x.kind === kind && x.name === name);
      const text = src?.text ?? placed.get(`${accountId}:${kind}:${name}`);
      if (!text) throw `Not found: ${name} in this account`;
      if (/sk-ant-|ghp_|github_pat_|-----BEGIN/.test(text)) throw "This looks like it contains a secret. The library never stores secrets; use an environment variable reference instead.";
      library.set(`${kind}:${name}`, { kind, name, text });
      return toItem(kind, name, text);
    },
    async controlRestorePreview(id, name) {
      const a = find(id);
      const hit = (backups.get(id) ?? []).find((x) => x.b.name === name);
      if (!hit) throw "That is not one of this account's settings backups.";
      const before = settings.get(id) ?? null;
      const diff = diffLines(pretty(hideSecrets(before)), pretty(hideSecrets(JSON.parse(hit.text) as Settings)));
      return { path: `${a.codeDir}/settings.json`, baseHash: hashOf(before), diff, changed: diff.some((l) => l.kind !== "same"), formatOnly: false, warnings: [] };
    },
    async controlRestore(id, name, baseHash) {
      const a = find(id);
      const hit = (backups.get(id) ?? []).find((x) => x.b.name === name);
      if (!hit) throw "That is not one of this account's settings backups.";
      const before = settings.get(id) ?? null;
      if (hashOf(before) !== baseHash) throw "settings.json changed since you reviewed the restore. Review again.";
      settings.set(id, JSON.parse(hit.text) as Settings);
      return { path: `${a.codeDir}/settings.json`, backup: null };
    },
    async controlEffective(id, project) {
      const a = find(id);
      const acc = hideSecrets(settings.get(id) ?? null) ?? {};
      const layers = [
        { name: "managed", path: "/etc/claude-code/managed-settings.json", exists: false, problem: null },
        ...(project
          ? [
              { name: "project-local", path: `${project}/.claude/settings.local.json`, exists: false, problem: null },
              { name: "project", path: `${project}/.claude/settings.json`, exists: true, problem: null },
            ]
          : []),
        { name: "account", path: `${a.codeDir}/settings.json`, exists: settings.get(id) != null, problem: null },
      ];
      const entries: { key: string; value: unknown; sources: string[]; overridden: string[] }[] = [];
      const walk = (prefix: string, v: unknown, source: string) => {
        if (v && typeof v === "object" && !Array.isArray(v)) for (const [k, c] of Object.entries(v)) walk(prefix ? `${prefix}.${k}` : k, c, source);
        else entries.push({ key: prefix, value: v, sources: [source], overridden: [] });
      };
      if (project) walk("", { permissions: { allow: ["Bash(npm test)"] } }, "project");
      walk("", acc, "account");
      const merged = new Map<string, (typeof entries)[number]>();
      for (const e of entries) {
        const have = merged.get(e.key);
        if (!have) merged.set(e.key, e);
        else if (Array.isArray(have.value) && Array.isArray(e.value) && e.key.startsWith("permissions.")) {
          have.value = [...have.value, ...e.value.filter((x) => !(have.value as unknown[]).includes(x))];
          have.sources.push(...e.sources);
        } else have.overridden.push(...e.sources);
      }
      return { layers, entries: [...merged.values()].sort((x, y) => x.key.localeCompare(y.key)) };
    },
    async controlPolicy() {
      return { refusedEnv: REFUSED_ENV, warnEnv: ["HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY", "NODE_EXTRA_CA_CERTS"], hiddenMarker: HIDDEN_MARKER };
    },
  };
}
