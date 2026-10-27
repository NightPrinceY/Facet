import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { createSampleApi } from "../lib/sample";
import { badgeError } from "./AccountModal";

afterEach(cleanup);

async function renderApp() {
  const api = createSampleApi();
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  return api;
}

describe("Account control", () => {
  it("renames an account in place from its card", async () => {
    const api = await renderApp();
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: "Rename Work" }));
    const input = within(work).getByRole("textbox", { name: "New name for Work" });
    fireEvent.change(input, { target: { value: "Acme Work" } });
    await act(async () => {
      fireEvent.keyDown(input, { key: "Enter" });
    });
    await screen.findByRole("article", { name: "Acme Work" });
    expect((await api.listAccounts()).find((a) => a.id === "work")!.name).toBe("Acme Work");
  });

  it("Enter commits once and focus returns to the rename button", async () => {
    const base = createSampleApi();
    let saves = 0;
    const api = { ...base, saveAccount: async (i: Parameters<typeof base.saveAccount>[0]) => (saves++, base.saveAccount(i)) };
    render(<App api={api} />);
    const work = await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: "Rename Work" }));
    const input = within(work).getByRole("textbox", { name: "New name for Work" });
    fireEvent.change(input, { target: { value: "Acme" } });
    await act(async () => {
      fireEvent.keyDown(input, { key: "Enter" });
      fireEvent.blur(input);
    });
    await screen.findByRole("article", { name: "Acme" });
    expect(saves).toBe(1);
    await waitFor(() => expect((document.activeElement as HTMLElement).getAttribute("aria-label")).toMatch(/^Rename /));
  });

  it("Escape cancels an inline rename", async () => {
    await renderApp();
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: "Rename Work" }));
    const input = within(work).getByRole("textbox", { name: "New name for Work" });
    fireEvent.change(input, { target: { value: "Nope" } });
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.getByRole("article", { name: "Work" })).toBeTruthy();
  });

  it("edits every option, including the short name, badge and notes", async () => {
    const api = await renderApp();
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: "Edit all options" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit Work" });
    fireEvent.change(within(dialog).getByLabelText("Short name"), { target: { value: "acme" } });
    expect(within(dialog).getByText(/rename claude-work to claude-acme/)).toBeTruthy();
    fireEvent.change(within(dialog).getByLabelText("Badge (optional)"), { target: { value: "ACM" } });
    fireEvent.change(within(dialog).getByLabelText("Notes (optional)"), { target: { value: "Invoices to finance@acme" } });
    fireEvent.change(within(dialog).getByLabelText("Terminal starts in (optional)"), { target: { value: "~/work/acme" } });
    expect(within(dialog).getByRole("group", { name: "Menu and dock icon" })).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Save changes" }));
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    const saved = (await api.listAccounts()).find((a) => a.id === "acme")!;
    expect(saved.badge).toBe("ACM");
    expect(saved.notes).toBe("Invoices to finance@acme");
    expect(saved.terminalDir).toBe("/home/you/work/acme");
    expect((await api.listRules())[0]!.accountId).toBe("acme");
    const card = screen.getByRole("article", { name: "Work" });
    expect(within(card).getByText("Invoices to finance@acme")).toBeTruthy();
    expect(within(card).getAllByText(/claude-acme/).length).toBeGreaterThan(0);
  });

  it("refuses a reserved or taken short name", async () => {
    await renderApp();
    fireEvent.click(within(screen.getByRole("article", { name: "Work" })).getByRole("button", { name: "Edit all options" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit Work" });
    const field = within(dialog).getByLabelText("Short name");
    fireEvent.change(field, { target: { value: "code" } });
    expect(within(dialog).getByText(/reserved/)).toBeTruthy();
    fireEvent.change(field, { target: { value: "personal" } });
    expect(within(dialog).getByText(/already exists/)).toBeTruthy();
    expect((within(dialog).getByRole("button", { name: "Save changes" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("warns before pointing an account at another folder", async () => {
    await renderApp();
    fireEvent.click(within(screen.getByRole("article", { name: "Work" })).getByRole("button", { name: "Edit all options" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit Work" });
    expect(within(dialog).queryByText(/different folder/)).toBeNull();
    fireEvent.change(within(dialog).getByLabelText("Claude Code folder"), { target: { value: "~/.claude-acme" } });
    expect(within(dialog).getByText(/different folder/)).toBeTruthy();
  });

  it("reorders accounts", async () => {
    const api = await renderApp();
    const personal = screen.getByRole("article", { name: "Personal" });
    expect((within(personal).getByRole("button", { name: "Move Personal earlier" }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      fireEvent.click(within(personal).getByRole("button", { name: "Move Personal later" }));
    });
    await waitFor(async () => expect((await api.listAccounts()).map((a) => a.id)).toEqual(["work", "personal"]));
    await waitFor(() => expect(screen.getAllByRole("article").map((a) => a.getAttribute("aria-labelledby") && a.querySelector("h2")?.textContent)).toEqual(["Work", "Personal"]));
  });

  it("names a found setup before adding it", async () => {
    const api = await renderApp();
    fireEvent.click(await screen.findByRole("button", { name: "Name and add…" }));
    const dialog = await screen.findByRole("dialog", { name: "Add account" });
    const name = within(dialog).getByLabelText("Name") as HTMLInputElement;
    expect(name.value).toBe("Client Acme");
    fireEvent.change(name, { target: { value: "Acme Corp" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Continue" }));
    expect((within(dialog).getByLabelText("Claude Code folder") as HTMLInputElement).value).toBe("/home/you/.claude-client-acme");
    fireEvent.click(within(dialog).getByRole("button", { name: "Continue" }));
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Add account" }));
    });
    const added = (await api.listAccounts()).find((a) => a.name === "Acme Corp")!;
    expect(added.codeDir).toBe("/home/you/.claude-client-acme");
    expect(added.desktopDir).toBeUndefined();
  });

  it("shows badges in place of the initial", async () => {
    await renderApp();
    const rail = screen.getByRole("toolbar", { name: "Switch account" });
    expect(within(rail).getByRole("button", { name: /^Work/ }).textContent).toContain("W");
  });

  it("badge rules match the core", () => {
    expect(badgeError("AB")).toBeNull();
    expect(badgeError("🚀")).toBeNull();
    expect(badgeError("👩‍💻")).toBeNull();
    expect(badgeError("ABCD")).not.toBeNull();
  });
});
