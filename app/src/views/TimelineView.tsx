import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Account } from "../lib/api";
import { errorText } from "../lib/api";
import { plural, tildify } from "../lib/format";
import { DAY, HOUR, MAX_SPAN, MIN_SPAN, bucketFor, detailFor, pan, preciseTime, shortDuration, tasksOf, tickLabel, tickStep, ticks, zoom, type Task, type TimelineEvent } from "../lib/timeline";
import type { FacetData } from "../lib/useFacet";
import { useApi } from "../lib/useFacet";
import { useUsage } from "../lib/useUsage";
import { compact, modelLabel, money, projectName, toolLabel, type SessionSummary } from "../lib/usage";
import { Avatar, Button, Callout, Pill, Segmented, accountStyle, avatarOf } from "../components/ui";
import { Icon } from "../components/Icon";

const PRESETS: { label: string; span: number }[] = [
  { label: "30 days", span: 30 * DAY },
  { label: "7 days", span: 7 * DAY },
  { label: "Day", span: DAY },
  { label: "1 h", span: HOUR },
  { label: "10 min", span: 600_000 },
  { label: "1 min", span: 60_000 },
];
const LANE = 40;
const HIST = 54;
const AXIS = 26;
const SPEEDS = [10, 60, 600] as const;

type Selection = { kind: "event"; event: TimelineEvent } | { kind: "task"; task: Task } | null;

/** Stable identity for an event across refetches. */
export const eventKey = (e: TimelineEvent) => `${e.accountId}:${e.sessionId}:${e.ts}:${e.kind}:${e.tool ?? ""}`;
const sameEvent = (s: Selection, e: TimelineEvent) => s?.kind === "event" && eventKey(s.event) === eventKey(e);

