import { useMemo, useState } from "react";
import { errorText } from "../lib/api";
import { plural, tildify } from "../lib/format";
import type { FacetData } from "../lib/useFacet";
import { useApi } from "../lib/useFacet";
import { useUsage } from "../lib/useUsage";
import { compact, duration, entrypointLabel, modelLabel, money, projectName, relativeTime, topTools, toolLabel, totalTokens, type SessionSummary } from "../lib/usage";
import { Avatar, Button, Callout, Field, Pill, Segmented, Skeleton, avatarOf } from "../components/ui";
import { useToast } from "../components/Toast";

type Range = "7" | "30" | "90" | "365";
type Sort = "recent" | "cost" | "long";
const PAGE = 40;

/** Words in a search: plain words match anywhere; `account:`, `model:`,
 *  `tool:`, `branch:`, `project:`, `where:` and `id:` narrow to one field. */
export function matchSession(s: SessionSummary, query: string, accountName: string): boolean {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const fields: Record<string, string> = {
    account: `${accountName} ${s.accountId}`,
    model: s.models.map((m) => `${m} ${modelLabel(m)}`).join(" "),
    tool: Object.keys(s.tools).map((t) => `${t} ${toolLabel(t)}`).join(" "),
    branch: s.branch ?? "",
    project: s.project ?? "",
    where: `${s.entrypoint ?? ""} ${entrypointLabel(s.entrypoint)}`,
    id: s.sessionId,
  };
  const all = Object.values(fields).join(" ").toLowerCase();
  return words.every((w) => {
    const m = /^([a-z]+):(.+)$/.exec(w);
    if (m && m[1]! in fields) return fields[m[1]!]!.toLowerCase().includes(m[2]!);
    return all.includes(w);
  });
}

/** Search sessions across accounts by metadata and resume one in its own
 *  account (PRD M6). Titles and message text are not shown (rule 9). */
