//! Linux adapter (Phase 1). Proven on Ubuntu 26.04 GNOME 50 with the
//! hand-made setup in `reference/linux-setup/`.
//!
//! Rules this file enforces:
//! - Only the official, unmodified `claude-desktop` and `claude` binaries are run (rule 1).
//! - Processes are found by command line only. The environment of Claude
//!   processes is never read: Desktop injects sign-in tokens there (rule 2).
//! - Every child gets a scrubbed environment and the account's
//!   `CLAUDE_CONFIG_DIR` (rule 6, spike 1 test 2).
//! - Nothing here clicks or types into Claude windows (rule 5). Window
//!   relabelling changes only the X11 class property, like the reference setup.

use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{ClaudeInstall, DesktopProcess, ShortcutSupport};
use crate::env;
use crate::error::{Error, Result};
use crate::profile::Account;

/// Known install locations of the Claude Desktop Linux build.
const DESKTOP_CANDIDATES: [&str; 3] = [
    "/usr/lib/claude-desktop/claude-desktop",
    "/opt/Claude/claude-desktop",
    "/usr/bin/claude-desktop",
];

pub fn find_claude_desktop() -> Option<ClaudeInstall> {
    // Developer override only; release builds always use the installed app.
    if cfg!(debug_assertions) {
        if let Some(custom) = std::env::var_os("FACET_CLAUDE_DESKTOP") {
            let path = PathBuf::from(custom);
            if path.is_file() {
                return Some(ClaudeInstall {
                    path,
                    version: None,
                    channel: "custom".into(),
                    official: false,
                });
            }
        }
    }
    let path = DESKTOP_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())?;
    // `${Version}|${Maintainer}` of the installed package (local dpkg database).
    let pkg = env::command("dpkg-query")
        .args(["-W", "-f=${Version}|${Maintainer}", "claude-desktop"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let (version, maintainer) = match pkg.as_deref().and_then(|p| p.split_once('|')) {
        Some((v, m)) => (Some(v.to_string()).filter(|v| !v.is_empty()), m.to_string()),
        None => (None, String::new()),
    };
    let channel = if version.is_some() { "deb" } else { "path" };
    Some(ClaudeInstall {
        path,
        version,
        channel: channel.into(),
        official: is_official_maintainer(&maintainer),
    })
}

/// The official Linux package is maintained by Anthropic (`Anthropic PBC`).
/// Community repackages use their own maintainer, so they show as unconfirmed.
pub fn is_official_maintainer(maintainer: &str) -> bool {
    maintainer.trim_start().starts_with("Anthropic PBC") || maintainer.contains("@anthropic.com>")
}

/// Official Claude Code installs: the native installer
/// (`~/.local/share/claude/versions/<v>`) or the npm package
/// `@anthropic-ai/claude-code`. A wrapper script elsewhere is unconfirmed.
pub fn is_official_code_path(real: &Path) -> bool {
    let s = real.to_string_lossy();
    s.contains("/.local/share/claude/versions/")
        || s.contains("/node_modules/@anthropic-ai/claude-code/")
}

/// The `claude` CLI on PATH, with `claude --version` (no prompt is sent).
pub fn find_claude_code() -> Option<ClaudeInstall> {
    let path = which("claude")?;
    let official = fs::canonicalize(&path)
        .map(|real| is_official_code_path(&real))
        .unwrap_or(false);
    let version = env::command(&path)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .filter(|v| !v.is_empty());
    Some(ClaudeInstall {
        path,
        version,
        channel: "path".into(),
        official,
    })
}

pub fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
    })
}

/// Claude Desktop's own folder when no `--user-data-dir` is given.
pub fn default_desktop_dir(home: &Path) -> PathBuf {
    home.join(".config/Claude")
}

/// Parse one `/proc/<pid>/cmdline`. Electron rewrites its argv into one
/// space-joined string, so both NUL- and space-separated forms are handled.
/// Returns the data folder for a Desktop *main* process; helpers carry `--type=`.
pub fn parse_desktop_cmdline(raw: &[u8], home: &Path) -> Option<(PathBuf, bool)> {
    let text = String::from_utf8_lossy(raw).replace('\0', " ");
    let text = text.trim();
    let first = text.split(' ').next()?;
    if !first.ends_with("/claude-desktop") && first != "claude-desktop" {
        return None;
    }
    if text.contains(" --type=") {
        return None;
    }
    let x11 = text.contains("--ozone-platform=x11");
    let dir = match text.find("--user-data-dir=") {
        Some(i) => {
            let rest = &text[i + "--user-data-dir=".len()..];
            // A value runs until the next " --flag" (paths may contain spaces).
            let end = rest.find(" --").unwrap_or(rest.len());
            PathBuf::from(rest[..end].trim().trim_matches('"'))
        }
        None => default_desktop_dir(home),
    };
    Some((dir, x11))
}

