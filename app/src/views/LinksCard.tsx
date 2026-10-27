import { useCallback, useEffect, useState } from "react";
import type { Account, Browser, LinkTarget, LinksState } from "../lib/api";
import { errorText } from "../lib/api";
import { relativeTime } from "../lib/usage";
import { useApi } from "../lib/useFacet";
import { Avatar, Button, Callout, Modal, Pill, Skeleton, avatarOf } from "../components/ui";
import { useToast } from "../components/Toast";

const key = (t: LinkTarget | null | undefined) => (t ? `${t.browser}|${t.profile ?? ""}` : "");
const parse = (k: string): LinkTarget | null => {
  if (!k) return null;
  const [browser, profile] = k.split("|");
  return { browser: browser!, profile: profile || null };
};

export function targetLabel(browsers: Browser[], t: LinkTarget | null | undefined): string {
  if (!t) return "Same as other links";
  const b = browsers.find((x) => x.id === t.browser);
  if (!b) return `${t.browser} (not installed)`;
  const p = b.profiles.find((x) => x.dir === t.profile);
  if (!t.profile) return b.name;
  return p ? `${b.name} · ${p.name}${p.emails[0] ? ` · ${p.emails[0]}` : ""}` : `${b.name} · ${t.profile} (missing)`;
}

function TargetSelect({ id, label, value, browsers, emptyLabel, onChange }: { id: string; label: string; value: LinkTarget | null | undefined; browsers: Browser[]; emptyLabel: string; onChange: (t: LinkTarget | null) => void }) {
  return (
    <select id={id} className="input" aria-label={label} value={key(value)} onChange={(e) => onChange(parse(e.target.value))}>
      <option value="">{emptyLabel}</option>
      {browsers.map((b) => (
        <optgroup key={b.id} label={b.name}>
          {b.profiles.length === 0 ? (
            <option value={key({ browser: b.id })}>{b.name}</option>
          ) : (
            b.profiles.map((p) => (
              <option key={p.dir} value={key({ browser: b.id, profile: p.dir })}>
                {p.name}
                {p.emails.length ? ` · ${p.emails.join(", ")}` : ""} ({p.dir})
              </option>
            ))
          )}
        </optgroup>
      ))}
    </select>
  );
}

