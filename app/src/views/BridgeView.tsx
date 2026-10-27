import { useCallback, useEffect, useState } from "react";
import type { Account, Handoff, HandoffFile } from "../lib/api";
import { errorText } from "../lib/api";
import type { ControlOp, ControlPreview } from "../lib/control";
import { plural, tildify } from "../lib/format";
import type { FacetData } from "../lib/useFacet";
import { useApi } from "../lib/useFacet";
import { useUsage } from "../lib/useUsage";
import { projectName, relativeTime } from "../lib/usage";
import { Avatar, Button, Callout, Checkbox, Field, Modal, Segmented, Skeleton, avatarOf, describedBy } from "../components/ui";
import { DiffView } from "../components/DiffView";
import { useToast } from "../components/Toast";

type Inbound = "accept" | "hold" | "refuse";
const INBOUND_WORD: Record<Inbound, string> = { accept: "Accepts messages", hold: "Holds messages for you", refuse: "Refuses messages" };

/** Bridge v1 (PRD M8, D-010): share context between your own accounts with a
 *  handoff file you open in the other account. */
export function BridgeView({ data, scope }: { data: FacetData; scope: string | null }) {
  const api = useApi();
  const toast = useToast();
  const { report } = useUsage(14);
  const [fromId, setFromId] = useState<string>(scope ?? data.accounts[0]?.id ?? "");
  const [sessionId, setSessionId] = useState("");
  const [toId, setToId] = useState<string>("");
  const [note, setNote] = useState("");
  const [includeGit, setIncludeGit] = useState(true);
  const [draft, setDraft] = useState<Handoff | null>(null);
  const [saved, setSaved] = useState<Handoff | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [files, setFiles] = useState<HandoffFile[] | null>(null);
  const [filesError, setFilesError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [gitConfirmed, setGitConfirmed] = useState(false);

  const others = data.accounts.filter((a) => a.id !== fromId);
  const target = toId && toId !== fromId ? toId : (others[0]?.id ?? "");
  const sessions = (report?.accounts.find((a) => a.accountId === fromId)?.sessions ?? []).slice(0, 25);
  const session = sessions.find((s) => s.sessionId === sessionId) ?? sessions[0] ?? null;
  const nameOf = (id: string) => data.accounts.find((a) => a.id === id)?.name ?? id;

  const loadFiles = useCallback(async () => {
    try {
      setFiles(await api.bridgeList());
      setFilesError(null);
    } catch (e) {
      setFilesError(errorText(e));
    }
  }, [api]);

  useEffect(() => {
    void loadFiles();
  }, [loadFiles]);

  const makeDraft = async () => {
    if (!session || !target) return;
    setBusy("draft");
    setError(null);
    try {
      setDraft(await api.bridgeDraft(fromId, session.sessionId, target, note, includeGit));
      setSaved(null);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    if (!draft) return;
    setBusy("save");
    setError(null);
    try {
      const h = await api.bridgeSave(draft.name, draft.text);
      setSaved(h);
      setDraft(null);
      toast(`Saved the handoff for ${nameOf(target)}.`);
      await loadFiles();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const open = async (name: string, toAccount: string, cwd: string | null, fromList = false) => {
    setBusy(`open:${name}`);
    if (fromList) setFilesError(null);
    else setError(null);
    try {
      await api.bridgeOpen(toAccount, name, cwd);
      toast(`Opened a terminal as ${nameOf(toAccount)} with the handoff.`);
    } catch (e) {
      if (fromList) setFilesError(errorText(e));
      else setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const remove = async (name: string) => {
    try {
      await api.bridgeRemove(name);
      setConfirmDelete(null);
      toast("Deleted the handoff.");
      await loadFiles();
    } catch (e) {
      setFilesError(errorText(e));
    }
  };

  const targetOf = (name: string) => /-to-([a-z0-9-]+)\.md$/.exec(name)?.[1] ?? null;
  const accountOf = (id: string) => data.accounts.find((a) => a.id === id);
  // Work accounts (or accounts whose plan Facet does not know) get a warning,
  // and their code changes stay out unless the user confirms (D-010).
  const isWorkish = (id: string) => {
    const plan = accountOf(id)?.plan;
    return !plan || /team|enterprise|business|work/i.test(plan);
  };
  const workish = [fromId, target].filter((id) => id && isWorkish(id));
  const gitNeedsConfirm = includeGit && isWorkish(fromId);
  const canWrite = Boolean(session && target) && (!gitNeedsConfirm || gitConfirmed);

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Bridge</h1>
          <p>Share context between your own accounts with a handoff file: where the work is, the project's uncommitted changes and your note. You open it in the other account yourself. Facet never sends messages between sessions and never copies the conversation.</p>
        </div>
      </header>

      {data.accounts.length < 2 ? (
        <div className="card empty">
          <h2>Bridge needs two accounts</h2>
          <p>Add another account to share context between them.</p>
        </div>
      ) : (
        <>
          <section className="card card-pad stack mb-6" aria-labelledby="br-new">
            <h2 className="section-title" id="br-new">
              New handoff
            </h2>
            <div className="form-grid">
              <Field label="From account" htmlFor="br-from">
                <div className="select-with-avatar">
                  {accountOf(fromId) && <Avatar {...avatarOf(accountOf(fromId)!)} />}
                <select id="br-from" className="input" value={fromId} onChange={(e) => (setFromId(e.target.value), setSessionId(""), setDraft(null))}>
                  {data.accounts.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.name}
                    </option>
                  ))}
                </select>
                </div>
              </Field>
              <Field label="Session" htmlFor="br-session" hint={report ? (sessions.length ? "Recent Claude Code sessions of this account." : "This account has no Claude Code sessions in the last 14 days.") : "Reading sessions…"}>
                <select id="br-session" className="input" value={session?.sessionId ?? ""} onChange={(e) => (setSessionId(e.target.value), setDraft(null))} disabled={!sessions.length} aria-describedby="br-session-hint">
                  {sessions.map((s) => (
                    <option key={s.sessionId} value={s.sessionId}>
                      {projectName(s.project)} · {relativeTime(s.endedMs)}
                      {s.branch ? ` · ${s.branch}` : ""}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="To account" htmlFor="br-to">
                <div className="select-with-avatar">
                  {accountOf(target) && <Avatar {...avatarOf(accountOf(target)!)} />}
                  <select id="br-to" className="input" value={target} onChange={(e) => (setToId(e.target.value), setDraft(null))}>
                    {others.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name}
                      </option>
                    ))}
                  </select>
                </div>
              </Field>
            </div>
            <Field label="Note for the other account" htmlFor="br-note" hint="What you were doing and what is next. This is the only text from you that goes in the file.">
              <textarea id="br-note" className="input textarea" rows={4} value={note} onChange={(e) => (setNote(e.target.value), setDraft(null))} aria-describedby={describedBy("br-note", null, "hint")} />
            </Field>
            <Checkbox checked={includeGit} onChange={(v) => (setIncludeGit(v), setDraft(null))} label="Include the project's uncommitted changes (git)" hint="Changed files and the diff. Lines that look like secrets, and files such as .env, are hidden." />
            {workish.length > 0 && (
              <Callout tone="warn" icon="alert">
                <p>
                  {(() => {
                    const known = workish.filter((id) => accountOf(id)?.plan);
                    const unknown = workish.filter((id) => !accountOf(id)?.plan);
                    const parts = [
                      known.length ? `${known.map(nameOf).join(" and ")} ${known.length > 1 ? "are" : "is"} on a work plan` : "",
                      unknown.length ? `${unknown.map(nameOf).join(" and ")} ${unknown.length > 1 ? "have" : "has"} no plan set in Facet, so ${unknown.length > 1 ? "they may be work accounts" : "it may be a work account"}` : "",
                    ].filter(Boolean);
                    return `${parts.join("; ")}. A handoff can carry an employer's code and notes into another account; check that this is allowed. Set plans in Edit to quiet this for personal accounts.`;
                  })()}
                </p>
              </Callout>
            )}
            {gitNeedsConfirm && <Checkbox checked={gitConfirmed} onChange={setGitConfirmed} label={`I may share ${nameOf(fromId)}'s code changes with ${nameOf(target)}`} />}
            <div className="actions">
              <Button variant="primary" loading={busy === "draft"} disabled={!canWrite} onClick={() => void makeDraft()}>
                Write handoff
              </Button>
            </div>
            {error && (
              <Callout tone="bad" icon="alert">
                <p>{error}</p>
              </Callout>
            )}
            {draft && (
              <div className="stack">
                <Field label="Handoff (you can edit it before saving)" htmlFor="br-text">
                  <textarea id="br-text" className="input textarea editor" rows={16} spellCheck={false} value={draft.text} onChange={(e) => setDraft({ ...draft, text: e.target.value })} />
                </Field>
                <div className="actions">
                  <Button onClick={() => setDraft(null)}>Discard</Button>
                  <Button variant="primary" loading={busy === "save"} onClick={() => void save()}>
                    Save handoff
                  </Button>
                </div>
              </div>
            )}
            {saved && (
              <Callout
                icon="check"
                action={
                  <Button size="sm" variant="primary" icon="term" loading={busy === `open:${saved.name}`} onClick={() => void open(saved.name, target, session?.project ?? null)}>
                    Open in {nameOf(target)}
                  </Button>
                }
              >
                <p>
                  Saved to <span className="mono">{tildify(saved.path, data.info?.home)}</span>. Open in {nameOf(target)} starts Claude Code as {nameOf(target)} in {projectName(session?.project ?? null)} with this first message:
                </p>
                <p className="mono">Read the handoff file at {saved.path} and continue the work it describes.</p>
              </Callout>
            )}
          </section>

          <section className="card card-pad stack mb-6" aria-labelledby="br-files">
            <h2 className="section-title" id="br-files">
              Saved handoffs
            </h2>
            {filesError && (
              <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void loadFiles()}>Try again</Button>}>
                <p>{filesError}</p>
              </Callout>
            )}
            {files === null ? (
              !filesError && <Skeleton />
            ) : files.length === 0 ? (
              <p className="muted">No handoffs yet.</p>
            ) : (
              <ul className="list" aria-label="Saved handoffs">
                {files.map((f) => {
                  const to = targetOf(f.name);
                  return (
                    <li key={f.name} className="list-row list-row--metric">
                      {to && accountOf(to) ? <Avatar {...avatarOf(accountOf(to)!)} label={`For ${nameOf(to)}`} /> : <span />}
                      <div>
                        <div className="title">{f.title}</div>
                        <div className="hint">
                          {relativeTime(f.modifiedMs)} · <span className="mono">{f.name}</span>
                        </div>
                      </div>
                      <div className="actions">
                        {to && data.accounts.some((a) => a.id === to) && (
                          <Button size="sm" icon="term" loading={busy === `open:${f.name}`} onClick={() => void open(f.name, to, null, true)}>
                            Open in {nameOf(to)}
                          </Button>
                        )}
                        {confirmDelete === f.name ? (
                          <>
                            <Button size="sm" autoFocus onClick={() => setConfirmDelete(null)}>
                              Keep
                            </Button>
                            <Button size="sm" variant="danger" onClick={() => void remove(f.name)}>
                              Delete handoff
                            </Button>
                          </>
                        ) : (
                          <Button size="sm" variant="ghost" icon="trash" aria-label={`Delete ${f.title}`} onClick={() => setConfirmDelete(f.name)} />
                        )}
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </section>

          <InboundSettings accounts={data.accounts} />
        </>
      )}
    </>
  );
}

/** Per-account `crossSessionInbound` (D-010): any of your sessions can reach
 *  any other session's message socket, so walls are set here. */
function InboundSettings({ accounts }: { accounts: Account[] }) {
  const api = useApi();
  const toast = useToast();
  const [values, setValues] = useState<Record<string, Inbound | null | "blocked">>({});
  const [review, setReview] = useState<{ account: Account; value: Inbound; ops: ControlOp[]; preview: ControlPreview; error?: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [askError, setAskError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const out: Record<string, Inbound | null | "blocked"> = {};
    for (const a of accounts) {
      try {
        const st = await api.controlRead(a.id);
        const v = st.current.settings?.crossSessionInbound;
        out[a.id] = st.current.blocked ? "blocked" : v === "accept" || v === "hold" || v === "refuse" ? v : null;
      } catch {
        out[a.id] = "blocked";
      }
    }
    setValues(out);
  }, [api, accounts]);

  useEffect(() => {
    void load();
  }, [load]);

  const ask = async (account: Account, value: Inbound) => {
    const ops: ControlOp[] = [{ path: ["crossSessionInbound"], value }];
    try {
      setReview({ account, value, ops, preview: await api.controlPreview(account.id, ops) });
      setAskError(null);
    } catch (e) {
      setAskError(`Could not prepare the change for ${account.name}: ${errorText(e)}`);
    }
  };

  const apply = async () => {
    if (!review) return;
    setBusy(true);
    try {
      const done = await api.controlApply(review.account.id, review.ops, review.preview.baseHash);
      setReview(null);
      toast(`${review.account.name} now ${INBOUND_WORD[review.value].toLowerCase()}${done.backup ? "; backup kept" : ""}. New sessions pick it up.`);
      await load();
    } catch (e) {
      setReview({ ...review, error: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card card-pad stack" aria-labelledby="br-inbound">
      <h2 className="section-title" id="br-inbound">
        Messages between sessions
      </h2>
      <p className="muted">Claude Code sessions can send each other messages. On one computer, a session of one account can reach a session of another account. Choose per account whether incoming messages are accepted, held for you to approve, or refused. Refuse for accounts that must stay walled off.</p>
      <ul className="list" aria-label="Incoming messages per account">
        {accounts.map((a) => {
          const v = values[a.id];
          return (
            <li key={a.id} className="list-row list-row--metric">
              <Avatar {...avatarOf(a)} label={a.name} />
              <div>
                <div className="title">{a.name}</div>
                <div className="hint">{v === undefined ? "Reading…" : v === "blocked" ? "Settings cannot be edited here; see Control." : v ? INBOUND_WORD[v] : "Not set: Claude Code's default applies"}</div>
              </div>
              {v === "blocked" || v === undefined ? (
                <span />
              ) : (
                <Segmented<Inbound | "">
                  label={`Incoming messages for ${a.name}`}
                  value={v ?? ""}
                  onChange={(opt) => opt && opt !== v && void ask(a, opt)}
                  options={[
                    { value: "accept", label: "Accept" },
                    { value: "hold", label: "Hold" },
                    { value: "refuse", label: "Refuse" },
                  ]}
                />
              )}
            </li>
          );
        })}
      </ul>
      {askError && (
        <Callout tone="bad" icon="alert">
          <p>{askError}</p>
        </Callout>
      )}
      {accounts.some((a) => values[a.id] === "accept" || values[a.id] === null) && (
        <p className="hint">
          {plural(accounts.filter((a) => values[a.id] === "accept" || values[a.id] === null).length, "account")} may accept messages from sessions of your other accounts.
        </p>
      )}
      {review && (
        <Modal
          title={`${INBOUND_WORD[review.value]}: ${review.account.name}`}
          onClose={() => setReview(null)}
          identity={<Avatar {...avatarOf(review.account)} size="lg" />}
          footer={
            <>
              <span />
              <div className="actions">
                <Button onClick={() => setReview(null)}>Cancel</Button>
                <Button variant="primary" loading={busy} onClick={() => void apply()}>
                  Set to {review.value === "accept" ? "Accept" : review.value === "hold" ? "Hold" : "Refuse"}
                </Button>
              </div>
            </>
          }
        >
          <p>Facet sets crossSessionInbound in {review.account.name}'s settings and keeps a backup. Open sessions keep the old setting until they restart.</p>
          <DiffView file="settings.json" lines={review.preview.diff} />
          {review.error && (
            <Callout tone="bad" icon="alert">
              <p>{review.error}</p>
            </Callout>
          )}
        </Modal>
      )}
      <p className="hint">Facet does not send messages between accounts itself; it waits for Anthropic to document that.</p>
    </section>
  );
}
