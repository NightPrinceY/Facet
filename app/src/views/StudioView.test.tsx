import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { createSampleApi } from "../lib/sample";
import { clearDrafts } from "./ControlView";
import { clearStudioDrafts } from "./StudioView";

afterEach(() => {
  cleanup();
  clearDrafts();
  clearStudioDrafts();
});

async function openStudio() {
  render(<App api={createSampleApi()} />);
  await screen.findByRole("article", { name: "Work" });
  fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Studio/ }));
  return screen.findByRole("list", { name: "Library" });
}

async function clickWhenEnabled(name: string) {
  const btn = (await screen.findByRole("button", { name })) as HTMLButtonElement;
  await waitFor(() => expect(btn.disabled).toBe(false));
  await act(async () => {
    fireEvent.click(btn);
  });
}

async function openItem(lib: HTMLElement, name: string) {
  fireEvent.click(within(lib).getByRole("button", { name: `Open ${name}` }));
  const editor = (await screen.findByLabelText("File")) as HTMLTextAreaElement;
  await waitFor(() => expect(editor.value).toContain(`name: ${name}`));
  return editor;
}

describe("Studio", () => {
  it("lists the library with placement state in words and pictures", async () => {
    const lib = await openStudio();
    const row = within(lib).getByText("review-pr").closest("li")!;
    expect(within(row).getByRole("img", { name: "Work: different from the library" })).toBeTruthy();
    expect(within(row).getByRole("img", { name: "Personal: not placed" })).toBeTruthy();
    expect(within(row).getByText("Different: Work")).toBeTruthy();
  });

  it("opens an item, lints it live, saves, and places it with a reviewed diff", async () => {
    const lib = await openStudio();
    const editor = await openItem(lib, "review-pr");
    expect(await screen.findByRole("meter", { name: /Description length/ })).toBeTruthy();
    fireEvent.change(editor, { target: { value: editor.value.replace("1. Read the diff.", "1. Read the whole diff.") } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save to library" }));
    });
    await waitFor(() => expect(screen.queryByText("Not saved")).toBeNull());
    await clickWhenEnabled("Review and place review-pr in Personal");
    const dialog = await screen.findByRole("dialog", { name: "Place review-pr in Personal" });
    expect(dialog.closest("[inert]")).toBeNull();
    const diff = within(dialog).getByRole("region", { name: /skills\/review-pr\/SKILL\.md: \d+ lines added/ });
    expect(diff.textContent).toContain("Read the whole diff");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Place in Personal" }));
    });
    const accounts = screen.getByRole("list", { name: "Accounts" });
    await waitFor(() => expect(within(accounts).getByText("Placed, same as the library")).toBeTruthy());
  });

  it("updating a changed copy warns about running sessions and backs it up", async () => {
    const lib = await openStudio();
    await openItem(lib, "review-pr");
    await clickWhenEnabled("Review update of review-pr in Work");
    const dialog = await screen.findByRole("dialog", { name: "Update review-pr in Work" });
    expect(within(dialog).getByText(/backs it up before replacing it/)).toBeTruthy();
    expect(within(dialog).getByText(/2 Claude Code sessions of Work are running/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Update in Work" }));
    });
    await waitFor(() => expect(document.querySelector(".toast")?.textContent).toMatch(/old copy is backed up/));
  });

  it("a risky item needs an explicit acknowledgement before placing", async () => {
    const lib = await openStudio();
    const editor = await openItem(lib, "review-pr");
    fireEvent.change(editor, { target: { value: editor.value + "\nNow: !`git status`\n" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save to library" }));
    });
    await waitFor(() => expect(screen.queryByText("Not saved")).toBeNull());
    await clickWhenEnabled("Review and place review-pr in Personal");
    const dialog = await screen.findByRole("dialog", { name: "Place review-pr in Personal" });
    expect(within(dialog).getByText("Runs commands")).toBeTruthy();
    const place = within(dialog).getByRole("button", { name: "Place in Personal" }) as HTMLButtonElement;
    expect(place.disabled).toBe(true);
    fireEvent.click(within(dialog).getByRole("checkbox", { name: /I understand this runs commands/ }));
    expect(place.disabled).toBe(false);
  });

  it("closing with unsaved edits asks first", async () => {
    const lib = await openStudio();
    const editor = await openItem(lib, "review-pr");
    fireEvent.change(editor, { target: { value: editor.value + "\nmore\n" } });
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.getByText(/Your changes to review-pr are not saved/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Keep editing" }));
    expect(screen.getByLabelText("File")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    const ask = screen.getByText(/not saved. Discard them/).closest(".callout") as HTMLElement;
    fireEvent.click(within(ask).getByRole("button", { name: "Discard changes" }));
    await waitFor(() => expect(screen.queryByLabelText("File")).toBeNull());
  });

  it("creates a new skill from the form and shows the exact file first", async () => {
    await openStudio();
    fireEvent.click(screen.getAllByRole("button", { name: "New item" })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "New skill" });
    expect(document.activeElement).toBe(within(dialog).getByLabelText("Name"));
    fireEvent.change(within(dialog).getByLabelText("Name"), { target: { value: "Ship-It" } });
    expect((within(dialog).getByLabelText("Name") as HTMLInputElement).value).toBe("ship-it");
    fireEvent.change(within(dialog).getByLabelText("Description (required)"), { target: { value: "Prepare a release: changelog, version bump and tag. Use when the user says ship it." } });
    await waitFor(() => expect(within(dialog).getByRole("region", { name: "The file Facet will save" }).textContent).toContain("name: ship-it"));
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Save to library" }));
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    const editor = (await screen.findByLabelText("File")) as HTMLTextAreaElement;
    await waitFor(() => expect(editor.value).toContain("name: ship-it"));
  });

  it("refuses to save a secret into the library", async () => {
    const lib = await openStudio();
    const editor = await openItem(lib, "review-pr");
    fireEvent.change(editor, { target: { value: editor.value + "\nToken: ghp_abc\n" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save to library" }));
    });
    expect(await screen.findByText(/never stores secrets/)).toBeTruthy();
  });

  it("imports from the account chosen in the rail", async () => {
    render(<App api={createSampleApi()} />);
    await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(screen.getByRole("toolbar", { name: "Switch account" })).getByRole("button", { name: /^Personal/ }));
    fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Studio/ }));
    await screen.findByRole("list", { name: "Library" });
    const section = screen.getByRole("heading", { name: "Already in your accounts" }).closest("section")!;
    await waitFor(() => expect(section.textContent).toContain("standup"), { timeout: 2000 }).catch(() => {
      throw new Error("import section: " + section.textContent);
    });
    await act(async () => {
      fireEvent.click(within(section).getByRole("button", { name: "Copy standup from Personal to the library" }));
    });
    const lib = await screen.findByRole("list", { name: "Library" });
    await waitFor(() => expect(within(lib).getByText("standup")).toBeTruthy());
  });
});