/// All running Desktop main processes. Reads `/proc/<pid>/comm` first and the
/// command line only of processes named `claude-desktop`, so other programs'
/// command lines are never read.
pub fn running_desktops(proc_root: &Path, home: &Path) -> Vec<DesktopProcess> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(proc_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        let is_desktop = fs::read_to_string(entry.path().join("comm"))
            .map(|c| c.trim_end() == "claude-desktop")
            .unwrap_or(false);
        if !is_desktop {
            continue;
        }
        let Ok(raw) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        if let Some((data_dir, x11)) = parse_desktop_cmdline(&raw, home) {
            out.push(DesktopProcess { pid, data_dir, x11 });
        }
    }
    out.sort_by_key(|p| p.pid);
    out
}

pub fn instance_for<'a>(
    account: &Account,
    running: &'a [DesktopProcess],
) -> Option<&'a DesktopProcess> {
    let dir = account.desktop_dir.as_ref()?;
    running.iter().find(|p| same_dir(&p.data_dir, dir))
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let strip = |p: &Path| p.to_string_lossy().trim_end_matches('/').to_string();
    strip(a) == strip(b)
}

/// Build the command that starts (or, if running, focuses) an account's
/// Desktop instance. Electron's single-instance lock forwards a second start
/// to the running window, so "focus" needs no window automation.
pub fn desktop_command(install: &ClaudeInstall, account: &Account, home: &Path) -> Result<Command> {
    let dir = account
        .desktop_dir
        .as_ref()
        .ok_or_else(|| Error::Invalid(format!("{} has no Desktop app set up.", account.name)))?;
    let mut cmd = env::command(&install.path);
    if !same_dir(dir, &default_desktop_dir(home)) {
        cmd.arg(format!("--user-data-dir={}", dir.display()));
    }
    if account.x11_identity {
        cmd.arg("--ozone-platform=x11");
    }
    env::prepare_with_home(&mut cmd, &account.code_dir, home);
    // Lets Facet's link opener tell which account a link came from (D-022).
    cmd.env("FACET_ACCOUNT", &account.id);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(cmd)
}

/// Start detached in its own session so quitting Facet leaves Claude running.
pub fn spawn_detached(mut cmd: Command) -> Result<u32> {
    // SAFETY: setsid is async-signal-safe and touches no Rust state.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let child = cmd.spawn()?;
    Ok(child.id())
}

/// Ask the Desktop main process to quit (SIGTERM, which Electron handles as a
/// normal quit). Does not touch its windows.
pub fn quit(pid: u32) -> Result<()> {
    // SAFETY: plain syscall with a pid we found in /proc.
    let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error().into())
    }
}

/// Window class used for an account's Desktop windows and its launcher.
pub fn wm_class(account: &Account) -> String {
    format!("facet-{}", account.id)
}

/// The only xdotool subcommands Facet may run (D-013): find windows and set
/// their class property. Never `key`, `type`, `click`, `mousemove`,
/// `windowactivate` or `windowfocus` (rule 5).
pub const XDOTOOL_ALLOWED: [&str; 2] = ["search", "set_window"];

pub fn xdotool_search_args(pid: u32) -> Vec<String> {
    [
        "search",
        "--pid",
        &pid.to_string(),
        "--class",
        "com.anthropic",
    ]
    .map(String::from)
    .to_vec()
}

pub fn xdotool_relabel_args(class: &str, window: &str) -> Vec<String> {
    ["set_window", "--classname", class, "--class", class, window]
        .map(String::from)
        .to_vec()
}

fn run_xdotool(xdotool: &Path, args: &[String]) -> Option<std::process::Output> {
    // Enforced at run time as well as by tests.
    if !args
        .first()
        .is_some_and(|a| XDOTOOL_ALLOWED.contains(&a.as_str()))
    {
        return None;
    }
    env::command(xdotool)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
}

