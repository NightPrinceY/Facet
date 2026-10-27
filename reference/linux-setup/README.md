# Linux reference setup (working, hand-made)

The setup that proved the idea on Ubuntu 26 GNOME Wayland with Claude Desktop 3.6556.2.

| File | Installed at | Purpose |
|---|---|---|
| `claude-desktop-work` | `~/.local/bin/claude-desktop-work` | Starts the work instance with `--user-data-dir=~/.config/Claude-Work --ozone-platform=x11` and `CLAUDE_CONFIG_DIR=~/.claude-work` (added 2026-10-26, see spike 1 test 2 notes), then relabels its windows' `WM_CLASS` to `claude-work` so it gets its own dock icon |
| `claude-work.desktop` | `~/.local/share/applications/` | Menu entry "Claude (Work)" with `StartupWMClass=claude-work` |

Also in `~/.bashrc`: `alias claude-work='CLAUDE_CONFIG_DIR=/home/you/.claude-work claude'`.

Lessons that carry into Facet's Linux adapter: see `docs/03-architecture/PLATFORM-ADAPTERS.md` and `docs/00-context/SESSION-HISTORY.md`.

## Test 7 notes (2026-10-26): cost of `--ozone-platform=x11`, GlobalShortcuts portal

Facts gathered without UI (`gnome-shell --version`, `gdbus introspect` on `org.freedesktop.portal.Desktop`, `gsettings`, Mutter `DisplayConfig.GetCurrentState`):

| Item | This machine |
|---|---|
| Session | GNOME Shell 50.1, Wayland, Ubuntu 26.04.1 LTS |
| Portals | `xdg-desktop-portal` 1.21.1, `xdg-desktop-portal-gnome` 50.0; **`org.freedesktop.portal.GlobalShortcuts` present, version 1** (also `InputCapture`, `RemoteDesktop`, `DynamicLauncher`, `Secret`, `Notification`) |
| Fractional scaling | Enabled (`org.gnome.mutter experimental-features = ['scale-monitor-framebuffer']`), but all three monitors (eDP-1 primary, DP-1, HDMI-1 rotated) run at **scale 1.0**; text scaling 1.0 |
| Input | `us` + `ara` layouts, IBus (`XMODIFIERS=@im=ibus`, `ibus-portal` present) |

Reading:
- **Scaling cost today: none.** XWayland windows go blurry only when a monitor uses a non-integer scale under `scale-monitor-framebuffer`. Mutter's `xwayland-native-scaling` experimental feature is meant to fix that **[unverified on GNOME 50]**. Facet's Health should warn when an X11-forced instance sits on a fractionally scaled monitor.
- **Input methods:** IBus serves XWayland clients via XIM; layout switching (`us`/`ara`) is handled by the compositor. Arabic typing into the X11 work window still needs a **visual check by the user** (type Arabic in the work instance; check that cursor movement and RTL shaping are right).
- **Input portals:** `RemoteDesktop` and `InputCapture` are also present. Facet must never use them on Claude windows (rule 5). It uses only GlobalShortcuts, for its own window.
- **Global shortcuts:** the GlobalShortcuts portal is available, so on this GNOME the quick switcher can bind a real global shortcut through the portal (the user approves it in a system dialog). The CLI fallback is needed only where the portal is missing. Not yet tested: binding and receiving a shortcut (needs a small portal client and the user's approval in the dialog).
