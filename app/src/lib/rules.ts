import type { FolderRule } from "./api";

/** Same rule as `rules::match_folder` in the core: the longest folder that
 *  contains `folder` (whole path components only) wins. */
export function matchFolder(rules: FolderRule[], folder: string): FolderRule | undefined {
  const clean = (p: string) => p.replace(/\/+$/, "") || "/";
  const target = clean(folder);
  let best: FolderRule | undefined;
  for (const r of rules) {
    const f = clean(r.folder);
    if (!f.startsWith("/")) continue;
    if (target === f || target.startsWith(f + "/")) {
      if (!best || f.split("/").length > clean(best.folder).split("/").length) best = r;
    }
  }
  return best;
}
