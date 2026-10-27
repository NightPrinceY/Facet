import { useEffect, useId, useRef, useState } from "react";
import type { Account, AccountState, AuthStatus, DetectedSetup } from "../lib/api";
import { errorText } from "../lib/api";
import { inputFrom, moved } from "../lib/accounts";
import { plural, planLabel, tildify } from "../lib/format";
import { useApi, type FacetData } from "../lib/useFacet";
import { Avatar, avatarOf, Button, Callout, Pill, Skeleton, accountStyle } from "../components/ui";
import { useToast } from "../components/Toast";
import { AccountModal, type AccountPreset } from "./AccountModal";

export function AccountsView({ data, scope, onAdd, onScope }: { data: FacetData; scope: string | null; onAdd: () => void; onScope: (id: string | null) => void }) {
  const api = useApi();
  const [editing, setEditing] = useState<Account | null>(null);
  const [detected, setDetected] = useState<DetectedSetup[]>([]);
  const [adopting, setAdopting] = useState<string | null>(null);
  const [adoptError, setAdoptError] = useState<string | null>(null);
  const [preset, setPreset] = useState<AccountPreset | null>(null);
  const toast = useToast();
  const home = data.info?.home;

  useEffect(() => {
    void api.detectExisting().then(setDetected, () => setDetected([]));
  }, [api, data.accounts.length]);

  const adopt = async (d: DetectedSetup) => {
    setAdopting(d.suggestedId);
    setAdoptError(null);
    try {
      const a = await api.saveAccount({
        name: d.suggestedName,
        withDesktop: d.desktopDir !== null,
        desktopDir: d.desktopDir ?? undefined,
        codeDir: d.codeDir ?? undefined,
      });
      toast(`Added ${a.name} with its existing folders.`);
      await data.refresh();
    } catch (e) {
      setAdoptError(errorText(e));
    } finally {
      setAdopting(null);
    }
  };

  const shown = scope ? data.accounts.filter((a) => a.id === scope) : data.accounts;
  const ids = data.accounts.map((a) => a.id);
  const [moveError, setMoveError] = useState<string | null>(null);
  const move = async (id: string, by: -1 | 1) => {
    const name = data.accounts.find((a) => a.id === id)?.name ?? id;
    try {
      const next = moved(ids, id, by);
      await api.reorderAccounts(next);
      await data.refresh();
      setMoveError(null);
      toast(`Moved ${name} to ${next.indexOf(id) + 1} of ${next.length}.`);
      // Keep keyboard focus on the moved card's controls.
      const q = name.replace(/["\\]/g, "\\$&");
      setTimeout(() => {
        const same = document.querySelector<HTMLButtonElement>(`[aria-label="Move ${q} ${by < 0 ? "earlier" : "later"}"]`);
        const other = document.querySelector<HTMLButtonElement>(`[aria-label="Move ${q} ${by < 0 ? "later" : "earlier"}"]`);
        (same && !same.disabled ? same : other)?.focus();
      }, 0);
    } catch (e) {
      setMoveError(errorText(e));
    }
  };

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Accounts</h1>
          <p>Each account has its own Claude Desktop folder and Claude Code folder. Facet starts each one separately and never reads or copies sign-in data.</p>
        </div>
        <div className="actions">
          {scope && (
            <Button variant="ghost" onClick={() => onScope(null)}>
              Show all accounts
            </Button>
          )}
          <Button variant="primary" icon="plus" onClick={onAdd}>
            Add account
          </Button>
        </div>
      </header>

      {data.error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void data.refresh()}>Try again</Button>}>
            <p>{data.error}</p>
          </Callout>
        </div>
      )}

      {data.loading && (
        <div className="grid-cards" aria-busy="true" aria-label="Loading accounts">
          <Skeleton card />
          <Skeleton card />
        </div>
      )}

      {!data.loading && !data.error && data.accounts.length === 0 && (
        <div className="card empty">
          <h2>No accounts yet</h2>
          <p>Add the accounts you use. If you already run Claude with separate folders, Facet lists them below so you can add them as they are.</p>
          <Button variant="primary" icon="plus" onClick={onAdd}>
            Add account
          </Button>
        </div>
      )}

      {moveError && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert">
            <p>Facet could not change the order: {moveError}</p>
          </Callout>
        </div>
      )}

      {shown.length > 0 && (
        <div className="grid-cards">
          {shown.map((a) => (
            <AccountCard
              key={a.id}
              account={a}
              state={data.states[a.id]}
              selected={scope === a.id}
              home={home}
              onEdit={() => setEditing(a)}
              onChanged={data.refresh}
              onMove={scope || data.accounts.length < 2 ? undefined : (by) => void move(a.id, by)}
              position={ids.indexOf(a.id)}
              count={ids.length}
            />
          ))}
        </div>
      )}

      {detected.length > 0 && !scope && (
        <>
          <h2 className="section-title">Found on this computer</h2>
          {adoptError && (
            <div className="mb-6">
              <Callout tone="bad" icon="alert">
                <p>{adoptError}</p>
              </Callout>
            </div>
          )}
          <div className="card">
            <ul className="list">
              {detected.map((d) => (
                <li key={d.suggestedId} className="list-row">
                  <span className="avatar avatar--neutral" aria-hidden="true">
                    {[...d.suggestedName][0]?.toUpperCase()}
                  </span>
                  <div>
                    <div className="title">{d.suggestedName}</div>
                    <div className="hint">
                      {d.desktopDir && (
                        <>
                          Desktop <span className="mono">{tildify(d.desktopDir, home)}</span>
                          {d.codeDir && " · "}
                        </>
                      )}
                      {d.codeDir && (
                        <>
                          Claude Code <span className="mono">{tildify(d.codeDir, home)}</span>
                        </>
                      )}
                    </div>
                  </div>
                  <div className="actions">
                    <Button size="sm" variant="ghost" icon="edit" onClick={() => setPreset({ name: d.suggestedName, desktopDir: d.desktopDir, codeDir: d.codeDir })}>
                      Name and add…
                    </Button>
                    <Button size="sm" loading={adopting === d.suggestedId} onClick={() => void adopt(d)} aria-label={`Add as ${d.suggestedName} with these folders`}>
                      Add as {d.suggestedName}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </>
      )}

      {preset && (
        <AccountModal
          existing={data.accounts}
          home={home ?? ""}
          preset={preset}
          onClose={() => setPreset(null)}
          onSaved={() => {
            setPreset(null);
            void data.refresh();
          }}
        />
      )}

      {editing && (
        <AccountModal
          existing={data.accounts}
          home={home ?? ""}
          account={editing}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null);
            void data.refresh();
          }}
          onDeleted={() => {
            setEditing(null);
            void data.refresh();
          }}
        />
      )}
    </>
  );
}

