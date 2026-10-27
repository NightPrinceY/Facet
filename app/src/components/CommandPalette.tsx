// Quick switcher (PRD M10, DESIGN-SYSTEM.md Command palette): Ctrl/⌘ K or
// `facet --switcher`. Filters accounts and actions; arrows + Enter; Esc closes.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { errorText } from "../lib/api";
import { useApi, type FacetData } from "../lib/useFacet";
import { useToast } from "./Toast";
import { Avatar, avatarOf } from "./ui";
import { Icon } from "./Icon";
import { NAV, type ViewId } from "../lib/nav";

interface Item {
  id: string;
  label: string;
  hint?: string;
  icon: ReactNode;
  run: () => void | Promise<void>;
}

export function CommandPalette({ data, onClose, onScope, onView, onAdd, onEdit }: { data: FacetData; onClose: () => void; onScope: (id: string | null) => void; onView: (v: ViewId) => void; onAdd: () => void; onEdit: (id: string) => void }) {
  const api = useApi();
  const toast = useToast();
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const listId = "palette-list";

  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    input.current?.focus();
    return () => prev?.focus();
  }, []);

  const items = useMemo<Item[]>(() => {
    const out: Item[] = [];
    for (const a of data.accounts) {
      const icon = <Avatar {...avatarOf(a)} />;
      const running = data.states[a.id]?.desktopPid != null;
      out.push({ id: `scope:${a.id}`, label: `Switch to ${a.name}`, hint: "Scope Facet", icon, run: () => onScope(a.id) });
      if (a.desktopDir) {
        out.push({
          id: `launch:${a.id}`,
          label: running ? `Bring ${a.name} to front` : `Launch ${a.name}`,
          hint: "Desktop app",
          icon,
          run: async () => {
            await api.launchDesktop(a.id);
            toast(running ? `Brought ${a.name} to front.` : `Started ${a.name}'s Desktop app.`);
          },
        });
      }
      out.push({
        id: `term:${a.id}`,
        label: `Open terminal as ${a.name}`,
        hint: "Claude Code",
        icon,
        run: async () => {
          await api.openTerminal(a.id);
          toast(`Opened a terminal as ${a.name}.`);
        },
      });
      out.push({ id: `edit:${a.id}`, label: `Edit ${a.name}`, hint: "Name, picture, folders, launching", icon, run: () => onEdit(a.id) });
    }
    out.push({ id: "scope:all", label: "Show all accounts", icon: <Icon name="grid" />, run: () => onScope(null) });
    out.push({ id: "add", label: "Add account", icon: <Icon name="plus" />, run: onAdd });
    for (const g of NAV)
      for (const v of g.items) out.push({ id: `view:${v.id}`, label: `Go to ${v.label}`, icon: <Icon name={v.icon} />, run: () => onView(v.id) });
    return out;
  }, [data.accounts, data.states, api, toast, onScope, onView, onAdd, onEdit]);

  const shown = useMemo(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return items.filter((it) => words.every((w) => `${it.label} ${it.hint ?? ""}`.toLowerCase().includes(w)));
  }, [items, query]);

  useEffect(() => setIndex(0), [query]);

  const choose = async (it: Item | undefined) => {
    if (!it) return;
    onClose();
    try {
      await it.run();
    } catch (e) {
      toast(errorText(e));
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndex((i) => Math.min(i + 1, shown.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      void choose(shown[index]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Tab") {
      e.preventDefault();
    }
  };

  const activeId = shown[index] ? `pal-${shown[index]!.id}` : undefined;

  return (
    <div className="overlay palette-overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="palette" role="dialog" aria-modal="true" aria-label="Switch or run">
        <input
          ref={input}
          className="input"
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-activedescendant={activeId}
          aria-label="Search accounts and actions"
          placeholder="Search accounts and actions"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKey}
        />
        <ul id={listId} role="listbox" aria-label="Results">
          {shown.length === 0 && <li className="empty-row">Nothing matches “{query}”.</li>}
          {shown.map((it, i) => (
            <li key={it.id} id={`pal-${it.id}`} role="option" aria-selected={i === index} onMouseEnter={() => setIndex(i)} onMouseDown={(e) => e.preventDefault()} onClick={() => void choose(it)}>
              {it.icon}
              <span>{it.label}</span>
              {it.hint && <span className="hint">{it.hint}</span>}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
