// @vitest-environment node
// Design-system rules enforced in CI (DESIGN-SYSTEM.md §1 priority actions).
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const app = fileURLToPath(new URL("../..", import.meta.url));
const read = (p: string) => readFileSync(join(app, p), "utf8");

function filesUnder(dir: string, exts: string[]): string[] {
  return readdirSync(join(app, dir), { withFileTypes: true }).flatMap((e) => {
    const rel = join(dir, e.name);
    if (e.isDirectory()) return filesUnder(rel, exts);
    return exts.some((x) => e.name.endsWith(x)) ? [rel] : [];
  });
}

describe("design rules", () => {
  it("app tokens.css is an exact copy of docs/05-design/tokens.css (run npm run tokens)", () => {
    expect(read("src/styles/tokens.css")).toBe(read("../docs/05-design/tokens.css"));
  });

  it("no raw hex colours outside tokens.css", () => {
    const offenders = filesUnder("src", [".css", ".tsx", ".ts"])
      .filter((f) => !f.endsWith("tokens.css") && !f.endsWith(".test.ts") && !f.endsWith(".test.tsx"))
      .flatMap((f) =>
        read(f)
          .split("\n")
          .map((line, i) => ({ f, i: i + 1, line }))
          .filter(({ line }) => /#[0-9a-fA-F]{3,8}\b/.test(line) && !/^\s*(\/\/|\*|\/\*)/.test(line)),
      );
    expect(offenders.map((o) => `${o.f}:${o.i}: ${o.line.trim()}`)).toEqual([]);
  });

  it("no physical left/right properties in app.css (right-to-left support)", () => {
    const bad = read("src/styles/app.css")
      .split("\n")
      .filter((l) => /\b(margin|padding|border)-(left|right)\b|\b(left|right)\s*:/.test(l));
    expect(bad).toEqual([]);
  });

  it("product strings never call Facet a Claude product", () => {
    const html = read("index.html");
    expect(html).toContain("<title>Facet</title>");
    const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
    expect(conf.productName).toBe("Facet");
    expect(String(conf.identifier).toLowerCase()).not.toContain("claude");
    expect(read("package.json").toLowerCase()).toContain('"name": "facet"');
  });
});