/// Relabel an X11 instance's windows so the dock groups them under the
/// account's launcher (as `reference/linux-setup/claude-desktop-work` does).
/// Changes only the windows' class property. Runs for up to `budget`, because
/// Electron opens windows after start.
pub fn relabel_windows(account: &Account, home: &Path, budget: Duration) {
    let Some(xdotool) = which("xdotool") else {
        return;
    };
    let class = wm_class(account);
    let started = Instant::now();
    while started.elapsed() < budget {
        let running = running_desktops(Path::new("/proc"), home);
        if let Some(proc_) = instance_for(account, &running) {
            if let Some(out) = run_xdotool(&xdotool, &xdotool_search_args(proc_.pid)) {
                for wid in String::from_utf8_lossy(&out.stdout).split_whitespace() {
                    let _ = run_xdotool(&xdotool, &xdotool_relabel_args(&class, wid));
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Terminal emulators Facet knows how to start, in order of preference, with
/// the argument that separates the terminal's options from the command.
const TERMINALS: [(&str, &[&str]); 6] = [
    ("ptyxis", &["--new-window", "--"]),
    ("gnome-terminal", &["--"]),
    ("kgx", &["--"]),
    ("konsole", &["-e"]),
    ("xfce4-terminal", &["-x"]),
    ("xterm", &["-e"]),
];

/// Start file for bash terminals: the user's own `~/.bashrc` first, then the
/// account's settings again, so a shell file cannot put an override variable
/// or another account's folder back (compliance review, finding 3).
pub fn bash_rc(account: &Account, home: &Path) -> String {
    format!(
        "# Written by Facet for the account \"{name}\". Rewritten each time a terminal opens.\n[ -f \"$HOME/.bashrc\" ] && . \"$HOME/.bashrc\"\nunset {vars}\n{select}\nexport FACET_ACCOUNT={id}\nPS1=\"[{prompt}] $PS1\"\nprintf '\\033[1mFacet\\033[0m · this terminal is \\033[1m%s\\033[0m (CLAUDE_CONFIG_DIR=%s). Run claude to start.\\n' {qname} {dir}\n",
        name = safe_comment(&account.name),
        vars = env::OVERRIDE_VARS.join(" "),
        select = env::config_dir_shell_line(&account.code_dir, home),
        dir = shell_quote(&account.code_dir.display().to_string()),
        id = shell_quote(&account.id),
        prompt = account.id,
        qname = shell_quote(&safe_comment(&account.name)),
    )
}

pub fn bash_rc_path(home: &Path, account: &Account) -> PathBuf {
    home.join(format!(".local/share/facet/terminal/{}.bashrc", account.id))
}

/// The shell command a Facet terminal runs: scrubbed env, the account's
/// folder, a one-line banner, then the user's shell. Uses `env` explicitly
/// because some terminals (gnome-terminal) start shells from a server process
/// and would otherwise ignore our environment. Bash gets `--rcfile` so the
/// account's settings are applied after `~/.bashrc`; other shells read their
/// own start files last, and Health warns when those set these variables.
pub fn terminal_shell_command(
    account: &Account,
    bash_rc: Option<&Path>,
    home: &Path,
) -> Vec<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let mut argv = vec!["env".to_string()];
    for name in env::OVERRIDE_VARS {
        argv.push("-u".into());
        argv.push(name.into());
    }
    if env::is_default_config_dir(&account.code_dir, home) {
        argv.push("-u".into());
        argv.push("CLAUDE_CONFIG_DIR".into());
    } else {
        argv.push(format!("CLAUDE_CONFIG_DIR={}", account.code_dir.display()));
    }
    argv.push(format!("FACET_ACCOUNT={}", account.id));
    let is_bash = Path::new(&shell).file_name().is_some_and(|n| n == "bash");
    if let (true, Some(rc)) = (is_bash, bash_rc) {
        argv.push(shell);
        argv.push("--rcfile".into());
        argv.push(rc.display().to_string());
        argv.push("-i".into());
        return argv;
    }
    argv.push(shell.clone());
    argv.push("-c".into());
    argv.push(format!(
        "printf '\\033[1mFacet\\033[0m · this terminal is \\033[1m%s\\033[0m (CLAUDE_CONFIG_DIR=%s). Run claude to start. If your shell files set CLAUDE_CONFIG_DIR or an API key, they apply after this line.\\n' {} {}; exec {} -i",
        shell_quote(&safe_comment(&account.name)),
        shell_quote(&account.code_dir.display().to_string()),
        shell_quote(&shell)
    ));
    argv
}

/// One line, printable characters only (for comments and banners).
pub fn safe_comment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .replace('"', "'")
}

pub fn open_terminal(home: &Path, account: &Account, cwd: Option<&Path>) -> Result<u32> {
    let rc = bash_rc_path(home, account);
    if let Some(dir) = rc.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&rc, bash_rc(account, home))?;
    let (bin, sep) = TERMINALS
        .iter()
        .find_map(|(name, sep)| which(name).map(|p| (p, *sep)))
        .ok_or_else(|| Error::Unavailable("No supported terminal found (looked for ptyxis, gnome-terminal, kgx, konsole, xfce4-terminal, xterm).".into()))?;
    let mut cmd = env::command(bin);
    cmd.args(sep)
        .args(terminal_shell_command(account, Some(&rc), home));
    env::prepare_with_home(&mut cmd, &account.code_dir, home);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_detached(cmd)
}

/// Shell command for a terminal that first runs `run` (already-resolved
/// argv, e.g. the official claude with `--resume <id>`), then stays open.
pub fn terminal_running_command(
    account: &Account,
    home: &Path,
    run: &[String],
    bash_rc: Option<&Path>,
) -> Vec<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let mut argv = vec!["env".to_string()];
    for name in env::OVERRIDE_VARS {
        argv.push("-u".into());
        argv.push(name.into());
    }
    if env::is_default_config_dir(&account.code_dir, home) {
        argv.push("-u".into());
        argv.push("CLAUDE_CONFIG_DIR".into());
    } else {
        argv.push(format!("CLAUDE_CONFIG_DIR={}", account.code_dir.display()));
    }
    argv.push(format!("FACET_ACCOUNT={}", account.id));
    argv.push(shell.clone());
    argv.push("-c".into());
    let cmd: Vec<String> = run.iter().map(|a| shell_quote(a)).collect();
    // Shells read start files even for -c (zsh: ~/.zshenv), so scrub and set
    // the folder again right before the command (rule 6, compliance finding 1).
    let is_bash = Path::new(&shell).file_name().is_some_and(|n| n == "bash");
    let then = match (is_bash, bash_rc) {
        (true, Some(rc)) => format!(
            "exec {} --rcfile {} -i",
            shell_quote(&shell),
            shell_quote(&rc.display().to_string())
        ),
        _ => format!("exec {} -i", shell_quote(&shell)),
    };
    argv.push(format!(
        "unset {vars}; {select}; printf '\\033[1mFacet\\033[0m · %s\\n' {name}; {cmd}; {then}",
        vars = env::OVERRIDE_VARS.join(" "),
        select = env::config_dir_shell_line(&account.code_dir, home),
        name = shell_quote(&safe_comment(&account.name)),
        cmd = cmd.join(" "),
    ));
    argv
}

/// Open a terminal in `cwd` as the account, running `run` first.
pub fn open_terminal_running(
    home: &Path,
    account: &Account,
    cwd: &Path,
    run: &[String],
) -> Result<u32> {
    let (bin, sep) = TERMINALS
        .iter()
        .find_map(|(name, sep)| which(name).map(|p| (p, *sep)))
        .ok_or_else(|| Error::Unavailable("No supported terminal found (looked for ptyxis, gnome-terminal, kgx, konsole, xfce4-terminal, xterm).".into()))?;
    let rc = bash_rc_path(home, account);
    if let Some(dir) = rc.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&rc, bash_rc(account, home))?;
    let mut cmd = env::command(bin);
    cmd.args(sep)
        .args(terminal_running_command(account, home, run, Some(&rc)));
    env::prepare_with_home(&mut cmd, &account.code_dir, home);
    cmd.current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    spawn_detached(cmd)
}

pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// A `claude-<id>` command on PATH that runs the official `claude` in this
/// account (PRD M1). It only sets the folder and removes override variables.
pub fn shim_script(account: &Account, claude: &Path, home: &Path) -> String {
    format!(
        "#!/bin/sh\n# Written by Facet for the account \"{name}\". Runs the official Claude Code in this account.\n# Safe to delete; Facet can write it again.\nunset {vars}\n{select}\nFACET_ACCOUNT={id}\nexport FACET_ACCOUNT\nexec {claude} \"$@\"\n",
        name = safe_comment(&account.name),
        vars = env::OVERRIDE_VARS.join(" "),
        select = env::config_dir_shell_line(&account.code_dir, home),
        id = shell_quote(&account.id),
        claude = shell_quote(&claude.display().to_string()),
    )
}

pub fn shim_path(home: &Path, account: &Account) -> PathBuf {
    home.join(".local/bin")
        .join(format!("claude-{}", account.id))
}

/// Write the shim. Refuses to overwrite a file Facet did not write.
pub fn install_shim(home: &Path, account: &Account, claude: &Path) -> Result<PathBuf> {
    let path = shim_path(home, account);
    let name = format!("claude-{}", account.id);
    if let Some(other) = which(&name).filter(|p| p != &path) {
        return Err(Error::Invalid(format!(
            "A command called {name} already exists at {}. Facet will not hide it.",
            other.display()
        )));
    }
    if let Ok(existing) = fs::read_to_string(&path) {
        if !existing.contains("Written by Facet") {
            return Err(Error::Invalid(format!(
                "{} already exists and was not written by Facet. Rename or remove it first.",
                path.display()
            )));
        }
    }
    write_executable(&path, &shim_script(account, claude, home))?;
    Ok(path)
}

