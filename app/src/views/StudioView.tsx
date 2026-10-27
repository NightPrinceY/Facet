import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import type { Account } from "../lib/api";
import { errorText } from "../lib/api";
import { tildify } from "../lib/format";
import { DESC_BUDGET, KIND_LABEL, SEVERITY_LABEL, relPath, validName, worst, type AccountItem, type Finding, type ItemKind, type Lint, type PlacePreview, type PlacementState, type StudioItem } from "../lib/studio";
import { useApi, type FacetData } from "../lib/useFacet";
import { Avatar, avatarOf, Button, Callout, Field, Meter, Modal, Pill, Segmented, Skeleton, Tag, describedBy } from "../components/ui";
import { Icon } from "../components/Icon";
import { DiffView } from "../components/DiffView";
import { useToast } from "../components/Toast";

const STATE_WORD: Record<PlacementState, string> = {
  "in-sync": "in sync",
  drifted: "different from the library",
  absent: "not placed",
  untouchable: "left alone (a link or holds a secret)",
};

/** Unsaved editor text per item, kept while Facet runs. */
const drafts = new Map<string, { saved: string; text: string }>();

/** For tests: forget kept drafts. */
export function clearStudioDrafts() {
  drafts.clear();
}

/** Studio v1 (PRD M11): a library of skills, subagents and commands, a
 *  linter, and placement into accounts. Nothing here runs a skill. */
