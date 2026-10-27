import type { Account, AppInfo, ClaudeInstall } from "../lib/api";
import { LinksCard } from "./LinksCard";
import { tildify } from "../lib/format";
import { Pill, Segmented } from "../components/ui";

export type Theme = "system" | "light" | "dark";

function InstallLine({ install, home }: { install: ClaudeInstall | null; home: string }) {
  if (!install) return <>Not found</>;
  return (
    <>
      <span className="mono">
        {tildify(install.path, home)}
        {install.version ? ` · ${install.version}` : ""}
      </span>{" "}
      {install.official ? <Pill tone="ok">Official build</Pill> : <Pill tone="warn">Not confirmed as official</Pill>}
    </>
  );
}

export function SettingsView({ info, theme, onTheme, accounts }: { info: AppInfo | null; theme: Theme; onTheme: (t: Theme) => void; accounts: Account[] }) {
  const loading = "Loading";
  return (
    <>
      <header className="view-head">
        <div>
          <h1 tabIndex={-1}>Settings</h1>
          <p>How Facet looks, where each account's links open, and what Facet found on this computer.</p>
        </div>
      </header>

      <div className="card card-pad stack">
        <div className="toggle-row">
          <div>
            <div className="label">Theme</div>
            <div className="hint">System follows your desktop's light or dark setting.</div>
          </div>
          <Segmented
            label="Theme"
            value={theme}
            onChange={onTheme}
            options={[
              { value: "system", label: "System" },
              { value: "light", label: "Light" },
              { value: "dark", label: "Dark" },
            ]}
          />
        </div>
      </div>

      <div className="mt-6">
        <LinksCard accounts={accounts} />
      </div>

      <h2 className="section-title">About</h2>
      <div className="card card-pad stack">
        <dl className="kv">
          <dt>Facet</dt>
          <dd>{info ? `Version ${info.version} · ${info.platform}` : loading}</dd>
          <dt>Claude Desktop</dt>
          <dd>{info ? <InstallLine install={info.desktop} home={info.home} /> : loading}</dd>
          <dt>Claude Code</dt>
          <dd>{info ? <InstallLine install={info.code} home={info.home} /> : loading}</dd>
          <dt>Accounts file</dt>
          <dd className="mono">{info ? tildify(info.storePath, info.home) : loading}</dd>
          <dt>Quick switcher</dt>
          <dd>
            {info ? (
              info.shortcut === "portal" ? (
                <>
                  Ctrl K inside Facet. Your desktop also offers global shortcuts; bind one to <span className="mono">facet --switcher</span>.
                </>
              ) : (
                <>
                  Ctrl K inside Facet. For a global shortcut, bind <span className="mono">facet --switcher</span> in your desktop's keyboard settings.
                </>
              )
            ) : (
              loading
            )}
          </dd>
        </dl>
        <p className="muted">Facet is unofficial and not affiliated with Anthropic. It launches only the official Claude apps, never reads or copies sign-in data, and makes no network calls to Anthropic.</p>
      </div>
    </>
  );
}
