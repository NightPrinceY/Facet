import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { createSampleApi } from "../lib/sample";

afterEach(cleanup);

describe("Links per account", () => {
  it("suggests the profile signed in with the account's email, and turns routing on with a confirmation", async () => {
    const api = createSampleApi();
    render(<App api={api} />);
    await screen.findByRole("article", { name: "Work" });
    fireEvent.click(within(screen.getByRole("navigation", { name: "Sections" })).getByRole("button", { name: /Settings/ }));
    const list = await screen.findByRole("list", { name: "Browser profile per account" });
    const work = within(list).getByText("Work").closest("li")!;
    expect(within(work).getByText(/Suggested: Google Chrome · Work · you@company.example/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(work).getByRole("button", { name: "Use it" }));
    });
    await waitFor(async () => expect((await api.linksState()).settings.accounts.work).toEqual({ browser: "chrome", profile: "Profile 1" }));
    await waitFor(() => expect((within(list).getByLabelText("Open Work's links in") as HTMLSelectElement).value).toBe("chrome|Profile 1"));

    fireEvent.change(within(list).getByLabelText("Open other links in"), { target: { value: "chrome|Default" } });
    await waitFor(async () => expect((await api.linksState()).settings.fallback).toEqual({ browser: "chrome", profile: "Default" }));

    fireEvent.click(screen.getByRole("button", { name: "Turn on" }));
    const dialog = await screen.findByRole("dialog", { name: "Open links per account" });
    expect(within(dialog).getByText(/never reads/)).toBeTruthy();
    await act(async () => {
      fireEvent.click(within(dialog).getByRole("button", { name: "Turn on" }));
    });
    await waitFor(() => expect(screen.getByRole("button", { name: "Turn off" })).toBeTruthy());
    expect(await screen.findByRole("list", { name: "Recent links" })).toBeTruthy();
  });
});
