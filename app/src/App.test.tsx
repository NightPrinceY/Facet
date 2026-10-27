import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import type { Api } from "./lib/api";
import { createSampleApi } from "./lib/sample";
import { matchFolder } from "./lib/rules";

afterEach(cleanup);

async function renderApp(api: Api = createSampleApi()) {
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  return api;
}

const nav = () => screen.getByRole("navigation", { name: "Sections" });

describe("Facet app (sample backend)", () => {
  it("labels sample data and shows both accounts with their folders", async () => {
    await renderApp();
    expect(screen.getByRole("note").textContent).toMatch(/sample data/i);
    const work = screen.getByRole("article", { name: "Work" });
    expect(within(work).getByText("~/.claude-work")).toBeTruthy();
    expect(within(work).getByText("Desktop closed")).toBeTruthy();
    expect(within(screen.getByRole("article", { name: "Personal" })).getByText("Desktop running")).toBeTruthy();
    expect(screen.getByText("Unofficial. Not affiliated with Anthropic.")).toBeTruthy();
  });

  it("launches an account and shows it running", async () => {
    await renderApp();
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: /Launch/ }));
    await waitFor(() => expect(within(work).getByText("Desktop running")).toBeTruthy());
    expect(within(work).getByRole("button", { name: /Quit/ })).toBeTruthy();
  });

  it("shows a failed action inside the card with a retry", async () => {
    const base = createSampleApi();
    let fail = true;
    const api: Api = {
      ...base,
      launchDesktop: async (id) => {
        if (fail) throw "Claude Desktop is not installed.";
        return base.launchDesktop(id);
      },
    };
    await renderApp(api);
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: /Launch/ }));
    const alert = await within(work).findByRole("alert");
    expect(alert.textContent).toContain("Claude Desktop is not installed.");
    fail = false;
    fireEvent.click(within(alert).getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(within(work).getByText("Desktop running")).toBeTruthy());
    expect(within(work).queryByRole("alert")).toBeNull();
  });

  it("scopes to an account from the rail, tints the window and names it in the title", async () => {
    await renderApp();
    const rail = screen.getByRole("toolbar", { name: "Switch account" });
    fireEvent.click(within(rail).getByRole("button", { name: /^Work/ }));
    expect(within(rail).getByRole("button", { name: /^Work/ }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByRole("article", { name: "Personal" })).toBeNull();
    expect(document.querySelector(".tint-ring")).not.toBeNull();
    await waitFor(() => expect(document.title).toBe("Facet · Work"));
  });

  it("rail moves with arrow keys", async () => {
    await renderApp();
    const rail = screen.getByRole("toolbar", { name: "Switch account" });
    const all = within(rail).getByRole("button", { name: "All accounts" });
    all.focus();
    fireEvent.keyDown(all, { key: "ArrowDown" });
    expect(document.activeElement?.textContent).toContain("Personal");
  });

  it("adds an account through the three steps", async () => {
    await renderApp();
    fireEvent.click(screen.getAllByRole("button", { name: /Add account/ })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Add account" });
    expect(within(dialog).getByText(/Add only accounts that are yours/)).toBeTruthy();
    const next = () => within(dialog).getByRole("button", { name: "Continue" }) as HTMLButtonElement;
    expect(next().disabled).toBe(true);
    fireEvent.change(within(dialog).getByLabelText("Name"), { target: { value: "Client Acme" } });
    expect(within(dialog).getByText("Short name: client-acme")).toBeTruthy();
    fireEvent.click(next());
    const email = within(dialog).getByLabelText(/Email you sign in with/);
    fireEvent.change(email, { target: { value: "nope" } });
    expect(within(dialog).getByText(/Enter the full email/)).toBeTruthy();
    expect(email.getAttribute("aria-describedby")).toBe("acc-email-err");
    expect(next().disabled).toBe(true);
    fireEvent.change(email, { target: { value: "me@acme.example" } });
    fireEvent.click(next());
    expect(within(dialog).getByText(/claude code ~\/\.claude-client-acme/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Add account" }));
    });
    await screen.findByRole("article", { name: "Client Acme" });
  });

  it("keeps focus in the dialog field when account state is pushed", async () => {
    const api = await renderApp();
    fireEvent.click(screen.getAllByRole("button", { name: /Add account/ })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Add account" });
    const plan = within(dialog).getByLabelText(/Plan/);
    plan.focus();
    await act(async () => {
      await api.launchDesktop("work"); // the core pushes new account states
    });
    expect(document.activeElement).toBe(plan);
  });

  it("refuses duplicate and reserved names before saving", async () => {
    await renderApp();
    fireEvent.click(screen.getAllByRole("button", { name: /Add account/ })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Add account" });
    fireEvent.change(within(dialog).getByLabelText("Name"), { target: { value: "Work" } });
    expect(within(dialog).getByText(/already exists/)).toBeTruthy();
    fireEvent.change(within(dialog).getByLabelText("Name"), { target: { value: "Desktop" } });
    expect(within(dialog).getByText(/is reserved/)).toBeTruthy();
  });

  it("removing an account asks first and says folders stay", async () => {
    await renderApp();
    const work = screen.getByRole("article", { name: "Work" });
    fireEvent.click(within(work).getByRole("button", { name: /Edit/ }));
    const dialog = await screen.findByRole("dialog", { name: "Edit Work" });
    fireEvent.click(within(dialog).getByRole("button", { name: /Remove from Facet/ }));
    expect(within(dialog).getByText(/folders, sign-in and history stay exactly as they are/)).toBeTruthy();
    expect(document.activeElement?.textContent).toBe("Keep");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
    });
    await waitFor(() => expect(screen.queryByRole("article", { name: "Work" })).toBeNull());
  });

  it("health runs local checks, lists worst first and follows the scope", async () => {
    await renderApp();
    fireEvent.click(within(nav()).getByRole("button", { name: /Health/ }));
    await screen.findByText(/deletes Personal's transcripts after 30 days/);
    const items = screen.getAllByRole("listitem");
    expect(items[0]!.textContent).toMatch(/30 days/);
    expect(screen.queryByText(/^(Personal|Work) is signed in$/)).toBeNull();
    const rail = screen.getByRole("toolbar", { name: "Switch account" });
    fireEvent.click(within(rail).getByRole("button", { name: /^Work/ }));
    await waitFor(() => expect(screen.queryByText(/deletes Personal's transcripts/)).toBeNull());
    expect(screen.getByText("Claude Desktop is installed")).toBeTruthy();
  });

  it("quick switcher opens with Ctrl K and launches an account", async () => {
    await renderApp();
    fireEvent.keyDown(document, { key: "k", ctrlKey: true });
    const box = await screen.findByRole("combobox", { name: /Search accounts and actions/ });
    fireEvent.change(box, { target: { value: "launch work" } });
    const options = screen.getAllByRole("option");
    expect(options[0]!.textContent).toContain("Launch Work");
    await act(async () => {
      fireEvent.keyDown(box, { key: "Enter" });
    });
    await waitFor(() => expect(within(screen.getByRole("article", { name: "Work" })).getByText("Desktop running")).toBeTruthy());
    expect(screen.queryByRole("combobox")).toBeNull();
  });

  it("folder rules: shows the sample rule, tests a folder and saves a new rule", async () => {
    await renderApp();
    fireEvent.click(within(nav()).getByRole("button", { name: /Folder rules/ }));
    const folder = await screen.findByLabelText("Folder for rule 1");
    expect((folder as HTMLInputElement).value).toBe("~/work");
    fireEvent.change(screen.getByLabelText("Folder"), { target: { value: "~/work/app" } });
    expect(screen.getByText(/from the rule for/).textContent).toContain("Work");
    fireEvent.change(screen.getByLabelText("Folder"), { target: { value: "~/workshop" } });
    expect(screen.getByText(/No rule covers this folder/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Add rule/ }));
    fireEvent.change(screen.getByLabelText("Folder for rule 2"), { target: { value: "~/personal" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Save rules" }));
    });
    await waitFor(() => expect((screen.getByRole("button", { name: "Save rules" }) as HTMLButtonElement).disabled).toBe(true));
  });

  it("every section opens a real view", async () => {
    await renderApp();
    for (const name of ["Today", "Timeline", "Costs", "Ecosystem", "Sessions", "Bridge"]) {
      fireEvent.click(within(nav()).getByRole("button", { name: new RegExp(name) }));
      expect(await screen.findByRole("heading", { name, level: 1 })).toBeTruthy();
      expect(screen.queryByText("Not built yet")).toBeNull();
    }
  });
});

describe("folder rule matching (mirrors rules.rs)", () => {
  const rules = [
    { folder: "/h/work", accountId: "work" },
    { folder: "/h/work/acme", accountId: "acme" },
  ];
  it("longest folder wins; siblings do not match", () => {
    expect(matchFolder(rules, "/h/work/acme/app")?.accountId).toBe("acme");
    expect(matchFolder(rules, "/h/work/other")?.accountId).toBe("work");
    expect(matchFolder(rules, "/h/workshop")).toBeUndefined();
    expect(matchFolder(rules, "/h/work/")?.accountId).toBe("work");
  });
});