/// Menu entry that opens the account's Desktop instance through Facet
/// (`facet --launch <id>`), grouped in the dock by `StartupWMClass`.
/// Name in the dock and app menu: the account's own choice, else "<name> · Facet".
pub fn launcher_title(account: &Account) -> String {
    account
        .launcher_name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("{} · Facet", account.name))
        .replace(['\n', '\r'], " ")
}

/// `hidden`: no menu entry, only the dock identity for the account's windows.
pub fn desktop_entry(account: &Account, facet_exe: &Path, icon: &Path, hidden: bool) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={title}\nComment=Opens the Claude desktop app as {name} (unofficial launcher, not affiliated with Anthropic)\nExec={exe} --launch {id}\nIcon={icon}\nStartupWMClass={class}\nTerminal=false\nCategories=Utility;\n{nodisplay}X-Facet-Account={id}\n",
        title = launcher_title(account),
        icon = if account.launcher_icon == crate::profile::LauncherIcon::Claude {
            "claude-desktop".to_string()
        } else {
            icon.display().to_string()
        },
        nodisplay = if hidden { "NoDisplay=true\n" } else { "" },
        name = account.name.replace(['\n', '\r'], " "),
        exe = desktop_exec_quote(&facet_exe.display().to_string()),
        id = account.id,
        class = wm_class(account),
    )
}

fn desktop_exec_quote(s: &str) -> String {
    if s.chars()
        .any(|c| c.is_whitespace() || "\"'\\$`".contains(c))
    {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('$', "\\$")
                .replace('`', "\\`")
        )
    } else {
        s.to_string()
    }
}

/// Facet colours, mirrored from `docs/05-design/tokens.css` for the launcher
/// icons (a test checks they still match).
pub const FACET_HEX: [&str; 6] = [
    "#0FA595", "#7C4DEB", "#E23D63", "#D98300", "#2F7BEB", "#5E9E12",
];
pub const ON_FACET_HEX: [&str; 6] = [
    "#0D1015", "#FFFFFF", "#0D1015", "#0D1015", "#0D1015", "#0D1015",
];

/// Launcher icon in Facet's own style: the account's colour and initial
/// (or badge). Facet ships no Anthropic logo (rule 8); the `ClaudeBadge`
/// style is composed on this computer from the icon the Claude package
/// installed (see `avatar::launcher_svg`).
pub fn account_icon_svg(account: &Account) -> String {
    let mut a = account.clone();
    a.launcher_icon = crate::profile::LauncherIcon::Avatar;
    crate::avatar::launcher_svg(&a, None, None)
}

pub fn launcher_path(home: &Path, id: &str) -> PathBuf {
    home.join(format!(".local/share/applications/facet-{id}.desktop"))
}

pub fn launcher_icon_path(home: &Path, id: &str) -> PathBuf {
    home.join(format!(".local/share/facet/icons/facet-{id}.svg"))
}

/// A launcher that only gives the dock an icon and name (not in the menu).
pub fn launcher_hidden(home: &Path, id: &str) -> bool {
    fs::read_to_string(launcher_path(home, id)).is_ok_and(|t| t.contains("\nNoDisplay=true\n"))
}

/// Remove a launcher and its icon, only when Facet wrote them.
pub fn remove_launcher(home: &Path, id: &str) {
    let entry = launcher_path(home, id);
    if fs::read_to_string(&entry).is_ok_and(|t| t.contains("X-Facet-Account=")) {
        let _ = fs::remove_file(&entry);
    }
    let _ = fs::remove_file(launcher_icon_path(home, id));
}

/// Remove a `claude-<id>` command, only when Facet wrote it.
pub fn remove_shim(home: &Path, id: &str) {
    let path = home.join(".local/bin").join(format!("claude-{id}"));
    if fs::read_to_string(&path).is_ok_and(|t| t.contains("Written by Facet")) {
        let _ = fs::remove_file(&path);
    }
}

pub fn install_launcher(
    home: &Path,
    account: &Account,
    facet_exe: &Path,
    icon_svg: &str,
    hidden: bool,
) -> Result<PathBuf> {
    let icon = launcher_icon_path(home, &account.id);
    if let Some(dir) = icon.parent() {
        fs::create_dir_all(dir)?;
    }
    write_atomic(&icon, icon_svg)?;
    let entry = launcher_path(home, &account.id);
    if let Some(dir) = entry.parent() {
        fs::create_dir_all(dir)?;
    }
    write_atomic(&entry, &desktop_entry(account, facet_exe, &icon, hidden))?;
    let _ = env::command("update-desktop-database")
        .arg(entry.parent().unwrap_or(home))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(entry)
}

