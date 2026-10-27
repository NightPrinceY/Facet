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

async function openEco(api: Api = createSampleApi()) {
  render(<App api={api} />);
  await screen.findByRole("article", { name: "Work" });
  fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Ecosystem/ }));
  return api;
}

describe("Ecosystem", () => {
  it("lists plugins with risk in words, and details on demand", async () => {
    await openEco();
    const list = await screen.findByRole("list", { name: "Plugins" });
    const deploy = within(list).getByText("deploy-kit").closest("li")!;
    expect(within(deploy).getByText("High risk")).toBeTruthy();
    fireEvent.click(within(deploy).getByRole("button", { expanded: false }));
    expect(within(deploy).getByRole("list", { name: "Why this risk level" })).toBeTruthy();
    expect(within(deploy).getByText(/Sends hook data to a web address/)).toBeTruthy();
    expect(within(deploy).getByText(/DB_PASSWORD \(values not shown\)/)).toBeTruthy();
  });

  it("turns a plugin off through a reviewed settings change", async () => {
    const api = await openEco();
    const list = await screen.findByRole("list", { name: "Plugins" });
    await act(async () => {
      fireEvent.click(within(list).getByRole("button", { name: "Turn off deploy-kit for Personal" }));
    });
    const dialog = await screen.findByRole("dialog", { name: "Turn off deploy-kit for Personal" });
    expect(within(dialog).getByRole("region", { name: /settings\.json/ }).textContent).toContain("deploy-kit@acme");
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Turn off" }));
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    const inv = await api.ecosystemInventory("personal");
    expect(inv.plugins.find((p) => p.id === "deploy-kit@acme")!.enabled).toBe(false);
  });

  it("marks a plugin as reviewed", async () => {
    const api = await openEco();
    const list = await screen.findByRole("list", { name: "Plugins" });
    const row = within(list).getByText("frontend-design").closest("li")!;
    fireEvent.click(within(row).getByRole("button", { expanded: false }));
    await act(async () => {
      fireEvent.click(within(row).getByRole("button", { name: "Mark this version as reviewed" }));
    });
    await waitFor(async () => expect((await api.ecosystemInventory("personal")).plugins.find((p) => p.name === "frontend-design")!.reviewedHash).toBe("00000000000000a1"));
    expect(await within(await screen.findByRole("list", { name: "Plugins" })).findByText("Reviewed")).toBeTruthy();
  });

  it("checks a folder and shows the risk", async () => {
    await openEco();
    fireEvent.click(screen.getByRole("button", { name: "Check a folder" }));
    fireEvent.change(screen.getByLabelText("Folder to check"), { target: { value: "~/Downloads/deploy-kit" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Check" }));
    });
    const preview = await screen.findByRole("region", { name: "Safety preview" });
    expect(within(preview).getAllByText("High risk").length).toBeGreaterThan(0);
    expect(within(preview).getByText(/\/home\/you\/Downloads\/deploy-kit|~\/Downloads\/deploy-kit/)).toBeTruthy();
  });

  it("shows another account from the picker", async () => {
    await openEco();
    await screen.findByRole("list", { name: "Plugins" });
    fireEvent.click(within(screen.getByRole("group", { name: "Account" })).getByRole("button", { name: "Work" }));
    expect(await screen.findByText("No plugins installed for Work.")).toBeTruthy();
  });

  it("adds and removes an MCP server through the official command", async () => {
    const api = await openEco();
    await screen.findByRole("list", { name: "Plugins" });
    fireEvent.click(screen.getByRole("button", { name: "Add MCP server" }));
    const dialog = await screen.findByRole("dialog", { name: "Add an MCP server to Personal" });
    fireEvent.change(within(dialog).getByLabelText("Name"), { target: { value: "files" } });
    fireEvent.change(within(dialog).getByLabelText("Program"), { target: { value: "npx" } });
    fireEvent.change(within(dialog).getByLabelText("Arguments (optional)"), { target: { value: '-y files-mcp@1.0.0 "/home/you/my docs"' } });
    fireEvent.change(within(dialog).getByLabelText("Environment variables (optional)"), { target: { value: "ANTHROPIC_API_KEY=x" } });
    expect(within(dialog).getByText(/does not pass sign-in or routing variables/)).toBeTruthy();
    fireEvent.change(within(dialog).getByLabelText("Environment variables (optional)"), { target: { value: "ROOT=/home/you" } });
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Add server" }));
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    const inv = await api.ecosystemInventory("personal");
    const added = inv.mcp.find((m) => m.name === "files")!;
    expect(added.args).toEqual(["-y", "files-mcp@1.0.0", "/home/you/my docs"]);
    const mcpList = screen.getByRole("list", { name: "MCP servers" });
    fireEvent.click(within(mcpList).getByRole("button", { name: "Remove files" }));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    });
    await waitFor(async () => expect((await api.ecosystemInventory("personal")).mcp.some((m) => m.name === "files")).toBe(false));
  });
});
