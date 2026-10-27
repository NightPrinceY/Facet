import { useId, useState } from "react";
import type { DiffLine } from "../lib/control";

/** Diff with a caption, collapsed unchanged runs and keyboard scrolling. */
export function DiffView({ file, lines }: { file: string; lines: DiffLine[] }) {
  const id = useId();
  const [open, setOpen] = useState<Set<number>>(new Set());
  const added = lines.filter((l) => l.kind === "add").length;
  const removed = lines.filter((l) => l.kind === "del").length;
  // Group lines; runs of more than 3 unchanged lines (except next to changes) collapse.
  const blocks: { start: number; lines: DiffLine[]; collapsible: boolean }[] = [];
  let i = 0;
  while (i < lines.length) {
    const kind = lines[i]!.kind === "same" ? "same" : "change";
    let j = i;
    while (j < lines.length && (lines[j]!.kind === "same") === (kind === "same")) j++;
    const run = lines.slice(i, j);
    if (kind === "same" && run.length > 6) {
      blocks.push({ start: i, lines: run.slice(0, 2), collapsible: false });
      blocks.push({ start: i + 2, lines: run.slice(2, -2), collapsible: true });
      blocks.push({ start: j - 2, lines: run.slice(-2), collapsible: false });
    } else blocks.push({ start: i, lines: run, collapsible: false });
    i = j;
  }
  return (
    <figure className="diff-figure">
      <figcaption id={id}>
        {file}: {added} {added === 1 ? "line" : "lines"} added, {removed} removed
      </figcaption>
      <div className="code-preview diff" role="region" aria-labelledby={id} tabIndex={0}>
        {blocks.map((b) =>
          b.collapsible && !open.has(b.start) ? (
            <button key={b.start} type="button" className="diff-fold" onClick={() => setOpen(new Set(open).add(b.start))}>
              Show {b.lines.length} unchanged lines
            </button>
          ) : (
            b.lines.map((l, k) => (
              <div key={`${b.start}-${k}`} className={`diff-line diff-line--${l.kind}`}>
                <span aria-hidden="true">{l.kind === "add" ? "+ " : l.kind === "del" ? "− " : "  "}</span>
                <span className="visually-hidden">{l.kind === "add" ? "added: " : l.kind === "del" ? "removed: " : ""}</span>
                {l.text}
              </div>
            ))
          ),
        )}
      </div>
    </figure>
  );
}

