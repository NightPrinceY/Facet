import { describe, expect, it } from "vitest";
import { defaultDirs, firstUnusedColor, slugify } from "./accounts";
import { initial, planLabel, plural, tildify } from "./format";

describe("account helpers (mirror profile.rs)", () => {
  it("slugify matches the Rust cases", () => {
    expect(slugify("Client A (Acme)")).toBe("client-a-acme");
    expect(slugify("  Work  ")).toBe("work");
    expect(slugify("عمل")).toBe("");
  });

  it("default folders: Claude's own for the first account, named ones after", () => {
    expect(defaultDirs("/h", "personal", true)).toEqual({ desktopDir: "/h/.config/Claude", codeDir: "/h/.claude" });
    expect(defaultDirs("/h", "client-acme", false)).toEqual({ desktopDir: "/h/.config/Claude-Client-Acme", codeDir: "/h/.claude-client-acme" });
  });

  it("first unused colour", () => {
    expect(firstUnusedColor([])).toBe(1);
    expect(firstUnusedColor([{ color: 1 }, { color: 3 }])).toBe(2);
  });
});

describe("format", () => {
  it("tildify shortens only the home prefix", () => {
    expect(tildify("/home/you/.claude-work")).toBe("~/.claude-work");
    expect(tildify("/home/you")).toBe("~");
    expect(tildify("/home/youx/x", "/home/you")).toBe("/home/youx/x");
    expect(tildify("/usr/lib/claude-desktop")).toBe("/usr/lib/claude-desktop");
    expect(tildify(null)).toBe("");
  });

  it("initial, plural, plan", () => {
    expect(initial("  work")).toBe("W");
    expect(initial("عمل")).toBe("ع");
    expect(initial("--")).toBe("?");
    expect(plural(1, "Code session")).toBe("1 Code session");
    expect(plural(3, "Code session")).toBe("3 Code sessions");
    expect(planLabel("max")).toBe("Max");
    expect(planLabel("claude.ai")).toBe("claude.ai");
  });
});
