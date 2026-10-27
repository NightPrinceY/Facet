import { useCallback, useEffect, useState } from "react";
import type { Account } from "../lib/api";
import { errorText } from "../lib/api";
import type { ControlOp, ControlPreview } from "../lib/control";
import { RISK_LABEL, reviewState, splitArgs, type HookInfo, type Inventory, type McpAdd, type McpInfo, type PackageReport, type PluginEntry, type Risk } from "../lib/ecosystem";
import { plural, tildify } from "../lib/format";
import { SEVERITY_LABEL } from "../lib/studio";
import type { FacetData } from "../lib/useFacet";
import { useApi } from "../lib/useFacet";
import { Avatar, Button, Callout, Field, Modal, Pill, Segmented, Skeleton, avatarOf, describedBy } from "../components/ui";
import { isRefusedEnv } from "../lib/control";
import { DiffView } from "../components/DiffView";
import { useToast } from "../components/Toast";

type Tab = "installed" | "check";

/** Short bar filled ⅓, ⅔ or full plus the words (DESIGN-SYSTEM.md Risk). */
export function RiskBadge({ risk }: { risk: Risk }) {
  return (
    <span className={`risk risk--${risk}`}>
      <span className="risk-bar" aria-hidden="true">
        <span />
      </span>
      {RISK_LABEL[risk]}
    </span>
  );
}

/** What Ecosystem knows about one account (PRD M5). */
export function EcosystemView({ data, scope }: { data: FacetData; scope: string | null }) {
  const [tab, setTab] = useState<Tab>("installed");
  const [picked, setPicked] = useState<string | null>(null);
  const accountId = scope ?? picked ?? data.accounts[0]?.id ?? null;
  const account = data.accounts.find((a) => a.id === accountId) ?? null;
  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Ecosystem</h1>
          <p>What each account has installed, and a safety preview of any plugin, skill pack or repository before you use it. Facet reads the files and never runs them.</p>
        </div>
        <div className="actions">
          <Segmented<Tab> label="Ecosystem view" value={tab} onChange={setTab} options={[{ value: "installed", label: "Installed" }, { value: "check", label: "Check a folder" }]} />
        </div>
      </header>
      {tab === "installed" ? (
        <>
          {!scope && data.accounts.length > 1 && (
            <div className="mb-6">
              <Segmented<string> label="Account" value={accountId ?? ""} onChange={setPicked} options={data.accounts.map((a) => ({ value: a.id, label: a.name }))} />
            </div>
          )}
          {account ? <Installed key={account.id} account={account} home={data.info?.home} /> : <p className="muted">Add an account to see what it has installed.</p>}
        </>
      ) : (
        <CheckFolder home={data.info?.home} />
      )}
    </>
  );
}

