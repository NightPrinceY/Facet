// Usage and cost (PRD M3, M6, Today). Types mirror usage.rs; helpers group
// the core's hourly buckets by the viewer's local day.

export interface Tokens {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  webSearches: number;
}

export interface SessionSummary {
  accountId: string;
  sessionId: string;
  project: string | null;
  branch: string | null;
  startedMs: number;
  endedMs: number;
  models: string[];
  entrypoint: string | null;
  version: string | null;
  prompts: number;
  turns: number;
  tools: Record<string, number>;
  tokens: Tokens;
  cost: number;
  unpricedModels: string[];
  reportedCost: number | null;
  subagents: number;
  bytes: number;
}

export interface HourBucket {
  hourMs: number;
  model: string;
  project: string | null;
  tokens: Tokens;
  cost: number;
  turns: number;
  prompts: number;
}

export interface AccountUsage {
  accountId: string;
  sessions: SessionSummary[];
  hours: HourBucket[];
}

export interface Price {
  prefix: string;
  input: number;
  output: number;
  cacheWrite5m: number;
  cacheWrite1h: number;
  cacheRead: number;
}

export interface UsageReport {
  generatedMs: number;
  sinceMs: number;
  accounts: AccountUsage[];
  pricesChecked: string;
  pricesSource: string;
  prices: Price[];
}

export const ZERO: Tokens = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, webSearches: 0 };

export function addTokens(a: Tokens, b: Tokens): Tokens {
  return { input: a.input + b.input, output: a.output + b.output, cacheRead: a.cacheRead + b.cacheRead, cacheWrite: a.cacheWrite + b.cacheWrite, webSearches: a.webSearches + b.webSearches };
}

export function totalTokens(t: Tokens): number {
  return t.input + t.output + t.cacheRead + t.cacheWrite;
}

/** DESIGN-SYSTEM.md data labels: "$1.23", "$1,234.50", "$0.0040" under a cent. */
export function money(n: number): string {
  if (n > 0 && n < 0.01) return `$${n.toFixed(4)}`;
  return `$${n.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

/** 1234 → "1.2K", 3_400_000 → "3.4M". */
export function compact(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1e6) return `${(n / 1e3).toFixed(n < 1e4 ? 1 : 0)}K`;
  if (n < 1e9) return `${(n / 1e6).toFixed(n < 1e7 ? 1 : 0)}M`;
  return `${(n / 1e9).toFixed(1)}B`;
}

/** "claude-opus-5-5-20261001" → "Opus 5.5". */
export function modelLabel(id: string): string {
  const m = /^claude-([a-z]+)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/.exec(id) ?? /^claude-(\d+)-(\d+)-([a-z]+)/.exec(id);
  if (!m) return id || "Unknown";
  if (/^\d/.test(m[1]!)) return `${m[3]![0]!.toUpperCase()}${m[3]!.slice(1)} ${m[1]}.${m[2]}`;
  return `${m[1]![0]!.toUpperCase()}${m[1]!.slice(1)} ${m[2]}${m[3] ? `.${m[3]}` : ""}`;
}

/** Local calendar day, "2026-10-26". */
export function dayKey(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function startOfLocalDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** The last `n` local days, oldest first, ending today. */
export function lastDays(n: number, now = Date.now()): string[] {
  const out: string[] = [];
  const d = new Date(startOfLocalDay(now));
  for (let i = n - 1; i >= 0; i--) {
    const x = new Date(d);
    x.setDate(d.getDate() - i);
    out.push(dayKey(x.getTime()));
  }
  return out;
}

/** Last folder name of a project path ("~/work/app" → "app"). */
export function projectName(path: string | null): string {
  if (!path) return "Unknown folder";
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export const ENTRYPOINT_LABEL: Record<string, string> = {
  cli: "Terminal",
  "claude-desktop": "Desktop app",
  "claude-vscode": "VS Code",
  "sdk-cli": "SDK",
  "sdk-ts": "SDK",
  "sdk-py": "SDK",
};

export function entrypointLabel(e: string | null): string {
  return e ? (ENTRYPOINT_LABEL[e] ?? e) : "Unknown";
}

export interface Rollup {
  key: string;
  cost: number;
  tokens: Tokens;
  turns: number;
  prompts: number;
  sessions: number;
  accounts: Set<string>;
}

function rollup(key: string): Rollup {
  return { key, cost: 0, tokens: { ...ZERO }, turns: 0, prompts: 0, sessions: 0, accounts: new Set() };
}

/** Sum hourly buckets by a key (day, model, project, account). */
export function groupHours(accounts: AccountUsage[], keyOf: (h: HourBucket, accountId: string) => string, sinceMs = 0): Map<string, Rollup> {
  const out = new Map<string, Rollup>();
  for (const a of accounts) {
    for (const h of a.hours) {
      if (h.hourMs + 3_600_000 <= sinceMs) continue;
      const k = keyOf(h, a.accountId);
      const r = out.get(k) ?? rollup(k);
      r.cost += h.cost;
      r.tokens = addTokens(r.tokens, h.tokens);
      r.turns += h.turns;
      r.prompts += h.prompts;
      r.accounts.add(a.accountId);
      out.set(k, r);
    }
  }
  return out;
}

/** Sessions touched since `sinceMs`, newest first. */
export function sessionsSince(accounts: AccountUsage[], sinceMs: number): SessionSummary[] {
  return accounts
    .flatMap((a) => a.sessions)
    .filter((s) => s.endedMs >= sinceMs)
    .sort((a, b) => b.endedMs - a.endedMs);
}

export function relativeTime(ms: number, now = Date.now()): string {
  const s = Math.round((now - ms) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  const days = Math.round(s / 86400);
  if (days < 14) return `${days} day${days === 1 ? "" : "s"} ago`;
  return new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

export function duration(ms: number): string {
  const m = Math.round(ms / 60000);
  if (m < 1) return "<1 min";
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60} min`;
}

