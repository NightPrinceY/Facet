import type { ReactNode } from "react";
import type { Account } from "../lib/api";
import { accountStyle } from "./ui";

/** Stat: eyebrow label, value in display size, one line of context. */
export function Stat({ label, value, sub }: { label: string; value: ReactNode; sub?: ReactNode }) {
  return (
    <div className="stat">
      <div className="stat-label">{label}</div>
      <div className="stat-value num">{value}</div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}

export interface StackPart {
  accountId: string;
  value: number;
}

export interface StackColumn {
  key: string;
  /** Short label under the column (day number, hour). */
  tick: string;
  /** Full description for screen readers and the tooltip. */
  label: string;
  parts: StackPart[];
}

/** Columns stacked by account, scaled to the largest column. Each column
 *  carries a text label, so the numbers never live in colour alone. */
export function StackedBars({ columns, accounts, caption, height = "md", tickEvery = 1 }: { columns: StackColumn[]; accounts: Account[]; caption: string; height?: "sm" | "md"; tickEvery?: number }) {
  const max = Math.max(...columns.map((c) => c.parts.reduce((s, p) => s + p.value, 0)), 0);
  const byId = Object.fromEntries(accounts.map((a) => [a.id, a]));
  return (
    <figure className={`bars bars--${height}`} aria-label={caption}>
      <div className="bars-plot">
        {columns.map((c, i) => {
          const total = c.parts.reduce((s, p) => s + p.value, 0);
          return (
            <div key={c.key} className="bars-col" role="img" aria-label={c.label} title={c.label}>
              <div className="bars-stack" style={{ blockSize: max > 0 ? `${(total / max) * 100}%` : "0%" }}>
                {c.parts
                  .filter((p) => p.value > 0)
                  .map((p) => (
                    <span key={p.accountId} className="bars-seg" style={{ ...accountStyle(byId[p.accountId]?.color ?? 1), flexGrow: p.value }} />
                  ))}
              </div>
              <span className="bars-tick" aria-hidden="true">
                {i % tickEvery === 0 ? c.tick : ""}
              </span>
            </div>
          );
        })}
      </div>
    </figure>
  );
}

/** A thin bar scaled to the largest value in view. */
export function ShareBar({ value, max, color }: { value: number; max: number; color?: Account["color"] }) {
  return (
    <span className="share-bar" aria-hidden="true" style={color ? accountStyle(color) : undefined}>
      <span style={{ inlineSize: max > 0 ? `${Math.max((value / max) * 100, value > 0 ? 2 : 0)}%` : "0%" }} />
    </span>
  );
}
