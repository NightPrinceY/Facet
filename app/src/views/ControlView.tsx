import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Account } from "../lib/api";
import { errorText } from "../lib/api";
import {
  HIDDEN_MARKER,
  REFUSED_ENV,
  draftFrom,
  isRefusedEnv,
  newEnvRow,
  opsFromDraft,
  validEnvName,
  type ControlOp,
  type ControlPreview,
  type ControlState,
  type Draft,
  type SettingsBackup,
  type Tri,
} from "../lib/control";
import { tildify } from "../lib/format";
import { useApi, type FacetData } from "../lib/useFacet";
import { Avatar, avatarOf, Button, Callout, Field, Modal, Skeleton, describedBy } from "../components/ui";
import { Icon } from "../components/Icon";
import { useToast } from "../components/Toast";
import { EffectiveSettings } from "./EffectiveSettings";
import { DiffView } from "../components/DiffView";

const MODES: { value: string; label: string; hint: string; warn?: string }[] = [
  { value: "", label: "Not set (asks before acting)", hint: "Claude Code's default: it asks before running tools that change things." },
  { value: "default", label: "Ask before acting", hint: "Asks before running tools that change files or run commands." },
  { value: "acceptEdits", label: "Accept file edits", hint: "Edits files without asking; still asks before running commands." },
  { value: "plan", label: "Plan only", hint: "Reads and plans; makes no changes until you approve a plan." },
  { value: "auto", label: "Auto", hint: "Claude Code decides when to ask, using its own safety checks." },
  { value: "dontAsk", label: "Don't ask", hint: "Never asks: anything not on the allow list is refused.", warn: "Don't ask refuses everything not on the allow list without telling you first." },
  { value: "bypassPermissions", label: "Bypass permissions", hint: "Runs every tool without asking.", warn: "Bypass permissions runs every tool without asking, including deleting files. Use it only in a throwaway sandbox." },
];

const INBOUND = [
  { value: "", label: "Not set (accept)" },
  { value: "accept", label: "Accept" },
  { value: "hold", label: "Hold for me to approve" },
  { value: "refuse", label: "Refuse" },
];

type FlagKey = "disableAllHooks" | "enableAllProjectMcpServers" | "disableSkillShellExecution" | "autoMemoryEnabled";
const FLAGS: { key: FlagKey; label: string; hint: string; options: [string, string, string] }[] = [
  { key: "disableAllHooks", label: "Hooks", hint: "Turning hooks off also stops the status line and any safety hooks.", options: ["Not set (hooks run)", "Turn off all hooks", "Keep hooks on"] },
  {
    key: "enableAllProjectMcpServers",
    label: "MCP servers listed by projects",
    hint: "Starting them all is risky in repositories you just cloned.",
    options: ["Not set (ask per server)", "Start all without asking", "Ask per server"],
  },
  { key: "disableSkillShellExecution", label: "Shell commands inside skills", hint: "Skills can run a command when they load, written as !command.", options: ["Not set (allowed)", "Block them", "Allow them"] },
  { key: "autoMemoryEnabled", label: "Auto memory", hint: "Claude Code keeps notes about your projects between sessions.", options: ["Not set (Claude Code default)", "On", "Off"] },
];

/** Unsaved drafts per account, kept while Facet runs, so switching account
 *  does not lose edits. Reused only if the file has not changed since. */
const drafts = new Map<string, { hash: string; draft: Draft }>();

/** For tests: forget kept drafts. */
export function clearDrafts() {
  drafts.clear();
}

/** Control (PRD M4): an account's Claude Code settings, with a diff and a
 *  backup before every save. Managed policy files are never edited. */
export function ControlView({ data, scope, onScope, onAdd, focusField }: { data: FacetData; scope: string | null; onScope: (id: string) => void; onAdd: () => void; focusField?: string | null }) {
  const account = data.accounts.find((a) => a.id === scope) ?? null;
  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Control</h1>
          <p>Claude Code settings for one account at a time. Every save shows the exact change first and keeps a backup you can restore.</p>
        </div>
      </header>
      {data.accounts.length === 0 ? (
        <div className="card empty">
          <h2>Add an account first</h2>
          <p>Control edits an account's Claude Code settings.</p>
          <Button variant="primary" icon="plus" onClick={onAdd}>
            Add account
          </Button>
        </div>
      ) : !account ? (
        <div className="card card-pad stack">
          <h2 className="card-title">Choose an account</h2>
          <p className="muted">Its name and colour stay with the form while you work.</p>
          <div className="actions">
            {data.accounts.map((a) => (
              <Button key={a.id} onClick={() => onScope(a.id)}>
                <Avatar {...avatarOf(a)} /> {a.name}
              </Button>
            ))}
          </div>
        </div>
      ) : (
        <ControlForm key={account.id} account={account} others={data.accounts.filter((a) => a.id !== account.id)} home={data.info?.home} focusField={focusField} />
      )}
    </>
  );
}

