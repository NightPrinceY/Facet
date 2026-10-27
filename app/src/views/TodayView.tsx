import type { Account } from "../lib/api";
import { tildify, plural } from "../lib/format";
import type { FacetData } from "../lib/useFacet";
import { useUsage } from "../lib/useUsage";
import { ZERO, addTokens, compact, groupHours, money, projectName, relativeTime, sessionsSince, startOfLocalDay, totalTokens } from "../lib/usage";
import { ShareBar, StackedBars, Stat } from "../components/Charts";
import { Avatar, Button, Callout, Pill, Skeleton, accountStyle, avatarOf } from "../components/ui";
import type { ViewId } from "../lib/nav";

const HOURS = Array.from({ length: 24 }, (_, i) => i);

/** What each account did since local midnight (PRD Today). */
export function TodayView({ data, scope, onGo, onAdd }: { data: FacetData; scope: string | null; onGo: (v: ViewId) => void; onAdd: () => void }) {
  const { report, error, loading, refresh } = useUsage(2);
  const home = data.info?.home;
  const midnight = startOfLocalDay(Date.now());
  const accounts = scope ? data.accounts.filter((a) => a.id === scope) : data.accounts;
  const ids = new Set(accounts.map((a) => a.id));
  const usage = (report?.accounts ?? []).filter((a) => ids.has(a.accountId));

  const perAccount = groupHours(usage, (_, id) => id, midnight);
  const all = [...perAccount.values()].reduce((t, r) => ({ cost: t.cost + r.cost, prompts: t.prompts + r.prompts, turns: t.turns + r.turns, tokens: addTokens(t.tokens, r.tokens) }), { cost: 0, prompts: 0, turns: 0, tokens: { ...ZERO } });
  const sessions = sessionsSince(usage, midnight);
  const projects = [...groupHours(usage, (h) => h.project ?? "", midnight).values()].sort((a, b) => b.cost - a.cost).slice(0, 8);
  const maxProject = Math.max(...projects.map((p) => p.cost), 0);
  const waiting = accounts.filter((a) => (data.states[a.id]?.waitingSessions ?? 0) > 0);
  const live = accounts.reduce((n, a) => n + (data.states[a.id]?.liveSessions ?? 0), 0);

  const hourColumns = (accountIds: string[]) =>
    HOURS.map((h) => {
      const start = midnight + h * 3_600_000;
      const parts = accountIds.map((id) => ({
        accountId: id,
        value: (usage.find((u) => u.accountId === id)?.hours ?? []).filter((b) => b.hourMs >= start && b.hourMs < start + 3_600_000).reduce((s, b) => s + b.cost, 0),
      }));
      return { key: String(h), tick: String(h).padStart(2, "0"), label: `${String(h).padStart(2, "0")}:00 to ${String(h + 1).padStart(2, "0")}:00: ${splitLabel(parts, data.accounts)}`, parts };
    });

  const hello = new Date().getHours() < 12 ? "Good morning" : new Date().getHours() < 18 ? "Good afternoon" : "Good evening";

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Today</h1>
          <p>
            {hello}. {accounts.length > 1 ? "Here is what your accounts did since midnight." : accounts[0] ? `Here is what ${accounts[0].name} did since midnight.` : "Add an account to see its day."} Costs are API-equivalent, from Claude Code transcripts on this computer.
          </p>
        </div>
        <div className="actions">
          <Button variant="ghost" loading={loading && Boolean(report)} onClick={() => void refresh()}>
            Refresh
          </Button>
        </div>
      </header>

      {waiting.length > 0 && (
        <div className="mb-6">
          <Callout tone="warn" icon="alert" action={<Button size="sm" onClick={() => onGo("sessions")}>Open sessions</Button>}>
            <p>
              {waiting.map((a) => `${a.name}: ${plural(data.states[a.id]!.waitingSessions, "session")}`).join(" · ")} waiting for you.
            </p>
          </Callout>
        </div>
      )}
      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void refresh()}>Try again</Button>}>
            <p>Facet could not read the transcripts: {error}</p>
          </Callout>
        </div>
      )}

      {!loading && accounts.length === 0 && (
        <div className="card empty mb-6">
          <h2>No accounts yet</h2>
          <p>Add the accounts you use, and Today shows what each one did and cost.</p>
          <Button variant="primary" icon="plus" onClick={onAdd}>
            Add account
          </Button>
        </div>
      )}

      {!report ? (
        !error && (
          <div className="grid-cards" aria-busy="true" aria-label="Loading today">
            <Skeleton card />
            <Skeleton card />
          </div>
        )
      ) : (
        <>
          <section className="stats" aria-label="Today in numbers">
            <Stat label="API-equivalent today" value={money(all.cost)} sub="Claude Code, at API prices" />
            <Stat label="Prompts" value={all.prompts.toLocaleString()} sub={`${all.turns.toLocaleString()} model replies`} />
            <Stat label="Tokens" value={compact(totalTokens(all.tokens))} sub={`${compact(all.tokens.output)} written by Claude`} />
            <Stat label="Sessions" value={sessions.length} sub={live > 0 ? `${live} running now` : "none running now"} />
          </section>

          {accounts.length > 0 && (
            <section className="card card-pad stack mb-6" aria-labelledby="today-hours">
              <h2 className="section-title" id="today-hours">
                By hour
              </h2>
              <StackedBars columns={hourColumns(accounts.map((a) => a.id))} accounts={data.accounts} caption="Cost by hour today, stacked by account" tickEvery={3} />
              {accounts.length > 1 && <Legend accounts={accounts} />}
            </section>
          )}

          <div className="grid-cards mb-6">
            {accounts.map((a) => (
              <AccountDay key={a.id} account={a} cost={perAccount.get(a.id)?.cost ?? 0} prompts={perAccount.get(a.id)?.prompts ?? 0} turns={perAccount.get(a.id)?.turns ?? 0} sessions={sessions.filter((s) => s.accountId === a.id).length} live={data.states[a.id]?.liveSessions ?? 0} running={data.states[a.id]?.desktopPid != null} />
            ))}
          </div>

          <div className="two-col">
            <section className="card card-pad stack" aria-labelledby="today-projects">
              <h2 className="section-title" id="today-projects">
                Projects today
              </h2>
              {projects.length === 0 ? (
                <p className="muted">No Claude Code work yet today.</p>
              ) : (
                <ul className="list" aria-label="Projects today">
                  {projects.map((p) => (
                    <li key={p.key} className="list-row list-row--metric">
                      <div>
                        <div className="title">{projectName(p.key || null)}</div>
                        <div className="hint mono">{tildify(p.key, home) || "unknown folder"}</div>
                        <ShareBar value={p.cost} max={maxProject} />
                      </div>
                      <span className="avatars">
                        {[...p.accounts].map((id) => {
                          const a = data.accounts.find((x) => x.id === id);
                          return a ? <Avatar key={id} {...avatarOf(a)} label={a.name} /> : null;
                        })}
                      </span>
                      <span className="num">{money(p.cost)}</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <section className="card card-pad stack" aria-labelledby="today-sessions">
              <h2 className="section-title" id="today-sessions">
                Latest sessions
              </h2>
              {sessions.length === 0 ? (
                <p className="muted">No sessions yet today.</p>
              ) : (
                <ul className="list" aria-label="Latest sessions">
                  {sessions.slice(0, 6).map((s) => {
                    const a = data.accounts.find((x) => x.id === s.accountId);
                    return (
                      <li key={s.sessionId} className="list-row list-row--metric">
                        {a ? <Avatar {...avatarOf(a)} label={a.name} /> : <span />}
                        <div>
                          <div className="title">{projectName(s.project)}</div>
                          <div className="hint">
                            {relativeTime(s.endedMs)} · {plural(s.prompts, "prompt")}
                            {s.branch ? ` · ${s.branch}` : ""}
                          </div>
                        </div>
                        <span className="num">{money(s.cost)}</span>
                      </li>
                    );
                  })}
                </ul>
              )}
              <div>
                <Button size="sm" variant="ghost" icon="sessions" onClick={() => onGo("sessions")}>
                  All sessions
                </Button>
              </div>
            </section>
          </div>
          <p className="hint mt-6">Chat in the Desktop app keeps no usage on this computer, so these figures cover Claude Code: the terminal, the Desktop app's Code tab and editor plugins.</p>
        </>
      )}
    </>
  );
}

function AccountDay({ account: a, cost, prompts, turns, sessions, live, running }: { account: Account; cost: number; prompts: number; turns: number; sessions: number; live: number; running: boolean }) {
  return (
    <article className="card account-card" style={accountStyle(a.color)} aria-label={`${a.name} today`}>
      <header>
        <Avatar {...avatarOf(a)} size="lg" running={running} label={a.name} />
        <div>
          <h2>{a.name}</h2>
          <div className="sub">{plural(sessions, "session")} today</div>
        </div>
        <span className="stat-value num">{money(cost)}</span>
      </header>
      <div className="pills">
        <Pill>{plural(prompts, "prompt")}</Pill>
        <Pill>{plural(turns, "reply", "replies")}</Pill>
        {running && (
          <Pill tone="ok" dot>
            Desktop running
          </Pill>
        )}
        {live > 0 && <Pill dot>{plural(live, "Code session")} open</Pill>}
      </div>
    </article>
  );
}

/** "Work $10.20, Personal $1.80" — the split in words, not only colour. */
export function splitLabel(parts: { accountId: string; value: number }[], accounts: Account[]): string {
  const total = parts.reduce((s, p) => s + p.value, 0);
  const named = parts.filter((p) => p.value > 0).map((p) => `${accounts.find((a) => a.id === p.accountId)?.name ?? p.accountId} ${money(p.value)}`);
  return named.length > 1 ? `${money(total)} (${named.join(", ")})` : money(total);
}

export function Legend({ accounts }: { accounts: Account[] }) {
  return (
    <ul className="legend" aria-label="Accounts">
      {accounts.map((a) => (
        <li key={a.id} style={accountStyle(a.color)}>
          <span className="legend-swatch" aria-hidden="true" />
          {a.name}
        </li>
      ))}
    </ul>
  );
}