/** Settings › Links (D-022): open each account's links in its own browser profile. */
export function LinksCard({ accounts }: { accounts: Account[] }) {
  const api = useApi();
  const toast = useToast();
  const [st, setSt] = useState<LinksState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<"on" | "off" | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setSt(await api.linksState());
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  }, [api]);

  useEffect(() => {
    void load();
  }, [load]);

  const act = async (fn: () => Promise<unknown>, done?: string) => {
    setBusy(true);
    try {
      await fn();
      setError(null);
      if (done) toast(done);
      await load();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  if (!st) return error ? <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Try again</Button>}><p>{error}</p></Callout> : <Skeleton card />;

  const on = st.settings.enabled && st.currentDefault === "facet-links.desktop";
  const drifted = st.settings.enabled && st.currentDefault !== "facet-links.desktop";
  const browsers = st.browsers;

  return (
    <section className="card card-pad stack" aria-labelledby="links-h">
      <div className="section-head">
        <h2 className="section-title" id="links-h">
          Links
        </h2>
        {on ? (
          <Pill tone="ok" dot>
            On
          </Pill>
        ) : (
          <Pill>Off</Pill>
        )}
      </div>
      <p className="muted">When a Claude window or Claude Code terminal opens a link (sign-in, a link in chat, docs), Facet opens it in that account's browser profile, so the work account's links land in your work Chrome profile.</p>
      {drifted && (
        <Callout tone="warn" icon="alert" action={<Button size="sm" onClick={() => setConfirm("on")}>Turn on again</Button>}>
          <p>Another app became the default browser, so links no longer go through Facet.</p>
        </Callout>
      )}
      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}

      <ul className="list" aria-label="Browser profile per account">
        {accounts.map((a) => {
          const value = st.settings.accounts[a.id] ?? null;
          const suggestion = st.suggestions[a.id];
          return (
            <li key={a.id} className="list-row list-row--metric links-row">
              <Avatar {...avatarOf(a)} label={a.name} />
              <div className="stack-sm">
                <label className="title" htmlFor={`links-${a.id}`}>
                  {a.name}
                </label>
                <TargetSelect id={`links-${a.id}`} label={`Open ${a.name}'s links in`} value={value} browsers={browsers} emptyLabel="Same as other links" onChange={(t) => void act(() => api.linksSetAccount(a.id, t), `${a.name}'s links open in ${targetLabel(browsers, t)}.`)} />
                {suggestion && key(suggestion) !== key(value) && (
                  <div className="hint">
                    Suggested: {targetLabel(browsers, suggestion)} (signed in as {a.expectedEmail}).{" "}
                    <Button size="sm" variant="ghost" onClick={() => void act(() => api.linksSetAccount(a.id, suggestion), `${a.name}'s links open in ${targetLabel(browsers, suggestion)}.`)}>
                      Use it
                    </Button>
                  </div>
                )}
              </div>
              <Button size="sm" disabled={!value && !st.settings.fallback} onClick={() => void act(() => api.linksTest(a.id), `Opened a test page as ${a.name}.`)} aria-label={`Open a test page in ${a.name}'s profile`}>
                Test
              </Button>
            </li>
          );
        })}
        <li className="list-row list-row--metric links-row">
          <span className="avatar avatar--neutral" aria-hidden="true">
            ?
          </span>
          <div className="stack-sm">
            <label className="title" htmlFor="links-fallback">
              Everything else
            </label>
            <TargetSelect id="links-fallback" label="Open other links in" value={st.settings.fallback} browsers={browsers} emptyLabel={st.settings.previousDefault ? "Previous default browser" : "The first browser found"} onChange={(t) => void act(() => api.linksSetFallback(t), `Other links open in ${t ? targetLabel(browsers, t) : "the previous default browser"}.`)} />
            <div className="hint">Links from other apps, and from Claude windows Facet did not start (for example from the system menu). Pick your personal profile here.</div>
          </div>
          <span />
        </li>
      </ul>

      <div className="actions">
        {on ? (
          <Button onClick={() => setConfirm("off")}>Turn off</Button>
        ) : (
          <Button variant="primary" disabled={browsers.length === 0} onClick={() => setConfirm("on")}>
            Turn on
          </Button>
        )}
      </div>

      {st.recent.length > 0 && (
        <div>
          <h3 className="eco-h">Recent links</h3>
          <ul className="eco-items" aria-label="Recent links">
            {st.recent.slice(0, 8).map((r) => (
              <li key={r.ms}>
                <span className="mono">{r.host}</span> <span className="hint">· {r.accountId ? accounts.find((a) => a.id === r.accountId)?.name ?? r.accountId : "unknown window"} → {r.browser ? targetLabel(browsers, { browser: r.browser, profile: r.profile }) : "previous default browser"} · {relativeTime(r.ms)}</span>
              </li>
            ))}
          </ul>
          <p className="hint">Facet keeps only the site name of the last 30 links, on this computer.</p>
        </div>
      )}

      {confirm && (
        <Modal
          title={confirm === "on" ? "Open links per account" : "Stop opening links per account"}
          onClose={() => setConfirm(null)}
          footer={
            <>
              <span />
              <div className="actions">
                <Button onClick={() => setConfirm(null)}>Cancel</Button>
                <Button
                  variant="primary"
                  loading={busy}
                  onClick={() =>
                    void act(confirm === "on" ? api.linksEnable : api.linksDisable, confirm === "on" ? "Facet now opens links in each account's profile." : "Your previous default browser opens links again.").then(() => setConfirm(null))
                  }
                >
                  {confirm === "on" ? "Turn on" : "Turn off"}
                </Button>
              </div>
            </>
          }
        >
          {confirm === "on" ? (
            <>
              <p>Facet becomes your system's default browser for links. It does not show a window; it passes every link straight to the right browser profile.</p>
              <ul className="eco-reasons">
                <li>Your current default ({st.currentDefault ?? "unknown"}) is remembered and comes back when you turn this off.</li>
                <li>Your desktop's default-apps file is backed up first.</li>
                <li>Facet tells accounts apart by the folder each Claude window was started with. It never reads cookies, passwords or sign-in data from the browser or from Claude.</li>
                <li>Windows that are already open pick this up for their next link.</li>
              </ul>
            </>
          ) : (
            <p>{st.settings.previousDefault ?? "Your previous browser"} becomes the default browser again. Your per-account choices stay saved for next time.</p>
          )}
        </Modal>
      )}
    </section>
  );
}
