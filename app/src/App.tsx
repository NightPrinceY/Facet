import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Api } from "./lib/api";
import { initial, planLabel } from "./lib/format";
import { ApiContext, useFacetData } from "./lib/useFacet";
import { Icon } from "./components/Icon";
import { NAV, type ViewId } from "./lib/nav";
import { Avatar, avatarOf, graphemes, Tag, accountStyle, rovingKeys } from "./components/ui";
import { ToastProvider, useToast } from "./components/Toast";
import { CommandPalette } from "./components/CommandPalette";
import { AccountsView } from "./views/AccountsView";
import { HealthView } from "./views/HealthView";
import { SettingsView, type Theme } from "./views/SettingsView";
import { FolderRulesView } from "./views/FolderRulesView";
import { ControlView } from "./views/ControlView";
import { StudioView } from "./views/StudioView";
import { TodayView } from "./views/TodayView";
import { CostsView } from "./views/CostsView";
import { SessionsView } from "./views/SessionsView";
import { EcosystemView } from "./views/EcosystemView";
import { TimelineView } from "./views/TimelineView";
import { BridgeView } from "./views/BridgeView";
import { AccountModal } from "./views/AccountModal";

function readTheme(): Theme {
  try {
    const t = localStorage.getItem("facet.theme");
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}

export function App({ api }: { api: Api }) {
  return (
    <ApiContext.Provider value={api}>
      <ToastProvider>
        <Shell api={api} />
      </ToastProvider>
    </ApiContext.Provider>
  );
}

function Shell({ api }: { api: Api }) {
  const data = useFacetData(api);
  const toast = useToast();
  // The browser preview (sample data only) can open a view from ?view=… for screenshots.
  const [view, setView] = useState<ViewId>(() => {
    const v = api.isSample ? new URLSearchParams(window.location.search).get("view") : null;
    return v && NAV.some((g) => g.items.some((i) => i.id === v)) ? (v as ViewId) : "accounts";
  });
  const [scopeFromUrl] = useState(() => (api.isSample ? new URLSearchParams(window.location.search).get("account") : null));
  const [scope, setScope] = useState<string | null>(scopeFromUrl);
  const [adding, setAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const editingAccount = data.accounts.find((a) => a.id === editingId) ?? null;
  const [palette, setPalette] = useState(false);
  const [controlFocus, setControlFocus] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>(readTheme);
  const firstView = useRef(true);

  useEffect(() => {
    const root = document.documentElement;
    if (theme === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", theme);
    try {
      localStorage.setItem("facet.theme", theme);
    } catch {
      /* storage unavailable: the theme still applies for this session */
    }
  }, [theme]);

  // Forget the scope if its account disappears.
  useEffect(() => {
    if (!data.loading && scope && !data.accounts.some((a) => a.id === scope)) setScope(null);
  }, [scope, data.accounts, data.loading]);

  const active = useMemo(() => data.accounts.find((a) => a.id === scope) ?? null, [data.accounts, scope]);

  // Window title names the account you are in (DESIGN-SYSTEM.md Window tint).
  useEffect(() => {
    void api.setTitle(active ? `Facet · ${active.name}` : "Facet");
  }, [api, active]);

  // Move focus to the new view's heading so the change is announced.
  useEffect(() => {
    if (firstView.current) {
      firstView.current = false;
      return;
    }
    document.querySelector<HTMLElement>("#main h1")?.focus();
  }, [view]);

  // Quick switcher: Ctrl/⌘ K, or `facet --switcher` from outside.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k" && !document.querySelector("[role=dialog]")) {
        e.preventDefault();
        setPalette(true);
      }
    };
    document.addEventListener("keydown", onKey);
    void api.takeSwitcherRequest().then((yes) => yes && setPalette(true));
    const offs: (() => void)[] = [];
    void api.onOpenSwitcher(() => setPalette(true)).then((f) => offs.push(f));
    void api.onNotice((m) => toast(m)).then((f) => offs.push(f));
    return () => {
      document.removeEventListener("keydown", onKey);
      offs.forEach((f) => f());
    };
  }, [api, toast]);

  const go = useCallback((v: ViewId) => setView(v), []);

  return (
    <>
      <div className="shell" style={active ? accountStyle(active.color) : undefined}>
        {api.isSample && (
          <div className="sample-banner" role="note">
            <Icon name="info" /> Browser preview with sample data. Nothing here touches your computer.
          </div>
        )}
        <div className="rail" role="toolbar" aria-orientation="vertical" aria-label="Switch account" onKeyDown={(e) => rovingKeys(e, ".rail-item", "vertical")}>
          <button type="button" className="rail-item rail-item--all" aria-pressed={scope === null} title="All accounts" tabIndex={scope === null ? 0 : -1} onClick={() => setScope(null)}>
            <Icon name="grid" />
            <span className="visually-hidden">All accounts</span>
          </button>
          <hr />
          {data.accounts.map((a) => {
            const running = data.states[a.id]?.desktopPid != null;
            return (
              <button
                key={a.id}
                type="button"
                className={`rail-item${!a.imageUrl && graphemes(a.badge || "") > 1 ? " rail-item--long" : ""}`}
                style={accountStyle(a.color)}
                aria-pressed={scope === a.id}
                tabIndex={scope === a.id ? 0 : -1}
                title={a.name}
                onClick={() => setScope(a.id)}
              >
                {a.imageUrl ? <img src={a.imageUrl} alt="" draggable={false} /> : <span aria-hidden="true">{a.badge?.trim() || initial(a.name)}</span>}
                <span className="visually-hidden">
                  {a.name}
                  {running ? ", Desktop app running" : ""}
                </span>
                {running && <span className="running-dot" />}
              </button>
            );
          })}
          <button type="button" className="rail-item rail-item--add" title="Add account" tabIndex={-1} onClick={() => setAdding(true)}>
            <Icon name="plus" />
            <span className="visually-hidden">Add account</span>
          </button>
        </div>

        <nav className="side" aria-label="Sections">
          <div className="scope">
            {active ? (
              <Avatar {...avatarOf(active)} size="lg" running={data.states[active.id]?.desktopPid != null} />
            ) : (
              <span className="avatar avatar--lg avatar--neutral" aria-hidden="true">
                <Icon name="grid" />
              </span>
            )}
            <div>
              <b>{active ? active.name : "All accounts"}</b>
              <span>{active ? planLabel(active.plan) || "Account" : `${data.accounts.length} ${data.accounts.length === 1 ? "account" : "accounts"}`}</span>
            </div>
          </div>
          <button type="button" className="switcher-btn" onClick={() => setPalette(true)}>
            <Icon name="search" />
            <span>Quick switch</span>
            <kbd className="kbd">Ctrl K</kbd>
          </button>
          {NAV.map((g) => {
            const gid = `nav-${g.group.toLowerCase()}`;
            return (
              <div key={g.group} role="group" aria-labelledby={gid}>
                <div className="nav-group" id={gid}>
                  {g.group}
                </div>
                {g.items.map((item) => (
                  <button key={item.id} type="button" className="nav-item" aria-current={view === item.id ? "page" : undefined} onClick={() => go(item.id)} title={item.label}>
                    <Icon name={item.icon} />
                    <span className="label">{item.label}</span>
                    {item.tag && <Tag>{item.tag}</Tag>}
                    {item.id === "accounts" && data.accounts.length > 0 && (
                      <span className="count">
                        {data.accounts.length}
                        <span className="visually-hidden"> accounts</span>
                      </span>
                    )}
                  </button>
                ))}
              </div>
            );
          })}
          <div className="side-foot">Unofficial. Not affiliated with Anthropic.</div>
        </nav>

        <main className="main" id="main">
          {view === "today" && <TodayView data={data} scope={scope} onGo={go} onAdd={() => setAdding(true)} />}
          {view === "timeline" && <TimelineView data={data} scope={scope} />}
          {view === "costs" && <CostsView data={data} scope={scope} onGo={go} />}
          {view === "sessions" && <SessionsView data={data} scope={scope} />}
          {view === "ecosystem" && <EcosystemView data={data} scope={scope} />}
          {view === "accounts" && <AccountsView data={data} scope={scope} onAdd={() => setAdding(true)} onScope={setScope} />}
          {view === "rules" && <FolderRulesView data={data} />}
          {view === "control" && <ControlView data={data} scope={scope} onScope={setScope} onAdd={() => setAdding(true)} focusField={controlFocus} />}
          {view === "studio" && <StudioView data={data} scope={scope} />}
          {view === "health" && (
            <HealthView
              data={data}
              scope={scope}
              onOpenControl={(id, field) => {
                setScope(id);
                setControlFocus(field);
                setView("control");
              }}
            />
          )}
          {view === "settings" && <SettingsView info={data.info} theme={theme} onTheme={setTheme} accounts={data.accounts} />}
          {view === "bridge" && <BridgeView data={data} scope={scope} />}
        </main>
      </div>
      {active?.tint && <div className="tint-ring" style={accountStyle(active.color)} aria-hidden="true" />}
      {adding && (
        <AccountModal
          existing={data.accounts}
          home={data.info?.home ?? ""}
          onClose={() => setAdding(false)}
          onSaved={(a) => {
            setAdding(false);
            setScope(a.id);
            setView("accounts");
            void data.refresh();
          }}
        />
      )}
      {palette && (
        <CommandPalette
          data={data}
          onClose={() => setPalette(false)}
          onScope={setScope}
          onView={go}
          onAdd={() => setAdding(true)}
          onEdit={setEditingId}
        />
      )}
      {editingAccount && (
        <AccountModal
          existing={data.accounts}
          home={data.info?.home ?? ""}
          account={editingAccount}
          onClose={() => setEditingId(null)}
          onSaved={() => {
            setEditingId(null);
            void data.refresh();
          }}
          onDeleted={() => {
            setEditingId(null);
            void data.refresh();
          }}
        />
      )}
    </>
  );
}
