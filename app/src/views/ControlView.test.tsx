import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import type { Api } from "../lib/api";
import { createSampleApi } from "../lib/sample";
import { HIDDEN_MARKER, diffLines, draftFrom, opsFromDraft } from "../lib/control";
import { backupLabel, clearDrafts } from "./ControlView";

afterEach(() => {
  cleanup();
  clearDrafts();
});

describe("control helpers (mirror control.rs)", () => {
  it("turns a draft into per-key edits for changed fields only", () => {
    const base = draftFrom({ model: "opus", permissions: { allow: ["Bash(git status)"] }, env: { A: "1", SECRET_TOKEN: HIDDEN_MARKER }, custom: 1 });
    expect(opsFromDraft(base, base)).toEqual([]);
    const next = {
      ...base,
      model: "",
      cleanupPeriodDays: "365",
      allow: "Bash(git status)\n\nRead(./docs/**)",
      disableAllHooks: "off" as const,
      env: [...base.env.filter((r) => r.key !== "A"), { id: 99, key: "B", value: "2", hidden: false }],
    };
    expect(opsFromDraft(base, next)).toEqual([
      { path: ["model"], value: null },
      { path: ["cleanupPeriodDays"], value: 365 },
      { path: ["permissions", "allow"], value: ["Bash(git status)", "Read(./docs/**)"] },
      { path: ["disableAllHooks"], value: false },
      { path: ["env", "A"], value: null },
      { path: ["env", "B"], value: "2" },
    ]);
  });

  it("hidden values are never sent back", () => {
    const base = draftFrom({ env: { SECRET_TOKEN: HIDDEN_MARKER } });
    expect(base.env[0]!.hidden).toBe(true);
    const removed = { ...base, env: [] };
    expect(opsFromDraft(base, removed)).toEqual([{ path: ["env", "SECRET_TOKEN"], value: null }]);
  });

  it("diff order matches the Rust core", () => {
    const d = diffLines('{\n  "a": 1,\n  "b": 2\n}\n', '{\n  "a": 1,\n  "b": 3\n}\n');
    expect(d.map((l) => l.kind)).toEqual(["same", "same", "add", "del", "same"]);
  });

  it("backup labels", () => {
    expect(backupLabel("20261026T031500Z-123-settings.json")).toBe("26 Oct 2026, 03:15 UTC");
    expect(backupLabel("20261026T031500Z-123-before-restore-settings.json")).toBe("26 Oct 2026, 03:15 UTC · before a restore");
  });
});

async function openControlFor(name: string, api: Api = createSampleApi()) {
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  const rail = screen.getByRole("toolbar", { name: "Switch account" });
  fireEvent.click(within(rail).getByRole("button", { name: new RegExp(`^${name}`) }));
  fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Control/ }));
  await screen.findByLabelText("Model");
  return api;
}

