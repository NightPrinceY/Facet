// The contract with the Rust core (app/src-tauri/src/commands.rs).
// Outside Tauri (plain `npm run dev` in a browser, and tests) a sample
// backend answers instead, and the UI says so.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ControlApplied, ControlOp, ControlPolicy, ControlPreview, ControlState, SettingsBackup } from "./control";
import type { AccountItem, ItemKind, Lint, PlacePreview, StudioItem } from "./studio";
import type { Effective } from "../views/EffectiveSettings";
import type { UsageReport } from "./usage";
import type { Inventory, McpAdd, PackageReport } from "./ecosystem";
import type { TimelineEvents } from "./timeline";

export interface LinkTarget {
  browser: string;
  profile?: string | null;
}

export interface BrowserProfile {
  dir: string;
  name: string;
  emails: string[];
}

export interface Browser {
  id: string;
  name: string;
  exe: string;
  profiles: BrowserProfile[];
}

export interface LinkSettings {
  enabled: boolean;
  fallback?: LinkTarget | null;
  previousDefault?: string | null;
  accounts: Record<string, LinkTarget>;
}

export interface LinksState {
  settings: LinkSettings;
  browsers: Browser[];
  suggestions: Record<string, LinkTarget>;
  currentDefault: string | null;
  recent: { ms: number; host: string; accountId: string | null; browser: string | null; profile: string | null }[];
}

export interface Handoff {
  name: string;
  path: string;
  text: string;
}

export interface HandoffFile {
  name: string;
  path: string;
  bytes: number;
  modifiedMs: number;
  title: string;
}

export type FacetColor = 1 | 2 | 3 | 4 | 5 | 6;

export interface Account {
  id: string;
  name: string;
  color: FacetColor;
  plan?: string;
  desktopDir?: string;
  codeDir: string;
  expectedEmail?: string;
  tint: boolean;
  x11Identity: boolean;
  createdAt: number;
  /** Up to three characters or one emoji shown instead of the initial. */
  badge?: string;
  notes?: string;
  launcherIcon: LauncherIcon;
  /** Folder that "Open terminal" starts in. */
  terminalDir?: string;
  /** `data:` URL of the account picture. */
  imageUrl?: string | null;
  /** Name in the dock and app menu; default "<name> · Facet". */
  launcherName?: string;
}

/** "claude-badge": the installed Claude icon with the account badge; "avatar": Facet's square. */
export type LauncherIcon = "claude-badge" | "avatar" | "claude";

export interface AccountInput {
  id?: string;
  name: string;
  color?: FacetColor;
  plan?: string;
  withDesktop: boolean;
  desktopDir?: string;
  codeDir?: string;
  expectedEmail?: string;
  tint?: boolean;
  x11Identity?: boolean;
  badge?: string;
  notes?: string;
  launcherIcon?: LauncherIcon;
  terminalDir?: string;
  launcherName?: string;
}

export interface AccountState {
  id: string;
  desktopPid: number | null;
  launchedByFacet: boolean;
  liveSessions: number;
  waitingSessions: number;
  shimInstalled: boolean;
  launcherInstalled: boolean;
}

export interface DetectedSetup {
  suggestedId: string;
  suggestedName: string;
  desktopDir: string | null;
  codeDir: string | null;
}

export interface ClaudeInstall {
  path: string;
  version: string | null;
  channel: string;
  /** Confirmed as Anthropic's official build. */
  official: boolean;
}

export interface AppInfo {
  version: string;
  platform: string;
  desktop: ClaudeInstall | null;
  code: ClaudeInstall | null;
  shortcut: "portal" | "cli-only";
  storePath: string;
  home: string;
}

export interface AuthStatus {
  loggedIn: boolean;
  authMethod?: string | null;
  email?: string | null;
  orgName?: string | null;
  subscriptionType?: string | null;
  configDirectory?: string | null;
}

export type CheckStatus = "bad" | "warn" | "info" | "ok";