/// Write through a temp file and rename, so a link at `path` is replaced,
/// never followed.
fn write_atomic(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_extension("facet-tmp");
    let _ = fs::remove_file(&tmp);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn write_executable(path: &Path, text: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("facet-tmp");
    fs::write(&tmp, text)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// The quick switcher can bind a real global shortcut when the portal exists.
pub fn global_shortcut_support() -> ShortcutSupport {
    let ok = env::command("gdbus")
        .args([
            "introspect",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
        ])
        .stderr(Stdio::null())
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout).contains("org.freedesktop.portal.GlobalShortcuts")
        })
        .unwrap_or(false);
    if ok {
        ShortcutSupport::Portal
    } else {
        ShortcutSupport::CliOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: &str, desktop: Option<&str>, code: &str) -> Account {
        Account {
            id: id.into(),
            name: id.to_uppercase(),
            color: 2,
            plan: None,
            desktop_dir: desktop.map(PathBuf::from),
            code_dir: PathBuf::from(code),
            expected_email: None,
            tint: true,
            x11_identity: true,
            created_at: 0,
            badge: None,
            image: None,
            notes: None,
            launcher_icon: Default::default(),
            terminal_dir: None,
            launcher_name: None,
        }
    }

    const HOME: &str = "/home/u";

    #[test]
    fn parses_main_process_with_and_without_user_data_dir() {
        let home = Path::new(HOME);
        let work = b"/usr/lib/claude-desktop/claude-desktop --user-data-dir=/home/u/.config/Claude-Work --ozone-platform=x11 ";
        assert_eq!(
            parse_desktop_cmdline(work, home),
            Some((PathBuf::from("/home/u/.config/Claude-Work"), true))
        );
        let personal = b"/usr/lib/claude-desktop/claude-desktop\0";
        assert_eq!(
            parse_desktop_cmdline(personal, home),
            Some((PathBuf::from("/home/u/.config/Claude"), false))
        );
    }

    #[test]
    fn ignores_helpers_and_other_programs() {
        let home = Path::new(HOME);
        assert_eq!(
            parse_desktop_cmdline(
                b"/usr/lib/claude-desktop/claude-desktop --type=renderer --user-data-dir=/x",
                home
            ),
            None
        );
        assert_eq!(
            parse_desktop_cmdline(
                b"/usr/lib/claude-desktop/resources/cowork-linux-helper -socket /run/x.sock",
                home
            ),
            None
        );
        assert_eq!(
            parse_desktop_cmdline(b"/bin/bash /home/u/.local/bin/claude-desktop-work", home),
            None
        );
        assert_eq!(parse_desktop_cmdline(b"", home), None);
    }

    #[test]
    fn user_data_dir_with_spaces_and_nul_separated_args() {
        let home = Path::new(HOME);
        let raw = b"/usr/lib/claude-desktop/claude-desktop\0--user-data-dir=/home/u/Claude Client A\0--ozone-platform=x11\0";
        assert_eq!(
            parse_desktop_cmdline(raw, home),
            Some((PathBuf::from("/home/u/Claude Client A"), true))
        );
    }

    #[test]
    fn scans_a_fake_proc_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let proc_root = tmp.path();
        let write = |pid: &str, comm: &str, cmd: &[u8]| {
            fs::create_dir_all(proc_root.join(pid)).unwrap();
            fs::write(proc_root.join(pid).join("comm"), format!("{comm}\n")).unwrap();
            fs::write(proc_root.join(pid).join("cmdline"), cmd).unwrap();
        };
        write(
            "100",
            "claude-desktop",
            b"/usr/lib/claude-desktop/claude-desktop\0",
        );
        write("200", "claude-desktop", b"/usr/lib/claude-desktop/claude-desktop --user-data-dir=/home/u/.config/Claude-Work --ozone-platform=x11");
        write(
            "201",
            "claude-desktop",
            b"/usr/lib/claude-desktop/claude-desktop --type=gpu-process",
        );
        // Another program whose command line mentions claude-desktop: its
        // comm differs, so its command line is never used.
        write(
            "300",
            "bash",
            b"/usr/lib/claude-desktop/claude-desktop --user-data-dir=/x",
        );
        write("self", "x", b"ignored");
        let found = running_desktops(proc_root, Path::new(HOME));
        assert_eq!(found.len(), 2);
        let work = account(
            "work",
            Some("/home/u/.config/Claude-Work/"),
            "/home/u/.claude-work",
        );
        assert_eq!(instance_for(&work, &found).map(|p| p.pid), Some(200));
        let code_only = account("ci", None, "/home/u/.claude-ci");
        assert!(instance_for(&code_only, &found).is_none());
    }

    #[test]
    fn desktop_command_sets_folder_scrubs_env_and_skips_default_dir_flag() {
        let install = ClaudeInstall {
            path: "/usr/lib/claude-desktop/claude-desktop".into(),
            version: None,
            channel: "deb".into(),
            official: true,
        };
        let home = Path::new(HOME);
        let work = account(
            "work",
            Some("/home/u/.config/Claude-Work"),
            "/home/u/.claude-work",
        );
        let cmd = desktop_command(&install, &work, home).unwrap();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "--user-data-dir=/home/u/.config/Claude-Work",
                "--ozone-platform=x11"
            ]
        );
        let envs: Vec<(String, Option<String>)> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        assert!(envs.contains(&(
            "CLAUDE_CONFIG_DIR".into(),
            Some("/home/u/.claude-work".into())
        )));
        for name in env::OVERRIDE_VARS {
            assert!(
                envs.contains(&(name.to_string(), None)),
                "{name} not removed"
            );
        }
        let mut personal = account(
            "personal",
            Some("/home/u/.config/Claude"),
            "/home/u/.claude",
        );
        personal.x11_identity = false;
        let cmd = desktop_command(&install, &personal, home).unwrap();
        assert_eq!(cmd.get_args().count(), 0);
        let code_only = account("ci", None, "/home/u/.claude-ci");
        assert!(desktop_command(&install, &code_only, home).is_err());
    }

    #[test]
    fn terminal_command_scrubs_and_quotes() {
        let mut acc = account("client-a", None, "/home/u/.claude-client a");
        acc.name = "Client's A".into();
        let argv = terminal_shell_command(&acc, None, Path::new("/home/u"));
        assert_eq!(argv[0], "env");
        for name in env::OVERRIDE_VARS {
            assert!(argv.windows(2).any(|w| w[0] == "-u" && w[1] == name));
        }
        assert!(argv.contains(&"CLAUDE_CONFIG_DIR=/home/u/.claude-client a".to_string()));
        let script = argv.last().unwrap();
        assert!(script.contains("'Client'\\''s A'"), "{script}");
    }

    #[test]
    fn resume_terminal_runs_the_command_scrubbed_and_quoted() {
        let acc = account("work", None, "/home/u/.claude-work");
        let run = vec![
            "/usr/bin/claude".to_string(),
            "--resume".into(),
            "abc-1; rm -rf /".into(),
        ];
        let argv = terminal_running_command(
            &acc,
            Path::new("/home/u"),
            &run,
            Some(Path::new("/r/work.bashrc")),
        );
        for name in env::OVERRIDE_VARS {
            assert!(argv.windows(2).any(|w| w[0] == "-u" && w[1] == name));
        }
        let script = argv.last().unwrap();
        assert!(
            script.contains("'/usr/bin/claude' '--resume' 'abc-1; rm -rf /'"),
            "{script}"
        );
        // The script is valid shell: run it with a harmless stand-in.
        let out = std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(script)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn xdotool_is_limited_to_search_and_set_window() {
        for args in [
            xdotool_search_args(7),
            xdotool_relabel_args("facet-work", "123"),
        ] {
            assert!(XDOTOOL_ALLOWED.contains(&args[0].as_str()));
            for banned in [
                "key",
                "type",
                "click",
                "mousemove",
                "windowactivate",
                "windowfocus",
            ] {
                assert!(!args.iter().any(|a| a == banned), "{banned} in {args:?}");
            }
        }
        // The runner refuses anything else, even if a future caller tries.
        assert!(run_xdotool(Path::new("/bin/true"), &["key".into(), "ctrl+v".into()]).is_none());
    }

    #[test]
    fn bash_terminal_reapplies_account_after_bashrc() {
        let mut acc = account("work", None, "/home/u/.claude-work");
        acc.name = "Work\nrm -rf ~".into();
        let rc = bash_rc(&acc, Path::new("/home/u"));
        let source = rc.find(". \"$HOME/.bashrc\"").unwrap();
        let unset = rc
            .find("unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN CLAUDE_CODE_OAUTH_TOKEN")
            .unwrap();
        let export = rc
            .find("export CLAUDE_CONFIG_DIR='/home/u/.claude-work'")
            .unwrap();
        assert!(
            source < unset && unset < export,
            "account settings must come after ~/.bashrc"
        );
        assert!(
            !rc.contains("\nrm -rf"),
            "names must not inject lines: {rc}"
        );
        std::env::set_var("SHELL", "/bin/bash");
        let argv = terminal_shell_command(
            &acc,
            Some(Path::new("/tmp/work.bashrc")),
            Path::new("/home/u"),
        );
        assert_eq!(
            &argv[argv.len() - 4..],
            ["/bin/bash", "--rcfile", "/tmp/work.bashrc", "-i"]
        );
    }

    #[test]
    fn official_build_detection() {
        assert!(is_official_maintainer(
            "Anthropic PBC <support@anthropic.com>"
        ));
        assert!(!is_official_maintainer("Some Packager <me@example.com>"));
        assert!(!is_official_maintainer(""));
        assert!(is_official_code_path(Path::new(
            "/home/u/.local/share/claude/versions/2.1.310"
        )));
        assert!(is_official_code_path(Path::new(
            "/usr/lib/node_modules/@anthropic-ai/claude-code/cli.js"
        )));
        assert!(!is_official_code_path(Path::new(
            "/home/u/bin/claude-wrapper"
        )));
    }

    #[test]
    fn shim_runs_official_claude_in_the_account_only() {
        let acc = account("work", None, "/home/u/.claude-work");
        let s = shim_script(
            &acc,
            Path::new("/home/u/.local/bin/claude"),
            Path::new("/home/u"),
        );
        let personal = account("personal", None, "/home/u/.claude");
        let p = shim_script(
            &personal,
            Path::new("/usr/bin/claude"),
            Path::new("/home/u"),
        );
        assert!(p.contains("\nunset CLAUDE_CONFIG_DIR\n"), "{p}");
        let argv = terminal_shell_command(&personal, None, Path::new("/home/u"));
        assert!(argv
            .windows(2)
            .any(|w| w[0] == "-u" && w[1] == "CLAUDE_CONFIG_DIR"));
        assert!(s.starts_with("#!/bin/sh\n"));
        assert!(
            s.contains("unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN CLAUDE_CODE_OAUTH_TOKEN\n")
        );
        assert!(s.contains("export CLAUDE_CONFIG_DIR='/home/u/.claude-work'\n"));
        assert!(s.ends_with("exec '/home/u/.local/bin/claude' \"$@\"\n"));
    }

    #[test]
    fn shim_never_overwrites_foreign_files() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let acc = account("work", None, "/x");
        let path = shim_path(home, &acc);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "#!/bin/sh\necho mine\n").unwrap();
        assert!(install_shim(home, &acc, Path::new("/usr/bin/claude")).is_err());
        fs::remove_file(&path).unwrap();
        install_shim(home, &acc, Path::new("/usr/bin/claude")).unwrap();
        install_shim(home, &acc, Path::new("/usr/bin/claude")).unwrap(); // own file: ok
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[test]
    fn desktop_entry_has_no_claude_branding_in_name_and_groups_by_class() {
        let acc = account("work", Some("/d"), "/c");
        let e = desktop_entry(
            &acc,
            Path::new("/opt/Facet App/facet"),
            Path::new("/i/facet-work.svg"),
            false,
        );
        assert!(!e.contains("NoDisplay"));
        let mut named = acc.clone();
        named.launcher_name = Some("Claude Dev Wider".into());
        let hidden = desktop_entry(
            &named,
            Path::new("/usr/bin/facet"),
            Path::new("/i/x.svg"),
            true,
        );
        assert!(hidden.contains("Name=Claude Dev Wider\n") && hidden.contains("NoDisplay=true\n"));
        assert!(e.contains("Name=WORK · Facet\n"));
        assert!(e.contains("Exec=\"/opt/Facet App/facet\" --launch work\n"));
        assert!(e.contains("StartupWMClass=facet-work\n"));
        assert!(e.contains("not affiliated with Anthropic"));
    }

    #[test]
    fn icon_colours_match_design_tokens() {
        let css = fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/05-design/tokens.css"
        ))
        .unwrap();
        for (i, hex) in FACET_HEX.iter().enumerate() {
            let needle = format!("--facet-{}: {};", i + 1, hex);
            assert!(css.contains(&needle), "tokens.css changed: {needle}");
            let on = format!("--on-facet-{}: {};", i + 1, ON_FACET_HEX[i]);
            assert!(css.contains(&on), "tokens.css changed: {on}");
        }
        let svg = account_icon_svg(&account("<x>", None, "/c"));
        assert!(svg.contains("&lt;") || svg.contains(">X<"), "{svg}");
    }
}
