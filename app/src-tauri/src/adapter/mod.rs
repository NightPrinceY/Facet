//! Platform adapters (PLATFORM-ADAPTERS.md). Every OS-specific trick lives
//! here so a Claude update breaks one adapter, not the app.

pub mod linux;

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeInstall {
    pub path: PathBuf,
    pub version: Option<String>,
    /// How it was installed: "deb", "path" (found on PATH), "custom".
    pub channel: String,
    /// Confirmed as Anthropic's official build (deb maintainer, or the
    /// official Claude Code install folders). False = not confirmed.
    pub official: bool,
}

/// A running Claude Desktop main process.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopProcess {
    pub pid: u32,
    /// `--user-data-dir`, or Claude's default folder when the flag is absent.
    pub data_dir: PathBuf,
    pub x11: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ShortcutSupport {
    /// XDG GlobalShortcuts portal (GNOME 50 has it; spike 7 notes).
    Portal,
    /// A desktop-specific custom shortcut that runs `facet --switcher`.
    CliOnly,
}
