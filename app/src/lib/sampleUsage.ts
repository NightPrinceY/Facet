// Deterministic sample usage for the browser preview and tests. Clearly
// sample data: folders under /home/you, made-up session ids.
import type { AccountUsage, HourBucket, SessionSummary, UsageReport } from "./usage";

const PROJECTS: Record<string, string[]> = {
  personal: ["/home/you/code/blog", "/home/you/code/dotfiles", "/home/you/code/game"],
  work: ["/home/you/work/api", "/home/you/work/web", "/home/you/work/infra"],
};
const TOOLS = ["Bash", "Read", "Edit", "Write", "Grep", "WebSearch", "mcp__github__create_pull_request"];

function rng(seed: number) {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 2 ** 32;
  };
}

export function sampleUsage(accountIds: string[], days: number, now = Date.now()): UsageReport {
  const since = now - days * 86_400_000;
  const accounts: AccountUsage[] = accountIds.map((id, ai) => {
    const r = rng(17 + ai * 101);
    const sessions: SessionSummary[] = [];
    const hours: HourBucket[] = [];
    const projects = PROJECTS[id] ?? [`/home/you/${id}`];
    const hourNow = now - (now % 3_600_000);
    for (let i = 0; i < 40; i++) {
      const end = hourNow - Math.floor(r() * 30 * 24) * 3_600_000 - (i === 0 ? 0 : 1);
      const lengthH = 1 + Math.floor(r() * 3);
      const start = end - lengthH * 3_600_000 + 600_000;
      const project = projects[Math.floor(r() * projects.length)]!;
      const model = r() < 0.8 ? "claude-opus-5-5" : "claude-sonnet-5-5";
      const tools: Record<string, number> = {};
      for (const t of TOOLS) if (r() < 0.6) tools[t] = 1 + Math.floor(r() * 40);
      let cost = 0;
      let turns = 0;
      let prompts = 0;
      const tok = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, webSearches: 0 };
      for (let h = 0; h < lengthH; h++) {
        const hTurns = 5 + Math.floor(r() * 60);
        const hPrompts = 1 + Math.floor(r() * 6);
        const t = { input: hTurns * 40, output: hTurns * 900, cacheRead: hTurns * 60_000, cacheWrite: hTurns * 3000, webSearches: r() < 0.2 ? 2 : 0 };
        const c = (t.input * 4 + t.output * 20 + t.cacheWrite * 5 + t.cacheRead * 0.2) / 1e6 * (model.includes("sonnet") ? 0.5 : 1) + t.webSearches * 0.01;
        hours.push({ hourMs: end - (end % 3_600_000) - h * 3_600_000, model, project, tokens: t, cost: c, turns: hTurns, prompts: hPrompts });
        cost += c;
        turns += hTurns;
        prompts += hPrompts;
        tok.input += t.input;
        tok.output += t.output;
        tok.cacheRead += t.cacheRead;
        tok.cacheWrite += t.cacheWrite;
        tok.webSearches += t.webSearches;
      }
      sessions.push({
        accountId: id,
        sessionId: `sample-${id}-${String(i).padStart(4, "0")}-0000-4000-8000-000000000000`.slice(0, 36),
        project,
        branch: r() < 0.7 ? "main" : `feature/${["login", "search", "billing"][Math.floor(r() * 3)]}`,
        startedMs: start,
        endedMs: end,
        models: [model],
        entrypoint: r() < 0.5 ? "cli" : "claude-desktop",
        version: "2.1.310",
        prompts,
        turns,
        tools,
        tokens: tok,
        cost,
        unpricedModels: [],
        reportedCost: r() < 0.5 ? cost * 1.01 : null,
        subagents: r() < 0.3 ? 2 : 0,
        bytes: turns * 4000,
      });
    }
    return {
      accountId: id,
      sessions: sessions.filter((s) => s.endedMs >= since).sort((a, b) => b.endedMs - a.endedMs),
      hours: hours.filter((h) => h.hourMs + 3_600_000 > since).sort((a, b) => a.hourMs - b.hourMs),
    };
  });
  return { generatedMs: now, sinceMs: since, accounts, pricesChecked: "2026-10-26", pricesSource: "https://platform.claude.com/docs/en/about-claude/pricing", prices: [] };
}

/** Sample events for the Timeline: a few tasks per account in the last two days. */
export function sampleEvents(accountIds: string[], since: number, until: number, now = Date.now()): import("./timeline").TimelineEvents {
  const out: import("./timeline").TimelineEvent[] = [];
  accountIds.forEach((id, ai) => {
    const r = rng(99 + ai * 7);
    for (let t = 0; t < 14; t++) {
      const sessionId = `sample-${id}-${String(t % 4).padStart(4, "0")}-0000-4000-8000-000000000000`.slice(0, 36);
      let ts = now - Math.floor(r() * 46 * 3_600_000) - 60_000;
      out.push({ accountId: id, sessionId, ts, kind: "prompt", endTs: null, model: null, cost: 0, outputTokens: 0, totalTokens: 0, tool: null, ok: null, subagent: false });
      const calls = 3 + Math.floor(r() * 12);
      for (let c = 0; c < calls; c++) {
        ts += 2_000 + Math.floor(r() * 20_000);
        const cost = 0.01 + r() * 0.3;
        out.push({ accountId: id, sessionId, ts, kind: "call", endTs: null, model: "claude-opus-5-5", cost, outputTokens: Math.floor(r() * 2000), totalTokens: 40_000 + Math.floor(r() * 90_000), tool: null, ok: null, subagent: false });
        if (r() < 0.8) {
          const dur = 300 + Math.floor(r() * (r() < 0.2 ? 60_000 : 4_000));
          out.push({ accountId: id, sessionId, ts: ts + 5, kind: "tool", endTs: ts + 5 + dur, model: null, cost: 0, outputTokens: 0, totalTokens: 0, tool: ["Bash", "Read", "Edit", "Grep"][Math.floor(r() * 4)]!, ok: r() > 0.1, subagent: false });
          ts += dur;
        }
      }
    }
  });
  const events = out.filter((e) => e.ts >= since && e.ts <= until).sort((a, b) => a.ts - b.ts);
  return { events, truncated: false };
}
