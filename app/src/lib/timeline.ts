// Timeline helpers (TIMELINE-SPEC.md): events from timeline_events, tasks,
// zoom and axis ticks. Pure functions so they can be tested alone.

export interface TimelineEvent {
  accountId: string;
  sessionId: string;
  ts: number;
  kind: "prompt" | "call" | "tool";
  endTs: number | null;
  model: string | null;
  cost: number;
  outputTokens: number;
  totalTokens: number;
  tool: string | null;
  ok: boolean | null;
  subagent: boolean;
}

export interface TimelineEvents {
  events: TimelineEvent[];
  truncated: boolean;
}

/** Everything from one prompt until the next prompt in the same session. */
export interface Task {
  key: string;
  accountId: string;
  sessionId: string;
  start: number;
  end: number;
  cost: number;
  calls: number;
  tools: number;
}

export const MIN_SPAN = 5_000;
export const MAX_SPAN = 30 * 86_400_000;
export const DAY = 86_400_000;
export const HOUR = 3_600_000;

export function tasksOf(events: TimelineEvent[]): Task[] {
  const bySession = new Map<string, TimelineEvent[]>();
  for (const e of events) {
    const k = `${e.accountId}:${e.sessionId}`;
    const list = bySession.get(k);
    if (list) list.push(e);
    else bySession.set(k, [e]);
  }
  const out: Task[] = [];
  for (const list of bySession.values()) {
    list.sort((a, b) => a.ts - b.ts);
    let cur: Task | null = null;
    for (const e of list) {
      if (!cur || (e.kind === "prompt" && !e.subagent)) {
        if (cur) out.push(cur);
        cur = { key: `${e.accountId}:${e.sessionId}:${e.ts}`, accountId: e.accountId, sessionId: e.sessionId, start: e.ts, end: e.ts, cost: 0, calls: 0, tools: 0 };
      }
      cur.end = Math.max(cur.end, e.endTs ?? e.ts);
      cur.cost += e.cost;
      if (e.kind === "call") cur.calls += 1;
      if (e.kind === "tool") cur.tools += 1;
    }
    if (cur) out.push(cur);
  }
  return out.sort((a, b) => a.start - b.start);
}

/** Zoom by `factor` (>1 zooms out) keeping time `at` under the cursor. */
export function zoom(start: number, end: number, factor: number, at: number, now: number): [number, number] {
  const span = Math.min(MAX_SPAN, Math.max(MIN_SPAN, (end - start) * factor));
  const ratio = end > start ? (at - start) / (end - start) : 0.5;
  let s = at - span * ratio;
  let e = s + span;
  if (e > now + span * 0.05) {
    e = now + span * 0.05;
    s = e - span;
  }
  return [Math.round(s), Math.round(e)];
}

/** Move the window by `dt` ms, never past a little after now. */
export function pan(start: number, end: number, dt: number, now: number): [number, number] {
  const span = end - start;
  let e = end + dt;
  if (e > now + span * 0.05) e = now + span * 0.05;
  return [Math.round(e - span), Math.round(e)];
}

const STEPS = [1_000, 5_000, 10_000, 30_000, 60_000, 300_000, 600_000, 1_800_000, HOUR, 3 * HOUR, 6 * HOUR, 12 * HOUR, DAY, 2 * DAY, 7 * DAY];

/** Tick step so that about `count` labels fit. */
export function tickStep(span: number, count = 8): number {
  return STEPS.find((s) => span / s <= count) ?? 7 * DAY;
}

/** Tick times aligned to local wall-clock boundaries. */
export function ticks(start: number, end: number, count = 8): number[] {
  const step = tickStep(end - start, count);
  const out: number[] = [];
  const offset = step >= DAY ? new Date(start).getTimezoneOffset() * 60_000 : step >= HOUR ? new Date(start).getTimezoneOffset() * 60_000 : 0;
  let t = Math.ceil((start - offset) / step) * step + offset;
  if (step >= DAY) {
    const d = new Date(start);
    d.setHours(0, 0, 0, 0);
    t = d.getTime();
    while (t < start) t += step;
  }
  for (; t <= end; t += step) out.push(t);
  return out;
}

export function tickLabel(t: number, step: number): string {
  const d = new Date(t);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  if (step >= DAY) return d.toLocaleDateString(undefined, { day: "numeric", month: "short" });
  if (step < 60_000) return `${hh}:${mm}:${ss}`;
  if (hh === "00" && mm === "00") return d.toLocaleDateString(undefined, { day: "numeric", month: "short" });
  return `${hh}:${mm}`;
}

/** "14:21:57.123", always to the millisecond (inspector). */
export function preciseTime(t: number): string {
  const d = new Date(t);
  return `${d.toLocaleDateString(undefined, { day: "numeric", month: "short" })} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}.${String(d.getMilliseconds()).padStart(3, "0")}`;
}

/** "850 ms", "2.5 s", "3 m 4 s", "1 h 2 m". */
export function shortDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(ms < 10_000 ? 1 : 0)} s`;
  if (ms < HOUR) {
    const s = Math.round(ms / 1000);
    return `${Math.floor(s / 60)} m ${s % 60} s`;
  }
  const m = Math.round(ms / 60_000);
  return `${Math.floor(m / 60)} h ${m % 60} m`;
}

/** Histogram bucket for a visible span (TIMELINE-SPEC.md zoom table). */
export function bucketFor(span: number): number {
  if (span > 2 * DAY) return HOUR;
  if (span > 2 * HOUR) return span > DAY ? 1_800_000 : 300_000;
  if (span > 45 * 60_000) return 30_000;
  if (span > 60_000) return 5_000;
  return 1_000;
}

export type Detail = "days" | "tasks" | "events";

/** What the lanes draw at this span. */
export function detailFor(span: number): Detail {
  if (span > 2 * DAY) return "days";
  if (span > 2 * HOUR) return "tasks";
  return "events";
}
