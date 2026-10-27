import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import type { Api } from "../lib/api";
import { createSampleApi } from "../lib/sample";
import { compact, modelLabel, money, sessionsCsv, toolLabel } from "../lib/usage";
import { sampleUsage } from "../lib/sampleUsage";
import { matchSession } from "./SessionsView";

afterEach(cleanup);

async function open(view: RegExp, api: Api = createSampleApi()) {
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: view }));
  return api;
}

describe("usage helpers", () => {
  it("formats money, tokens, models and tools", () => {
    expect(money(0)).toBe("$0.00");
    expect(money(0.004)).toBe("$0.0040");
    expect(money(12.345)).toBe("$12.35");
    expect(money(1234.5)).toBe("$1,234.50");
    expect(compact(950)).toBe("950");
    expect(compact(1250)).toBe("1.3K");
    expect(compact(3_400_000)).toBe("3.4M");
    expect(modelLabel("claude-opus-5-5")).toBe("Opus 5.5");
    expect(modelLabel("claude-haiku-4-5-20251001")).toBe("Haiku 4.5");
    expect(modelLabel("claude-opus-5")).toBe("Opus 5");
    expect(modelLabel("claude-3-5-haiku-20241022")).toBe("Haiku 3.5");
    expect(toolLabel("mcp__github__create_issue")).toBe("github: create_issue");
    expect(toolLabel("Bash")).toBe("Bash");
  });

  it("searches metadata by word and by field", () => {
    const s = sampleUsage(["work"], 30).accounts[0]!.sessions[0]!;
    expect(matchSession(s, "", "Work")).toBe(true);
    expect(matchSession(s, "account:work", "Work")).toBe(true);
    expect(matchSession(s, "account:personal", "Work")).toBe(false);
    expect(matchSession(s, `id:${s.sessionId.slice(0, 8)}`, "Work")).toBe(true);
    expect(matchSession(s, "model:opus model:sonnet", "Work")).toBe(false);
  });

  it("exports one CSV row per session with quoted cells", () => {
    const s = { ...sampleUsage(["work"], 30).accounts[0]!.sessions[0]!, project: '/home/you/a "b", c' };
    const csv = sessionsCsv([s], () => "Work");
    const [head, row] = csv.trim().split("\n");
    expect(head!.split(",")[0]).toBe("account");
    expect(row).toContain('"/home/you/a ""b"", c"');
    const evil = sessionsCsv([{ ...s, branch: "=HYPERLINK(1)" }], () => "+Work");
    expect(evil).toContain("'=HYPERLINK(1)");
    expect(evil).toContain("'+Work");
  });
});

describe("Today, Costs and Sessions", () => {
  it("Today shows per-account cost and the hourly chart", async () => {
    await open(/Today/);
    expect(await screen.findByRole("heading", { name: "Today", level: 1 })).toBeTruthy();
    await screen.findByRole("region", { name: "Today in numbers" });
    expect(screen.getByRole("article", { name: "Work today" })).toBeTruthy();
    expect(screen.getByRole("figure", { name: /Cost by hour today/ })).toBeTruthy();
  });

  it("Costs groups by project, switches grouping and exports", async () => {
    await open(/Costs/);
    const table = await screen.findByRole("table", { name: /Cost grouped by project/ });
    expect(within(table).getAllByRole("row").length).toBeGreaterThan(1);
    fireEvent.click(screen.getByRole("button", { name: "Model" }));
    const byModel = await screen.findByRole("table", { name: /Cost grouped by model/ });
    expect(within(byModel).getByText("Opus 5.5")).toBeTruthy();
    expect(screen.getByRole("figure", { name: /Cost by day, last 30 days/ })).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Export CSV" }));
    });
    await waitFor(() => expect(document.querySelector(".toast")?.textContent).toMatch(/Downloads\/facet-costs-/));
  });

  it("Sessions filters by search and resumes in the right account", async () => {
    const base = createSampleApi();
    const calls: string[] = [];
    const api: Api = { ...base, resumeSession: async (id, sid) => (calls.push(`${id}:${sid}`), 1) };
    await open(/Sessions/, api);
    await screen.findByRole("list", { name: "Sessions" });
    const count = () => Number(/^(\d+) session/.exec(screen.getByText(/^\d+ sessions? ·/).textContent ?? "")?.[1]);
    const before = count();
    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "account:work" } });
    await waitFor(() => expect(count()).toBeLessThan(before));
    for (const row of within(screen.getByRole("list", { name: "Sessions" })).getAllByRole("listitem")) {
      expect(within(row).getByRole("img", { name: "Work" })).toBeTruthy();
    }
    const first = within(screen.getByRole("list", { name: "Sessions" })).getAllByRole("listitem")[0]!;
    await act(async () => {
      fireEvent.click(within(first).getByRole("button", { name: /Resume the .* session in Work/ }));
    });
    expect(calls[0]).toMatch(/^work:sample-work-/);
    fireEvent.click(within(first).getByRole("button", { expanded: false }));
    expect(within(first).getByText("Session id")).toBeTruthy();
  });

  it("an empty search result says so", async () => {
    await open(/Sessions/);
    await screen.findByRole("list", { name: "Sessions" });
    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "nothing-matches-this" } });
    expect(await screen.findByText("Nothing matches")).toBeTruthy();
  });

  it("a failed read shows the error, not zeros", async () => {
    const base = createSampleApi();
    const api: Api = { ...base, usageReport: async () => Promise.reject("Permission denied") };
    await open(/Costs/, api);
    expect(await screen.findByText(/Facet could not read the transcripts: Permission denied/)).toBeTruthy();
    expect(screen.queryByText("$0.00")).toBeNull();
    expect(screen.queryByText(/Prices: Anthropic/)).toBeNull();
  });

  it("chart columns say the per-account split in words", async () => {
    await open(/Costs/);
    const fig = await screen.findByRole("figure", { name: /Cost by day/ });
    const labels = within(fig).getAllByRole("img").map((c) => c.getAttribute("aria-label") ?? "");
    expect(labels.some((l) => /\(Personal \$[\d.,]+, Work \$[\d.,]+\)|\(Work \$[\d.,]+, Personal \$[\d.,]+\)/.test(l))).toBe(true);
  });
});
