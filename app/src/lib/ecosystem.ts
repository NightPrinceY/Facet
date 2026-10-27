// Ecosystem (PRD M5): types mirror ecosystem.rs.
import type { Finding } from "./studio";

export type Risk = "low" | "medium" | "high";

export interface NamedItem {
  name: string;
  path: string;
  findings: Finding[];
}

export interface HookInfo {
  event: string;
  matcher: string | null;
  kind: string;
  target: string;
  source: string;
  notes: string[];
}

export interface McpInfo {
  name: string;
  transport: string;
  command: string | null;
  args: string[];
  url: string | null;
  envNames: string[];
  headerNames: string[];
  scope: string;
  project: string | null;
  notes: string[];
}

export interface PackageReport {
  root: string;
  kind: "plugin" | "repo" | "folder";
  name: string | null;
  version: string | null;
  description: string | null;
  hash: string;
  skills: NamedItem[];
  commands: NamedItem[];
  agents: NamedItem[];
  hooks: HookInfo[];
  mcp: McpInfo[];
  bin: string[];
  endpoints: string[];
  envVars: string[];
  findings: Finding[];
  risk: Risk;
  reasons: string[];
  files: number;
  bytes: number;
  truncated: boolean;
}

export interface PluginEntry {
  id: string;
  name: string;
  marketplace: string;
  version: string | null;
  scope: string | null;
  enabled: boolean | null;
  installPath: string | null;
  installedAt: string | null;
  lastUpdated: string | null;
  gitSha: string | null;
  report: PackageReport | null;
  reviewedHash: string | null;
}

export interface MarketEntry {
  name: string;
  source: string;
  location: string;
  pinned: boolean;
  lastUpdated: string | null;
}

export interface Inventory {
  accountId: string;
  plugins: PluginEntry[];
  marketplaces: MarketEntry[];
  mcp: McpInfo[];
  hooks: HookInfo[];
  settingsFindings: Finding[];
  skills: number;
  agents: number;
  commands: number;
  problems: string[];
}

export const RISK_LABEL: Record<Risk, string> = { low: "Low risk", medium: "Medium risk", high: "High risk" };

/** Reviewed, changed since review, or never reviewed. */
export function reviewState(p: PluginEntry): "reviewed" | "changed" | "new" {
  if (!p.reviewedHash || !p.report) return "new";
  return p.reviewedHash === p.report.hash ? "reviewed" : "changed";
}

export interface McpAdd {
  name: string;
  scope: "user" | "local" | "project";
  transport: "stdio" | "http" | "sse";
  commandOrUrl: string;
  args: string[];
  env: [string, string][];
  headers: [string, string][];
  folder: string | null;
}

/** Split a command line into words, honouring simple quotes. */
export function splitArgs(s: string): string[] {
  const out: string[] = [];
  const re = /"([^"]*)"|'([^']*)'|(\S+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(s))) out.push(m[1] ?? m[2] ?? m[3]!);
  return out;
}
