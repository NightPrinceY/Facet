// Account rules shared by the UI and the sample backend. They mirror
// `profile.rs` in the Rust core, which stays the authority on save.

import type { Account, AccountInput, FacetColor } from "./api";

export function slugify(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
}

export function firstUnusedColor(accounts: Pick<Account, "color">[]): FacetColor {
  for (let c = 1; c <= 6; c++) {
    if (!accounts.some((a) => a.color === c)) return c as FacetColor;
  }
  return ((accounts.length % 6) + 1) as FacetColor;
}

function titleCase(id: string): string {
  return id
    .split("-")
    .map((p) => (p ? p[0]!.toUpperCase() + p.slice(1) : p))
    .join("-");
}

/** Same defaults as `profile::default_dirs`: the first account uses Claude's
 *  own folders, later ones get `Claude-<Name>` and `.claude-<id>`. */
export function defaultDirs(home: string, id: string, first: boolean): { desktopDir: string; codeDir: string } {
  return first
    ? { desktopDir: `${home}/.config/Claude`, codeDir: `${home}/.claude` }
    : { desktopDir: `${home}/.config/Claude-${titleCase(id)}`, codeDir: `${home}/.claude-${id}` };
}

/** The save input that keeps an account exactly as it is (for small edits
 *  such as an inline rename). */
export function inputFrom(a: Account): AccountInput {
  return {
    id: a.id,
    name: a.name,
    color: a.color,
    plan: a.plan,
    withDesktop: Boolean(a.desktopDir),
    desktopDir: a.desktopDir,
    codeDir: a.codeDir,
    expectedEmail: a.expectedEmail,
    tint: a.tint,
    x11Identity: a.x11Identity,
    badge: a.badge,
    notes: a.notes,
    launcherIcon: a.launcherIcon,
    terminalDir: a.terminalDir,
    launcherName: a.launcherName,
  };
}

/** Move one id a step earlier (-1) or later (+1) in the list. */
export function moved(ids: string[], id: string, by: -1 | 1): string[] {
  const i = ids.indexOf(id);
  const j = i + by;
  if (i < 0 || j < 0 || j >= ids.length) return ids;
  const next = [...ids];
  [next[i], next[j]] = [next[j]!, next[i]!];
  return next;
}