describe("Control view", () => {
  it("reviews a change as a diff, saves with a backup, and restores after its own review", async () => {
    await openControlFor("Personal");
    expect((screen.getByLabelText("Model") as HTMLInputElement).value).toBe("opus");
    fireEvent.change(screen.getByLabelText(/Keep transcripts for/), { target: { value: "365" } });
    expect(screen.getByText("1 change not saved for Personal")).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Review and save" }));
    });
    const dialog = await screen.findByRole("dialog", { name: "Review changes for Personal" });
    expect(dialog.closest("[inert]")).toBeNull();
    const diff = within(dialog).getByRole("region", { name: /lines added/ });
    expect(diff.textContent).toContain('"cleanupPeriodDays": 365');
    expect(diff.textContent).not.toContain("sample-not-a-real-token");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Save with backup" }));
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect((screen.getByLabelText(/Keep transcripts for/) as HTMLInputElement).value).toBe("365");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Restore this backup" }));
    });
    const restore = await screen.findByRole("dialog", { name: "Restore backup for Personal" });
    await act(async () => {
      fireEvent.click(within(restore).getByRole("button", { name: "Restore this backup" }));
    });
    await waitFor(() => expect((screen.getByLabelText(/Keep transcripts for/) as HTMLInputElement).value).toBe(""));
  });

  it("shows secret-looking values as hidden and read-only", async () => {
    await openControlFor("Personal");
    const value = screen.getByLabelText("Value of GITHUB_TOKEN") as HTMLInputElement;
    expect(value.value).toBe(HIDDEN_MARKER);
    expect(value.readOnly).toBe(true);
    expect(screen.queryByDisplayValue("sample-not-a-real-token")).toBeNull();
  });

  it("saves exactly the edits the diff showed", async () => {
    const base = createSampleApi();
    let applied: unknown = null;
    const api: Api = { ...base, controlApply: async (id, ops, h) => ((applied = ops), base.controlApply(id, ops, h)) };
    await openControlFor("Work", api);
    fireEvent.change(screen.getByLabelText("Model"), { target: { value: "sonnet" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Review and save" }));
    });
    const dialog = await screen.findByRole("dialog");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Save with backup" }));
    });
    expect(applied).toEqual([{ path: ["model"], value: "sonnet" }]);
  });

  it("refuses variables that would replace the sign-in, case-insensitively", async () => {
    await openControlFor("Work");
    fireEvent.click(screen.getByRole("button", { name: "Add variable" }));
    const name = screen.getByLabelText("Name of variable 1");
    await waitFor(() => expect(document.activeElement).toBe(name));
    fireEvent.change(name, { target: { value: "anthropic_api_key" } });
    expect(screen.getByText(/does not set anthropic_api_key/i)).toBeTruthy();
    expect(name.getAttribute("aria-invalid")).toBe("true");
    expect((screen.getByRole("button", { name: "Review and save" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Fix the highlighted environment rows to continue.")).toBeTruthy();
  });

  it("warns inline and in the review before bypassPermissions", async () => {
    await openControlFor("Work");
    fireEvent.change(screen.getByLabelText("When Claude Code wants to act"), { target: { value: "bypassPermissions" } });
    expect(screen.getByText(/throwaway sandbox/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Review and save" }));
    });
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/without asking/)).toBeTruthy();
  });

  it("keeps unsaved edits when switching accounts and back", async () => {
    await openControlFor("Work");
    fireEvent.change(screen.getByLabelText("Model"), { target: { value: "haiku" } });
    const rail = screen.getByRole("toolbar", { name: "Switch account" });
    fireEvent.click(within(rail).getByRole("button", { name: /^Personal/ }));
    await waitFor(() => expect((screen.getByLabelText("Model") as HTMLInputElement).value).toBe("opus"));
    fireEvent.click(within(rail).getByRole("button", { name: /^Work/ }));
    await waitFor(() => expect((screen.getByLabelText("Model") as HTMLInputElement).value).toBe("haiku"));
    expect(screen.getByText(/unsaved changes for Work from earlier are back/)).toBeTruthy();
  });

  it("Health links the retention warning to the field in Control", async () => {
    render(<App api={createSampleApi()} />);
    await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Health/ }));
    fireEvent.click(await screen.findByRole("button", { name: /Set transcript retention for Personal/ }));
    const field = await screen.findByLabelText(/Keep transcripts for/);
    await waitFor(() => expect(document.activeElement).toBe(field));
  });

  it("dialogs opened from inside the main view are not inert", async () => {
    render(<App api={createSampleApi()} />);
    const work = await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: /Edit/ }));
    const dialog = await screen.findByRole("dialog", { name: "Edit Work" });
    expect(dialog.closest("[inert]")).toBeNull();
    expect(document.querySelector(".shell")?.hasAttribute("inert")).toBe(true);
  });
});

describe("What applies", () => {
  it("shows each setting with the file it comes from, and combines permission lists", async () => {
    await openControlFor("Personal");
    const section = screen.getByRole("heading", { name: "What applies" }).closest("section")!;
    await waitFor(() => expect(within(section).getByText("model")).toBeTruthy());
    fireEvent.change(within(section).getByLabelText(/Project folder/), { target: { value: "/home/you/work/app" } });
    await act(async () => {
      fireEvent.click(within(section).getByRole("button", { name: "Show" }));
    });
    const row = (await within(section).findByText("permissions.allow")).closest("tr")!;
    expect(row.textContent).toContain("Bash(npm test)");
    expect(row.textContent).toContain("Bash(git status)");
    expect(row.textContent).toContain("Project + Account");
    expect(within(section).queryByText("sample-not-a-real-token")).toBeNull();
  });
});

describe("Copy from another account", () => {
  it("fills the form with the other account's settings, never hidden values", async () => {
    await openControlFor("Work");
    fireEvent.click(screen.getByText("Copy settings from another account"));
    fireEvent.change(screen.getByLabelText("Copy from"), { target: { value: "personal" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy into this form" }));
    });
    await waitFor(() => expect((screen.getByLabelText("Model") as HTMLInputElement).value).toBe("opus"));
    expect(screen.queryByLabelText("Value of GITHUB_TOKEN")).toBeNull();
    expect((screen.getByLabelText("Value of OTEL_METRICS_EXPORTER") as HTMLInputElement).value).toBe("otlp");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Review and save" }));
    });
    const dialog = await screen.findByRole("dialog", { name: "Review changes for Work" });
    expect(within(dialog).getByRole("region", { name: /lines added/ }).textContent).toContain('"model": "opus"');
  });
});