export function StudioView({ data, scope }: { data: FacetData; scope: string | null }) {
  const api = useApi();
  const toast = useToast();
  const [items, setItems] = useState<StudioItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [open, setOpen] = useState<{ kind: ItemKind; name: string } | null>(null);
  const [ask, setAsk] = useState<{ kind: ItemKind; name: string } | "close" | null>(null);
  const [editorDirty, setEditorDirty] = useState(false);

  const load = useCallback(async () => {
    try {
      setItems(await api.studioList());
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  }, [api]);

  useEffect(() => {
    void load();
  }, [load, data.accounts.length]);

  const byId = Object.fromEntries(data.accounts.map((a) => [a.id, a]));
  const current = items?.find((i) => open && i.kind === open.kind && i.name === open.name) ?? null;
  const focusRow = (kind: ItemKind, name: string) => setTimeout(() => document.getElementById(`open-${kind}-${name}`)?.focus(), 0);

  /** Open another item or close; asks first when the editor has unsaved text. */
  const switchTo = (next: { kind: ItemKind; name: string } | null) => {
    if (editorDirty) {
      setAsk(next ?? "close");
      return;
    }
    const prev = open;
    setOpen(next);
    if (!next && prev) focusRow(prev.kind, prev.name);
  };

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Studio</h1>
          <p>Write skills, subagents and commands once, check them, and place them in the accounts you choose. Facet keeps them in its own library and never runs them.</p>
        </div>
        {items && items.length > 0 && (
          <div className="actions">
            <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
              New item
            </Button>
          </div>
        )}
      </header>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Try again</Button>}>
            <p>{error}</p>
          </Callout>
        </div>
      )}

      {items === null && !error && (
        <div className="card card-pad stack" aria-busy="true">
          <span className="visually-hidden">Loading the library</span>
          <Skeleton />
          <Skeleton />
        </div>
      )}

      {items && items.length === 0 && (
        <div className="card empty">
          <h2>Your library is empty</h2>
          <p>Create a skill, subagent or command, or copy one you already use from an account below.</p>
          <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
            New item
          </Button>
        </div>
      )}

      {items && items.length > 0 && (
        <>
          <h2 className="section-title" id="studio-lib-h">
            Library
          </h2>
          <div className="card mb-6">
            <ul className="list" aria-labelledby="studio-lib-h">
              {items.map((it) => {
                const w = worst(it.lint);
                const problems = it.lint.filter((f) => f.severity === "error").length;
                const groups = (["in-sync", "drifted", "untouchable"] as PlacementState[])
                  .map((st) => [st, it.placements.filter((p) => p.state === st).map((p) => byId[p.accountId]?.name).filter(Boolean)] as const)
                  .filter(([, names]) => names.length > 0);
                return (
                  <li key={`${it.kind}:${it.name}`} className="list-row">
                    <Tag>{KIND_LABEL[it.kind]}</Tag>
                    <div>
                      <div className="actions">
                        <span className="title mono">{it.name}</span>
                        {w === "error" && <Pill tone="bad">{problems} must fix</Pill>}
                        {w === "risk" && <Pill tone="warn">Runs commands</Pill>}
                        {w === "warn" && <Pill tone="warn">{it.lint.filter((f) => f.severity === "warn").length} worth fixing</Pill>}
                      </div>
                      <div className="hint" dir="auto">
                        {it.description || "No description"}
                      </div>
                      <div className="placements" role="group" aria-label={`Where ${it.name} is placed`}>
                        {it.placements.map((p) => {
                          const a = byId[p.accountId];
                          if (!a) return null;
                          return (
                            <span key={p.accountId} className={`placement placement--${p.state}`}>
                              <Avatar {...avatarOf(a)} label={`${a.name}: ${STATE_WORD[p.state]}`} />
                              {p.state !== "absent" && (
                                <span className={`placement-badge placement-badge--${p.state}`} aria-hidden="true">
                                  <Icon name={p.state === "in-sync" ? "check" : "alert"} size={10} />
                                </span>
                              )}
                            </span>
                          );
                        })}
                      </div>
                      <div className="placement-summary">{groups.length === 0 ? "Not placed in any account" : groups.map(([st, names]) => `${st === "in-sync" ? "In sync" : st === "drifted" ? "Different" : "Left alone"}: ${names.join(", ")}`).join(" · ")}</div>
                    </div>
                    <Button size="sm" icon="edit" id={`open-${it.kind}-${it.name}`} onClick={() => switchTo({ kind: it.kind, name: it.name })} aria-label={`Open ${it.name}`}>
                      Open
                    </Button>
                  </li>
                );
              })}
            </ul>
          </div>
        </>
      )}

      {ask && (
        <div className="mb-6">
          <Callout
            tone="warn"
            icon="alert"
            action={
              <div className="actions">
                <Button size="sm" autoFocus onClick={() => setAsk(null)}>
                  Keep editing
                </Button>
                <Button
                  size="sm"
                  variant="danger"
                  onClick={() => {
                    if (open) drafts.delete(`${open.kind}:${open.name}`);
                    setEditorDirty(false);
                    setOpen(ask === "close" ? null : ask);
                    setAsk(null);
                  }}
                >
                  Discard changes
                </Button>
              </div>
            }
          >
            <p>Your changes to {open?.name} are not saved. Discard them?</p>
          </Callout>
        </div>
      )}

      {current && (
        <ItemEditor
          key={`${current.kind}:${current.name}`}
          item={current}
          accounts={data.accounts}
          scope={scope}
          home={data.info?.home}
          onDirty={setEditorDirty}
          onChanged={load}
          onClose={() => switchTo(null)}
          onDeleted={() => {
            setEditorDirty(false);
            setOpen(null);
            void load();
            setTimeout(() => document.querySelector<HTMLElement>("#main h1")?.focus(), 0);
          }}
        />
      )}

      <ImportSection
        accounts={data.accounts}
        scope={scope}
        library={items ?? []}
        home={data.info?.home}
        onImported={async (kind, name) => {
          await load();
          setOpen({ kind, name });
          toast(`Copied ${name} into the library.`);
        }}
      />

      {creating && (
        <NewItemModal
          existing={items ?? []}
          onClose={() => setCreating(false)}
          onCreated={async (kind, name) => {
            setCreating(false);
            await load();
            setOpen({ kind, name });
          }}
        />
      )}
    </>
  );
}

function Findings({ lint }: { lint: Finding[] }) {
  if (lint.length === 0)
    return (
      <p className="muted">
        <Icon name="check" size={14} /> No problems found.
      </p>
    );
  return (
    <ul className="findings" aria-label="Findings">
      {lint.map((f, i) => (
        <li key={i}>
          <Pill tone={f.severity === "error" ? "bad" : f.severity === "info" ? "neutral" : "warn"}>{SEVERITY_LABEL[f.severity]}</Pill>
          <span dir="auto">{f.message}</span>
        </li>
      ))}
    </ul>
  );
}