function Installed({ account, home }: { account: Account; home?: string }) {
  const api = useApi();
  const toast = useToast();
  const [inv, setInv] = useState<Inventory | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const [toggle, setToggle] = useState<{ plugin: PluginEntry; enable: boolean; ops: ControlOp[]; preview: ControlPreview; error?: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [addingMcp, setAddingMcp] = useState(false);
  const [removing, setRemoving] = useState<McpInfo | null>(null);
  const [mcpError, setMcpError] = useState<string | null>(null);

  const removeMcp = async (m: McpInfo) => {
    setBusy(true);
    try {
      await api.mcpRemove(account.id, m.name, m.scope, m.project);
      setRemoving(null);
      setMcpError(null);
      toast(`Removed ${m.name} from ${account.name}. New sessions no longer start it.`);
      await load();
    } catch (e) {
      setMcpError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const load = useCallback(async () => {
    try {
      setInv(await api.ecosystemInventory(account.id));
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  }, [api, account.id]);

  useEffect(() => {
    void load();
  }, [load]);

  const askToggle = async (p: PluginEntry, enable: boolean) => {
    const ops: ControlOp[] = [{ path: ["enabledPlugins", p.id], value: enable }];
    try {
      setToggle({ plugin: p, enable, ops, preview: await api.controlPreview(account.id, ops) });
    } catch (e) {
      setError(errorText(e));
    }
  };

  const confirmToggle = async () => {
    if (!toggle) return;
    setBusy(true);
    try {
      const done = await api.controlApply(account.id, toggle.ops, toggle.preview.baseHash);
      setToggle(null);
      toast(`${toggle.enable ? "Turned on" : "Turned off"} ${toggle.plugin.name} for ${account.name}${done.backup ? "; backup kept" : ""}. New sessions pick it up.`);
      await load();
    } catch (e) {
      setToggle({ ...toggle, error: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  const markReviewed = async (p: PluginEntry, on: boolean) => {
    try {
      await api.ecosystemMarkReviewed(account.id, p.id, on ? (p.report?.hash ?? null) : null);
      toast(on ? `Marked ${p.name} as reviewed; Facet flags it if its files change.` : `Cleared the review of ${p.name}.`);
      await load();
    } catch (e) {
      setError(errorText(e));
    }
  };

  if (error && !inv)
    return (
      <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Try again</Button>}>
        <p>Facet could not read what {account.name} has installed: {error}</p>
      </Callout>
    );
  if (!inv) return <Skeleton card />;

  const enabledCount = inv.plugins.filter((p) => p.enabled).length;
  const changed = inv.plugins.filter((p) => reviewState(p) === "changed");

  return (
    <>
      <section className="card card-pad stack mb-6" aria-label={`${account.name} summary`}>
        <div className="cell-title">
          <Avatar {...avatarOf(account)} label={account.name} />
          <span className="title">{account.name}</span>
        </div>
        <div className="pills">
          <Pill>
            {plural(inv.plugins.length, "plugin")}, {enabledCount} on
          </Pill>
          <Pill>{plural(inv.mcp.length, "MCP server")}</Pill>
          <Pill>{plural(inv.hooks.length, "hook")}</Pill>
          <Pill>{plural(inv.skills, "personal skill")}</Pill>
          <Pill>{plural(inv.agents, "subagent")}</Pill>
          <Pill>{plural(inv.commands, "command")}</Pill>
        </div>
        {changed.length > 0 && (
          <Callout tone="warn" icon="alert">
            <p>
              {changed.map((p) => p.name).join(", ")} changed since you reviewed {changed.length === 1 ? "it" : "them"}. Open the details to see what it does now.
            </p>
          </Callout>
        )}
        {inv.settingsFindings
          .filter((f) => f.severity !== "info")
          .map((f) => (
            <Callout key={f.message} tone="warn" icon="alert">
              <p>{f.message}</p>
            </Callout>
          ))}
        {inv.problems.map((p) => (
          <Callout key={p} tone="bad" icon="alert">
            <p>{p}</p>
          </Callout>
        ))}
      </section>

      <section className="card mb-6" aria-labelledby="eco-plugins">
        <div className="card-pad">
          <h2 className="section-title" id="eco-plugins">
            Plugins
          </h2>
        </div>
        {error && (
          <div className="card-pad">
            <Callout tone="bad" icon="alert" action={<Button size="sm" onClick={() => void load()}>Try again</Button>}>
              <p>{error}</p>
            </Callout>
          </div>
        )}
        {inv.plugins.length === 0 ? (
          <p className="muted card-pad">No plugins installed for {account.name}.</p>
        ) : (
          <ul className="list" aria-label="Plugins">
            {inv.plugins.map((p) => {
              const st = reviewState(p);
              const expanded = open === p.id;
              return (
                <li key={p.id} className="session-row">
                  <div className="session-main eco-row">
                    <button type="button" className="session-toggle" aria-expanded={expanded} onClick={() => setOpen(expanded ? null : p.id)}>
                      <span className="title">{p.name}</span>
                      <span className="hint">
                        {p.marketplace || "local"}
                        {p.version ? ` · ${p.version}` : ""}
                        {p.lastUpdated ? ` · updated ${p.lastUpdated.slice(0, 10)}` : ""}
                      </span>
                    </button>
                    <span className="eco-badges">
                      {p.report ? <RiskBadge risk={p.report.risk} /> : <Pill>{p.marketplace === "builtin" ? "Built in" : "Not on disk"}</Pill>}
                      {st === "reviewed" && <Pill tone="ok">Reviewed</Pill>}
                      {st === "changed" && <Pill tone="warn">Changed since review</Pill>}
                      {p.enabled ? (
                        <Pill tone="ok" dot>
                          On
                        </Pill>
                      ) : (
                        <Pill>Off</Pill>
                      )}
                    </span>
                    <Button size="sm" onClick={() => void askToggle(p, !p.enabled)} aria-label={`${p.enabled ? "Turn off" : "Turn on"} ${p.name} for ${account.name}`}>
                      {p.enabled ? "Turn off" : "Turn on"}
                    </Button>
                  </div>
                  {expanded && (
                    <div className="session-detail">
                      {p.report ? <Report report={p.report} home={home} /> : <p className="muted">{p.marketplace === "builtin" ? "Built into Claude Code; there are no files to check." : "Claude Code lists this plugin, but its files are not on this computer."}</p>}
                      {p.report && (
                        <div className="actions">
                          {st === "reviewed" ? (
                            <Button size="sm" variant="ghost" onClick={() => void markReviewed(p, false)}>
                              Clear review
                            </Button>
                          ) : (
                            <Button size="sm" icon="check" onClick={() => void markReviewed(p, true)}>
                              Mark this version as reviewed
                            </Button>
                          )}
                        </div>
                      )}
                    </div>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <section className="card card-pad stack mb-6" aria-labelledby="eco-mcp">
        <h2 className="section-title" id="eco-mcp">
          MCP servers
        </h2>
        {inv.mcp.length === 0 ? <p className="muted">No MCP servers set up for {account.name} outside plugins.</p> : <McpList servers={inv.mcp} home={home} onRemove={(m) => setRemoving(m)} />}
        {removing && (
          <Callout
            tone="warn"
            icon="alert"
            action={
              <div className="actions">
                <Button size="sm" autoFocus onClick={() => setRemoving(null)}>
                  Keep
                </Button>
                <Button size="sm" variant="danger" loading={busy} onClick={() => void removeMcp(removing)}>
                  Remove
                </Button>
              </div>
            }
          >
            <p>
              Remove {removing.name} ({removing.scope === "local" ? "this folder only" : removing.scope}) from {account.name}? Facet runs the official claude mcp remove for this account.
            </p>
          </Callout>
        )}
        {mcpError && (
          <Callout tone="bad" icon="alert">
            <p>{mcpError}</p>
          </Callout>
        )}
        <div className="actions">
          <Button icon="plus" onClick={() => setAddingMcp(true)}>
            Add MCP server
          </Button>
        </div>
        <p className="hint">Facet adds and removes servers with the official claude mcp command for this account; it never edits Claude Code's state file itself.</p>
      </section>

      <section className="card card-pad stack mb-6" aria-labelledby="eco-hooks">
        <h2 className="section-title" id="eco-hooks">
          Hooks in settings
        </h2>
        {inv.hooks.length === 0 ? <p className="muted">No hooks in {account.name}'s settings.</p> : <HookList hooks={inv.hooks} />}
      </section>

      <section className="card card-pad stack" aria-labelledby="eco-markets">
        <h2 className="section-title" id="eco-markets">
          Marketplaces
        </h2>
        {inv.marketplaces.length === 0 ? (
          <p className="muted">No plugin marketplaces added.</p>
        ) : (
          <ul className="list" aria-label="Marketplaces">
            {inv.marketplaces.map((m) => (
              <li key={m.name} className="list-row list-row--metric">
                <div>
                  <div className="title">{m.name}</div>
                  <div className="hint mono">
                    {m.source} · {m.location}
                  </div>
                </div>
                {m.pinned ? <Pill tone="ok">Pinned</Pill> : <Pill tone="warn">Follows latest</Pill>}
                <span className="hint">{m.lastUpdated?.slice(0, 10)}</span>
              </li>
            ))}
          </ul>
        )}
        {inv.marketplaces.some((m) => !m.pinned) && <p className="hint">A marketplace that follows the latest version can change what its plugins do on the next update. Check plugins again after updates; Facet marks reviewed ones that change.</p>}
      </section>

      {addingMcp && (
        <McpAddModal
          account={account}
          home={home}
          onClose={() => setAddingMcp(false)}
          onAdded={async (msg) => {
            setAddingMcp(false);
            toast(msg);
            await load();
          }}
        />
      )}

      {toggle && (
        <Modal
          title={`${toggle.enable ? "Turn on" : "Turn off"} ${toggle.plugin.name} for ${account.name}`}
          onClose={() => setToggle(null)}
          identity={<Avatar {...avatarOf(account)} size="lg" />}
          footer={
            <>
              <span />
              <div className="actions">
                <Button onClick={() => setToggle(null)}>Cancel</Button>
                <Button variant="primary" loading={busy} onClick={() => void confirmToggle()}>
                  {toggle.enable ? "Turn on" : "Turn off"}
                </Button>
              </div>
            </>
          }
        >
          <p>Facet changes one line in {account.name}'s settings and keeps a backup. Sessions that are already open keep the old setting until they restart.</p>
          {toggle.enable && toggle.plugin.report && toggle.plugin.report.risk !== "low" && (
            <Callout tone="warn" icon="alert">
              <p>
                {RISK_LABEL[toggle.plugin.report.risk]}: {toggle.plugin.report.reasons.join(" ")}
              </p>
            </Callout>
          )}
          <DiffView file="settings.json" lines={toggle.preview.diff} />
          {toggle.error && (
            <Callout tone="bad" icon="alert">
              <p>{toggle.error}</p>
            </Callout>
          )}
        </Modal>
      )}
    </>
  );
}

function CheckFolder({ home }: { home?: string }) {
  const api = useApi();
  const [path, setPath] = useState("");
  const [report, setReport] = useState<PackageReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const scan = async () => {
    const p = path.trim();
    const full = p === "~" ? home : p.startsWith("~/") && home ? home + p.slice(1) : p;
    setBusy(true);
    setError(null);
    try {
      setReport(await api.ecosystemScan(full ?? p));
    } catch (e) {
      setReport(null);
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <form
        className="card card-pad stack mb-6"
        onSubmit={(e) => {
          e.preventDefault();
          void scan();
        }}
      >
        <div className="search-row">
          <Field label="Folder to check" htmlFor="eco-path" error={error} hint="A downloaded plugin or skill pack, or a cloned repository before you open it in Claude Code. Facet reads the files only.">
            <input id="eco-path" className="input mono" value={path} placeholder="~/Downloads/some-plugin" onChange={(e) => setPath(e.target.value)} aria-invalid={Boolean(error)} aria-describedby={describedBy("eco-path", error, "hint")} spellCheck={false} />
          </Field>
          <Button type="submit" variant="primary" loading={busy} disabled={!path.trim()}>
            Check
          </Button>
        </div>
      </form>
      {report && (
        <section className="card card-pad stack" aria-label="Safety preview">
          <div className="section-head">
            <h2 className="section-title">
              {report.name ?? report.root.split("/").pop()} {report.version && <span className="hint">{report.version}</span>}
            </h2>
            <RiskBadge risk={report.risk} />
          </div>
          <Report report={report} home={home} />
        </section>
      )}
    </>
  );
}

const KIND_WORD = { plugin: "Plugin", repo: "Repository with Claude settings", folder: "Folder" } as const;

/** Everything a scan found, in plain sentences. */
export function Report({ report: r, home }: { report: PackageReport; home?: string }) {
  return (
    <div className="stack eco-report">
      <p className="hint">
        {KIND_WORD[r.kind]} at <span className="mono">{tildify(r.root, home)}</span> · {plural(r.files, "file")} read · fingerprint <span className="mono">{r.hash}</span>
        {r.description ? ` · ${r.description}` : ""}
      </p>
      <ul className="eco-reasons" aria-label="Why this risk level">
        {r.reasons.map((x) => (
          <li key={x}>{x}</li>
        ))}
      </ul>
      {(r.skills.length > 0 || r.commands.length > 0 || r.agents.length > 0) && (
        <div>
          <h3 className="eco-h">Adds</h3>
          <ul className="eco-items">
            {[...r.skills.map((i) => ({ ...i, k: "Skill" })), ...r.commands.map((i) => ({ ...i, k: "Command" })), ...r.agents.map((i) => ({ ...i, k: "Subagent" }))].map((i) => (
              <li key={i.path}>
                <span className="tag">{i.k}</span> <span className="mono">{i.name}</span>
                {i.findings.map((f) => (
                  <div key={f.message} className="hint">
                    {SEVERITY_LABEL[f.severity]}: {f.message}
                  </div>
                ))}
              </li>
            ))}
          </ul>
        </div>
      )}
      {r.hooks.length > 0 && (
        <div>
          <h3 className="eco-h">Hooks</h3>
          <HookList hooks={r.hooks} />
        </div>
      )}
      {r.mcp.length > 0 && (
        <div>
          <h3 className="eco-h">MCP servers</h3>
          <McpList servers={r.mcp} home={home} />
        </div>
      )}
      {r.bin.length > 0 && (
        <div>
          <h3 className="eco-h">Programs added to PATH</h3>
          <p className="mono">{r.bin.join(", ")}</p>
        </div>
      )}
      {r.findings.length > 0 && (
        <div>
          <h3 className="eco-h">Flags</h3>
          <ul className="eco-items">
            {r.findings.map((f) => (
              <li key={f.message}>
                <span className="tag">{SEVERITY_LABEL[f.severity]}</span> {f.message}
              </li>
            ))}
          </ul>
        </div>
      )}
      {r.endpoints.length > 0 && (
        <div>
          <h3 className="eco-h">Web addresses mentioned</h3>
          <p className="mono eco-wrap">{r.endpoints.join(" · ")}</p>
        </div>
      )}
      {r.envVars.length > 0 && (
        <div>
          <h3 className="eco-h">Environment variables read</h3>
          <p className="mono eco-wrap">{r.envVars.join(" · ")}</p>
        </div>
      )}
      <p className="hint">Facet's checks are patterns, not a guarantee. A clean result means nothing on this list was found.</p>
    </div>
  );
}

function HookList({ hooks }: { hooks: HookInfo[] }) {
  return (
    <ul className="eco-items" aria-label="Hooks">
      {hooks.map((h, i) => (
        <li key={`${h.event}-${i}`}>
          <span className="tag">{h.event}</span>
          {h.matcher && <span className="hint"> when {h.matcher}</span>} <span className="mono eco-wrap">{h.target || `(${h.kind})`}</span>
          {h.notes.map((n) => (
            <div key={n} className="hint">
              {n}
            </div>
          ))}
        </li>
      ))}
    </ul>
  );
}

function McpList({ servers, home, onRemove }: { servers: McpInfo[]; home?: string; onRemove?: (m: McpInfo) => void }) {
  return (
    <ul className="eco-items" aria-label="MCP servers">
      {servers.map((m, i) => (
        <li key={`${m.scope}-${m.project}-${m.name}-${i}`}>
          <span className="title">{m.name}</span> <span className="tag">{m.scope === "local" ? "this folder only" : m.scope}</span> <span className="hint">{m.transport}</span>
          <div className="mono eco-wrap">{m.url ?? [m.command, ...m.args].filter(Boolean).join(" ")}</div>
          {m.project && <div className="hint">Folder: {tildify(m.project, home)}</div>}
          {(m.envNames.length > 0 || m.headerNames.length > 0) && <div className="hint">Uses: {[...m.envNames, ...m.headerNames].join(", ")} (values not shown)</div>}
          {m.notes.map((n) => (
            <div key={n} className="hint">
              {n}
            </div>
          ))}
          {onRemove && (m.scope === "user" || m.scope === "local") && (
            <Button size="sm" variant="ghost" icon="trash" onClick={() => onRemove(m)} aria-label={`Remove ${m.name}`}>
              Remove
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}

function McpAddModal({ account, home, onClose, onAdded }: { account: Account; home?: string; onClose: () => void; onAdded: (msg: string) => void }) {
  const api = useApi();
  const [name, setName] = useState("");
  const [scope, setScope] = useState<McpAdd["scope"]>("user");
  const [transport, setTransport] = useState<McpAdd["transport"]>("stdio");
  const [target, setTarget] = useState("");
  const [args, setArgs] = useState("");
  const [envText, setEnvText] = useState("");
  const [headerText, setHeaderText] = useState("");
  const [folder, setFolder] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const pairs = (text: string, sep: "=" | ":") =>
    text
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => {
        const i = l.indexOf(sep);
        return (i < 0 ? [l, ""] : [l.slice(0, i).trim(), l.slice(i + 1).trim()]) as [string, string];
      });
  const env = pairs(envText, "=");
  const headers = pairs(headerText, ":");
  const nameError = name && !/^[A-Za-z0-9_][A-Za-z0-9_-]{0,63}$/.test(name) ? "Use letters, digits, - or _ (up to 64)." : null;
  const envError = env.find(([k]) => isRefusedEnv(k)) ? "Facet does not pass sign-in or routing variables to MCP servers." : env.find(([k]) => !/^[A-Za-z_][A-Za-z0-9_]*$/.test(k)) ? "Write one NAME=value per line." : null;
  const targetError = target && transport !== "stdio" && !/^https?:\/\//.test(target.trim()) ? "The address must start with https://." : null;
  const expand = (p: string) => (p.trim() === "~" ? (home ?? p) : p.trim().startsWith("~/") && home ? home + p.trim().slice(1) : p.trim());
  const folderNeeded = scope !== "user";
  const ok = Boolean(name && target.trim() && !nameError && !envError && !targetError && (!folderNeeded || folder.trim()));

  const add = async () => {
    setBusy(true);
    setError(null);
    try {
      const out = await api.mcpAdd(account.id, { name, scope, transport, commandOrUrl: target.trim(), args: transport === "stdio" ? splitArgs(args) : [], env: transport === "stdio" ? env : [], headers: transport === "stdio" ? [] : headers, folder: folderNeeded ? expand(folder) : null });
      onAdded(out.split("\n")[0] || `Added ${name} to ${account.name}.`);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      title={`Add an MCP server to ${account.name}`}
      onClose={onClose}
      dismissable={!busy && !name && !target}
      identity={<Avatar {...avatarOf(account)} size="lg" />}
      footer={
        <>
          <span />
          <div className="actions">
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" loading={busy} disabled={!ok} onClick={() => void add()}>
              Add server
            </Button>
          </div>
        </>
      }
    >
      <p className="muted">An MCP server runs a program or connects to a service and gives Claude new tools in {account.name}. Add only servers you trust; check a downloaded one with Check a folder first.</p>
      <Field label="Name" htmlFor="mcp-name" error={nameError}>
        <input id="mcp-name" className="input mono" value={name} onChange={(e) => setName(e.target.value)} aria-invalid={Boolean(nameError)} aria-describedby={describedBy("mcp-name", nameError)} autoComplete="off" spellCheck={false} />
      </Field>
      <div className="field">
        <span className="field-label">Kind</span>
        <Segmented<McpAdd["transport"]> label="Kind" value={transport} onChange={setTransport} options={[{ value: "stdio", label: "Program on this computer" }, { value: "http", label: "Web address (HTTP)" }, { value: "sse", label: "Web address (SSE)" }]} />
      </div>
      <Field label={transport === "stdio" ? "Program" : "Address"} htmlFor="mcp-target" error={targetError} hint={transport === "stdio" ? "For example npx, uvx, node or a full path. Pin a version, for example npx -y package@1.2.3." : "For example https://mcp.example.com/mcp. Sign-in for the service happens later in Claude Code (/mcp)."}>
        <input id="mcp-target" className="input mono" value={target} onChange={(e) => setTarget(e.target.value)} aria-invalid={Boolean(targetError)} aria-describedby={describedBy("mcp-target", targetError, "hint")} spellCheck={false} />
      </Field>
      {transport === "stdio" ? (
        <>
          <Field label="Arguments (optional)" htmlFor="mcp-args" hint='Separated by spaces; quote an argument that has spaces, for example "my folder".'>
            <input id="mcp-args" className="input mono" value={args} onChange={(e) => setArgs(e.target.value)} aria-describedby="mcp-args-hint" spellCheck={false} />
          </Field>
          <Field label="Environment variables (optional)" htmlFor="mcp-env" error={envError} hint="One NAME=value per line. Claude Code stores the values in its own settings for this account.">
            <textarea id="mcp-env" className="input mono textarea" rows={3} value={envText} onChange={(e) => setEnvText(e.target.value)} aria-invalid={Boolean(envError)} aria-describedby={describedBy("mcp-env", envError, "hint")} spellCheck={false} />
          </Field>
        </>
      ) : (
        <Field label="Headers (optional)" htmlFor="mcp-headers" hint="One Name: value per line, for example Authorization: Bearer …">
          <textarea id="mcp-headers" className="input mono textarea" rows={2} value={headerText} onChange={(e) => setHeaderText(e.target.value)} aria-describedby="mcp-headers-hint" spellCheck={false} />
        </Field>
      )}
      <div className="field">
        <span className="field-label">Where it applies</span>
        <Segmented<McpAdd["scope"]> label="Where it applies" value={scope} onChange={setScope} options={[{ value: "user", label: "Every folder" }, { value: "local", label: "One folder, just you" }, { value: "project", label: "One folder, shared (.mcp.json)" }]} />
      </div>
      {folderNeeded && (
        <Field label="Project folder" htmlFor="mcp-folder" hint={scope === "project" ? "Claude Code writes .mcp.json in this folder, so it is shared with anyone who uses the project." : "Only this account, only in this folder."}>
          <input id="mcp-folder" className="input mono" value={folder} placeholder="~/work/app" onChange={(e) => setFolder(e.target.value)} aria-describedby="mcp-folder-hint" spellCheck={false} />
        </Field>
      )}
      {error && (
        <Callout tone="bad" icon="alert">
          <p>{error}</p>
        </Callout>
      )}
    </Modal>
  );
}
