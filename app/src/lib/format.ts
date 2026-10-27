// Display helpers (DESIGN-SYSTEM.md "Data labels").

/** Shorten the home folder to `~` for display. Paths stay exact in titles. */
export function tildify(path: string | null | undefined, home?: string): string {
  if (!path) return "";
  const h = home ?? guessHome(path);
  if (h && (path === h || path.startsWith(h + "/"))) return "~" + path.slice(h.length);
  return path;
}

function guessHome(path: string): string | undefined {
  const m = /^(\/home\/[^/]+|\/Users\/[^/]+|\/root)(?=\/|$)/.exec(path);
  return m?.[1];
}

export function initial(name: string): string {
  const ch = [...name.trim()].find((c) => /[\p{L}\p{N}]/u.test(c));
  return ch ? ch.toLocaleUpperCase() : "?";
}

export function plural(n: number, one: string, many = one + "s"): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** "Max" → "Max", "max" → "Max", "claude.ai" untouched. */
export function planLabel(plan: string | null | undefined): string {
  if (!plan) return "";
  return /^[a-z]+$/.test(plan) ? plan[0]!.toUpperCase() + plan.slice(1) : plan;
}
