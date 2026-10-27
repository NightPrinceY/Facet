import { useCallback, useEffect, useState } from "react";
import type { Account } from "../lib/api";
import { errorText } from "../lib/api";
import { tildify } from "../lib/format";
import { useApi } from "../lib/useFacet";
import { Button, Callout, Field, Skeleton, describedBy } from "../components/ui";

export interface EffectiveEntry {
  key: string;
  value: unknown;
  sources: string[];
  overridden: string[];
}

export interface EffectiveLayer {
  name: string;
  path: string;
  exists: boolean;
  problem: string | null;
}

export interface Effective {
  layers: EffectiveLayer[];
  entries: EffectiveEntry[];
}

export const LAYER_LABEL: Record<string, string> = {
  managed: "Managed policy",
  "project-local": "Project, this computer",
  project: "Project",
  account: "Account",
};

function show(v: unknown): string {
  if (typeof v === "string") return v;
  return JSON.stringify(v);
}

/** Read-only view of what applies after Claude Code merges its layers
 *  (PRD M4 "effective settings"). Secret values arrive hidden from the core. */
export function EffectiveSettings({ account, home }: { account: Account; home?: string }) {
  const api = useApi();
  const [project, setProject] = useState("");
  const [data, setData] = useState<Effective | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(
    async (folder: string) => {
      setLoading(true);
      setError(null);
      try {
        const expanded = folder.trim() === "~" ? home : folder.trim().startsWith("~/") && home ? home + folder.trim().slice(1) : folder.trim();
        setData(await api.controlEffective(account.id, expanded || undefined));
      } catch (e) {
        setError(errorText(e));
      } finally {
        setLoading(false);
      }
    },
    [api, account.id, home],
  );

  useEffect(() => {
    void load("");
  }, [load]);

  return (
    <section className="card card-pad stack" aria-labelledby="eff-h" id="ctl-effective">
      <h2 className="section-title" id="eff-h">
        What applies
      </h2>
      <p className="muted">
        Claude Code combines several files. Managed policy wins, then the project's local file, then the project file, then this account's settings. Lists of permission rules add up across files.
      </p>
      <form
        className="rule-row"
        onSubmit={(e) => {
          e.preventDefault();
          void load(project);
        }}
      >
        <Field label="Project folder (optional)" htmlFor="eff-project" hint="Leave empty to see the account alone.">
          <input id="eff-project" className="input mono" value={project} placeholder="~/work/app" onChange={(e) => setProject(e.target.value)} aria-describedby={describedBy("eff-project", null, "hint")} />
        </Field>
        <span />
        <Button type="submit" loading={loading}>
          Show
        </Button>
      </form>
      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}
      {!data && !error && <Skeleton />}
      {data && (
        <>
          <ul className="list" aria-label="Files">
            {data.layers.map((l) => (
              <li key={l.name} className="list-row">
                <span className={`status-icon${l.problem ? " status-icon--bad" : l.exists ? " status-icon--ok" : ""}`} aria-hidden="true">
                  {l.exists ? "✓" : "–"}
                </span>
                <div>
                  <div className="title">{LAYER_LABEL[l.name] ?? l.name}</div>
                  <div className="hint">
                    <span className="mono">{tildify(l.path, home)}</span> · {l.problem ? `Ignored: ${l.problem}` : l.exists ? "Used" : "Not present"}
                  </div>
                </div>
                <span />
              </li>
            ))}
          </ul>
          {data.entries.length === 0 ? (
            <p className="muted">No settings are set in any of these files; Claude Code uses its defaults.</p>
          ) : (
            <div className="table-wrap">
              <table className="table">
                <caption className="visually-hidden">Effective settings and where each comes from</caption>
                <thead>
                  <tr>
                    <th scope="col">Setting</th>
                    <th scope="col">Value</th>
                    <th scope="col">From</th>
                  </tr>
                </thead>
                <tbody>
                  {data.entries.map((e) => (
                    <tr key={e.key}>
                      <td className="mono">{e.key}</td>
                      <td className="mono">{show(e.value)}</td>
                      <td>
                        {e.sources.map((s) => LAYER_LABEL[s] ?? s).join(" + ")}
                        {e.overridden.length > 0 && <div className="hint">wins over {e.overridden.map((s) => LAYER_LABEL[s] ?? s).join(", ")}</div>}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </section>
  );
}
