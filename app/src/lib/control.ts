// Control (PRD M4): the settings Facet edits, and how a form draft becomes
// edit operations. The Rust core validates again and is the authority.

export type JsonValue = string | number | boolean | null | JsonValue[] | { [k: string]: JsonValue };
export type Settings = { [k: string]: JsonValue };

export interface ControlOp {
  path: string[];
  /** `null` removes the key. */
  value: JsonValue;
}

export interface DiffLine {
  kind: "same" | "add" | "del";
  text: string;
}

export interface ControlPreview {
  path: string;
  baseHash: string;
  diff: DiffLine[];
  changed: boolean;
  /** Only the layout of the file changes. */
  formatOnly: boolean;
  warnings: string[];
}

export interface ControlPolicy {
  refusedEnv: string[];
  warnEnv: string[];
  hiddenMarker: string;
}

export interface ControlState {
  current: { path: string; exists: boolean; settings: Settings | null; hash: string; blocked: string | null };
  managedPath: string | null;
}

export interface ControlApplied {
  path: string;
  backup: string | null;
}

export interface SettingsBackup {
  name: string;
  bytes: number;
}

/** Tri-state for booleans: not set (Claude Code's default), on, off. */
export type Tri = "" | "on" | "off";

export interface Draft {
  model: string;
  outputStyle: string;
  cleanupPeriodDays: string;
  defaultMode: string;
  allow: string;
  ask: string;
  deny: string;
  additionalDirectories: string;
  crossSessionInbound: string;
  disableAllHooks: Tri;
  enableAllProjectMcpServers: Tri;
  disableSkillShellExecution: Tri;
  autoMemoryEnabled: Tri;
  env: EnvRow[];
}

export interface EnvRow {
  /** Stable id for React keys and focus. */
  id: number;
  key: string;
  value: string;
  /** Value hidden by the core (secret-looking); it is never sent back. */
  hidden: boolean;
}

let rowSeq = 0;
export const newEnvRow = (key = "", value = ""): EnvRow => ({ id: ++rowSeq, key, value, hidden: false });

const get = (s: Settings | null, path: string[]): JsonValue | undefined => {
  let cur: JsonValue | undefined = s ?? undefined;
  for (const k of path) {
    if (cur && typeof cur === "object" && !Array.isArray(cur)) cur = (cur as Settings)[k];
    else return undefined;
  }
  return cur;
};

const str = (v: JsonValue | undefined) => (typeof v === "string" ? v : typeof v === "number" ? String(v) : "");
const lines = (v: JsonValue | undefined) => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string").join("\n") : "");
const tri = (v: JsonValue | undefined): Tri => (v === true ? "on" : v === false ? "off" : "");

export function draftFrom(s: Settings | null): Draft {
  const env = get(s, ["env"]);
  return {
    model: str(get(s, ["model"])),
    outputStyle: str(get(s, ["outputStyle"])),
    cleanupPeriodDays: str(get(s, ["cleanupPeriodDays"])),
    defaultMode: str(get(s, ["permissions", "defaultMode"])),
    allow: lines(get(s, ["permissions", "allow"])),
    ask: lines(get(s, ["permissions", "ask"])),
    deny: lines(get(s, ["permissions", "deny"])),
    additionalDirectories: lines(get(s, ["permissions", "additionalDirectories"])),
    crossSessionInbound: str(get(s, ["crossSessionInbound"])),
    disableAllHooks: tri(get(s, ["disableAllHooks"])),
    enableAllProjectMcpServers: tri(get(s, ["enableAllProjectMcpServers"])),
    disableSkillShellExecution: tri(get(s, ["disableSkillShellExecution"])),
    autoMemoryEnabled: tri(get(s, ["autoMemoryEnabled"])),
    env:
      env && typeof env === "object" && !Array.isArray(env)
        ? Object.entries(env).map(([key, value]) => {
            const v = typeof value === "string" ? value : JSON.stringify(value);
            return { id: ++rowSeq, key, value: v, hidden: v === HIDDEN_MARKER };
          })
        : [],
  };
}

const splitLines = (t: string) =>
  t
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);