export interface HealthCheck {
  id: string;
  status: CheckStatus;
  title: string;
  detail: string;
  fix?: string;
  accountId?: string;
}

export interface FolderRule {
  folder: string;
  accountId: string;
}

export interface RulesSaved {
  rules: FolderRule[];
  commandUpdated: string | null;
}

export interface AutoCommandInfo {
  path: string;
  installed: boolean;
}

export interface Api {
  readonly isSample: boolean;
  appInfo(): Promise<AppInfo>;
  listAccounts(): Promise<Account[]>;
  accountStates(): Promise<AccountState[]>;
  saveAccount(input: AccountInput): Promise<Account>;
  deleteAccount(id: string): Promise<void>;
  /** Set a picture (base64 or data URL) or clear it with null. */
  setAccountImage(id: string, data: string | null): Promise<Account>;
  reorderAccounts(ids: string[]): Promise<Account[]>;
  /** Change the short name: moves claude-<id>, launcher, rules and backups. */
  renameAccount(id: string, newId: string): Promise<Account>;
  /** The menu and dock icon, as a data URL; the overrides preview unsaved edits. */
  /** Usage and API-equivalent cost from transcript metadata, last `days` days. */
  usageReport(days: number): Promise<UsageReport>;
  /** Open a terminal in the session's account and folder running claude --resume. */
  resumeSession(id: string, sessionId: string, cwd: string | null): Promise<number>;
  /** Prompts, model calls and tool runs between two times. */
  timelineEvents(sinceMs: number, untilMs: number): Promise<TimelineEvents>;
  /** Draft a handoff (not saved) from a session in one account to another. */
  bridgeDraft(fromId: string, sessionId: string, toId: string, note: string, includeGit: boolean): Promise<Handoff>;
  bridgeSave(name: string, text: string): Promise<Handoff>;
  bridgeList(): Promise<HandoffFile[]>;
  bridgeRemove(name: string): Promise<void>;
  /** Terminal as the receiving account running claude with a prompt that points at the handoff. */
  bridgeOpen(toId: string, name: string, cwd: string | null): Promise<number>;
  linksState(): Promise<LinksState>;
  linksSetAccount(id: string, target: LinkTarget | null): Promise<LinkSettings>;
  linksSetFallback(target: LinkTarget | null): Promise<LinkSettings>;
  /** Make Facet's link opener the default browser (backs up mimeapps.list). */
  linksEnable(): Promise<LinkSettings>;
  linksDisable(): Promise<LinkSettings>;
  /** Open a test page in an account's browser profile (or the fallback). */
  linksTest(id: string | null): Promise<void>;
  /** Official `claude mcp add` as the account; returns its (redacted) output. */
  mcpAdd(id: string, input: McpAdd): Promise<string>;
  mcpRemove(id: string, name: string, scope: string, folder: string | null): Promise<string>;
  ecosystemInventory(id: string): Promise<Inventory>;
  /** Read-only safety scan of a folder. */
  ecosystemScan(path: string): Promise<PackageReport>;
  /** Mark a plugin's current files as reviewed (null clears). */
  ecosystemMarkReviewed(id: string, pluginId: string, hash: string | null): Promise<void>;
  /** Save an export in Downloads; returns the path. */
  exportText(name: string, text: string): Promise<string>;
  launcherPreview(id: string, overrides?: { style?: LauncherIcon; badge?: string; color?: FacetColor; name?: string }): Promise<string>;
  detectExisting(): Promise<DetectedSetup[]>;
  launchDesktop(id: string): Promise<number>;
  quitDesktop(id: string): Promise<void>;
  openTerminal(id: string, cwd?: string): Promise<number>;
  installShim(id: string): Promise<string>;
  installLauncher(id: string): Promise<string>;
  checkSignIn(id: string): Promise<AuthStatus>;
  runHealth(checkSignIn: boolean): Promise<HealthCheck[]>;
  onAccountStates(cb: (states: AccountState[]) => void): Promise<UnlistenFn>;
  listRules(): Promise<FolderRule[]>;
  saveRules(rules: FolderRule[]): Promise<RulesSaved>;
  installAutoCommand(): Promise<string>;
  autoCommandInfo(): Promise<AutoCommandInfo>;
  /** True once if Facet was started or re-invoked with --switcher. */
  takeSwitcherRequest(): Promise<boolean>;
  onOpenSwitcher(cb: () => void): Promise<UnlistenFn>;
  /** Messages from the core that are not replies to a command (tray actions). */
  onNotice(cb: (message: string) => void): Promise<UnlistenFn>;
  /** Window title, so the active account shows in the task switcher. */
  setTitle(title: string): Promise<void>;
  controlRead(id: string): Promise<ControlState>;
  controlPreview(id: string, ops: ControlOp[]): Promise<ControlPreview>;
  controlApply(id: string, ops: ControlOp[], baseHash: string): Promise<ControlApplied>;
  controlBackups(id: string): Promise<SettingsBackup[]>;
  controlRestorePreview(id: string, name: string): Promise<ControlPreview>;
  controlRestore(id: string, name: string, baseHash: string): Promise<ControlApplied>;
  controlPolicy(): Promise<ControlPolicy>;
  controlEffective(id: string, project?: string): Promise<Effective>;
  studioList(): Promise<StudioItem[]>;
  studioRead(kind: ItemKind, name: string): Promise<string>;
  studioLint(kind: ItemKind, name: string, text: string): Promise<Lint>;
  studioTemplate(kind: ItemKind, name: string, description: string, body: string): Promise<string>;
  studioSave(kind: ItemKind, name: string, text: string): Promise<AccountItem>;
  studioRemove(kind: ItemKind, name: string): Promise<void>;
  studioPlacePreview(kind: ItemKind, name: string, accountId: string): Promise<PlacePreview>;
  studioPlace(kind: ItemKind, name: string, accountId: string, baseHash: string, riskAck: boolean): Promise<string | null>;
  studioUnplace(kind: ItemKind, name: string, accountId: string): Promise<string>;
  studioAccountItems(accountId: string): Promise<AccountItem[]>;
  studioImport(accountId: string, kind: ItemKind, name: string): Promise<AccountItem>;
}

