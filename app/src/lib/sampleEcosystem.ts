// Sample Ecosystem data for the browser preview and tests.
import type { Inventory, PackageReport } from "./ecosystem";

const lowReport: PackageReport = {
  root: "/home/you/.claude/plugins/cache/official/frontend-design/1.2.0",
  kind: "plugin",
  name: "frontend-design",
  version: "1.2.0",
  description: "Design guidance for web front ends.",
  hash: "00000000000000a1",
  skills: [{ name: "frontend-design", path: "skills/frontend-design/SKILL.md", findings: [] }],
  commands: [],
  agents: [],
  hooks: [],
  mcp: [],
  bin: [],
  endpoints: ["fonts.googleapis.com"],
  envVars: [],
  findings: [],
  risk: "low",
  reasons: ["Only instructions and text; nothing runs on its own."],
  files: 4,
  bytes: 12000,
  truncated: false,
};

export const SAMPLE_HIGH: PackageReport = {
  root: "/home/you/.claude/plugins/cache/acme/deploy-kit/0.4.1",
  kind: "plugin",
  name: "deploy-kit",
  version: "0.4.1",
  description: "Deploy helpers.",
  hash: "00000000000000b2",
  skills: [{ name: "deploy", path: "skills/deploy/SKILL.md", findings: [{ severity: "risk", message: "Runs shell commands when it loads (!`command`)." }] }],
  commands: [{ name: "ship", path: "commands/ship.md", findings: [] }],
  agents: [],
  hooks: [
    { event: "SessionStart", matcher: null, kind: "command", target: "${CLAUDE_PLUGIN_ROOT}/bin/setup", source: "hooks/hooks.json", notes: ["Runs every time a session starts."] },
    { event: "PostToolUse", matcher: "Edit", kind: "http", target: "https://hooks.acme.example/edit", source: "hooks/hooks.json", notes: ["Sends hook data to a web address."] },
  ],
  mcp: [{ name: "acme-db", transport: "stdio", command: "npx", args: ["-y", "acme-db-mcp"], url: null, envNames: ["DB_PASSWORD"], headerNames: [], scope: "plugin", project: null, notes: ["Starts acme-db-mcp without a fixed version, so it can change on any start.", "Runs a program on this computer when a session starts.", "Needs a secret (key or token) to work."] }],
  bin: ["setup"],
  endpoints: ["hooks.acme.example", "get.acme.example"],
  envVars: ["ACME_TOKEN", "DB_PASSWORD"],
  findings: [{ severity: "risk", message: "bin/setup: Downloads and runs a script (curl or wget piped into a shell)." }],
  risk: "high",
  reasons: ["1 skill, command or agent file(s) run shell commands or download scripts.", "2 hook(s) run automatically, 2 of them on every session, prompt or tool call.", "1 MCP server(s) give Claude new tools.", "An MCP server is unpinned or unencrypted, so what runs can change.", "Adds 1 program(s) to PATH while Claude runs."],
  files: 11,
  bytes: 48000,
  truncated: false,
};

export function sampleInventory(id: string, enabled: Map<string, boolean>, reviewed: Map<string, string>): Inventory {
  const plugins =
    id === "personal"
      ? [
          { id: "frontend-design@official", name: "frontend-design", marketplace: "official", version: "1.2.0", scope: "user", installPath: lowReport.root, installedAt: "2026-10-01T10:00:00Z", lastUpdated: "2026-10-20T10:00:00Z", gitSha: "abc1234", report: lowReport },
          { id: "deploy-kit@acme", name: "deploy-kit", marketplace: "acme", version: "0.4.1", scope: "user", installPath: SAMPLE_HIGH.root, installedAt: "2026-10-12T10:00:00Z", lastUpdated: "2026-10-25T10:00:00Z", gitSha: null, report: SAMPLE_HIGH },
        ]
      : [];
  return {
    accountId: id,
    plugins: plugins.map((p) => ({ ...p, enabled: enabled.get(`${id}:${p.id}`) ?? true, reviewedHash: reviewed.get(`${id}:${p.id}`) ?? null })),
    marketplaces:
      id === "personal"
        ? [
            { name: "official", source: "github", location: "anthropics/claude-plugins-official", pinned: false, lastUpdated: "2026-10-25" },
            { name: "acme", source: "git", location: "https://git.acme.example/plugins.git", pinned: false, lastUpdated: "2026-10-25" },
          ]
        : [],
    mcp:
      id === "personal"
        ? [
            { name: "context7", transport: "http", command: null, args: [], url: "https://mcp.context7.example/mcp", envNames: [], headerNames: ["Authorization"], scope: "user", project: null, notes: ["Needs a secret (key or token) to work."] },
            { name: "notes", transport: "stdio", command: "node", args: ["/home/you/tools/notes.js"], url: null, envNames: [], headerNames: [], scope: "local", project: "/home/you/code/blog", notes: ["Runs a program on this computer when a session starts."] },
          ]
        : [],
    hooks: id === "personal" ? [{ event: "Stop", matcher: null, kind: "command", target: "notify-send 'Claude is done'", source: "settings.json", notes: [] }] : [],
    settingsFindings: [],
    skills: id === "personal" ? 3 : 0,
    agents: 0,
    commands: id === "personal" ? 1 : 0,
    problems: [],
  };
}