/** Operations that turn `base` into `draft`, only for fields that changed. */
export function opsFromDraft(base: Draft, draft: Draft): ControlOp[] {
  const ops: ControlOp[] = [];
  const text = (path: string[], a: string, b: string) => {
    if (a.trim() !== b.trim()) ops.push({ path, value: b.trim() ? b.trim() : null });
  };
  const num = (path: string[], a: string, b: string) => {
    if (a.trim() !== b.trim()) ops.push({ path, value: b.trim() ? Number(b) : null });
  };
  const list = (path: string[], a: string, b: string) => {
    const la = splitLines(a);
    const lb = splitLines(b);
    if (la.join("\n") !== lb.join("\n")) ops.push({ path, value: lb.length ? lb : null });
  };
  const flag = (path: string[], a: Tri, b: Tri) => {
    if (a !== b) ops.push({ path, value: b === "" ? null : b === "on" });
  };
  text(["model"], base.model, draft.model);
  text(["outputStyle"], base.outputStyle, draft.outputStyle);
  num(["cleanupPeriodDays"], base.cleanupPeriodDays, draft.cleanupPeriodDays);
  text(["permissions", "defaultMode"], base.defaultMode, draft.defaultMode);
  list(["permissions", "allow"], base.allow, draft.allow);
  list(["permissions", "ask"], base.ask, draft.ask);
  list(["permissions", "deny"], base.deny, draft.deny);
  list(["permissions", "additionalDirectories"], base.additionalDirectories, draft.additionalDirectories);
  text(["crossSessionInbound"], base.crossSessionInbound, draft.crossSessionInbound);
  flag(["disableAllHooks"], base.disableAllHooks, draft.disableAllHooks);
  flag(["enableAllProjectMcpServers"], base.enableAllProjectMcpServers, draft.enableAllProjectMcpServers);
  flag(["disableSkillShellExecution"], base.disableSkillShellExecution, draft.disableSkillShellExecution);
  flag(["autoMemoryEnabled"], base.autoMemoryEnabled, draft.autoMemoryEnabled);
  // Environment: one edit per changed key, so untouched values (and hidden
  // ones) never travel back to the core.
  const map = (rows: Draft["env"]) => new Map(rows.filter((r) => r.key.trim()).map((r) => [r.key.trim(), r]));
  const ea = map(base.env);
  const eb = map(draft.env);
  for (const k of ea.keys()) if (!eb.has(k)) ops.push({ path: ["env", k], value: null });
  for (const [k, row] of eb) {
    const old = ea.get(k);
    if (row.hidden) continue;
    if (!old || old.value !== row.value || old.hidden) ops.push({ path: ["env", k], value: row.value });
  }
  return ops;
}

/** Line diff (same algorithm as control.rs), used by the sample backend. */
export function diffLines(before: string, after: string): DiffLine[] {
  const a = before ? before.replace(/\n$/, "").split("\n") : [];
  const b = after ? after.replace(/\n$/, "").split("\n") : [];
  const n = a.length;
  const m = b.length;
  const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--) lcs[i]![j] = a[i] === b[j] ? lcs[i + 1]![j + 1]! + 1 : Math.max(lcs[i + 1]![j]!, lcs[i]![j + 1]!);
  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < n || j < m) {
    if (i < n && j < m && a[i] === b[j]) {
      out.push({ kind: "same", text: a[i]! });
      i++;
      j++;
    } else if (j < m && (i === n || lcs[i]![j + 1]! >= lcs[i + 1]![j]!)) {
      out.push({ kind: "add", text: b[j]! });
      j++;
    } else {
      out.push({ kind: "del", text: a[i]! });
      i++;
    }
  }
  return out;
}

/** Apply ops to a settings object (sample backend; mirrors control.rs). */
export function applyOps(before: Settings | null, ops: ControlOp[]): Settings {
  const root: Settings = structuredClone(before ?? {});
  for (const op of ops) {
    const parents = op.path.slice(0, -1);
    const last = op.path[op.path.length - 1]!;
    let cur: Settings = root;
    for (const k of parents) {
      if (!cur[k] || typeof cur[k] !== "object" || Array.isArray(cur[k])) cur[k] = {};
      cur = cur[k] as Settings;
    }
    if (op.value === null) {
      delete cur[last];
      if (parents.length === 1 && Object.keys(cur).length === 0) delete root[parents[0]!];
    } else cur[last] = op.value;
  }
  return root;
}

/** Mirrors control::HIDDEN; the policy command is the authority. */
export const HIDDEN_MARKER = "•••• hidden by Facet";

/** Fallback copy of control::REFUSED_ENV for the sample backend and tests;
 *  the app asks the core (controlPolicy) for the real list. */
export const REFUSED_ENV = [
  "ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN", "ANTHROPIC_BASE_URL", "ANTHROPIC_CUSTOM_HEADERS",
  "CLAUDE_CONFIG_DIR", "CLAUDE_CODE_USE_BEDROCK", "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY", "ANTHROPIC_BEDROCK_BASE_URL",
  "ANTHROPIC_VERTEX_BASE_URL", "ANTHROPIC_FOUNDRY_BASE_URL", "ANTHROPIC_FOUNDRY_RESOURCE", "ANTHROPIC_FOUNDRY_API_KEY",
  "CLAUDE_CODE_SKIP_BEDROCK_AUTH", "CLAUDE_CODE_SKIP_VERTEX_AUTH", "AWS_BEARER_TOKEN_BEDROCK", "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN", "GOOGLE_APPLICATION_CREDENTIALS", "CLAUDE_CODE_CLIENT_KEY_PASSPHRASE",
  "NODE_TLS_REJECT_UNAUTHORIZED",
];

export const isRefusedEnv = (name: string, list: string[] = REFUSED_ENV) => list.includes(name.trim().toUpperCase());
export const validEnvName = (name: string) => /^[A-Za-z_][A-Za-z0-9_]*$/.test(name.trim());