export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const tauriApi: Api = {
  isSample: false,
  appInfo: () => invoke("app_info"),
  listAccounts: () => invoke("list_accounts"),
  accountStates: () => invoke("get_account_states"),
  saveAccount: (input) => invoke("save_account", { input }),
  deleteAccount: (id) => invoke("delete_account", { id }),
  setAccountImage: (id, data) => invoke("set_account_image", { id, data }),
  reorderAccounts: (ids) => invoke("reorder_accounts", { ids }),
  renameAccount: (id, newId) => invoke("rename_account", { id, newId }),
  usageReport: (days) => invoke("usage_report", { days }),
  resumeSession: (id, sessionId, cwd) => invoke("resume_session", { id, sessionId, cwd }),
  exportText: (name, text) => invoke("export_text", { name, text }),
  timelineEvents: (sinceMs, untilMs) => invoke("timeline_events", { sinceMs: Math.round(sinceMs), untilMs: Math.round(untilMs) }),
  bridgeDraft: (fromId, sessionId, toId, note, includeGit) => invoke("bridge_draft", { fromId, sessionId, toId, note, includeGit }),
  bridgeSave: (name, text) => invoke("bridge_save", { name, text }),
  bridgeList: () => invoke("bridge_list"),
  bridgeRemove: (name) => invoke("bridge_remove", { name }),
  bridgeOpen: (toId, name, cwd) => invoke("bridge_open", { toId, name, cwd }),
  linksState: () => invoke("links_state"),
  linksSetAccount: (id, target) => invoke("links_set_account", { id, target }),
  linksSetFallback: (target) => invoke("links_set_fallback", { target }),
  linksEnable: () => invoke("links_enable"),
  linksDisable: () => invoke("links_disable"),
  linksTest: (id) => invoke("links_test", { id }),
  mcpAdd: (id, input) => invoke("mcp_add", { id, input }),
  mcpRemove: (id, name, scope, folder) => invoke("mcp_remove", { id, name, scope, folder }),
  ecosystemInventory: (id) => invoke("ecosystem_inventory", { id }),
  ecosystemScan: (path) => invoke("ecosystem_scan", { path }),
  ecosystemMarkReviewed: (id, pluginId, hash) => invoke("ecosystem_mark_reviewed", { id, pluginId, hash }),
  launcherPreview: (id, o) => invoke("launcher_preview", { id, style: o?.style ?? null, badge: o?.badge ?? null, color: o?.color ?? null, name: o?.name ?? null }),
  detectExisting: () => invoke("detect_existing"),
  launchDesktop: (id) => invoke("launch_desktop", { id }),
  quitDesktop: (id) => invoke("quit_desktop", { id }),
  openTerminal: (id, cwd) => invoke("open_terminal", { id, cwd: cwd ?? null }),
  installShim: (id) => invoke("install_shim", { id }),
  installLauncher: (id) => invoke("install_launcher", { id }),
  checkSignIn: (id) => invoke("check_sign_in", { id }),
  runHealth: (checkSignIn) => invoke("run_health", { checkSignIn }),
  onAccountStates: (cb) => listen<AccountState[]>("account-states", (e) => cb(e.payload)),
  listRules: () => invoke("list_rules"),
  saveRules: (rules) => invoke("save_rules", { rules }),
  installAutoCommand: () => invoke("install_auto_command"),
  autoCommandInfo: () => invoke("auto_command_info"),
  takeSwitcherRequest: () => invoke("take_switcher_request"),
  onOpenSwitcher: (cb) => listen("open-switcher", () => cb()),
  onNotice: (cb) => listen<string>("notice", (e) => cb(e.payload)),
  setTitle: async (title) => {
    document.title = title;
    await getCurrentWindow().setTitle(title);
  },
  controlRead: (id) => invoke("control_read", { id }),
  controlPreview: (id, ops) => invoke("control_preview", { id, ops }),
  controlApply: (id, ops, baseHash) => invoke("control_apply", { id, ops, baseHash }),
  controlBackups: (id) => invoke("control_backups", { id }),
  controlRestorePreview: (id, name) => invoke("control_restore_preview", { id, name }),
  controlRestore: (id, name, baseHash) => invoke("control_restore", { id, name, baseHash }),
  controlPolicy: () => invoke("control_policy"),
  controlEffective: (id, project) => invoke("control_effective", { id, project: project ?? null }),
  studioList: () => invoke("studio_list"),
  studioRead: (kind, name) => invoke("studio_read", { kind, name }),
  studioLint: (kind, name, text) => invoke("studio_lint", { kind, name, text }),
  studioTemplate: (kind, name, description, body) => invoke("studio_template", { kind, name, description, body }),
  studioSave: (kind, name, text) => invoke("studio_save", { kind, name, text }),
  studioRemove: (kind, name) => invoke("studio_remove", { kind, name }),
  studioPlacePreview: (kind, name, accountId) => invoke("studio_place_preview", { kind, name, accountId }),
  studioPlace: (kind, name, accountId, baseHash, riskAck) => invoke("studio_place", { kind, name, accountId, baseHash, riskAck }),
  studioUnplace: (kind, name, accountId) => invoke("studio_unplace", { kind, name, accountId }),
  studioAccountItems: (accountId) => invoke("studio_account_items", { accountId }),
  studioImport: (accountId, kind, name) => invoke("studio_import", { accountId, kind, name }),
};

/** Error text from a rejected command (Rust sends plain strings). */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Something went wrong.";
}
