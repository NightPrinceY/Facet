import { useCallback, useEffect, useState } from "react";
import type { CheckStatus, HealthCheck } from "../lib/api";
import { errorText } from "../lib/api";
import { useApi, type FacetData } from "../lib/useFacet";
import { Avatar, avatarOf, Button, Callout, Checkbox, Pill, Skeleton } from "../components/ui";
import { Icon, type IconName } from "../components/Icon";

const TONE: Record<CheckStatus, { cls: string; label: string; icon: IconName }> = {
  bad: { cls: "status-icon status-icon--bad", label: "Needs fixing", icon: "alert" },
  warn: { cls: "status-icon status-icon--warn", label: "Worth a look", icon: "alert" },
  info: { cls: "status-icon", label: "Info", icon: "info" },
  ok: { cls: "status-icon status-icon--ok", label: "OK", icon: "check" },
};

export function HealthView({ data, scope, onOpenControl }: { data: FacetData; scope: string | null; onOpenControl?: (accountId: string, field: string) => void }) {
  const api = useApi();
  const [checks, setChecks] = useState<HealthCheck[] | null>(null);
  const [running, setRunning] = useState(false);
  const [withSignIn, setWithSignIn] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = useCallback(
    async (signIn: boolean) => {
      setRunning(true);
      setError(null);
      try {
        setChecks(await api.runHealth(signIn));
      } catch (e) {
        setError(errorText(e));
      } finally {
        setRunning(false);
      }
    },
    [api],
  );

  // Local checks only on open; the sign-in check runs on request.
  useEffect(() => {
    void run(false);
  }, [run]);

  // A scoped view shows that account's checks plus machine-wide ones.
  const shown = (checks ?? []).filter((c) => !scope || !c.accountId || c.accountId === scope);
  const counts = shown.reduce<Record<CheckStatus, number>>((m, c) => ((m[c.status] += 1), m), { bad: 0, warn: 0, info: 0, ok: 0 });
  const byId = Object.fromEntries(data.accounts.map((a) => [a.id, a]));
  const scoped = scope ? byId[scope] : undefined;

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Health</h1>
          <p>
            Checks that keep accounts apart: the Claude installs, settings that could switch an account, each account's folders, how long Claude Code keeps history, and sign-ins.
            {scoped ? ` Showing ${scoped.name} and machine-wide checks.` : ""} Facet never looks inside Claude's processes or sign-in files.
          </p>
        </div>
        <div className="actions">
          <Button variant="primary" loading={running} onClick={() => void run(withSignIn)}>
            Run checks
          </Button>
        </div>
      </header>

      <div className="card card-pad mb-6">
        <Checkbox
          checked={withSignIn}
          onChange={setWithSignIn}
          label="Also check each account's sign-in"
          hint="Runs the official claude auth status for each account. It reports which email is signed in; no prompt is sent."
        />
      </div>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void run(withSignIn)}>Try again</Button>}>
            <p>{error}</p>
          </Callout>
        </div>
      )}

      {checks === null && running && (
        <div className="card card-pad stack" aria-busy="true">
          <p className="muted">Running checks…</p>
          <Skeleton />
          <Skeleton />
          <Skeleton />
        </div>
      )}

      {checks && shown.length === 0 && (
        <div className="card empty">
          <h2>Nothing to check yet</h2>
          <p>Add an account, then run the checks again.</p>
        </div>
      )}

      {checks && shown.length > 0 && (
        <>
          <div className="pills mb-6" aria-live="polite">
            {counts.bad > 0 && (
              <Pill tone="bad" dot>
                <span className="num">{counts.bad}</span> need fixing
              </Pill>
            )}
            {counts.warn > 0 && (
              <Pill tone="warn" dot>
                <span className="num">{counts.warn}</span> worth a look
              </Pill>
            )}
            <Pill tone="ok" dot>
              <span className="num">{counts.ok}</span> OK
            </Pill>
            {counts.info > 0 && (
              <Pill dot>
                <span className="num">{counts.info}</span> info
              </Pill>
            )}
          </div>
          <div className="card">
            <ul className="list">
              {shown.map((c) => {
                const t = TONE[c.status];
                const acc = c.accountId ? byId[c.accountId] : undefined;
                return (
                  <li key={c.id} className="list-row">
                    <span className={t.cls} title={t.label}>
                      <Icon name={t.icon} size={14} />
                      <span className="visually-hidden">{t.label}:</span>
                    </span>
                    <div>
                      <div className="title">{c.title}</div>
                      <div className="hint">{c.detail}</div>
                      {c.fix && <div className="fix">Fix: {c.fix}</div>}
                    </div>
                    <div className="actions">
                      {acc && onOpenControl && c.id.startsWith("retention:") && (
                        <Button size="sm" onClick={() => onOpenControl(acc.id, "ctl-days")} aria-label={`Set transcript retention for ${acc.name} in Control`}>
                          Set in Control
                        </Button>
                      )}
                      {acc ? <Avatar {...avatarOf(acc)} label={acc.name} /> : <span />}
                    </div>
                  </li>
                );
              })}
            </ul>
          </div>
        </>
      )}
    </>
  );
}
