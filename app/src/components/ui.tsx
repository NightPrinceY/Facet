// Core components from DESIGN-SYSTEM.md §4. Each keeps the states the spec
// lists (hover, focus-visible, disabled, loading) and its ARIA role.

import {
  useEffect,
  useId,
  useRef,
  type ButtonHTMLAttributes,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
  type Ref,
} from "react";
import type { Account, FacetColor } from "../lib/api";
import { initial } from "../lib/format";
import { createPortal } from "react-dom";
import { Icon, type IconName } from "./Icon";

/** CSS variables that carry an account's identity colour to any component. */
export function accountStyle(color: FacetColor): CSSProperties {
  return {
    "--account-color": `var(--facet-${color})`,
    "--on-account-color": `var(--on-facet-${color})`,
  } as CSSProperties;
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  icon?: IconName;
  /** Spinner replaces the icon; the label stays and the button keeps focus. */
  loading?: boolean;
  ref?: Ref<HTMLButtonElement>;
};

export function Button({ variant = "secondary", size = "md", icon, loading, disabled, children, className, onClick, ...rest }: ButtonProps) {
  const cls = ["btn", variant !== "secondary" && `btn--${variant}`, size === "sm" && "btn--sm", className]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      type="button"
      className={cls}
      disabled={disabled}
      aria-busy={loading || undefined}
      aria-disabled={loading || undefined}
      onClick={loading ? undefined : onClick}
      {...rest}
    >
      {loading ? <span className="spinner" aria-hidden="true" /> : icon ? <Icon name={icon} /> : null}
      {children}
    </button>
  );
}

export function Pill({ tone = "neutral", dot, children }: { tone?: "neutral" | "ok" | "warn" | "bad" | "example"; dot?: boolean; children: ReactNode }) {
  return (
    <span className={tone === "neutral" ? "pill" : `pill pill--${tone}`}>
      {dot && <span className="dot" aria-hidden="true" />}
      {children}
    </span>
  );
}

export function Tag({ children }: { children: ReactNode }) {
  return <span className="tag">{children}</span>;
}

/** Avatar props from an account: picture, else badge, else initial. */
export function avatarOf(a: Pick<Account, "name" | "color" | "badge" | "imageUrl">): { name: string; color: FacetColor; badge?: string; image?: string | null } {
  return { name: a.name, color: a.color, badge: a.badge, image: a.imageUrl };
}

/** User-perceived characters (an emoji with modifiers counts once). */
export function graphemes(s: string): number {
  const Seg = (Intl as unknown as { Segmenter?: new (l?: string, o?: { granularity: string }) => { segment(t: string): Iterable<unknown> } }).Segmenter;
  return Seg ? [...new Seg(undefined, { granularity: "grapheme" }).segment(s)].length : [...s].length;
}

export function Avatar({ name, color, size = "sm", running, label, badge, image }: { name: string; color: FacetColor; size?: "sm" | "lg" | "xl"; running?: boolean; label?: string; badge?: string; image?: string | null }) {
  const mark = badge?.trim() || initial(name);
  return (
    <span
      className={`avatar${size === "sm" ? "" : ` avatar--${size}`}${image ? " avatar--image" : ""}${graphemes(mark) > 1 && !image ? " avatar--long" : ""}`}
      style={accountStyle(color)}
      role={label ? "img" : undefined}
      aria-label={label ? `${label}${running ? ", Desktop app running" : ""}` : undefined}
      aria-hidden={label ? undefined : true}
    >
      {image ? <img src={image} alt="" draggable={false} /> : mark}
      {running && <span className="running-dot" />}
    </span>
  );
}

export function Callout({ tone = "neutral", icon, children, action }: { tone?: "neutral" | "warn" | "bad"; icon?: IconName; children: ReactNode; action?: ReactNode }) {
  return (
    <div className={tone === "neutral" ? "callout" : `callout callout--${tone}`} role={tone === "bad" ? "alert" : undefined}>
      {icon && <Icon name={icon} />}
      <div className="callout-body">{children}</div>
      {action}
    </div>
  );
}

