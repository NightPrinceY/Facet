# Platform adapters

Every OS-specific trick lives behind one Rust trait so a Claude update only breaks one adapter.

```rust
trait PlatformAdapter {
    fn find_claude_desktop(&self) -> Result<ClaudeInstall>;       // path, version, channel (deb, MSIX, .app)
    fn launch(&self, acc: &Account, args: &[String]) -> Result<Pid>;
    fn focus(&self, acc: &Account) -> Result<()>;
    fn quit(&self, acc: &Account) -> Result<()>;
    fn running(&self) -> Vec<(AccountId, Pid)>;
    fn install_launcher(&self, acc: &Account) -> Result<()>;      // menu entry / shortcut / launcher app
    fn apply_identity(&self, acc: &Account, pid: Pid) -> Result<IdentityLevel>; // dock icon, badge, title
    fn desktop_config_path(&self, acc: &Account) -> PathBuf;
    fn global_shortcut(&self) -> ShortcutSupport;                 // native, portal, cli-only
}
```

## Linux (Phase 1)
| Capability | Method | Status |
|---|---|---|
| Find Claude | `/usr/lib/claude-desktop/claude-desktop` (deb); `dpkg-query` for version | Proven |
| Separate instance | `--user-data-dir=<dir>` | Proven |
| Identity | X11: `--ozone-platform=x11` + relabel `WM_CLASS` to `facet-<id>` (xdotool or x11rb in Rust) + `StartupWMClass` in launcher. Wayland: cannot change app id; use coloured badge and title | Proven (X11) |
| Launcher | `~/.local/share/applications/facet-<id>.desktop`, `update-desktop-database` | Proven |
| Find PID | Match substring `--user-data-dir=<dir> ` in `/proc/*/cmdline` (Electron joins argv with spaces) | Proven |
| Global shortcut | XDG GlobalShortcuts portal; fallback: `facet open --switcher` bound to a GNOME custom shortcut | To test (7) |
| Trade-off | X11 mode loses native Wayland fractional scaling and some IME behaviour | To measure (7) |

## Windows (Phase 2)
| Capability | Method | Status |
|---|---|---|
| Find Claude | MSIX package (Store). Direct launch from `WindowsApps` is blocked | Known |
| Separate instance | Store execution alias `claude-desktop.exe --user-data-dir=...` (single source) or portable copy (no auto-update) | To test (9) |
| Identity | Coloured title badge; Start menu shortcut per account. Taskbar grouping via per-window AppUserModelID is unproven | Don't promise |
| Signing | Microsoft Store for individuals outside USA/Canada; else Artifact Signing ($9.99/mo, US/CA individuals) | Decide in Phase 2 |

## macOS (Phase 2–3)
| Capability | Method | Status |
|---|---|---|
| Separate instance | `open -n -a Claude --args --user-data-dir=...` | Proven by claude-multiprofile |
| Identity | Launcher app per account (own icon). Running windows share Claude's Dock icon; re-signed clones break signatures and Keychain, so no | Known |
| Auto-update | Updates have reset profile copies in other tools; Facet never copies Claude.app | Test (9) |
| Signing | Apple Developer $99/yr, notarization | Required |

## All platforms
- `claude://` sign-in links go to the last-registered instance. Facet does not take over the scheme; it serialises sign-ins (guided flow) and verifies the email afterwards.
- Claude Code isolation is official on all three via `CLAUDE_CONFIG_DIR`.