/** Lint as you type (debounced), through the core's linter. */
function useLint(kind: ItemKind, name: string, text: string): { lint: Lint | null; checking: boolean; error: string | null } {
  const api = useApi();
  const [lint, setLint] = useState<Lint | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    setChecking(true);
    const t = setTimeout(() => {
      void api.studioLint(kind, name, text).then(
        (l) => {
          if (!live) return;
          setLint(l);
          setError(null);
          setChecking(false);
        },
        (e) => {
          if (!live) return;
          setError(errorText(e));
          setChecking(false);
        },
      );
    }, 250);
    return () => {
      live = false;
      clearTimeout(t);
    };
  }, [api, kind, name, text]);
  return { lint, checking, error };
}

function summary(l: Lint | null): string {
  if (!l) return "";
  const must = l.findings.filter((f) => f.severity === "error").length;
  const others = l.findings.filter((f) => f.severity === "risk" || f.severity === "warn").length;
  if (must + others === 0) return "No problems found.";
  return `${must + others} ${must + others === 1 ? "problem" : "problems"}${must ? `, ${must} must fix` : ""}.`;
}

function ItemEditor({ item, accounts, scope, home, onDirty, onChanged, onClose, onDeleted }: { item: StudioItem; accounts: Account[]; scope: string | null; home?: string; onDirty: (d: boolean) => void; onChanged: () => Promise<void>; onClose: () => void; onDeleted: () => void }) {
  const api = useApi();
  const toast = useToast();
  const key = `${item.kind}:${item.name}`;
  const [saved, setSaved] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [place, setPlace] = useState<{ account: Account; preview: PlacePreview; error?: string; ack: boolean } | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const deleteBtn = useRef<HTMLButtonElement>(null);
  const reasonId = useId();
  const { lint, checking, error: lintError } = useLint(item.kind, item.name, text);
  const dirty = saved !== null && text !== saved;
  const hasErrors = Boolean(lint?.findings.some((f) => f.severity === "error"));

  useEffect(() => onDirty(dirty), [dirty, onDirty]);

  const read = useCallback(() => {
    setReadError(null);
    void api.studioRead(item.kind, item.name).then(
      (t) => {
        const kept = drafts.get(key);
        setSaved(t);
        setText(kept && kept.saved === t ? kept.text : t);
        heading.current?.focus();
      },
      (e) => setReadError(errorText(e)),
    );
  }, [api, item.kind, item.name, key]);

  useEffect(read, [read]);

  const edit = (t: string) => {
    setText(t);
    if (saved !== null) drafts.set(key, { saved, text: t });
  };

  const save = async () => {
    setBusy("save");
    setError(null);
    try {
      await api.studioSave(item.kind, item.name, text);
      setSaved(text);
      drafts.delete(key);
      toast(`Saved ${item.name} in the library.`);
      await onChanged();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const review = async (account: Account) => {
    setBusy(`place:${account.id}`);
    setError(null);
    try {
      setPlace({ account, preview: await api.studioPlacePreview(item.kind, item.name, account.id), ack: false });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const confirmPlace = async () => {
    if (!place) return;
    setBusy("placing");
    try {
      const backup = await api.studioPlace(item.kind, item.name, place.account.id, place.preview.baseHash, place.ack);
      toast(backup ? `Updated ${item.name} in ${place.account.name}; the old copy is backed up.` : `Placed ${item.name} in ${place.account.name}.`);
      setPlace(null);
      await onChanged();
    } catch (e) {
      setPlace({ ...place, error: errorText(e) });
    } finally {
      setBusy(null);
    }
  };

  const remove = async (account: Account) => {
    setBusy(`remove:${account.id}`);
    try {
      await api.studioUnplace(item.kind, item.name, account.id);
      setConfirmRemove(null);
      toast(`Removed ${item.name} from ${account.name}; a backup is kept.`);
      await onChanged();
      setTimeout(() => document.getElementById(`place-${account.id}`)?.focus(), 0);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const deleteFromLibrary = async () => {
    setBusy("delete");
    try {
      await api.studioRemove(item.kind, item.name);
      drafts.delete(key);
      toast(`Deleted ${item.name} from the library; copies in accounts stay.`);
      onDeleted();
    } catch (e) {
      setError(errorText(e));
      setBusy(null);
    }
  };

  if (saved === null)
    return readError ? (
      <div className="mb-6">
        <Callout
          tone="bad"
          icon="alert"
          action={
            <div className="actions">
              <Button size="sm" onClick={read}>
                Try again
              </Button>
              <Button size="sm" onClick={onClose}>
                Close
              </Button>
            </div>
          }
        >
          <p>{readError}</p>
        </Callout>
      </div>
    ) : null;

  const placeDisabledReason = dirty ? "Save first; accounts get the saved version." : hasErrors ? "Fix the findings marked “Must fix” before placing it." : !lint ? "Checking…" : null;
  const sortedAccounts = [...accounts].sort((a, b) => (a.id === scope ? -1 : b.id === scope ? 1 : 0));

  return (
    <section className="stack mb-6" aria-labelledby="studio-edit-h">
      <div className="card card-pad control-head">
        <Tag>{KIND_LABEL[item.kind]}</Tag>
        <h2 className="card-title mono" id="studio-edit-h" tabIndex={-1} ref={heading}>
          {item.name}
        </h2>
        <span className="hint mono">{relPath(item.kind, item.name)}</span>
        <div className="actions push-end">
          {dirty && <span className="muted">Not saved</span>}
          <Button variant="ghost" disabled={!dirty} onClick={() => edit(saved)}>
            Discard changes
          </Button>
          <Button variant="primary" loading={busy === "save"} disabled={!dirty} onClick={() => void save()}>
            Save to library
          </Button>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>

      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}

      <div className="studio-grid">
        <div className="card card-pad stack">
          <Field label="File" htmlFor="studio-text" hint="Settings between the --- lines at the top, then the instructions. Saved in Facet's library only, until you place it.">
            <textarea id="studio-text" className="input textarea editor" rows={18} spellCheck={false} value={text} onChange={(e) => edit(e.target.value)} aria-describedby={describedBy("studio-text", null, "hint")} />
          </Field>
        </div>

        <div className="stack">
          <div className="card card-pad stack">
            <h3 className="section-title">Checks</h3>
            <p className="visually-hidden" role="status">
              {summary(lint)}
            </p>
            {item.kind === "skill" && lint && <Meter label="Description length (with when_to_use)" value={lint.descriptionChars} max={DESC_BUDGET} unit="characters" />}
            {lintError ? <p className="field-error">Checks could not run: {lintError}</p> : lint ? <Findings lint={lint.findings} /> : <Skeleton />}
            {checking && lint && <p className="muted">Checking…</p>}
          </div>

          <div className="card card-pad stack">
            <h3 className="section-title" id="studio-acc-h">
              Accounts
            </h3>
            {placeDisabledReason && (
              <p className={hasErrors ? "field-error" : "muted"} id={reasonId}>
                {placeDisabledReason}
              </p>
            )}
            <ul className="list" aria-labelledby="studio-acc-h">
              {sortedAccounts.map((a) => {
                const state = item.placements.find((p) => p.accountId === a.id)?.state ?? "absent";
                return (
                  <li key={a.id} className="list-row">
                    <Avatar {...avatarOf(a)} />
                    <div>
                      <div className="title">{a.name}</div>
                      <div className="hint">
                        {state === "in-sync" ? "Placed, same as the library" : state === "drifted" ? "Placed, but different from the library" : state === "untouchable" ? "Left alone: the account's copy is a link or holds a secret" : "Not placed"}
                      </div>
                      {confirmRemove === a.id && <p className="hint">Remove {item.name} from {a.name}? Facet keeps a backup.</p>}
                    </div>
                    {confirmRemove === a.id ? (
                      <div className="actions">
                        <Button
                          size="sm"
                          autoFocus
                          onClick={() => {
                            setConfirmRemove(null);
                            setTimeout(() => document.getElementById(`remove-${a.id}`)?.focus(), 0);
                          }}
                        >
                          Keep
                        </Button>
                        <Button size="sm" variant="danger" loading={busy === `remove:${a.id}`} onClick={() => void remove(a)}>
                          Remove from {a.name}
                        </Button>
                      </div>
                    ) : (
                      <div className="actions">
                        {(state === "absent" || state === "drifted") && (
                          <Button
                            size="sm"
                            id={`place-${a.id}`}
                            loading={busy === `place:${a.id}`}
                            disabled={Boolean(placeDisabledReason)}
                            aria-describedby={placeDisabledReason ? reasonId : undefined}
                            aria-label={`${state === "absent" ? "Review and place" : "Review update of"} ${item.name} in ${a.name}`}
                            onClick={() => void review(a)}
                          >
                            {state === "absent" ? "Review and place" : "Review update"}
                          </Button>
                        )}
                        {(state === "in-sync" || state === "drifted") && (
                          <Button size="sm" variant="ghost" id={`remove-${a.id}`} onClick={() => setConfirmRemove(a.id)} aria-label={`Remove ${item.name} from ${a.name}`}>
                            Remove
                          </Button>
                        )}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>

          <div className="card card-pad stack">
            {confirmDelete ? (
              <Callout
                tone="warn"
                icon="alert"
                action={
                  <div className="actions">
                    <Button
                      size="sm"
                      autoFocus
                      onClick={() => {
                        setConfirmDelete(false);
                        setTimeout(() => deleteBtn.current?.focus(), 0);
                      }}
                    >
                      Keep
                    </Button>
                    <Button size="sm" variant="danger" loading={busy === "delete"} onClick={() => void deleteFromLibrary()}>
                      Delete from library
                    </Button>
                  </div>
                }
              >
                <p>Delete {item.name} from the library? Copies already placed in accounts stay where they are.</p>
              </Callout>
            ) : (
              <div>
                <Button size="sm" variant="ghost" icon="trash" ref={deleteBtn} onClick={() => setConfirmDelete(true)}>
                  Delete from library
                </Button>
              </div>
            )}
          </div>
        </div>
      </div>

      {place && (
        <Modal
          title={`${place.preview.state === "absent" ? "Place" : "Update"} ${item.name} in ${place.account.name}`}
          identity={<Avatar {...avatarOf(place.account)} size="lg" />}
          dismissable={busy !== "placing"}
          onClose={() => setPlace(null)}
          footer={
            <>
              <span className="muted mono">{tildify(place.preview.target, home)}</span>
              <div className="actions">
                <Button disabled={busy === "placing"} onClick={() => setPlace(null)}>
                  Back
                </Button>
                <Button
                  variant="primary"
                  loading={busy === "placing"}
                  disabled={place.preview.blockers.length > 0 || (place.preview.risks.length > 0 && !place.ack)}
                  onClick={() => void confirmPlace()}
                >
                  {place.preview.state === "absent" ? `Place in ${place.account.name}` : `Update in ${place.account.name}`}
                </Button>
              </div>
            </>
          }
        >
          {place.error && (
            <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void review(place.account)}>Review again</Button>}>
              <p>{place.error}</p>
            </Callout>
          )}
          {place.preview.blockers.length > 0 && (
            <Callout tone="bad" icon="alert">
              <p>This cannot be placed yet:</p>
              <Findings lint={place.preview.blockers} />
            </Callout>
          )}
          {place.preview.state === "drifted" && (
            <Callout tone="warn" icon="alert">
              <p>{place.account.name}'s copy is different. Facet backs it up before replacing it.</p>
            </Callout>
          )}
          {place.preview.risks.length > 0 && (
            <>
              <Findings lint={place.preview.risks} />
              <label className="ack">
                <input type="checkbox" checked={place.ack} onChange={(e) => setPlace({ ...place, ack: e.target.checked })} />
                <span>I understand this runs commands on this computer when {place.account.name}'s Claude Code uses it.</span>
              </label>
            </>
          )}
          <DiffView file={relPath(item.kind, item.name)} lines={place.preview.diff} />
          <p className="muted">
            {place.preview.liveSessions > 0
              ? `${place.preview.liveSessions} Claude Code ${place.preview.liveSessions === 1 ? "session" : "sessions"} of ${place.account.name} ${place.preview.liveSessions === 1 ? "is" : "are"} running and may pick this up right away. Other accounts are not touched.`
              : `Claude Code in ${place.account.name} can use it from its next session, or right away in running ones. Other accounts are not touched.`}
          </p>
        </Modal>
      )}
    </section>
  );
}

function NewItemModal({ existing, onClose, onCreated }: { existing: StudioItem[]; onClose: () => void; onCreated: (kind: ItemKind, name: string) => Promise<void> }) {
  const api = useApi();
  const [kind, setKind] = useState<ItemKind>("skill");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [body, setBody] = useState("");
  const [preview, setPreview] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [askDiscard, setAskDiscard] = useState(false);
  const clean = name.trim();
  const nameError = !clean ? null : !validName(clean) ? "Use lowercase letters, digits and dashes, for example review-pr." : existing.some((i) => i.kind === kind && i.name === clean) ? `A ${KIND_LABEL[kind].toLowerCase()} called ${clean} is already in the library.` : null;
  const { lint } = useLint(kind, clean || "new-item", preview);

  useEffect(() => {
    if (!clean || nameError) {
      setPreview("");
      return;
    }
    let live = true;
    void api.studioTemplate(kind, clean, description, body).then((t) => live && setPreview(t), () => undefined);
    return () => {
      live = false;
    };
  }, [api, kind, clean, description, body, nameError]);

  const dirty = Boolean(name || description || body);
  const descChars = useMemo(() => description.split(/\s+/).filter(Boolean).join(" ").length, [description]);
  const cancel = () => (dirty ? setAskDiscard(true) : onClose());

  const create = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.studioSave(kind, clean, preview);
      await onCreated(kind, clean);
    } catch (e) {
      setError(errorText(e));
      setSaving(false);
    }
  };

  return (
    <Modal
      title={`New ${KIND_LABEL[kind].toLowerCase()}`}
      onClose={cancel}
      dismissable={!saving}
      footer={
        <>
          <span />
          <div className="actions">
            <Button onClick={cancel}>Cancel</Button>
            <Button variant="primary" loading={saving} disabled={!preview || Boolean(nameError) || !description.trim()} onClick={() => void create()}>
              Save to library
            </Button>
          </div>
        </>
      }
    >
      {askDiscard && (
        <Callout
          tone="warn"
          icon="alert"
          action={
            <div className="actions">
              <Button size="sm" autoFocus onClick={() => setAskDiscard(false)}>
                Keep editing
              </Button>
              <Button size="sm" variant="danger" onClick={onClose}>
                Discard
              </Button>
            </div>
          }
        >
          <p>Discard this new item?</p>
        </Callout>
      )}
      <Field label="Name" htmlFor="new-name" error={nameError} hint={`Becomes ${relPath(kind, clean || "name")}.`}>
        <input id="new-name" className="input mono" autoFocus value={name} onChange={(e) => setName(e.target.value.toLowerCase())} aria-invalid={Boolean(nameError)} aria-describedby={describedBy("new-name", nameError, "hint")} autoComplete="off" />
      </Field>
      <Segmented
        label="Kind"
        value={kind}
        onChange={setKind}
        options={[
          { value: "skill", label: "Skill" },
          { value: "agent", label: "Subagent" },
          { value: "command", label: "Command" },
        ]}
      />
      <p className="muted">
        {kind === "skill"
          ? "Claude uses a skill on its own when the description matches the task."
          : kind === "agent"
            ? "Claude hands work to a subagent with its own instructions and tools."
            : "A command runs when you type /name in Claude Code."}
      </p>
      <Field label="Description (required)" htmlFor="new-desc" hint="Start with what it does, then when to use it. Claude decides from this text.">
        <textarea id="new-desc" className="input textarea" rows={3} value={description} onChange={(e) => setDescription(e.target.value)} aria-describedby={describedBy("new-desc", null, "hint")} />
      </Field>
      {kind === "skill" && <Meter label="Description length (with when_to_use)" value={descChars} max={DESC_BUDGET} unit="characters" />}
      <Field label="Instructions (optional)" htmlFor="new-body" hint="Leave empty for a starting outline you can edit later.">
        <textarea id="new-body" className="input textarea" rows={4} value={body} onChange={(e) => setBody(e.target.value)} aria-describedby={describedBy("new-body", null, "hint")} />
      </Field>
      {preview && (
        <>
          <h3 className="section-title" id="new-preview-h">
            The file Facet will save
          </h3>
          <pre className="code-preview" tabIndex={0} role="region" aria-labelledby="new-preview-h">
            {preview}
          </pre>
          {lint && lint.findings.filter((f) => f.severity !== "info").length > 0 && <Findings lint={lint.findings.filter((f) => f.severity !== "info")} />}
        </>
      )}
      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}
    </Modal>
  );
}

function ImportSection({ accounts, scope, library, home, onImported }: { accounts: Account[]; scope: string | null; library: StudioItem[]; home?: string; onImported: (kind: ItemKind, name: string) => Promise<void> }) {
  const api = useApi();
  const [chosen, setChosen] = useState<string | null>(null);
  const accountId = chosen ?? scope ?? accounts[0]?.id ?? "";
  const account = accounts.find((a) => a.id === accountId);
  const [items, setItems] = useState<AccountItem[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!accountId) return;
    setItems(null);
    setError(null);
    void api.studioAccountItems(accountId).then(setItems, (e) => {
      setItems([]);
      setError(errorText(e));
    });
  }, [api, accountId, library.length, attempt]);

  if (accounts.length === 0) return null;
  const inLibrary = (i: AccountItem) => library.some((l) => l.kind === i.kind && l.name === i.name);
  const shown = (items ?? []).filter((i) => !inLibrary(i));

  return (
    <section aria-labelledby="studio-import-h">
      <h2 className="section-title" id="studio-import-h">
        Already in your accounts
      </h2>
      <div className="card card-pad stack">
        <div className="actions">
          {account && <Avatar {...avatarOf(account)} />}
          <Field label="Account" htmlFor="import-account">
            <select id="import-account" className="input" value={accountId} onChange={(e) => setChosen(e.target.value)}>
              {accounts.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          </Field>
        </div>
        {error && (
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => setAttempt((n) => n + 1)}>Try again</Button>}>
            <p>{error}</p>
          </Callout>
        )}
        {items === null ? (
          <Skeleton />
        ) : shown.length === 0 && !error ? (
          <p className="muted">Everything in {account?.name ?? "this account"} is already in the library. Skills synced from claude.ai are managed by Claude Code and not listed here.</p>
        ) : (
          <ul className="list" aria-labelledby="studio-import-h">
            {shown.map((i) => (
              <li key={`${i.kind}:${i.name}`} className="list-row">
                <Tag>{KIND_LABEL[i.kind]}</Tag>
                <div>
                  <div className="title mono">{i.name}</div>
                  <div className="hint" dir="auto">
                    {i.description || <span className="mono">{tildify(i.path, home)}</span>}
                  </div>
                </div>
                <Button
                  size="sm"
                  loading={busy === `${i.kind}:${i.name}`}
                  aria-label={`Copy ${i.name} from ${account?.name ?? "the account"} to the library`}
                  onClick={async () => {
                    setBusy(`${i.kind}:${i.name}`);
                    setError(null);
                    try {
                      await api.studioImport(accountId, i.kind, i.name);
                      await onImported(i.kind, i.name);
                    } catch (e) {
                      setError(errorText(e));
                    } finally {
                      setBusy(null);
                    }
                  }}
                >
                  Copy to library
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}
