import { useEffect, useMemo, useState } from "react";
import type { AutoCommandInfo, FolderRule } from "../lib/api";
import { errorText } from "../lib/api";
import { matchFolder } from "../lib/rules";
import { tildify } from "../lib/format";
import { useApi, type FacetData } from "../lib/useFacet";
import { Avatar, avatarOf, Button, Callout, Field } from "../components/ui";
import { useToast } from "../components/Toast";

/** Folder rules (PRD M7): the most specific folder picks the account for the
 *  `claude-auto` command. The real `claude` is never replaced. */
export function FolderRulesView({ data }: { data: FacetData }) {
  const api = useApi();
  const toast = useToast();
  const home = data.info?.home ?? "";
  const [saved, setSaved] = useState<FolderRule[] | null>(null);
  const [draft, setDraft] = useState<FolderRule[]>([]);
  const [auto, setAuto] = useState<AutoCommandInfo | null>(null);
  const [saving, setSaving] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [probe, setProbe] = useState("");

  useEffect(() => {
    void api.listRules().then(
      (r) => {
        const shown = r.map((x) => ({ ...x, folder: tildify(x.folder, home) }));
        setSaved(shown);
        setDraft(shown);
      },
      (e) => setError(errorText(e)),
    );
    void api.autoCommandInfo().then(setAuto, () => setAuto(null));
  }, [api, home]);

  const dirty = saved !== null && JSON.stringify(saved) !== JSON.stringify(draft);
  const byId = Object.fromEntries(data.accounts.map((a) => [a.id, a]));
  const expand = (f: string) => (f === "~" ? home : f.startsWith("~/") ? home + f.slice(1) : f);
  const probeMatch = useMemo(() => {
    const folder = expand(probe.trim());
    if (!folder) return undefined;
    return matchFolder(draft.map((r) => ({ ...r, folder: expand(r.folder) })), folder) ?? null;
  }, [probe, draft, home]);

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await api.saveRules(draft.filter((r) => r.folder.trim()).map((r) => ({ ...r, folder: expand(r.folder.trim()) })));
      const shown = res.rules.map((x) => ({ ...x, folder: tildify(x.folder, home) }));
      setSaved(shown);
      setDraft(shown);
      toast(res.commandUpdated ? "Saved the rules and updated claude-auto." : "Saved the folder rules.");
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSaving(false);
    }
  };

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      const path = await api.installAutoCommand();
      setAuto({ path, installed: true });
      toast(`Added the command ${tildify(path, home)}.`);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setInstalling(false);
    }
  };

  const firstAccount = data.accounts[0]?.id ?? "";

  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Folder rules</h1>
          <p>Pick the account from the folder you work in. The most specific folder wins. The command <span className="mono">claude-auto</span> follows these rules; the official <span className="mono">claude</span> command is never replaced.</p>
        </div>
        <div className="actions">
          <Button variant="primary" loading={saving} disabled={!dirty} onClick={() => void save()}>
            Save rules
          </Button>
        </div>
      </header>

      {error && (
        <div className="mb-6">
          <Callout tone="bad" icon="alert">
            <p>{error}</p>
          </Callout>
        </div>
      )}

      {data.accounts.length === 0 ? (
        <div className="card empty">
          <h2>Add an account first</h2>
          <p>Folder rules point folders at accounts, so they need at least one account.</p>
        </div>
      ) : (
        <div className="card mb-6">
          {draft.length === 0 && <p className="card-pad muted">No rules yet. Add one, for example ~/work for your work account.</p>}
          {draft.map((r, i) => (
            <div className="rule-row" key={i}>
              <input
                className="input mono"
                aria-label={`Folder for rule ${i + 1}`}
                value={r.folder}
                placeholder="~/work"
                onChange={(e) => setDraft(draft.map((x, j) => (j === i ? { ...x, folder: e.target.value } : x)))}
              />
              <select
                className="input"
                aria-label={`Account for rule ${i + 1}`}
                value={r.accountId}
                onChange={(e) => setDraft(draft.map((x, j) => (j === i ? { ...x, accountId: e.target.value } : x)))}
              >
                {data.accounts.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.name}
                  </option>
                ))}
              </select>
              <Button size="sm" variant="ghost" icon="trash" onClick={() => setDraft(draft.filter((_, j) => j !== i))}>
                Remove
              </Button>
            </div>
          ))}
          <div className="card-pad">
            <Button size="sm" icon="plus" onClick={() => setDraft([...draft, { folder: "", accountId: firstAccount }])}>
              Add rule
            </Button>
          </div>
        </div>
      )}

      {data.accounts.length > 0 && (
        <>
          <h2 className="section-title">Try a folder</h2>
          <div className="card card-pad stack mb-6">
            <Field label="Folder" htmlFor="probe" hint="See which account claude-auto would use there, with the rules above (saved or not).">
              <input id="probe" className="input mono" value={probe} placeholder="~/work/client/app" onChange={(e) => setProbe(e.target.value)} aria-describedby="probe-hint" />
            </Field>
            {probeMatch !== undefined && (
              <p aria-live="polite">
                {probeMatch && byId[probeMatch.accountId] ? (
                  <>
                    <Avatar {...avatarOf(byId[probeMatch.accountId]!)} /> {byId[probeMatch.accountId]!.name}, from the rule for <span className="mono">{tildify(probeMatch.folder, home)}</span>
                  </>
                ) : (
                  "No rule covers this folder. claude-auto stops there and asks you to add a rule or use claude-<account>."
                )}
              </p>
            )}
          </div>

          <h2 className="section-title">Terminal command</h2>
          <div className="card card-pad stack">
            <p className="muted">
              {auto?.installed ? (
                <>
                  <span className="mono">{tildify(auto.path, home)}</span> is installed and is rewritten whenever you save rules.
                </>
              ) : (
                <>
                  Adds <span className="mono">claude-auto</span> to <span className="mono">~/.local/bin</span>. It runs the official Claude Code in the account whose rule matches the current folder, after removing variables that would override the sign-in.
                </>
              )}
            </p>
            {!auto?.installed && (
              <div>
                <Button icon="term" loading={installing} onClick={() => void install()}>
                  Add claude-auto
                </Button>
              </div>
            )}
          </div>
        </>
      )}
    </>
  );
}
