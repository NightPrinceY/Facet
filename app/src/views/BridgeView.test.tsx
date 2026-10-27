import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import type { Api } from "../lib/api";
import { createSampleApi } from "../lib/sample";
import { clearDrafts } from "./ControlView";

afterEach(() => {
  cleanup();
  clearDrafts();
});

async function openBridge(api: Api = createSampleApi()) {
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Bridge/ }));
  await screen.findByRole("heading", { name: "Bridge", level: 1 });
  return api;
}

describe("Bridge", () => {
  it("writes, edits, saves and opens a handoff in the other account", async () => {
    const base = createSampleApi();
    const opened: string[] = [];
    const api: Api = { ...base, bridgeOpen: async (to, name) => (opened.push(`${to}:${name}`), 1) };
    await openBridge(api);
    await waitFor(() => expect((screen.getByLabelText("Session") as HTMLSelectElement).options.length).toBeGreaterThan(0));
    expect((screen.getByLabelText("To account") as HTMLSelectElement).value).toBe("work");
    fireEvent.change(screen.getByLabelText("Note for the other account"), { target: { value: "Finish the billing page." } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Write handoff" }));
    });
    const text = (await screen.findByLabelText(/Handoff \(you can edit it/)) as HTMLTextAreaElement;
    expect(text.value).toContain("# Handoff from Personal to Work");
    expect(text.value).toContain("Finish the billing page.");
    fireEvent.change(text, { target: { value: text.value + "\nAlso check the tests.\n" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save handoff" }));
    });
    const savedList = await screen.findByRole("list", { name: "Saved handoffs" });
    expect(within(savedList).getByText("Handoff from Personal to Work")).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getAllByRole("button", { name: "Open in Work" })[0]!);
    });
    expect(opened[0]).toMatch(/^work:.*-personal-to-work\.md$/);
    expect(screen.getByText(/Work is on a work plan/)).toBeTruthy();
    // Deleting asks first.
    fireEvent.click(within(savedList).getByRole("button", { name: /^Delete Handoff from Personal to Work/ }));
    await act(async () => {
      fireEvent.click(within(savedList).getByRole("button", { name: "Delete handoff" }));
    });
    expect(await screen.findByText("No handoffs yet.")).toBeTruthy();
  });

  it("refuses to save a handoff holding a secret", async () => {
    await openBridge();
    await waitFor(() => expect((screen.getByLabelText("Session") as HTMLSelectElement).options.length).toBeGreaterThan(0));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Write handoff" }));
    });
    const text = (await screen.findByLabelText(/Handoff \(you can edit it/)) as HTMLTextAreaElement;
    fireEvent.change(text, { target: { value: text.value + "\ntoken ghp_abcdefghijklmnopqrstuvwxyz0123\n" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save handoff" }));
    });
    expect(await screen.findByText(/looks like it holds a secret/)).toBeTruthy();
  });

  it("sets incoming messages per account through a reviewed change", async () => {
    const api = await openBridge();
    const list = await screen.findByRole("list", { name: "Incoming messages per account" });
    const work = within(list).getByText("Work").closest("li")!;
    const group = await within(work).findByRole("group", { name: "Incoming messages for Work" });
    await act(async () => {
      fireEvent.click(within(group).getByRole("button", { name: "Refuse" }));
    });
    const dialog = await screen.findByRole("dialog", { name: "Refuses messages: Work" });
    expect(within(dialog).getByRole("region", { name: /settings\.json/ }).textContent).toContain("crossSessionInbound");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Set to Refuse" }));
    });
    await waitFor(async () => expect((await api.controlRead("work")).current.settings?.crossSessionInbound).toBe("refuse"));
  });
});
