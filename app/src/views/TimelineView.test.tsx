import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { createSampleApi } from "../lib/sample";
import { DAY, HOUR, MIN_SPAN, detailFor, pan, shortDuration, tasksOf, ticks, zoom, type TimelineEvent } from "../lib/timeline";

afterEach(cleanup);

const ev = (p: Partial<TimelineEvent>): TimelineEvent => ({ accountId: "w", sessionId: "s", ts: 0, kind: "call", endTs: null, model: "claude-opus-5-5", cost: 0, outputTokens: 0, totalTokens: 0, tool: null, ok: null, subagent: false, ...p });

describe("timeline helpers", () => {
  it("splits tasks at prompts and sums their cost", () => {
    const t = tasksOf([ev({ kind: "prompt", ts: 0 }), ev({ ts: 10, cost: 1 }), ev({ kind: "tool", ts: 11, endTs: 50 }), ev({ kind: "prompt", ts: 100 }), ev({ ts: 120, cost: 2 }), ev({ kind: "prompt", ts: 130, subagent: true })]);
    expect(t.map((x) => [x.start, x.end, x.cost, x.calls, x.tools])).toEqual([
      [0, 50, 1, 1, 1],
      [100, 130, 2, 1, 0],
    ]);
  });

  it("zooms around the cursor and clamps", () => {
    const now = 10 * DAY;
    const [s, e] = zoom(0, 1000 * 1000, 0.5, 250_000, now);
    expect([s, e]).toEqual([125_000, 625_000]);
    const [s2, e2] = zoom(0, 1000, 0.001, 500, now);
    expect(e2 - s2).toBe(MIN_SPAN);
    const [, e3] = pan(now - HOUR, now, HOUR, now);
    expect(e3).toBeLessThanOrEqual(now + HOUR * 0.05);
  });

  it("picks detail and ticks by span", () => {
    expect(detailFor(7 * DAY)).toBe("days");
    expect(detailFor(DAY)).toBe("tasks");
    expect(detailFor(HOUR)).toBe("events");
    const t = ticks(0, HOUR, 6);
    expect(t.length).toBeGreaterThan(2);
    expect(shortDuration(2500)).toBe("2.5 s");
    expect(shortDuration(65_000)).toBe("1 m 5 s");
  });
});

describe("Timeline view", () => {
  it("shows lanes, steps through events with the keyboard and inspects them", async () => {
    render(<App api={createSampleApi()} />);
    await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Timeline/ }));
    const plot = await screen.findByRole("group", { name: /^Timeline of Personal and Work/ });
    fireEvent.click(within(screen.getByRole("group", { name: "Zoom presets" })).getByRole("button", { name: "Day" }));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 250));
    });
    fireEvent.keyDown(plot, { key: "ArrowLeft" });
    const inspector = screen.getByRole("heading", { name: "Inspector" }).closest("section")!;
    // At day zoom the lanes draw tasks, so the arrows step through tasks.
    await waitFor(() => expect(within(inspector).getByText("Task")).toBeTruthy());
    expect(within(inspector).getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/)).toBeTruthy();
    const first = within(inspector).getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/).textContent;
    fireEvent.keyDown(plot, { key: "ArrowLeft" });
    await waitFor(() => expect(within(inspector).getByText(/\d{2}:\d{2}:\d{2}\.\d{3}/).textContent).not.toBe(first));
    // Zooming to the task shows its events; stepping then selects an event.
    fireEvent.click(within(inspector).getByRole("button", { name: "Zoom to task" }));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 250));
    });
    fireEvent.keyDown(plot, { key: "ArrowRight" });
    await waitFor(() => expect(within(inspector).getByText("When")).toBeTruthy());
    expect(within(inspector).getByRole("button", { name: /Previous/ })).toBeTruthy();
    expect(screen.getByRole("list", { name: "Most expensive sessions in view" })).toBeTruthy();
  });
});