interface Review {
  kind: "save" | "restore";
  preview: ControlPreview;
  ops: ControlOp[];
  backupName?: string;
  error?: string;
  conflict?: boolean;
}

function ControlForm({ account, others, home, focusField }: { account: Account; others: Account[]; home?: string; focusField?: string | null }) {
  const api = useApi();
  const toast = useToast();
  const [state, setState] = useState<ControlState | null>(null);
  const [base, setBase] = useState<Draft | null>(null);
  const [draft, setDraftState] = useState<Draft | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [restoredDraft, setRestoredDraft] = useState(false);
  const [review, setReview] = useState<Review | null>(null);
  const [busy, setBusy] = useState<"preview" | "save" | "restore" | null>(null);
  const [backups, setBackups] = useState<SettingsBackup[]>([]);
  const [refused, setRefused] = useState<string[]>(REFUSED_ENV);
  const lastEnvRow = useRef<HTMLInputElement | null>(null);
  const addEnvBtn = useRef<HTMLButtonElement | null>(null);
  const focusNewRow = useRef(false);

  const setDraft = useCallback(
    (d: Draft) => {
      setDraftState(d);
      if (state) drafts.set(account.id, { hash: state.current.hash, draft: d });
    },
    [account.id, state],
  );

  /** Load from disk. `keepDraft` rebases the current edits onto the new file. */
  const load = useCallback(
    async (keepDraft?: Draft | null) => {
      setLoadError(null);
      try {
        const s = await api.controlRead(account.id);
        setState(s);
        const fresh = draftFrom(s.current.settings);
        setBase(fresh);
        const kept = drafts.get(account.id);
        if (keepDraft) {
          setDraftState(keepDraft);
          drafts.set(account.id, { hash: s.current.hash, draft: keepDraft });
        } else if (kept && kept.hash === s.current.hash) {
          setDraftState(kept.draft);
          setRestoredDraft(opsFromDraft(fresh, kept.draft).length > 0);
        } else {
          drafts.delete(account.id);
          setDraftState(fresh);
        }
        setBackups(await api.controlBackups(account.id));
      } catch (e) {
        setLoadError(errorText(e));
      }
    },
    [api, account.id],
  );

  useEffect(() => {
    void load();
    void api.controlPolicy().then((p) => setRefused(p.refusedEnv), () => undefined);
  }, [load, api]);

  // "Set in Control" from Health focuses the field it is about.
  useEffect(() => {
    if (state && focusField) document.getElementById(focusField)?.focus();
  }, [state, focusField]);

  useEffect(() => {
    if (focusNewRow.current) {
      lastEnvRow.current?.focus();
      focusNewRow.current = false;
    }
  });

  const ops = useMemo(() => (base && draft ? opsFromDraft(base, draft) : []), [base, draft]);

  const envProblems = useMemo(() => {
    const out = new Map<number, string>();
    if (!draft) return out;
    const seen = new Map<string, number>();
    for (const r of draft.env) {
      const k = r.key.trim();
      if (!k) continue;
      if (isRefusedEnv(k, refused)) out.set(r.id, `Facet does not set ${k}: it would replace the sign-in, redirect traffic or weaken TLS. Remove this row.`);
      else if (!validEnvName(k)) out.set(r.id, "Use letters, digits and _ only, not starting with a digit.");
      else if (seen.has(k.toUpperCase())) out.set(r.id, `${k} is listed twice; keep one row.`);
      seen.set(k.toUpperCase(), r.id);
    }
    return out;
  }, [draft, refused]);
  const daysError = draft && draft.cleanupPeriodDays.trim() && !(Number.isInteger(Number(draft.cleanupPeriodDays)) && Number(draft.cleanupPeriodDays) >= 1) ? "Enter a whole number of days, 1 or more." : null;
  const blockedReason = envProblems.size > 0 ? "Fix the highlighted environment rows to continue." : daysError ? "Fix the number of days to continue." : null;

  const startReview = async () => {
    const snapshot = ops;
    setBusy("preview");
    try {
      setReview({ kind: "save", preview: await api.controlPreview(account.id, snapshot), ops: snapshot });
    } catch (e) {
      toast(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const startRestore = async (name: string) => {
    setBusy("restore");
    try {
      setReview({ kind: "restore", preview: await api.controlRestorePreview(account.id, name), ops: [], backupName: name });
    } catch (e) {
      toast(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const confirm = async () => {
    if (!review) return;
    setBusy("save");
    try {
      if (review.kind === "save") {
        const done = await api.controlApply(account.id, review.ops, review.preview.baseHash);
        drafts.delete(account.id);
        setReview(null);
        setRestoredDraft(false);
        toast(done.backup ? `Saved for ${account.name}; backup kept in Backups.` : `Created settings for ${account.name}.`);
        await load();
      } else {
        await api.controlRestore(account.id, review.backupName!, review.preview.baseHash);
        setReview(null);
        toast(`Restored ${account.name}'s settings; the replaced version is in Backups.`);
        await load(ops.length > 0 ? draft : null);
      }
    } catch (e) {
      const msg = errorText(e);
      setReview({ ...review, error: msg, conflict: /changed since|changed while/.test(msg) });
    } finally {
      setBusy(null);
    }
  };

  const reloadKeepingEdits = async () => {
    const mine = draft;
    setReview(null);
    await load(mine);
  };

  if (!state || !draft) {
    return loadError ? (
      <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Try again</Button>}>
        <p>{loadError}</p>
      </Callout>
    ) : (
      <div className="card card-pad stack" aria-busy="true">
        <span className="visually-hidden">Loading {account.name}'s settings</span>
        <Skeleton />
        <Skeleton />
      </div>
    );
  }

  const set = <K extends keyof Draft>(k: K, v: Draft[K]) => setDraft({ ...draft, [k]: v });
  const mode = MODES.find((m) => m.value === draft.defaultMode) ?? MODES[0]!;
  const blocked = state.current.blocked;

  return (
    <div className="stack">
      <div className="card card-pad control-head">
        <Avatar {...avatarOf(account)} size="lg" />
        <div>
          <h2 className="card-title">{account.name}</h2>
          <div className="hint">
            <span className="mono">{tildify(state.current.path, home)}</span>
            {!state.current.exists && <span> · does not exist yet; Facet creates it on save</span>}
          </div>
        </div>
        <nav className="section-links" aria-label="Sections of this form">
          <a href="#ctl-general">General</a>
          <a href="#ctl-perm">Permissions</a>
          <a href="#ctl-safety">Safety</a>
          <a href="#ctl-env">Environment</a>
          <a href="#ctl-backups">Backups</a>
          <a href="#ctl-effective">What applies</a>
        </nav>
      </div>

      {state.managedPath && (
        <Callout icon="shield">
          <p>
            A managed policy file exists at <span className="mono">{tildify(state.managedPath, home)}</span>. Its settings win over these, and Facet never edits it.
          </p>
        </Callout>
      )}
      {blocked && (
        <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Check again</Button>}>
          <p>{blocked}</p>
        </Callout>
      )}
      {!blocked && others.length > 0 && <CopyFrom others={others} onCopy={(from, copied) => {
        // Settings from another account become unsaved edits here; hidden
        // (secret) values are never copied. The usual review and backup apply.
        const env = [...draft.env.filter((r) => !from.env.some((o) => o.key === r.key))];
        for (const r of from.env) if (!r.hidden) env.push({ ...r, id: r.id + 100000 });
        setDraft({ ...from, env });
        toast(`Copied ${copied}'s settings into the form; review and save to apply them to ${account.name}.`);
      }} />}

      {restoredDraft && (
        <Callout icon="info" action={<Button size="sm" onClick={() => { drafts.delete(account.id); setDraftState(base); setRestoredDraft(false); }}>Discard them</Button>}>
          <p>Your unsaved changes for {account.name} from earlier are back.</p>
        </Callout>
      )}

      {!blocked && (
        <fieldset className="plain-fieldset stack" disabled={busy === "preview" || busy === "save"}>
          <section className="card card-pad stack" aria-labelledby="ctl-general-h" id="ctl-general">
            <h2 className="section-title" id="ctl-general-h">
              General
            </h2>
            <Field label="Model" htmlFor="ctl-model" hint="For example opus, sonnet or haiku, or a full model id. Empty means Claude Code's default.">
              <input id="ctl-model" className="input ltr" list="ctl-models" value={draft.model} onChange={(e) => set("model", e.target.value)} aria-describedby={describedBy("ctl-model", null, "hint")} />
              <datalist id="ctl-models">
                {["opus", "sonnet", "haiku"].map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </Field>
            <Field label="Output style" htmlFor="ctl-style" hint="Built in: Default, Proactive, Concise, Explanatory, Learning. Names are case-sensitive.">
              <input id="ctl-style" className="input ltr" list="ctl-styles" value={draft.outputStyle} onChange={(e) => set("outputStyle", e.target.value)} aria-describedby={describedBy("ctl-style", null, "hint")} />
              <datalist id="ctl-styles">
                {["Default", "Proactive", "Concise", "Explanatory", "Learning"].map((m) => (
                  <option key={m} value={m} />
                ))}
              </datalist>
            </Field>
            <Field label="Keep transcripts for (days)" htmlFor="ctl-days" error={daysError} hint="Claude Code deletes older sessions; empty means its default of 30 days. Facet's history needs them, so 365 is a good choice.">
              <input id="ctl-days" className="input num" inputMode="numeric" value={draft.cleanupPeriodDays} placeholder="30" onChange={(e) => set("cleanupPeriodDays", e.target.value)} aria-invalid={Boolean(daysError)} aria-describedby={describedBy("ctl-days", daysError, "hint")} />
            </Field>
          </section>

          <section className="card card-pad stack" aria-labelledby="ctl-perm-h" id="ctl-perm">
            <h2 className="section-title" id="ctl-perm-h">
              Permissions
            </h2>
            <Field label="When Claude Code wants to act" htmlFor="ctl-mode" hint={mode.hint}>
              <select id="ctl-mode" className="input" value={draft.defaultMode} onChange={(e) => set("defaultMode", e.target.value)} aria-describedby={describedBy("ctl-mode", null, "hint")}>
                {MODES.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </select>
            </Field>
            {mode.warn && (
              <Callout tone="warn" icon="alert">
                <p>{mode.warn}</p>
              </Callout>
            )}
            {(["allow", "ask", "deny"] as const).map((k) => (
              <Field key={k} label={k === "allow" ? "Always allow" : k === "ask" ? "Always ask" : "Never allow"} htmlFor={`ctl-${k}`} hint="One rule per line, for example Bash(git status) or Read(./docs/**).">
                <textarea id={`ctl-${k}`} className="input mono textarea" rows={3} value={draft[k]} onChange={(e) => set(k, e.target.value)} aria-describedby={describedBy(`ctl-${k}`, null, "hint")} />
              </Field>
            ))}
            <Field label="Extra folders Claude Code may use" htmlFor="ctl-dirs" hint="One folder per line. Avoid other accounts' folders: that weakens the separation.">
              <textarea id="ctl-dirs" className="input mono textarea" rows={2} value={draft.additionalDirectories} onChange={(e) => set("additionalDirectories", e.target.value)} aria-describedby={describedBy("ctl-dirs", null, "hint")} />
            </Field>
          </section>

          <section className="card card-pad stack" aria-labelledby="ctl-safety-h" id="ctl-safety">
            <h2 className="section-title" id="ctl-safety-h">
              Safety and messages
            </h2>
            <Field label="Messages from other Claude sessions" htmlFor="ctl-inbound" hint="Any session on this computer can reach any other. Hold or refuse keeps this account's sessions to themselves.">
              <select id="ctl-inbound" className="input" value={draft.crossSessionInbound} onChange={(e) => set("crossSessionInbound", e.target.value)} aria-describedby={describedBy("ctl-inbound", null, "hint")}>
                {INBOUND.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </select>
            </Field>
            {FLAGS.map((f) => {
              // The first option writes true and the second false; the labels
              // say what that means for each setting.
              const [unset, first, second] = f.options;
              return (
                <Field key={f.key} label={f.label} htmlFor={`ctl-${f.key}`} hint={f.hint}>
                  <select id={`ctl-${f.key}`} className="input" value={draft[f.key]} onChange={(e) => set(f.key, e.target.value as Tri)} aria-describedby={describedBy(`ctl-${f.key}`, null, "hint")}>
                    <option value="">{unset}</option>
                    <option value="on">{first}</option>
                    <option value="off">{second}</option>
                  </select>
                </Field>
              );
            })}
          </section>

          <section className="card card-pad stack" aria-labelledby="ctl-env-h" id="ctl-env">
            <h2 className="section-title" id="ctl-env-h">
              Environment
            </h2>
            <p className="muted">Variables Claude Code sets for this account. Facet refuses names that would replace the sign-in, redirect traffic or weaken TLS, and hides values that look secret.</p>
            {draft.env.length > 0 && (
              <div className="env-head" aria-hidden="true">
                <span>Name</span>
                <span>Value</span>
                <span />
              </div>
            )}
            {draft.env.map((row, i) => {
              const problem = envProblems.get(row.id);
              const label = row.key.trim() || `variable ${i + 1}`;
              return (
                <div key={row.id} className="env-row">
                  <div className="rule-row">
                    <input
                      ref={i === draft.env.length - 1 ? lastEnvRow : undefined}
                      className="input mono"
                      aria-label={`Name of ${label}`}
                      value={row.key}
                      readOnly={row.hidden}
                      onChange={(e) => set("env", draft.env.map((r) => (r.id === row.id ? { ...r, key: e.target.value } : r)))}
                      aria-invalid={Boolean(problem)}
                      aria-describedby={problem ? `env-err-${row.id}` : undefined}
                    />
                    <input
                      className="input mono"
                      aria-label={`Value of ${label}`}
                      value={row.hidden ? HIDDEN_MARKER : row.value}
                      readOnly={row.hidden}
                      onChange={(e) => set("env", draft.env.map((r) => (r.id === row.id ? { ...r, value: e.target.value } : r)))}
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      icon="trash"
                      aria-label={`Remove ${label}`}
                      onClick={() => {
                        set("env", draft.env.filter((r) => r.id !== row.id));
                        addEnvBtn.current?.focus();
                      }}
                    >
                      Remove
                    </Button>
                  </div>
                  {problem && (
                    <div className="field-error" id={`env-err-${row.id}`}>
                      <Icon name="alert" size={14} /> {problem}
                    </div>
                  )}
                  {row.hidden && <div className="hint">This value looks secret, so Facet does not show or send it. Remove the row to delete it.</div>}
                </div>
              );
            })}
            <div>
              <Button
                size="sm"
                icon="plus"
                ref={addEnvBtn}
                onClick={() => {
                  focusNewRow.current = true;
                  set("env", [...draft.env, newEnvRow()]);
                }}
              >
                Add variable
              </Button>
            </div>
          </section>
        </fieldset>
      )}

      {!blocked && (
        <div className="action-bar">
          <Avatar {...avatarOf(account)} />
          <span className="muted">{ops.length === 0 ? `No changes for ${account.name}` : `${ops.length === 1 ? "1 change" : `${ops.length} changes`} not saved for ${account.name}`}</span>
          {blockedReason && <span className="field-error">{blockedReason}</span>}
          <div className="actions">
            <Button variant="ghost" disabled={ops.length === 0} onClick={() => { drafts.delete(account.id); setDraftState(base); }}>
              Discard changes
            </Button>
            <Button variant="primary" loading={busy === "preview"} disabled={ops.length === 0 || Boolean(blockedReason)} onClick={() => void startReview()}>
              Review and save
            </Button>
          </div>
        </div>
      )}

      <section className="stack" aria-labelledby="ctl-backups-h" id="ctl-backups">
        <h2 className="section-title" id="ctl-backups-h">
          Backups
        </h2>
        <div className="card">
          {backups.length === 0 ? (
            <p className="card-pad muted">No backups yet. Facet keeps one before each save, and the last 30.</p>
          ) : (
            <ul className="list">
              {backups.map((b) => (
                <li key={b.name} className="list-row">
                  <span className="status-icon" aria-hidden="true">
                    <Icon name="window" size={14} />
                  </span>
                  <div>
                    <div className="title">{backupLabel(b.name)}</div>
                    <div className="hint">
                      <span className="mono">{b.name}</span> · {formatBytes(b.bytes)}
                    </div>
                  </div>
                  <Button size="sm" loading={busy === "restore"} disabled={Boolean(blocked)} onClick={() => void startRestore(b.name)}>
                    Restore this backup
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </section>

      <EffectiveSettings account={account} home={home} />

      {review && (
        <Modal
          title={review.kind === "save" ? `Review changes for ${account.name}` : `Restore backup for ${account.name}`}
          identity={<Avatar {...avatarOf(account)} size="lg" />}
          dismissable={busy !== "save"}
          onClose={() => setReview(null)}
          footer={
            <>
              <span className="muted mono">{tildify(review.preview.path, home)}</span>
              <div className="actions">
                <Button disabled={busy === "save"} onClick={() => setReview(null)}>
                  Back to editing
                </Button>
                {review.conflict ? (
                  <Button variant="primary" onClick={() => void reloadKeepingEdits()}>
                    Reload and keep my edits
                  </Button>
                ) : (
                  <Button variant="primary" loading={busy === "save"} disabled={!review.preview.changed} onClick={() => void confirm()}>
                    {review.kind === "save" ? "Save with backup" : "Restore this backup"}
                  </Button>
                )}
              </div>
            </>
          }
        >
          {review.error && (
            <Callout tone="bad" icon="alert" action={!review.conflict && <Button size="sm" onClick={() => void confirm()}>Try again</Button>}>
              <p>{review.conflict ? `${account.name}'s settings.json changed since you opened it. Reload to see it; your edits stay.` : review.error}</p>
            </Callout>
          )}
          {!review.preview.changed && (
            <Callout icon="info">
              <p>The file would stay the same. Go back to edit.</p>
            </Callout>
          )}
          {review.preview.formatOnly && (
            <Callout icon="info">
              <p>Only the layout changes: Facet writes the file in its standard format.</p>
            </Callout>
          )}
          {review.preview.warnings.map((w) => (
            <Callout key={w} tone="warn" icon="alert">
              <p>{w}</p>
            </Callout>
          ))}
          <DiffView file="settings.json" lines={review.preview.diff} />
          <p className="muted">
            {review.kind === "save"
              ? "Facet copies the current file to Backups, then replaces it in one step."
              : "Facet backs up the current file first, so this restore can be undone too."}
          </p>
        </Modal>
      )}
    </div>
  );
}

/** `20261026T031500Z-123-settings.json` → "26 Oct 2026, 03:15 UTC". */
export function backupLabel(name: string): string {
  const m = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})\d{2}Z/.exec(name);
  if (!m) return name;
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const label = `${Number(m[3])} ${months[Number(m[2]) - 1]} ${m[1]}, ${m[4]}:${m[5]} UTC`;
  return name.includes("before-restore") ? `${label} · before a restore` : label;
}

export function formatBytes(n: number): string {
  return n < 1024 ? `${n} B` : `${(n / 1024).toFixed(1)} KB`;
}

/** Copy another account's settings into this form (PRD M4). */
function CopyFrom({ others, onCopy }: { others: Account[]; onCopy: (draft: Draft, fromName: string) => void }) {
  const api = useApi();
  const [from, setFrom] = useState(others[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const source = others.find((a) => a.id === from);
  return (
    <details className="card card-pad">
      <summary className="title">Copy settings from another account</summary>
      <div className="stack mt-5">
        <p className="muted">Fills this form with the other account's settings. Nothing is saved until you review and save; values that look secret are not copied.</p>
        <div className="actions">
          {source && <Avatar {...avatarOf(source)} />}
          <select className="input input--auto" aria-label="Copy from" value={from} onChange={(e) => setFrom(e.target.value)}>
            {others.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <Button
            loading={busy}
            onClick={async () => {
              setBusy(true);
              setError(null);
              try {
                const s = await api.controlRead(from);
                if (s.current.blocked) throw s.current.blocked;
                onCopy(draftFrom(s.current.settings), source?.name ?? from);
              } catch (e) {
                setError(errorText(e));
              } finally {
                setBusy(false);
              }
            }}
          >
            Copy into this form
          </Button>
        </div>
        {error && (
          <Callout tone="bad" icon="alert">
            <p>{error}</p>
          </Callout>
        )}
      </div>
    </details>
  );
}
