// Studio (PRD M11): types shared with studio.rs.
import type { DiffLine } from "./control";

export type ItemKind = "skill" | "agent" | "command";
export type Severity = "error" | "risk" | "warn" | "info";

export interface Finding {
  severity: Severity;
  message: string;
}

export interface Lint {
  findings: Finding[];
  descriptionChars: number;
  description: string;
}

export type PlacementState = "absent" | "in-sync" | "drifted" | "untouchable";

export interface StudioItem {
  kind: ItemKind;
  name: string;
  path: string;
  description: string;
  hash: string;
  lint: Finding[];
  placements: { accountId: string; state: PlacementState }[];
}

export interface AccountItem {
  kind: ItemKind;
  name: string;
  path: string;
  description: string;
  hash: string;
  lint: Finding[];
}

export interface PlacePreview {
  target: string;
  state: PlacementState;
  baseHash: string;
  diff: DiffLine[];
  /** What the item does that needs care; placing requires acknowledging these. */
  risks: Finding[];
  /** Problems that stop placement. */
  blockers: Finding[];
  liveSessions: number;
}

export const SEVERITY_LABEL: Record<Severity, string> = { error: "Must fix", risk: "Runs commands", warn: "Worth fixing", info: "Note" };

/** The worst severity present, for a summary pill. */
export function worst(findings: Finding[]): Severity | null {
  for (const s of ["error", "risk", "warn"] as const) if (findings.some((f) => f.severity === s)) return s;
  return null;
}

export const DESC_BUDGET = 1536;

export const KIND_LABEL: Record<ItemKind, string> = { skill: "Skill", agent: "Subagent", command: "Command" };

/** Where an item lives inside an account folder (mirrors Kind::rel_path). */
export function relPath(kind: ItemKind, name: string): string {
  return kind === "skill" ? `skills/${name}/SKILL.md` : kind === "agent" ? `agents/${name}.md` : `commands/${name}.md`;
}

export function validName(name: string): boolean {
  return /^[a-z0-9][a-z0-9-]{0,63}$/.test(name);
}