/** Top tool names of a session, most used first. */
export function topTools(tools: Record<string, number>, n = 3): [string, number][] {
  return Object.entries(tools)
    .sort((a, b) => b[1] - a[1])
    .slice(0, n);
}

/** MCP tool names carry the server: "mcp__github__create_issue" → "github: create_issue". */
export function toolLabel(name: string): string {
  const m = /^mcp__(.+?)__(.+)$/.exec(name);
  if (!m) return name;
  const server = /^[0-9a-f-]{20,}$/.test(m[1]!) ? "connector" : m[1]!.replace(/^claude_ai_/, "");
  return `${server}: ${m[2]}`;
}

function csvCell(v: string | number): string {
  let s = String(v);
  // A cell starting with = + - @ runs as a formula in spreadsheets.
  if (typeof v === "string" && /^[=+\-@\t\r]/.test(s)) s = `'${s}`;
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** One row per session, for invoices and spreadsheets. */
export function sessionsCsv(sessions: SessionSummary[], accountName: (id: string) => string): string {
  const head = ["account", "session_id", "project", "branch", "started", "ended", "models", "prompts", "replies", "input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens", "web_searches", "api_equivalent_usd"];
  const rows = sessions.map((s) => [
    accountName(s.accountId),
    s.sessionId,
    s.project ?? "",
    s.branch ?? "",
    new Date(s.startedMs).toISOString(),
    new Date(s.endedMs).toISOString(),
    s.models.join(" "),
    s.prompts,
    s.turns,
    s.tokens.input,
    s.tokens.output,
    s.tokens.cacheRead,
    s.tokens.cacheWrite,
    s.tokens.webSearches,
    s.cost.toFixed(4),
  ]);
  return [head, ...rows].map((r) => r.map(csvCell).join(",")).join("\n") + "\n";
}