/** Switch for settings that apply at once (DESIGN-SYSTEM.md Toggle). */
export function Toggle({ checked, onChange, label, hint, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; hint?: string; disabled?: boolean }) {
  const id = useId();
  return (
    <div className="toggle-row">
      <div>
        <div className="label" id={`${id}-l`}>
          {label}
        </div>
        {hint && (
          <div className="hint" id={`${id}-h`}>
            {hint}
          </div>
        )}
      </div>
      <button
        type="button"
        className="toggle"
        role="switch"
        aria-checked={checked}
        aria-labelledby={`${id}-l`}
        aria-describedby={hint ? `${id}-h` : undefined}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      />
    </div>
  );
}

/** Checkbox for choices inside a form that is saved later. */
export function Checkbox({ checked, onChange, label, hint, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; hint?: string; disabled?: boolean }) {
  const id = useId();
  return (
    <div className="checkbox">
      <input id={id} type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} aria-describedby={hint ? `${id}-h` : undefined} />
      <label className="label" htmlFor={id}>
        {label}
      </label>
      {hint && (
        <div className="hint" id={`${id}-h`}>
          {hint}
        </div>
      )}
    </div>
  );
}

export function Segmented<T extends string>({ value, options, onChange, label }: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string }) {
  return (
    <div className="segmented" role="group" aria-label={label}>
      {options.map((o) => (
        <button key={o.value} type="button" aria-pressed={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** Ids an input inside `Field` should list in `aria-describedby`. */
export function describedBy(htmlFor: string, error?: string | null, hint?: string): string | undefined {
  if (error) return `${htmlFor}-err`;
  return hint ? `${htmlFor}-hint` : undefined;
}

export function Field({ label, hint, error, children, htmlFor }: { label: string; hint?: string; error?: string | null; children: ReactNode; htmlFor: string }) {
  return (
    <div className="field">
      <label htmlFor={htmlFor}>{label}</label>
      {children}
      {error ? (
        <div className="error" id={`${htmlFor}-err`}>
          <Icon name="alert" size={14} />
          {error}
        </div>
      ) : (
        hint && (
          <div className="hint" id={`${htmlFor}-hint`}>
            {hint}
          </div>
        )
      )}
    </div>
  );
}

const FOCUSABLE = "input:not(:disabled), button:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])";

/** Dialog: focus moves in once and returns on close; Tab stays inside; the
 *  rest of the app is inert; Esc closes unless `dismissable` is false. */
export function Modal({ title, onClose, children, footer, dismissable = true, header, identity }: { title: string; onClose: () => void; children: ReactNode; footer: ReactNode; dismissable?: boolean; header?: ReactNode; identity?: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const id = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const dismissRef = useRef(dismissable);
  dismissRef.current = dismissable;

  // Runs once: focus in, make the app inert, restore both on close.
  useEffect(() => {
    const prev = document.activeElement as HTMLElement | null;
    ref.current?.querySelector<HTMLElement>(FOCUSABLE)?.focus();
    const shell = document.querySelector<HTMLElement>(".shell");
    shell?.setAttribute("inert", "");
    return () => {
      shell?.removeAttribute("inert");
      prev?.focus();
    };
  }, []);

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      const el = ref.current;
      if (!el) return;
      if (e.key === "Escape" && dismissRef.current) {
        e.preventDefault();
        closeRef.current();
      }
      if (e.key === "Tab") {
        const items = [...el.querySelectorAll<HTMLElement>(FOCUSABLE)];
        if (items.length === 0) return;
        const first = items[0]!;
        const last = items[items.length - 1]!;
        if (!el.contains(document.activeElement)) {
          e.preventDefault();
          first.focus();
        } else if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  // Rendered on <body>, outside the app shell that is made inert while a
  // dialog is open (a dialog inside an inert region could not be used).
  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && dismissRef.current && closeRef.current()}>
      <div className="modal" role="dialog" aria-modal="true" aria-labelledby={id} ref={ref}>
        <div className="modal-head">
          <div className="modal-title">
            {identity}
            <h2 id={id}>{title}</h2>
          </div>
          {header}
        </div>
        <div className="modal-body">{children}</div>
        <div className="modal-foot">{footer}</div>
      </div>
    </div>,
    document.body,
  );
}

export function Stepper({ steps, current }: { steps: string[]; current: number }) {
  return (
    <ol className="stepper" aria-label="Steps">
      {steps.map((s, i) => (
        <li key={s} className={i < current ? "done" : undefined} aria-current={i === current ? "step" : undefined}>
          <span className="n" aria-hidden="true">
            {i < current ? "✓" : i + 1}
          </span>
          {s}
          {i < current && <span className="visually-hidden"> (done)</span>}
        </li>
      ))}
    </ol>
  );
}

/** Arrow-key movement for a group of buttons (roving tabindex). */
export function rovingKeys(e: KeyboardEvent<HTMLElement>, selector: string, orientation: "vertical" | "horizontal" | "both" = "both") {
  const group = e.currentTarget;
  const items = [...group.querySelectorAll<HTMLElement>(selector)];
  const i = items.indexOf(document.activeElement as HTMLElement);
  if (i < 0) return;
  const rtl = getComputedStyle(group).direction === "rtl";
  const next = (rtl ? "ArrowLeft" : "ArrowRight");
  const prev = (rtl ? "ArrowRight" : "ArrowLeft");
  let target = -1;
  if ((e.key === "ArrowDown" && orientation !== "horizontal") || (e.key === next && orientation !== "vertical")) target = (i + 1) % items.length;
  if ((e.key === "ArrowUp" && orientation !== "horizontal") || (e.key === prev && orientation !== "vertical")) target = (i - 1 + items.length) % items.length;
  if (e.key === "Home") target = 0;
  if (e.key === "End") target = items.length - 1;
  if (target >= 0) {
    e.preventDefault();
    items[target]!.focus();
  }
}

export function Skeleton({ card }: { card?: boolean }) {
  return <div className={card ? "skeleton skeleton--card" : "skeleton"} aria-hidden="true" />;
}

/** Meter (DESIGN-SYSTEM.md): label, value and a bar in the account colour
 *  (or neutral ink when no account applies); amber from `warnAt` of the
 *  limit, red above it, and the overage is also said in words. */
export function Meter({ label, value, max, unit, warnAt = 0.8, color }: { label: string; value: number; max: number; unit?: string; warnAt?: number; color?: FacetColor }) {
  const id = useId();
  const over = value > max;
  const near = !over && value >= max * warnAt;
  const pct = Math.min(100, Math.round((value / max) * 100));
  const text = `${value.toLocaleString()} of ${max.toLocaleString()}${unit ? ` ${unit}` : ""}${over ? `, ${(value - max).toLocaleString()} over` : ""}`;
  return (
    <div className="meter" style={color ? accountStyle(color) : undefined}>
      <div className="meter-head">
        <span id={`${id}-l`}>{label}</span>
        <span className={over ? "num meter-over" : "num"}>
          {value.toLocaleString()} / {max.toLocaleString()}
          {unit ? ` ${unit}` : ""}
          {over && ` · ${(value - max).toLocaleString()} over`}
        </span>
      </div>
      <div
        className="meter-track"
        role="meter"
        aria-labelledby={`${id}-l`}
        aria-valuemin={0}
        aria-valuemax={max}
        aria-valuenow={Math.min(value, max)}
        aria-valuetext={text}
      >
        <div className={over ? "meter-bar meter-bar--over" : near ? "meter-bar meter-bar--near" : color ? "meter-bar meter-bar--account" : "meter-bar"} style={{ inlineSize: `${pct}%` }} />
      </div>
    </div>
  );
}