/** Every account on one time axis (PRD M2, TIMELINE-SPEC.md). */
export function TimelineView({ data, scope }: { data: FacetData; scope: string | null }) {
  const api = useApi();
  const now = useNow();
  const [[start, end], setWindow] = useState<[number, number]>(() => [Date.now() - DAY, Date.now() + DAY * 0.02]);
  const span = end - start;
  const detail = detailFor(span);
  const accounts = scope ? data.accounts.filter((a) => a.id === scope) : data.accounts;
  const ids = useMemo(() => new Set(accounts.map((a) => a.id)), [accounts]);
  const usageDays = Math.min(366, Math.max(2, Math.ceil((now - start) / DAY) + 1));
  const { report } = useUsage(usageDays);
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [truncated, setTruncated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Selection>(null);
  const [replay, setReplay] = useState<{ speed: (typeof SPEEDS)[number]; at: number } | null>(null);
  const [speed, setSpeed] = useState<(typeof SPEEDS)[number]>(60);
  const [loading, setLoading] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [dragging, setDragging] = useState(false);

  // Fine detail needs individual events; fetch the visible window (and a margin).
  useEffect(() => {
    if (detail === "days") {
      setEvents([]);
      return;
    }
    let stale = false;
    setLoading(true);
    const t = setTimeout(() => {
      api.timelineEvents(start - span * 0.5, end + span * 0.5).then(
        (r) => {
          if (stale) return;
          setEvents(r.events);
          setTruncated(r.truncated);
          setError(null);
          setLoading(false);
        },
        (e) => {
          if (stale) return;
          setError(errorText(e));
          setLoading(false);
        },
      );
    }, 120);
    return () => {
      stale = true;
      clearTimeout(t);
    };
  }, [api, detail, start, end, span, attempt]);

  const visibleEvents = useMemo(() => events.filter((e) => ids.has(e.accountId)), [events, ids]);
  const tasks = useMemo(() => tasksOf(visibleEvents), [visibleEvents]);
  const inView = useMemo(() => visibleEvents.filter((e) => e.ts >= start && e.ts <= end), [visibleEvents, start, end]);
  const sessions = useMemo(() => {
    const m = new Map<string, SessionSummary>();
    for (const a of report?.accounts ?? []) for (const s of a.sessions) m.set(`${s.accountId}:${s.sessionId}`, s);
    return m;
  }, [report]);

  // Replay: move a playhead through the window and select what it passes.
  useEffect(() => {
    if (!replay) return;
    let raf = 0;
    let last = performance.now();
    const step = (t: number) => {
      const dt = (t - last) * replay.speed;
      last = t;
      setReplay((r) => {
        if (!r) return r;
        const at = r.at + dt;
        if (at >= Math.min(end, now)) return null;
        const passed = [...inView].reverse().find((e) => e.ts <= at);
        if (passed) setSelected((s) => (sameEvent(s, passed) ? s : { kind: "event", event: passed }));
        return { ...r, at };
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [replay?.speed, end, now, inView]); // eslint-disable-line react-hooks/exhaustive-deps

  const setSpan = (s: number) => setWindow([Math.min(now, end) - s, Math.min(now, end) - s + s * 1.02]);
  // Arrow keys step through what the lanes draw: events when zoomed in,
  // tasks at day zoom, and move the window above two days.
  const stepEvent = useCallback(
    (dir: -1 | 1) => {
      if (detail === "days") {
        setWindow(([s, en]) => pan(s, en, (dir * (en - s)) / 4, now));
        return;
      }
      if (detail === "tasks") {
        const list = tasks;
        if (list.length === 0) return;
        const cur = selected?.kind === "task" ? list.findIndex((t) => t.key === selected.task.key) : -1;
        let i = cur >= 0 ? cur + dir : dir > 0 ? list.findIndex((t) => t.end >= start) : list.findLastIndex((t) => t.start <= end);
        i = Math.max(0, Math.min(list.length - 1, i < 0 ? list.length - 1 : i));
        const t = list[i]!;
        setSelected({ kind: "task", task: t });
        if (t.end < start || t.start > end) setWindow(([s, en]) => [t.start - (en - s) / 2, t.start + (en - s) / 2]);
        return;
      }
      const list = visibleEvents;
      if (list.length === 0) return;
      const cur = selected?.kind === "event" ? list.findIndex((e) => eventKey(e) === eventKey(selected.event)) : -1;
      let i = cur >= 0 ? cur + dir : dir > 0 ? list.findIndex((e) => e.ts >= start) : list.findLastIndex((e) => e.ts <= end);
      i = Math.max(0, Math.min(list.length - 1, i < 0 ? list.length - 1 : i));
      const e = list[i]!;
      setSelected({ kind: "event", event: e });
      if (e.ts < start || e.ts > end) setWindow(([s, en]) => [e.ts - (en - s) / 2, e.ts + (en - s) / 2]);
    },
    [detail, tasks, visibleEvents, selected, start, end, now],
  );

  // Announce the visible range only once the window settles.
  const [settled, setSettled] = useState<[number, number]>([start, end]);
  useEffect(() => {
    const t = setTimeout(() => setSettled([start, end]), 500);
    return () => clearTimeout(t);
  }, [start, end]);
  const quiet = Boolean(replay) || dragging;

  // Range summary and leaderboard for what is visible.
  const callsInView = inView.filter((e) => e.kind === "call");
  const toolsInView = inView.filter((e) => e.kind === "tool");
  const hoursInView = (report?.accounts ?? []).filter((a) => ids.has(a.accountId)).flatMap((a) => a.hours.filter((h) => h.hourMs + HOUR > start && h.hourMs < end).map((h) => ({ ...h, accountId: a.accountId })));
  const rangeCost = detail === "days" ? hoursInView.reduce((n, h) => n + h.cost, 0) : callsInView.reduce((n, e) => n + e.cost, 0);
  const rangeTokens = detail === "days" ? hoursInView.reduce((n, h) => n + h.tokens.input + h.tokens.output + h.tokens.cacheRead + h.tokens.cacheWrite, 0) : callsInView.reduce((n, e) => n + e.totalTokens, 0);
  const topTasks = tasks.filter((t) => t.end >= start && t.start <= end).sort((a, b) => b.cost - a.cost).slice(0, 5);
  const longestTools = toolsInView.filter((e) => e.endTs).sort((a, b) => b.endTs! - b.ts - (a.endTs! - a.ts)).slice(0, 5);
  const topSessions = (() => {
    const m = new Map<string, { accountId: string; sessionId: string; cost: number }>();
    if (detail === "days") {
      for (const a of report?.accounts ?? []) if (ids.has(a.accountId)) for (const s of a.sessions) if (s.endedMs >= start && s.startedMs <= end) m.set(s.sessionId, { accountId: s.accountId, sessionId: s.sessionId, cost: s.cost });
    } else {
      for (const e of callsInView) {
        const k = `${e.accountId}:${e.sessionId}`;
        const r = m.get(k) ?? { accountId: e.accountId, sessionId: e.sessionId, cost: 0 };
        r.cost += e.cost;
        m.set(k, r);
      }
    }
    return [...m.values()].sort((a, b) => b.cost - a.cost).slice(0, 5);
  })();

  const sessionName = (accountId: string, sessionId: string) => projectName(sessions.get(`${accountId}:${sessionId}`)?.project ?? null);
  const nameOf = (id: string) => data.accounts.find((a) => a.id === id)?.name ?? id;

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Timeline</h1>
          <p>Every account on one time axis, from a month down to the second. Scroll to zoom, drag to move, arrow keys to step through events. Costs are API-equivalent.</p>
        </div>
      </header>

      <div className="card card-pad stack mb-6">
        <div className="tl-toolbar">
          <Segmented<string> label="Zoom presets" value={PRESETS.find((p) => Math.abs(p.span - span) / p.span < 0.05)?.label ?? ""} onChange={(l) => setSpan(PRESETS.find((p) => p.label === l)!.span)} options={PRESETS.map((p) => ({ value: p.label, label: p.label }))} />
          <div className="actions" dir="ltr">
            <Button size="sm" variant="ghost" aria-label="Zoom out" disabled={span >= MAX_SPAN} onClick={() => setWindow(zoom(start, end, 2, (start + end) / 2, now))}>
              −
            </Button>
            <Button size="sm" variant="ghost" aria-label="Zoom in" disabled={span <= MIN_SPAN} onClick={() => setWindow(zoom(start, end, 0.5, (start + end) / 2, now))}>
              +
            </Button>
            <Button size="sm" variant="ghost" icon="earlier" aria-label="Earlier" onClick={() => setWindow(pan(start, end, -span / 2, now))} />
            <Button size="sm" variant="ghost" icon="later" aria-label="Later" onClick={() => setWindow(pan(start, end, span / 2, now))} />
            <Button size="sm" variant="ghost" onClick={() => setWindow([now - span * 0.98, now + span * 0.02])}>
              Now
            </Button>
          </div>
          <div className="actions">
            <Segmented<string> label="Replay speed" value={String(speed)} onChange={(v) => setSpeed(Number(v) as (typeof SPEEDS)[number])} options={SPEEDS.map((s) => ({ value: String(s), label: `${s}×` }))} />
            <Button size="sm" icon={replay ? "stop" : "play"} disabled={detail === "days"} aria-describedby={detail === "days" ? "tl-replay-hint" : undefined} onClick={() => setReplay(replay ? null : { speed, at: start })}>
              {replay ? "Stop" : "Replay"}
            </Button>
            {detail === "days" && (
              <span className="hint" id="tl-replay-hint">
                Zoom to a day or less to replay.
              </span>
            )}
          </div>
        </div>
        <p className="hint" aria-live={quiet ? "off" : "polite"} aria-atomic="true">
          {preciseTime(settled[0]).slice(0, -4)} to {preciseTime(Math.min(settled[1], now)).slice(0, -4)} · {shortDuration(Math.min(end, now) - start)} · {money(rangeCost)} · {compact(rangeTokens)} tokens
          {detail !== "days" && ` · ${plural(callsInView.length, "model call")} · ${plural(toolsInView.length, "tool run")}`}
          {truncated && " · showing the most recent 20,000 events"}
          {loading && " · reading…"}
        </p>
      </div>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => setAttempt((n) => n + 1)}>Try again</Button>}>
            <p>Facet could not read the transcripts: {error}</p>
          </Callout>
        </div>
      )}

      <Plot
        accounts={accounts}
        start={start}
        end={end}
        now={now}
        detail={detail}
        events={visibleEvents}
        tasks={tasks}
        hours={hoursInView}
        selected={selected}
        playhead={replay?.at ?? null}
        onSelect={setSelected}
        onWindow={setWindow}
        onStep={stepEvent}
        onDragging={setDragging}
        loading={loading && events.length === 0}
      />

      <div className="two-col mt-6">
        <section className="card card-pad stack" aria-labelledby="tl-inspector">
          <h2 className="section-title" id="tl-inspector">
            Inspector
          </h2>
          <div id="tl-inspector-body" className="stack" aria-live={quiet ? "off" : "polite"}>
          {selected ? (
            <Inspector selection={selected} account={data.accounts.find((a) => a.id === (selected.kind === "event" ? selected.event.accountId : selected.task.accountId))} session={sessions.get(selected.kind === "event" ? `${selected.event.accountId}:${selected.event.sessionId}` : `${selected.task.accountId}:${selected.task.sessionId}`)} home={data.info?.home} tasks={tasks} onZoom={(s, e) => setWindow([s, e])} onStep={stepEvent} />
          ) : (
            <p className="muted">{detail === "days" ? "Zoom to a day or less, then click a task or event to see it here, to the millisecond. Arrow keys move the view." : "Click a task or event, or use the arrow keys on the timeline."}</p>
          )}
          </div>
        </section>
        <section className="card card-pad stack" aria-labelledby="tl-board">
          <h2 className="section-title" id="tl-board">
            Most expensive in view
          </h2>
          {topSessions.length === 0 ? (
            <p className="muted">{loading || !report ? "Reading…" : "Nothing in this range."}</p>
          ) : (
            <ul className="list" aria-label="Most expensive sessions in view">
              {topSessions.map((s) => {
                const a = data.accounts.find((x) => x.id === s.accountId);
                const sum = sessions.get(`${s.accountId}:${s.sessionId}`);
                return (
                  <li key={s.sessionId} className="list-row list-row--metric">
                    {a ? <Avatar {...avatarOf(a)} label={a.name} /> : <span />}
                    <button type="button" className="session-toggle" onClick={() => sum && setWindow([sum.startedMs - 60_000, sum.endedMs + 60_000])}>
                      <span className="title">{sessionName(s.accountId, s.sessionId)}</span>
                      <span className="hint">Session · zoom to it</span>
                    </button>
                    <span className="num">{money(s.cost)}</span>
                  </li>
                );
              })}
            </ul>
          )}
          {topTasks.length > 0 && (
            <>
              <h3 className="eco-h">Tasks</h3>
              <ul className="list" aria-label="Most expensive tasks in view">
                {topTasks.map((t) => (
                  <li key={t.key} className="list-row list-row--metric">
                    {data.accounts.find((a) => a.id === t.accountId) ? <Avatar {...avatarOf(data.accounts.find((a) => a.id === t.accountId)!)} label={nameOf(t.accountId)} /> : <span />}
                    <button type="button" className="session-toggle" onClick={() => (setSelected({ kind: "task", task: t }), setWindow([t.start - (t.end - t.start) * 0.1 - 5000, t.end + (t.end - t.start) * 0.1 + 5000]))}>
                      <span className="title">{sessionName(t.accountId, t.sessionId)}</span>
                      <span className="hint">
                        {nameOf(t.accountId)} · {shortDuration(t.end - t.start)} · {plural(t.calls, "call")}
                      </span>
                    </button>
                    <span className="num">{money(t.cost)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
          {longestTools.length > 0 && (
            <>
              <h3 className="eco-h">Longest tool runs</h3>
              <ul className="list" aria-label="Longest tool runs in view">
                {longestTools.map((e, i) => (
                  <li key={`${e.ts}-${i}`} className="list-row list-row--metric">
                    {data.accounts.find((a) => a.id === e.accountId) ? <Avatar {...avatarOf(data.accounts.find((a) => a.id === e.accountId)!)} label={nameOf(e.accountId)} /> : <span />}
                    <button type="button" className="session-toggle" onClick={() => setSelected({ kind: "event", event: e })}>
                      <span className="title">{toolLabel(e.tool ?? "")}</span>
                      <span className="hint">
                        {nameOf(e.accountId)} · {sessionName(e.accountId, e.sessionId)}
                      </span>
                    </button>
                    <span className="num">{shortDuration(e.endTs! - e.ts)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </section>
      </div>
    </>
  );
}

function useNow() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(t);
  }, []);
  return now;
}

function Plot({
  accounts,
  start,
  end,
  now,
  detail,
  events,
  tasks,
  hours,
  selected,
  playhead,
  onSelect,
  onWindow,
  onStep,
  onDragging,
  loading,
}: {
  accounts: Account[];
  start: number;
  end: number;
  now: number;
  detail: ReturnType<typeof detailFor>;
  events: TimelineEvent[];
  tasks: Task[];
  hours: { hourMs: number; cost: number; accountId: string }[];
  selected: Selection;
  playhead: number | null;
  onSelect: (s: Selection) => void;
  onWindow: (w: [number, number]) => void;
  onStep: (dir: -1 | 1) => void;
  onDragging: (on: boolean) => void;
  loading: boolean;
}) {
  const wrap = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);
  const drag = useRef<{ x: number; start: number; end: number; moved: boolean } | null>(null);
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const measure = () => setWidth(Math.max(240, el.clientWidth || 800));
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const span = end - start;
  const x = (t: number) => ((t - start) / span) * width;
  const tOf = (px: number) => start + (px / width) * span;
  const height = HIST + accounts.length * LANE + AXIS;
  const laneY = (id: string) => HIST + accounts.findIndex((a) => a.id === id) * LANE;
  const colorOf = (id: string) => accounts.find((a) => a.id === id)?.color ?? 1;

  // Spend histogram, stacked by account.
  const bucket = bucketFor(span);
  const hist = new Map<number, Map<string, number>>();
  const add = (t: number, id: string, c: number) => {
    if (t < start - bucket || t > end) return;
    const b = Math.floor(t / bucket) * bucket;
    const m = hist.get(b) ?? new Map<string, number>();
    m.set(id, (m.get(id) ?? 0) + c);
    hist.set(b, m);
  };
  if (detail === "days") for (const h of hours) add(h.hourMs, h.accountId, h.cost);
  else for (const e of events) if (e.kind === "call") add(e.ts, e.accountId, e.cost);
  const peak = Math.max(0, ...[...hist.values()].map((m) => [...m.values()].reduce((a, b) => a + b, 0)));

  const step = tickStep(span, Math.max(3, Math.floor(width / 110)));
  const tickList = ticks(start, end, Math.max(3, Math.floor(width / 110)));
  const maxHour = Math.max(0, ...hours.map((h) => h.cost));

  // Wheel zoom needs a non-passive listener so the page does not scroll too.
  const wheelRef = useRef<(e: WheelEvent) => void>(() => {});
  wheelRef.current = (e: WheelEvent) => {
    e.preventDefault();
    const rect = wrap.current!.getBoundingClientRect();
    const at = tOf(e.clientX - rect.left);
    onWindow(zoom(start, end, e.deltaY > 0 ? 1.25 : 0.8, at, now));
  };
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const fn = (e: WheelEvent) => wheelRef.current(e);
    el.addEventListener("wheel", fn, { passive: false });
    return () => el.removeEventListener("wheel", fn);
  }, []);
  const pick = (px: number, py: number) => {
    const lane = accounts[Math.floor((py - HIST) / LANE)];
    if (!lane) return;
    const t = tOf(px);
    if (detail === "events") {
      let best: TimelineEvent | null = null;
      let bestD = 8;
      for (const e of events) {
        if (e.accountId !== lane.id) continue;
        const d = Math.abs(x(e.ts) - px);
        if (d < bestD) {
          best = e;
          bestD = d;
        }
      }
      if (best) return onSelect({ kind: "event", event: best });
    }
    const task = tasks.find((k) => k.accountId === lane.id && t >= k.start - span * 0.005 && t <= k.end + span * 0.005);
    onSelect(task ? { kind: "task", task } : null);
  };

  const label = `Timeline of ${accounts.map((a) => a.name).join(" and ")} from ${preciseTime(start)} to ${preciseTime(end)}. Arrow keys step through events, plus and minus zoom.`;

  return (
    <div className="card tl-card">
      <div className="tl-grid">
        <div className="tl-labels" aria-hidden="true">
          <div style={{ blockSize: HIST }} className="tl-label tl-label--hist">
            Spend
            <span className="num">{peak > 0 ? `peak ${money(peak)}` : ""}</span>
          </div>
          {accounts.map((a) => (
            <div key={a.id} className="tl-label" style={{ blockSize: LANE }}>
              <Avatar {...avatarOf(a)} />
              <span>{a.name}</span>
            </div>
          ))}
        </div>
        <div
          ref={wrap}
          className="tl-plot"
          tabIndex={0}
          role="group"
          aria-label={label}
          aria-describedby="tl-inspector-body"
          onKeyDown={(e) => {
            if (e.key === "ArrowRight") (e.preventDefault(), onStep(1));
            else if (e.key === "ArrowLeft") (e.preventDefault(), onStep(-1));
            else if (e.key === "+" || e.key === "=") onWindow(zoom(start, end, 0.5, selected?.kind === "event" ? selected.event.ts : (start + end) / 2, now));
            else if (e.key === "-") onWindow(zoom(start, end, 2, (start + end) / 2, now));
            else if (e.key === "Escape") onSelect(null);
          }}
          onPointerDown={(e) => {
            (e.target as Element).setPointerCapture?.(e.pointerId);
            drag.current = { x: e.clientX, start, end, moved: false };
            onDragging(true);
          }}
          onPointerMove={(e) => {
            const d = drag.current;
            if (!d) return;
            const dx = e.clientX - d.x;
            if (Math.abs(dx) > 3) d.moved = true;
            if (d.moved) onWindow(pan(d.start, d.end, (-dx / width) * (d.end - d.start), now));
          }}
          onPointerUp={(e) => {
            const d = drag.current;
            drag.current = null;
            onDragging(false);
            if (d && !d.moved) {
              const rect = wrap.current!.getBoundingClientRect();
              pick(e.clientX - rect.left, e.clientY - rect.top);
            }
          }}
          onDoubleClick={() => {
            if (selected?.kind === "task") onWindow([selected.task.start - 5000, selected.task.end + 5000]);
          }}
        >
          <svg width={width} height={height} className="tl-svg" aria-hidden="true">
            {/* histogram */}
            {[...hist.entries()].map(([b, m]) => {
              let y = HIST - 4;
              const w = Math.max(1, x(b + bucket) - x(b) - 1);
              return [...m.entries()].map(([id, c]) => {
                const h = peak > 0 ? (c / peak) * (HIST - 10) : 0;
                y -= h;
                return <rect key={`${b}-${id}`} x={x(b)} y={y} width={w} height={h} className="tl-acc" style={accountStyle(colorOf(id))} />;
              });
            })}
            {/* lanes */}
            {accounts.map((a, i) => (
              <rect key={a.id} x={0} y={HIST + i * LANE} width={width} height={LANE} className={i % 2 ? "tl-lane tl-lane--alt" : "tl-lane"} />
            ))}
            {tickList.map((t) => (
              <line key={t} x1={x(t)} x2={x(t)} y1={HIST} y2={height - AXIS} className="tl-grid-line" />
            ))}
            {detail === "days" &&
              hours.map((h) => (
                <rect key={`${h.accountId}-${h.hourMs}`} x={x(h.hourMs)} y={laneY(h.accountId) + 8} width={Math.max(1, x(h.hourMs + HOUR) - x(h.hourMs))} height={LANE - 16} className="tl-acc" style={{ ...accountStyle(colorOf(h.accountId)), opacity: maxHour > 0 ? 0.5 + 0.5 * (h.cost / maxHour) : 0.5 }} />
              ))}
            {detail !== "days" &&
              tasks
                .filter((t) => t.end >= start && t.start <= end)
                .map((t) => (
                  <rect
                    key={t.key}
                    x={x(t.start)}
                    y={laneY(t.accountId) + (detail === "events" ? 26 : 12)}
                    width={Math.max(2, x(t.end) - x(t.start))}
                    height={detail === "events" ? 4 : LANE - 24}
                    rx={2}
                    className={`tl-acc tl-task${selected?.kind === "task" && selected.task.key === t.key ? " tl-selected" : ""}`}
                    style={accountStyle(colorOf(t.accountId))}
                  />
                ))}
            {detail === "events" &&
              events
                .filter((e) => (e.endTs ?? e.ts) >= start && e.ts <= end)
                .map((e, i) => {
                  const y = laneY(e.accountId);
                  const sel = sameEvent(selected, e);
                  if (e.kind === "prompt") return <rect key={i} x={x(e.ts) - 1} y={y + 4} width={2} height={LANE - 8} className={`tl-prompt${sel ? " tl-selected" : ""}`} />;
                  if (e.kind === "call") {
                    const r = Math.min(10, 2.5 + Math.sqrt(e.cost) * 6);
                    return <circle key={i} cx={x(e.ts)} cy={y + 14} r={r} className={`tl-acc${sel ? " tl-selected" : ""}`} style={accountStyle(colorOf(e.accountId))} />;
                  }
                  const x0 = x(e.ts);
                  const x1 = e.endTs ? x(e.endTs) : x0;
                  return (
                    <g key={i}>
                      {x1 - x0 > 3 && <line x1={x0} x2={x1} y1={y + 32} y2={y + 32} className={e.ok === false ? "tl-tail tl-bad" : "tl-tail"} />}
                      <rect x={x0 - 3} y={y + 29} width={6} height={6} className={`${e.ok === false ? "tl-tool tl-bad" : "tl-tool"}${sel ? " tl-selected" : ""}`} />
                    </g>
                  );
                })}
            {now >= start && now <= end && <line x1={x(now)} x2={x(now)} y1={0} y2={height - AXIS} className="tl-now" />}
            {playhead !== null && <line x1={x(playhead)} x2={x(playhead)} y1={0} y2={height - AXIS} className="tl-playhead" />}
            {/* axis */}
            {tickList.map((t) => (
              <text key={t} x={x(t) > width - 48 ? x(t) - 3 : x(t) + 3} y={height - 9} className="tl-tick" textAnchor={x(t) > width - 48 ? "end" : "start"}>
                {tickLabel(t, step)}
              </text>
            ))}
          </svg>
          {accounts.length === 0 && <p className="muted card-pad">Add an account to see its timeline.</p>}
          {loading && accounts.length > 0 && (
            <div className="tl-loading" role="status">
              Reading transcripts…
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function Inspector({ selection, account, session, home, tasks, onZoom, onStep }: { selection: NonNullable<Selection>; account?: Account; session?: SessionSummary; home?: string; tasks: Task[]; onZoom: (s: number, e: number) => void; onStep: (dir: -1 | 1) => void }) {
  const head = (
    <div className="cell-title">
      {account && <Avatar {...avatarOf(account)} label={account.name} />}
      <span>
        {account?.name} · {projectName(session?.project ?? null)}
        {session?.branch ? ` · ${session.branch}` : ""}
      </span>
    </div>
  );
  if (selection.kind === "task") {
    const t = selection.task;
    return (
      <>
        {head}
        <dl className="kv">
          <dt>Task</dt>
          <dd>
            {preciseTime(t.start)} to {preciseTime(t.end)} ({shortDuration(t.end - t.start)})
          </dd>
          <dt>Work</dt>
          <dd>
            {plural(t.calls, "model call")}, {plural(t.tools, "tool run")}
          </dd>
          <dt>Cost</dt>
          <dd>{money(t.cost)} API-equivalent</dd>
          <dt>Folder</dt>
          <dd className="mono">{tildify(session?.project, home) || "unknown"}</dd>
        </dl>
        <div className="actions">
          <Button size="sm" onClick={() => onZoom(t.start - 5000, t.end + 5000)}>
            Zoom to task
          </Button>
        </div>
      </>
    );
  }
  const e = selection.event;
  const task = tasks.find((k) => k.accountId === e.accountId && k.sessionId === e.sessionId && e.ts >= k.start && e.ts <= k.end);
  return (
    <>
      {head}
      <dl className="kv">
        <dt>When</dt>
        <dd className="mono">{preciseTime(e.ts)}</dd>
        <dt>What</dt>
        <dd>
          {e.kind === "prompt" ? "You sent a prompt" : e.kind === "call" ? `Model call to ${modelLabel(e.model ?? "")}` : `Tool run: ${toolLabel(e.tool ?? "")}`}
          {e.subagent && <Pill>in a subagent</Pill>}
        </dd>
        {e.kind === "call" && (
          <>
            <dt>Tokens</dt>
            <dd>
              {compact(e.totalTokens)} total, {compact(e.outputTokens)} written
            </dd>
            <dt>Cost</dt>
            <dd>{money(e.cost)} API-equivalent</dd>
          </>
        )}
        {e.kind === "tool" && (
          <>
            <dt>Took</dt>
            <dd>{e.endTs ? shortDuration(e.endTs - e.ts) : "no result recorded"}</dd>
            <dt>Result</dt>
            <dd>{e.ok === null ? "unknown" : e.ok ? "succeeded" : "reported an error"}</dd>
          </>
        )}
        {task && (
          <>
            <dt>In task</dt>
            <dd>
              {shortDuration(task.end - task.start)}, {money(task.cost)}
            </dd>
          </>
        )}
      </dl>
      <div className="actions" dir="ltr">
        <Button size="sm" variant="ghost" icon="earlier" onClick={() => onStep(-1)}>
          Previous
        </Button>
        <Button size="sm" variant="ghost" onClick={() => onStep(1)}>
          Next <Icon name="later" />
        </Button>
        {task && (
          <Button size="sm" onClick={() => onZoom(task.start - 5000, task.end + 5000)}>
            Zoom to task
          </Button>
        )}
      </div>
    </>
  );
}