function AccountCard({ account: a, state, selected, home, onEdit, onChanged, onMove, position, count }: { account: Account; state?: AccountState; selected: boolean; home?: string; onEdit: () => void; onChanged: () => Promise<void>; onMove?: (by: -1 | 1) => void; position: number; count: number }) {
  const api = useApi();
  const toast = useToast();
  const titleId = useId();
  const [busy, setBusy] = useState<string | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [failure, setFailure] = useState<{ message: string; retry: () => void } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [newName, setNewName] = useState(a.name);
  const renameDone = useRef(false);
  const renameButton = useRef<HTMLButtonElement>(null);
  const running = state?.desktopPid != null;

  const startRename = () => {
    renameDone.current = false;
    setNewName(a.name);
    setRenaming(true);
  };

  // Enter, Escape and blur all end editing; only the first one counts.
  const finishRename = async (commit: boolean) => {
    if (renameDone.current) return;
    renameDone.current = true;
    setRenaming(false);
    setTimeout(() => renameButton.current?.focus(), 0);
    const n = newName.trim();
    if (!commit || !n || n === a.name) return;
    await run("rename", async () => {
      await api.saveAccount({ ...inputFrom(a), name: n });
      return `Renamed to ${n}.`;
    });
  };

  const run = async (key: string, fn: () => Promise<string | void>) => {
    setBusy(key);
    try {
      const msg = await fn();
      setFailure(null);
      if (msg) toast(msg);
      await onChanged();
    } catch (e) {
      setFailure({ message: errorText(e), retry: () => void run(key, fn) });
    } finally {
      setBusy(null);
    }
  };

  const checkSignIn = () =>
    run("auth", async () => {
      setAuth(await api.checkSignIn(a.id));
    });

  const emailMismatch = Boolean(auth?.loggedIn && a.expectedEmail && auth.email && auth.email.toLowerCase() !== a.expectedEmail.toLowerCase());

  return (
    <article className="card account-card" style={accountStyle(a.color)} data-selected={selected} aria-labelledby={titleId}>
      <header>
        <Avatar {...avatarOf(a)} size="lg" running={running} />
        <div>
          <h2 id={titleId} className={renaming ? "visually-hidden" : undefined}>
            {a.name}
          </h2>
          {renaming && (
            <input
              className="input"
              autoFocus
              aria-label={`New name for ${a.name}`}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onBlur={() => void finishRename(true)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void finishRename(true);
                if (e.key === "Escape") {
                  e.stopPropagation();
                  void finishRename(false);
                }
              }}
            />
          )}
          <div className="sub">
            {[planLabel(a.plan), a.desktopDir ? "Desktop and Claude Code" : "Claude Code only"].filter(Boolean).join(" · ")} · <span className="nowrap">claude-{a.id}</span>
          </div>
        </div>
        <div className="card-head-tools">
          <Button ref={renameButton} size="sm" variant="ghost" icon="edit" aria-label={`Rename ${a.name}`} title="Rename" onClick={startRename} />
          {onMove && (
            <>
              <Button size="sm" variant="ghost" icon="earlier" aria-label={`Move ${a.name} earlier`} title="Move earlier" disabled={position === 0} onClick={() => onMove(-1)} />
              <Button size="sm" variant="ghost" icon="later" aria-label={`Move ${a.name} later`} title="Move later" disabled={position === count - 1} onClick={() => onMove(1)} />
            </>
          )}
        </div>
      </header>
      {a.notes && <p className="account-notes">{a.notes}</p>}

      <div className="pills">
        {a.desktopDir &&
          (running ? (
            <Pill tone="ok" dot>
              Desktop running
            </Pill>
          ) : (
            <Pill>Desktop closed</Pill>
          ))}
        {state && state.liveSessions > 0 && <Pill dot>{plural(state.liveSessions, "Code session")}</Pill>}
        {state && state.waitingSessions > 0 && (
          <Pill tone="warn" dot>
            {state.waitingSessions} waiting for you
          </Pill>
        )}
        {running && state && !state.launchedByFacet && <Pill>Started outside Facet</Pill>}
      </div>

      <dl className="kv">
        {a.desktopDir && (
          <>
            <dt>Desktop folder</dt>
            <dd className="mono" title={a.desktopDir}>
              {tildify(a.desktopDir, home)}
            </dd>
          </>
        )}
        <dt>Claude Code folder</dt>
        <dd className="mono" title={a.codeDir}>
          {tildify(a.codeDir, home)}
        </dd>
        {a.expectedEmail && (
          <>
            <dt>Signs in as</dt>
            <dd className="ltr">{a.expectedEmail}</dd>
          </>
        )}
        {auth && (
          <>
            <dt>Signed in now</dt>
            <dd>
              {auth.loggedIn ? (
                <>
                  <span className="ltr">{auth.email}</span>
                  {[auth.orgName, auth.subscriptionType && `plan ${planLabel(auth.subscriptionType)}`].filter(Boolean).map((x) => ` · ${x}`)}
                </>
              ) : (
                "Not signed in to Claude Code"
              )}
            </dd>
          </>
        )}
      </dl>

      {emailMismatch && (
        <Callout tone="bad" icon="alert">
          <p>
            Signed in as <span className="ltr">{auth?.email}</span>, expected <span className="ltr">{a.expectedEmail}</span>. Sign out in a terminal for this account (<span className="mono">claude auth logout</span>) and sign in with the right email.
          </p>
        </Callout>
      )}
      {failure && (
        <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={failure.retry}>Try again</Button>}>
          <p>{failure.message}</p>
        </Callout>
      )}

      <div className="card-actions">
        {a.desktopDir &&
          (running ? (
            <>
              <Button size="sm" icon="window" loading={busy === "launch"} onClick={() => run("launch", async () => (await api.launchDesktop(a.id), `Brought ${a.name} to front.`))}>
                Bring to front
              </Button>
              <Button size="sm" icon="stop" loading={busy === "quit"} onClick={() => run("quit", async () => (await api.quitDesktop(a.id), `Asked ${a.name}'s Desktop app to quit.`))}>
                Quit
              </Button>
            </>
          ) : (
            <Button size="sm" variant="primary" icon="play" loading={busy === "launch"} onClick={() => run("launch", async () => (await api.launchDesktop(a.id), `Started ${a.name}'s Desktop app.`))}>
              Launch
            </Button>
          ))}
        <Button size="sm" icon="term" loading={busy === "term"} onClick={() => run("term", async () => (await api.openTerminal(a.id), `Opened a terminal as ${a.name}.`))}>
          Open terminal
        </Button>
        <Button size="sm" icon="user" loading={busy === "auth"} onClick={() => void checkSignIn()}>
          Check sign-in
        </Button>
        <Button size="sm" variant="ghost" icon="settings" onClick={onEdit}>
          Edit all options
        </Button>
      </div>

      {(!state?.shimInstalled || (a.desktopDir && !state?.launcherInstalled)) && (
        <div className="setup-row">
          <span>Set up:</span>
          {!state?.shimInstalled && (
            <Button size="sm" variant="ghost" icon="term" loading={busy === "shim"} onClick={() => run("shim", async () => `Added the terminal command ${tildify(await api.installShim(a.id), home)}.`)}>
              Add terminal command <span className="mono">claude-{a.id}</span>
            </Button>
          )}
          {a.desktopDir && !state?.launcherInstalled && (
            <Button size="sm" variant="ghost" icon="launcher" loading={busy === "launcher"} onClick={() => run("launcher", async () => (await api.installLauncher(a.id), `Added ${a.name} to the app menu.`))}>
              Add to app menu
            </Button>
          )}
        </div>
      )}
    </article>
  );
}
