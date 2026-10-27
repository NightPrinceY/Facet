import { useState } from "react";
import { errorText } from "../lib/api";
import { tildify, plural } from "../lib/format";
import type { FacetData } from "../lib/useFacet";
import { useApi } from "../lib/useFacet";
import { useUsage } from "../lib/useUsage";
import { ZERO, addTokens, compact, dayKey, groupHours, lastDays, modelLabel, money, projectName, sessionsCsv, sessionsSince, startOfLocalDay, totalTokens, type Rollup } from "../lib/usage";
import { ShareBar, StackedBars, Stat } from "../components/Charts";
import { Avatar, Button, Callout, Segmented, Skeleton, avatarOf } from "../components/ui";
import { useToast } from "../components/Toast";
import { Legend, splitLabel } from "./TodayView";
import type { ViewId } from "../lib/nav";

type Range = "1" | "7" | "30" | "90";
type GroupBy = "account" | "project" | "model" | "where";
const RANGE_LABEL: Record<Range, string> = { "1": "Today", "7": "7 days", "30": "30 days", "90": "90 days" };

/** API-equivalent cost by account, project, model and session (PRD M3). */
export function CostsView({ data, scope, onGo }: { data: FacetData; scope: string | null; onGo: (v: ViewId) => void }) {
  const api = useApi();
  const toast = useToast();
  const [exportError, setExportError] = useState<string | null>(null);
  const [range, setRange] = useState<Range>("30");
  const [groupBy, setGroupBy] = useState<GroupBy>("project");
  const days = Number(range);
  const { report, error, loading, refresh } = useUsage(Math.max(days, 2));
  const home = data.info?.home;
  const accounts = scope ? data.accounts.filter((a) => a.id === scope) : data.accounts;
  const ids = new Set(accounts.map((a) => a.id));
  const usage = (report?.accounts ?? []).filter((a) => ids.has(a.accountId));
  const since = days === 1 ? startOfLocalDay(Date.now()) : startOfLocalDay(Date.now()) - (days - 1) * 86_400_000;
  const nameOf = (id: string) => data.accounts.find((a) => a.id === id)?.name ?? id;

  const sessions = sessionsSince(usage, since);
  const perAccount = groupHours(usage, (_, id) => id, since);
  const total = [...perAccount.values()].reduce((t, r) => ({ cost: t.cost + r.cost, prompts: t.prompts + r.prompts, turns: t.turns + r.turns, tokens: addTokens(t.tokens, r.tokens) }), { cost: 0, prompts: 0, turns: 0, tokens: { ...ZERO } });

  // Accuracy: sessions where Claude Code wrote its own figure.
  const checked = sessions.filter((s) => s.reportedCost !== null && s.reportedCost > 0);
  const reported = checked.reduce((n, s) => n + (s.reportedCost ?? 0), 0);
  const estimated = checked.reduce((n, s) => n + s.cost, 0);
  const unpriced = [...new Set(sessions.flatMap((s) => s.unpricedModels))];

  const dayList = lastDays(days === 1 ? 1 : days);
  const dayCols = dayList.map((d) => {
    const parts = accounts.map((a) => ({
      accountId: a.id,
      value: (usage.find((u) => u.accountId === a.id)?.hours ?? []).filter((h) => dayKey(h.hourMs) === d).reduce((s, h) => s + h.cost, 0),
    }));
    const date = new Date(`${d}T12:00:00`);
    return { key: d, tick: String(date.getDate()), label: `${date.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })}: ${splitLabel(parts, data.accounts)}`, parts };
  });

  const groups: Rollup[] = (() => {
    if (groupBy === "where") {
      const m = new Map<string, Rollup>();
      for (const s of sessions) {
        const k = s.entrypoint ?? "unknown";
        const r = m.get(k) ?? { key: k, cost: 0, tokens: { ...ZERO }, turns: 0, prompts: 0, sessions: 0, accounts: new Set<string>() };
        r.cost += s.cost;
        r.tokens = addTokens(r.tokens, s.tokens);
        r.turns += s.turns;
        r.prompts += s.prompts;
        r.sessions += 1;
        r.accounts.add(s.accountId);
        m.set(k, r);
      }
      return [...m.values()];
    }
    const g = groupHours(usage, groupBy === "account" ? (_, id) => id : groupBy === "model" ? (h) => h.model || "(prompts only)" : (h) => h.project ?? "", since);
    for (const r of g.values()) {
      r.sessions = sessions.filter((s) => (groupBy === "account" ? s.accountId === r.key : groupBy === "model" ? s.models.includes(r.key) : (s.project ?? "") === r.key)).length;
    }
    return [...g.values()];
  })()
    .filter((r) => r.cost > 0 || r.prompts > 0)
    .sort((a, b) => b.cost - a.cost);
  const maxGroup = Math.max(...groups.map((g) => g.cost), 0);
  const top = [...sessions].sort((a, b) => b.cost - a.cost).slice(0, 10);

  const groupName = (r: Rollup) => {
    if (groupBy === "account") return nameOf(r.key);
    if (groupBy === "model") return r.key === "(prompts only)" ? "Prompts with no reply yet" : modelLabel(r.key);
    if (groupBy === "where") return { cli: "Terminal", "claude-desktop": "Desktop app, Code tab", "claude-vscode": "VS Code", "sdk-cli": "SDK and scripts" }[r.key] ?? r.key;
    return projectName(r.key || null);
  };

  const exportCsv = async () => {
    try {
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await api.exportText(`facet-costs-${stamp}.csv`, sessionsCsv(sessions, nameOf));
      setExportError(null);
      toast(`Saved ${plural(sessions.length, "session")} to ${tildify(path, home)}.`);
    } catch (e) {
      setExportError(errorText(e));
    }
  };

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Costs</h1>
          <p>What your Claude Code work would cost at API prices, by account, project, model and session. Your subscription price does not change; this shows where the usage goes.</p>
        </div>
        <div className="actions">
          <Segmented<Range> label="Range" value={range} onChange={setRange} options={(Object.keys(RANGE_LABEL) as Range[]).map((r) => ({ value: r, label: RANGE_LABEL[r] }))} />
          <Button icon="download" disabled={sessions.length === 0} onClick={() => void exportCsv()} title="One row per session, with folder and branch names, saved to Downloads">
            Export CSV
          </Button>
        </div>
      </header>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void refresh()}>Try again</Button>}>
            <p>Facet could not read the transcripts: {error}</p>
          </Callout>
        </div>
      )}

      {exportError && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void exportCsv()}>Try again</Button>}>
            <p>The export was not saved: {exportError}</p>
          </Callout>
        </div>
      )}

      {!report ? (
        !error && (
          <div className="grid-cards" aria-busy="true" aria-label="Loading costs">
            <Skeleton card />
            <Skeleton card />
          </div>
        )
      ) : (
        <>
          <section className="stats" aria-label="Totals">
            <Stat label={`API-equivalent, ${RANGE_LABEL[range].toLowerCase()}`} value={money(total.cost)} sub={days > 1 ? `${money(total.cost / days)} a day on average` : "since midnight"} />
            <Stat label="Tokens" value={compact(totalTokens(total.tokens))} sub={`${compact(total.tokens.cacheRead)} read from cache`} />
            <Stat label="Sessions" value={sessions.length} sub={plural(total.prompts, "prompt")} />
            <Stat label="Per prompt" value={total.prompts ? money(total.cost / total.prompts) : "–"} sub={`${total.prompts ? (total.turns / total.prompts).toFixed(1) : 0} replies each`} />
          </section>

          {days > 1 && (
            <section className="card card-pad stack mb-6" aria-labelledby="costs-days">
              <h2 className="section-title" id="costs-days">
                By day
              </h2>
              <StackedBars columns={dayCols} accounts={data.accounts} caption={`Cost by day, last ${days} days, stacked by account`} tickEvery={days > 30 ? 7 : days > 7 ? 3 : 1} />
              {accounts.length > 1 && <Legend accounts={accounts} />}
            </section>
          )}

          <section className="card card-pad stack mb-6" aria-labelledby="costs-groups">
            <div className="section-head">
              <h2 className="section-title" id="costs-groups">
                Breakdown
              </h2>
              <Segmented<GroupBy>
                label="Group by"
                value={groupBy}
                onChange={setGroupBy}
                options={[
                  { value: "project", label: "Project" },
                  { value: "account", label: "Account" },
                  { value: "model", label: "Model" },
                  { value: "where", label: "Where" },
                ]}
              />
            </div>
            {groups.length === 0 ? (
              <p className="muted">No Claude Code usage in this range.</p>
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <caption className="visually-hidden">Cost grouped by {{ project: "project", account: "account", model: "model", where: "where Claude Code ran" }[groupBy]}</caption>
                  <thead>
                    <tr>
                      <th scope="col">{{ project: "Project", account: "Account", model: "Model", where: "Where" }[groupBy]}</th>
                      <th scope="col" className="num">
                        Sessions
                      </th>
                      <th scope="col" className="num">
                        Prompts
                      </th>
                      <th scope="col" className="num">
                        Tokens
                      </th>
                      <th scope="col" className="num">
                        Cost
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {groups.map((g) => (
                      <tr key={g.key}>
                        <th scope="row" className="cell-main">
                          <div className="cell-title">
                            {groupBy === "account" && data.accounts.find((a) => a.id === g.key) && <Avatar {...avatarOf(data.accounts.find((a) => a.id === g.key)!)} />}
                            {groupName(g)}
                          </div>
                          {groupBy === "project" && <div className="hint mono">{tildify(g.key, home) || "unknown folder"}</div>}
                          <ShareBar value={g.cost} max={maxGroup} color={groupBy === "account" ? data.accounts.find((a) => a.id === g.key)?.color : undefined} />
                        </th>
                        <td className="num">{g.sessions}</td>
                        <td className="num">{g.prompts.toLocaleString()}</td>
                        <td className="num">{compact(totalTokens(g.tokens))}</td>
                        <td className="num">{money(g.cost)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          <section className="card card-pad stack mb-6" aria-labelledby="costs-top">
            <div className="section-head">
              <h2 className="section-title" id="costs-top">
                Most expensive sessions
              </h2>
              <Button size="sm" variant="ghost" icon="sessions" onClick={() => onGo("sessions")}>
                All sessions
              </Button>
            </div>
            {top.length === 0 ? (
              <p className="muted">No sessions in this range.</p>
            ) : (
              <ul className="list" aria-label="Most expensive sessions">
                {top.map((s) => {
                  const a = data.accounts.find((x) => x.id === s.accountId);
                  return (
                    <li key={s.sessionId} className="list-row list-row--metric">
                      {a ? <Avatar {...avatarOf(a)} label={a.name} /> : <span />}
                      <div>
                        <div className="title">{projectName(s.project)}</div>
                        <div className="hint">
                          {new Date(s.endedMs).toLocaleDateString(undefined, { day: "numeric", month: "short" })} · {plural(s.prompts, "prompt")} · {s.models.map(modelLabel).join(", ")}
                          {s.branch ? ` · ${s.branch}` : ""}
                        </div>
                      </div>
                      <span className="num">{money(s.cost)}</span>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>

          <p className="hint">
            Prices: Anthropic API list prices checked {report?.pricesChecked} (<span className="mono">{report?.pricesSource}</span>), including cache reads and writes, web searches and fast mode.
            {checked.length > 0 && ` For ${plural(checked.length, "session")} where Claude Code recorded its own figure (${money(reported)}), Facet's estimate is ${money(estimated)} (${((estimated / reported) * 100).toFixed(1)}%).`}
            {unpriced.length > 0 && ` No price known for ${unpriced.join(", ")}; their tokens are counted but not priced.`}
            {loading && " Refreshing…"}
          </p>
        </>
      )}
    </>
  );
}