export function SessionsView({ data, scope }: { data: FacetData; scope: string | null }) {
  const api = useApi();
  const toast = useToast();
  const [range, setRange] = useState<Range>("30");
  const [sort, setSort] = useState<Sort>("recent");
  const [query, setQuery] = useState("");
  const [shown, setShown] = useState(PAGE);
  const [open, setOpen] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const { report, error, refresh } = useUsage(Number(range));
  const home = data.info?.home;
  const nameOf = (id: string) => data.accounts.find((a) => a.id === id)?.name ?? id;

  const all = useMemo(() => (report?.accounts ?? []).filter((a) => !scope || a.accountId === scope).flatMap((a) => a.sessions), [report, scope]);
  const list = useMemo(() => {
    const found = all.filter((s) => matchSession(s, query, data.accounts.find((a) => a.id === s.accountId)?.name ?? s.accountId));
    return found.sort((a, b) => (sort === "cost" ? b.cost - a.cost : sort === "long" ? b.endedMs - b.startedMs - (a.endedMs - a.startedMs) : b.endedMs - a.endedMs));
  }, [all, query, sort, data.accounts]);

  const resume = async (s: SessionSummary) => {
    try {
      await api.resumeSession(s.accountId, s.sessionId, s.project);
      setActionError(null);
      toast(`Opened a terminal as ${nameOf(s.accountId)} resuming the session.`);
    } catch (e) {
      setActionError(`Could not resume the ${projectName(s.project)} session: ${errorText(e)}`);
    }
  };

  const copyId = async (s: SessionSummary) => {
    try {
      await navigator.clipboard.writeText(s.sessionId);
      toast("Copied the session id.");
    } catch {
      toast(`Session id: ${s.sessionId}`);
    }
  };

  const total = list.reduce((n, s) => n + s.cost, 0);

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Sessions</h1>
          <p>Every Claude Code session on this computer, across accounts. Search by folder, branch, model, tool or id, and resume a session in the account it belongs to. Message text stays out of Facet.</p>
        </div>
        <div className="actions">
          <Segmented<Range> label="Range" value={range} onChange={(r) => (setRange(r), setShown(PAGE))} options={[{ value: "7", label: "7 days" }, { value: "30", label: "30 days" }, { value: "90", label: "90 days" }, { value: "365", label: "Year" }]} />
        </div>
      </header>

      <div className="card card-pad stack mb-6">
        <div className="search-row">
          <Field label="Search" htmlFor="ses-q" hint="Try: api, branch:main, tool:WebSearch, model:sonnet, account:work, where:desktop">
            <input id="ses-q" className="input" type="search" value={query} placeholder="Folder, branch, model, tool or id" onChange={(e) => (setQuery(e.target.value), setShown(PAGE))} aria-describedby="ses-q-hint" autoComplete="off" spellCheck={false} />
          </Field>
          <Segmented<Sort> label="Sort" value={sort} onChange={setSort} options={[{ value: "recent", label: "Recent" }, { value: "cost", label: "Cost" }, { value: "long", label: "Longest" }]} />
        </div>
        <p className="hint" aria-live="polite">
          {report ? `${plural(list.length, "session")} · ${money(total)} API-equivalent for whole sessions` : "Reading transcripts…"}
        </p>
      </div>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void refresh()}>Try again</Button>}>
            <p>Facet could not read the transcripts: {error}</p>
          </Callout>
        </div>
      )}

      {actionError && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert">
            <p>{actionError}</p>
          </Callout>
        </div>
      )}

      {!report ? (
        !error && <Skeleton card />
      ) : list.length === 0 ? (
        <div className="card empty">
          <h2>{all.length === 0 ? "No sessions in this range" : "Nothing matches"}</h2>
          <p>{all.length === 0 ? "Claude Code sessions appear here once an account has used Claude Code." : "Try fewer words, or a different field such as branch: or tool:."}</p>
        </div>
      ) : (
        <div className="card">
          <ul className="list" aria-label="Sessions">
            {list.slice(0, shown).map((s) => {
              const a = data.accounts.find((x) => x.id === s.accountId);
              const expanded = open === s.sessionId;
              return (
                <li key={`${s.accountId}:${s.sessionId}`} className="session-row">
                  <div className="session-main">
                    {a ? <Avatar {...avatarOf(a)} label={a.name} /> : <span />}
                    <button type="button" className="session-toggle" aria-expanded={expanded} onClick={() => setOpen(expanded ? null : s.sessionId)}>
                      <span className="title">{projectName(s.project)}</span>
                      <span className="hint">
                        {relativeTime(s.endedMs)} · {duration(s.endedMs - s.startedMs)} · {plural(s.prompts, "prompt")} · {entrypointLabel(s.entrypoint)}
                        {s.branch ? ` · ${s.branch}` : ""}
                      </span>
                    </button>
                    <span className="num">{money(s.cost)}</span>
                    <Button size="sm" icon="term" onClick={() => void resume(s)} aria-label={`Resume the ${projectName(s.project)} session in ${a?.name ?? s.accountId}`}>
                      Resume
                    </Button>
                  </div>
                  {expanded && (
                    <div className="session-detail">
                      <dl className="kv">
                        <dt>Folder</dt>
                        <dd className="mono">{tildify(s.project, home) || "unknown"}</dd>
                        <dt>Session id</dt>
                        <dd className="mono">
                          {s.sessionId}{" "}
                          <Button size="sm" variant="ghost" onClick={() => void copyId(s)}>
                            Copy
                          </Button>
                        </dd>
                        <dt>When</dt>
                        <dd>
                          {new Date(s.startedMs).toLocaleString()} to {new Date(s.endedMs).toLocaleTimeString()}
                        </dd>
                        <dt>Models</dt>
                        <dd>{s.models.map(modelLabel).join(", ") || "none"}</dd>
                        <dt>Activity</dt>
                        <dd>
                          {plural(s.prompts, "prompt")}, {plural(s.turns, "reply", "replies")}
                          {s.subagents > 0 && `, ${plural(s.subagents, "subagent")}`}
                        </dd>
                        <dt>Tokens</dt>
                        <dd>
                          {compact(totalTokens(s.tokens))} total: {compact(s.tokens.input)} in, {compact(s.tokens.output)} out, {compact(s.tokens.cacheRead)} cache read, {compact(s.tokens.cacheWrite)} cache write
                          {s.tokens.webSearches > 0 && `, ${plural(s.tokens.webSearches, "web search", "web searches")}`}
                        </dd>
                        <dt>Cost</dt>
                        <dd>
                          {money(s.cost)} API-equivalent
                          {s.reportedCost !== null && ` · Claude Code recorded ${money(s.reportedCost)}`}
                        </dd>
                        {s.version && (
                          <>
                            <dt>Claude Code</dt>
                            <dd>{s.version}</dd>
                          </>
                        )}
                      </dl>
                      {Object.keys(s.tools).length > 0 && (
                        <div className="pills" aria-label="Tools used">
                          {topTools(s.tools, 12).map(([t, n]) => (
                            <Pill key={t}>
                              {toolLabel(t)} × {n}
                            </Pill>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
          {list.length > shown && (
            <div className="card-pad">
              <Button onClick={() => setShown(shown + PAGE)}>Show {Math.min(PAGE, list.length - shown)} more</Button>
            </div>
          )}
        </div>
      )}
    </>
  );
}
