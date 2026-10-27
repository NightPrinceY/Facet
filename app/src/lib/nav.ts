import type { IconName } from "../components/Icon";

export type ViewId = "today" | "timeline" | "accounts" | "costs" | "control" | "studio" | "ecosystem" | "sessions" | "rules" | "bridge" | "health" | "settings";

export const NAV: { group: string; items: { id: ViewId; label: string; icon: IconName; tag?: string }[] }[] = [
  { group: "Overview", items: [{ id: "today", label: "Today", icon: "today" }, { id: "timeline", label: "Timeline", icon: "timeline" }] },
  {
    group: "Manage",
    items: [
      { id: "accounts", label: "Accounts", icon: "accounts" },
      { id: "costs", label: "Costs", icon: "costs" },
      { id: "control", label: "Control", icon: "control" },
      { id: "studio", label: "Studio", icon: "studio" },
      { id: "ecosystem", label: "Ecosystem", icon: "ecosystem" },
    ],
  },
  {
    group: "Work",
    items: [
      { id: "sessions", label: "Sessions", icon: "sessions" },
      { id: "rules", label: "Folder rules", icon: "folder" },
      { id: "bridge", label: "Bridge", icon: "bridge" },
    ],
  },
  { group: "System", items: [{ id: "health", label: "Health", icon: "health" }, { id: "settings", label: "Settings", icon: "settings" }] },
];
