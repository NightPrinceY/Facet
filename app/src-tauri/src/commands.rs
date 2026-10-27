//! Tauri commands: the whole contract between the UI and the Rust core.
//! Mirrored in `app/src/lib/api.ts`. Every command is triggered by a user
//! action in the UI; none sends anything to Anthropic.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::adapter::{linux, ClaudeInstall, ShortcutSupport};
use crate::auth::{self, AuthStatus};
use crate::avatar;
use crate::control;
use crate::error::{Error, Result};
use crate::health::{self, Check};
use crate::launched::{self, Launch, LaunchFile};
use crate::profile::{self, Account, AccountInput, DetectedSetup, Store};
use crate::registry;
use crate::rules::{self, FolderRule, RuleStore};
use crate::studio;
use crate::usage;

pub struct AppState {
    pub store: Store,
    pub rules: RuleStore,
    pub home: PathBuf,
    /// Desktop instances Facet started (shared with `facet --launch`).
    pub launched: LaunchFile,
    /// Facet was started with `--switcher` (UI opens the quick switcher once).
    pub startup_switcher: Mutex<bool>,
    /// Transcript digests, refreshed only for files that changed.
    pub usage: usage::Cache,
}

impl AppState {
    pub fn new(store: Store, home: PathBuf) -> Self {
        let rules_path = store
            .path()
            .parent()
            .map(|d| d.join("rules.toml"))
            .unwrap_or_else(|| home.join(".config/facet/rules.toml"));
        let launched = LaunchFile::new(launched::default_path(&home));
        Self {
            store,
            rules: RuleStore::new(rules_path),
            launched,
            home,
            startup_switcher: Mutex::new(false),
            usage: usage::Cache::default(),
        }
    }

    /// Account id → pid of instances Facet started that are still running.
    pub fn launched_map(&self) -> BTreeMap<String, u32> {
        self.launched
            .load(Path::new("/proc"))
            .into_iter()
            .map(|(k, v)| (k, v.pid))
            .collect()
    }

    fn launched_pids(&self) -> BTreeSet<u32> {
        self.launched_map().into_values().collect()
    }

    fn record_launch(&self, account_id: &str, pid: u32) {
        let mut all = self.launched.load(Path::new("/proc"));
        if let Some(start_ticks) = launched::start_ticks(Path::new("/proc"), pid) {
            all.insert(account_id.to_string(), Launch { pid, start_ticks });
            self.launched.save(&all);
        }
    }

    fn forget_launch(&self, account_id: &str) {
        let mut all = self.launched.load(Path::new("/proc"));
        all.remove(account_id);
        self.launched.save(&all);
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub desktop: Option<ClaudeInstall>,
    pub code: Option<ClaudeInstall>,
    pub shortcut: ShortcutSupport,
    pub store_path: PathBuf,
    pub home: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountState {
    pub id: String,
    pub desktop_pid: Option<u32>,
    pub launched_by_facet: bool,
    /// Claude Code sessions registered in this account's folder right now.
    pub live_sessions: usize,
    /// Of those, how many are waiting for the user (registry `status`).
    pub waiting_sessions: usize,
    pub shim_installed: bool,
    pub launcher_installed: bool,
}

pub fn account_states(state: &AppState) -> Result<Vec<AccountState>> {
    let accounts = state.store.load()?;
    let running = linux::running_desktops(Path::new("/proc"), &state.home);
    let launched = state.launched_map();
    Ok(accounts
        .iter()
        .map(|a| {
            let pid = linux::instance_for(a, &running).map(|p| p.pid);
            let sessions = registry::live_sessions(&a.code_dir, registry::pid_alive);
            AccountState {
                id: a.id.clone(),
                desktop_pid: pid,
                launched_by_facet: pid.is_some() && launched.get(&a.id) == pid.as_ref(),
                live_sessions: sessions.len(),
                waiting_sessions: sessions
                    .iter()
                    .filter(|s| {
                        matches!(
                            s.status.as_deref(),
                            Some("waiting" | "needs-input" | "blocked")
                        )
                    })
                    .count(),
                shim_installed: linux::shim_path(&state.home, a).is_file(),
                launcher_installed: linux::launcher_path(&state.home, &a.id).is_file()
                    && !linux::launcher_hidden(&state.home, &a.id),
            }
        })
        .collect())
}

#[tauri::command]
pub fn app_info(state: tauri::State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        desktop: linux::find_claude_desktop(),
        code: linux::find_claude_code(),
        shortcut: linux::global_shortcut_support(),
        store_path: state.store.path().to_path_buf(),
        home: state.home.clone(),
    }
}

/// An account as the UI sees it: the stored fields plus its picture.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(flatten)]
    pub account: Account,
    /// `data:` URL of the account picture, if one is set.
    pub image_url: Option<String>,
}

fn view(state: &AppState, account: Account) -> AccountView {
    let image_url = account
        .image
        .as_deref()
        .and_then(|n| avatar::data_url(&avatar::avatars_dir(&state.home), n));
    AccountView { account, image_url }
}

#[tauri::command]
pub fn list_accounts(state: tauri::State<'_, AppState>) -> Result<Vec<AccountView>> {
    Ok(state
        .store
        .load()?
        .into_iter()
        .map(|a| view(&state, a))
        .collect())
}

/// The icon the menu and dock show for this account.
pub fn launcher_svg(state: &AppState, account: &Account) -> String {
    if account.launcher_icon == profile::LauncherIcon::Claude {
        // The installed Claude icon itself (preview only; the launcher names it).
        if let Some(png) = avatar::installed_claude_icon() {
            return format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"0 0 128 128\"><image x=\"0\" y=\"0\" width=\"128\" height=\"128\" xlink:href=\"data:image/png;base64,{}\"/></svg>\n",
                avatar::base64(&png)
            );
        }
    }
    let picture = account
        .image
        .as_deref()
        .and_then(|n| avatar::load(&avatar::avatars_dir(&state.home), n));
    let claude = avatar::installed_claude_icon();
    avatar::launcher_svg(
        account,
        picture.as_ref().map(|(m, b)| (*m, b.as_slice())),
        claude.as_deref(),
    )
}

/// Preview of the launcher and dock icon, as a `data:` URL. The optional
/// values preview unsaved edits from the account form.
#[tauri::command]
pub fn launcher_preview(
    state: tauri::State<'_, AppState>,
    id: String,
    style: Option<profile::LauncherIcon>,
    badge: Option<String>,
    color: Option<u8>,
    name: Option<String>,
) -> Result<String> {
    let mut account = state.store.get(&id)?;
    if let Some(s) = style {
        account.launcher_icon = s;
    }
    if let Some(b) = badge {
        account.badge = profile::validate_badge(&b).ok();
    }
    if let Some(c) = color.filter(|c| (1..=profile::FACET_COLORS).contains(c)) {
        account.color = c;
    }
    if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
        account.name = n;
    }
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        avatar::base64(launcher_svg(&state, &account).as_bytes())
    ))
}

/// Rewrite what Facet installed for an account (command, launcher) so it
/// follows a changed name, folder, icon or id. Skips what is not installed.
fn refresh_installed(state: &AppState, account: &Account) {
    if linux::shim_path(&state.home, account).is_file() {
        if let Some(claude) = linux::find_claude_code() {
            let _ = linux::install_shim(&state.home, account, &claude.path);
        }
    }
    if account.desktop_dir.is_some() && linux::launcher_path(&state.home, &account.id).is_file() {
        if let Ok(exe) = std::env::current_exe() {
            let hidden = linux::launcher_hidden(&state.home, &account.id);
            let _ = linux::install_launcher(
                &state.home,
                account,
                &exe,
                &launcher_svg(state, account),
                hidden,
            );
        }
    } else if account.desktop_dir.is_none() {
        linux::remove_launcher(&state.home, &account.id);
    }
}

/// Folders may change only while nothing of that account runs, because a
/// running Desktop or Code session keeps writing to the old folder.
fn check_folder_change(state: &AppState, old: &Account, input: &AccountInput) -> Result<()> {
    let code_changed = input.code_dir.as_ref().is_some_and(|d| d != &old.code_dir);
    let desktop_changed = match (input.with_desktop, &old.desktop_dir) {
        (false, Some(_)) | (true, None) => true,
        (true, Some(d)) => input.desktop_dir.as_ref().is_some_and(|n| n != d),
        (false, None) => false,
    };
    if !code_changed && !desktop_changed {
        return Ok(());
    }
    let running = linux::running_desktops(Path::new("/proc"), &state.home);
    if linux::instance_for(old, &running).is_some() {
        return Err(Error::Invalid(format!(
            "Quit {}'s Desktop app before changing its folders.",
            old.name
        )));
    }
    if code_changed && !registry::live_sessions(&old.code_dir, registry::pid_alive).is_empty() {
        return Err(Error::Invalid(format!(
            "Close {}'s Claude Code sessions before changing its Claude Code folder.",
            old.name
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn get_account_states(state: tauri::State<'_, AppState>) -> Result<Vec<AccountState>> {
    account_states(&state)
}

#[tauri::command]
pub fn save_account(state: tauri::State<'_, AppState>, input: AccountInput) -> Result<AccountView> {
    if let Some(old) = input.id.as_deref().and_then(|id| state.store.get(id).ok()) {
        check_folder_change(&state, &old, &input)?;
    }
    let saved = state.store.upsert(input, &state.home, now_ms())?;
    refresh_installed(&state, &saved);
    Ok(view(&state, saved))
}

/// Set (base64 picture) or clear (`None`) the account picture.
#[tauri::command]
pub fn set_account_image(
    state: tauri::State<'_, AppState>,
    id: String,
    data: Option<String>,
) -> Result<AccountView> {
    let old = state.store.get(&id)?;
    let dir = avatar::avatars_dir(&state.home);
    let name = match data {
        Some(b64) => {
            let b64 = b64.split_once(',').map(|(_, d)| d).unwrap_or(&b64);
            Some(avatar::save(&dir, &id, &avatar::base64_decode(b64)?)?)
        }
        None => None,
    };
    let saved = state.store.set_image(&id, name.clone())?;
    if let Some(prev) = old.image.filter(|p| Some(p) != name.as_ref()) {
        avatar::remove(&dir, &prev);
    }
    refresh_installed(&state, &saved);
    Ok(view(&state, saved))
}

/// Put the accounts in this order (rail, switcher, tray).
#[tauri::command]
pub fn reorder_accounts(
    state: tauri::State<'_, AppState>,
    ids: Vec<String>,
) -> Result<Vec<AccountView>> {
    Ok(state
        .store
        .reorder(&ids)?
        .into_iter()
        .map(|a| view(&state, a))
        .collect())
}

/// Change an account's short name (its id): moves the `claude-<id>` command,
/// the launcher, folder rules and backups along with it. Its Claude folders
/// do not change.
#[tauri::command]
pub fn rename_account(
    state: tauri::State<'_, AppState>,
    id: String,
    new_id: String,
) -> Result<AccountView> {
    let new_id = new_id.trim().to_string();
    if new_id == id {
        return Ok(view(&state, state.store.get(&id)?));
    }
    let old = state.store.get(&id)?;
    let running = linux::running_desktops(Path::new("/proc"), &state.home);
    if linux::instance_for(&old, &running).is_some() {
        return Err(Error::Invalid(format!(
            "Quit {}'s Desktop app first: its windows are named after the old short name.",
            old.name
        )));
    }
    let had_shim = linux::shim_path(&state.home, &old).is_file();
    let had_launcher = linux::launcher_path(&state.home, &old.id).is_file();
    if had_shim {
        if let Some(other) = linux::which(&format!("claude-{new_id}")) {
            return Err(Error::Invalid(format!(
                "A command called claude-{new_id} already exists at {}. Pick another short name.",
                other.display()
            )));
        }
    }
    // Install the new command and launcher first, then switch the store, so a
    // failure never leaves the account half-renamed (compliance finding 8).
    profile::validate_id(&new_id)?;
    let mut next = old.clone();
    next.id = new_id.clone();
    let undo = |state: &AppState| {
        if had_shim {
            linux::remove_shim(&state.home, &new_id);
        }
        if had_launcher {
            linux::remove_launcher(&state.home, &new_id);
        }
    };
    let installed = (|| -> Result<()> {
        if had_shim {
            let claude = linux::find_claude_code()
                .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
            linux::install_shim(&state.home, &next, &claude.path)?;
        }
        if had_launcher {
            let exe = std::env::current_exe()?;
            let hidden = linux::launcher_hidden(&state.home, &old.id);
            linux::install_launcher(
                &state.home,
                &next,
                &exe,
                &launcher_svg(&state, &next),
                hidden,
            )?;
        }
        Ok(())
    })();
    if let Err(e) = installed {
        undo(&state);
        return Err(e);
    }
    let renamed = match state.store.rename_id(&id, &new_id) {
        Ok(r) => r,
        Err(e) => {
            undo(&state);
            return Err(e);
        }
    };
    if had_shim {
        linux::remove_shim(&state.home, &id);
    }
    if had_launcher {
        linux::remove_launcher(&state.home, &id);
    }

    // Folder rules point at the id.
    if let Ok(mut rules) = state.rules.load() {
        if rules.iter().any(|r| r.account_id == id) {
            for r in rules.iter_mut().filter(|r| r.account_id == id) {
                r.account_id = new_id.clone();
            }
            let accounts = state.store.load()?;
            state.rules.save(&rules, &accounts, &state.home)?;
            if rules::auto_script_path(&state.home).is_file() {
                if let Some(claude) = linux::find_claude_code() {
                    let _ =
                        rules::install_auto_script(&state.home, &rules, &accounts, &claude.path);
                }
            }
        }
    }
    // Backups follow the account.
    let (from, to) = (
        control::backup_dir(&state.home, &id),
        control::backup_dir(&state.home, &new_id),
    );
    if from.is_dir() && !to.exists() {
        let _ = fs::rename(&from, &to);
    }
    state.forget_launch(&id);
    // The account's browser profile choice follows it.
    let mut links = crate::browser::load(&state.home);
    if let Some(t) = links.accounts.remove(&id) {
        links.accounts.insert(new_id.clone(), t);
        let _ = crate::browser::save(&state.home, &links);
    }
    let _ = fs::remove_file(linux::bash_rc_path(&state.home, &old));
    Ok(view(&state, renamed))
}

/// Forget an account in Facet. Its folders and sign-in stay as they are.
#[tauri::command]
pub fn delete_account(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    state.store.remove(&id)
}

/// Folder pairs on disk that no account uses yet.
#[tauri::command]
pub fn detect_existing(state: tauri::State<'_, AppState>) -> Result<Vec<DetectedSetup>> {
    let accounts = state.store.load()?;
    let used = |p: &Option<PathBuf>| {
        p.as_ref().is_some_and(|p| {
            accounts
                .iter()
                .any(|a| &a.code_dir == p || a.desktop_dir.as_ref() == Some(p))
        })
    };
    Ok(profile::detect_existing(&state.home)
        .into_iter()
        .filter(|d| !used(&d.code_dir) && !used(&d.desktop_dir))
        .collect())
}

/// Start the account's Desktop instance, or bring it forward if it runs.
pub fn launch(state: &AppState, id: &str) -> Result<u32> {
    let account = state.store.get(id)?;
    let install = linux::find_claude_desktop()
        .ok_or_else(|| Error::Unavailable("Claude Desktop is not installed.".into()))?;
    let running = linux::running_desktops(Path::new("/proc"), &state.home);
    let already = linux::instance_for(&account, &running).map(|p| p.pid);
    ensure_dock_identity(state, &account);
    let cmd = linux::desktop_command(&install, &account, &state.home)?;
    let spawned = linux::spawn_detached(cmd)?;
    if already.is_some() {
        // Electron forwards the second start to the running window and exits.
        return Ok(already.unwrap_or(spawned));
    }
    // The main process may re-exec; find its real pid, then relabel windows.
    let home = state.home.clone();
    let mut pid = spawned;
    for _ in 0..40 {
        let running = linux::running_desktops(Path::new("/proc"), &home);
        if let Some(p) = linux::instance_for(&account, &running) {
            pid = p.pid;
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    state.record_launch(&account.id, pid);
    if account.x11_identity {
        let acc = account.clone();
        std::thread::spawn(move || linux::relabel_windows(&acc, &home, Duration::from_secs(30)));
    }
    Ok(pid)
}

/// Windows Facet relabels to `facet-<id>` need a launcher with that class, or
/// the dock shows a blank icon and the class name. Write a hidden one when
/// the user has not added a menu entry.
pub fn ensure_dock_identity(state: &AppState, account: &Account) {
    if !account.x11_identity || linux::launcher_path(&state.home, &account.id).is_file() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        let _ = linux::install_launcher(
            &state.home,
            account,
            &exe,
            &launcher_svg(state, account),
            true,
        );
    }
}

#[tauri::command]
pub async fn launch_desktop(state: tauri::State<'_, AppState>, id: String) -> Result<u32> {
    launch(&state, &id)
}

#[tauri::command]
pub fn quit_desktop(state: tauri::State<'_, AppState>, id: String) -> Result<()> {
    let account = state.store.get(&id)?;
    let running = linux::running_desktops(Path::new("/proc"), &state.home);
    let p = linux::instance_for(&account, &running)
        .ok_or_else(|| Error::NotFound(format!("a running Desktop app for {}", account.name)))?;
    linux::quit(p.pid)?;
    state.forget_launch(&id);
    Ok(())
}

#[tauri::command]
pub fn open_terminal(
    state: tauri::State<'_, AppState>,
    id: String,
    cwd: Option<PathBuf>,
) -> Result<u32> {
    let account = state.store.get(&id)?;
    let dir = cwd
        .or_else(|| account.terminal_dir.clone().filter(|d| d.is_dir()))
        .unwrap_or_else(|| state.home.clone());
    linux::open_terminal(&state.home, &account, Some(&dir))
}

#[tauri::command]
pub fn install_shim(state: tauri::State<'_, AppState>, id: String) -> Result<PathBuf> {
    let account = state.store.get(&id)?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    linux::install_shim(&state.home, &account, &claude.path)
}

#[tauri::command]
pub fn install_launcher(state: tauri::State<'_, AppState>, id: String) -> Result<PathBuf> {
    let account = state.store.get(&id)?;
    if account.desktop_dir.is_none() {
        return Err(Error::Invalid(format!(
            "{} has no Desktop app set up.",
            account.name
        )));
    }
    let exe = std::env::current_exe()?;
    linux::install_launcher(
        &state.home,
        &account,
        &exe,
        &launcher_svg(&state, &account),
        false,
    )
}

#[tauri::command]
pub async fn check_sign_in(state: tauri::State<'_, AppState>, id: String) -> Result<AuthStatus> {
    let account = state.store.get(&id)?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    let code_dir = account.code_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        auth::check(&claude.path, &code_dir, Duration::from_secs(15))
    })
    .await
    .map_err(|e| Error::Unavailable(e.to_string()))?
}

#[tauri::command]
pub async fn run_health(
    state: tauri::State<'_, AppState>,
    check_sign_in: bool,
) -> Result<Vec<Check>> {
    let accounts = state.store.load()?;
    let home = state.home.clone();
    let launched = state.launched_pids();
    tauri::async_runtime::spawn_blocking(move || {
        health::evaluate(&health::gather(&home, &accounts, launched, check_sign_in))
    })
    .await
    .map_err(|e| Error::Unavailable(e.to_string()))
}

#[tauri::command]
pub fn list_rules(state: tauri::State<'_, AppState>) -> Result<Vec<FolderRule>> {
    state.rules.load()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesSaved {
    pub rules: Vec<FolderRule>,
    /// Set when `claude-auto` exists and was rewritten with the new rules.
    pub command_updated: Option<PathBuf>,
}

/// Save all folder rules; keeps `claude-auto` in step if it is installed.
#[tauri::command]
pub fn save_rules(state: tauri::State<'_, AppState>, rules: Vec<FolderRule>) -> Result<RulesSaved> {
    let accounts = state.store.load()?;
    let saved = state.rules.save(&rules, &accounts, &state.home)?;
    let command_updated = if rules::auto_script_path(&state.home).is_file() {
        let claude = linux::find_claude_code()
            .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
        Some(rules::install_auto_script(
            &state.home,
            &saved,
            &accounts,
            &claude.path,
        )?)
    } else {
        None
    };
    Ok(RulesSaved {
        rules: saved,
        command_updated,
    })
}

#[tauri::command]
pub fn install_auto_command(state: tauri::State<'_, AppState>) -> Result<PathBuf> {
    let accounts = state.store.load()?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    rules::install_auto_script(&state.home, &state.rules.load()?, &accounts, &claude.path)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCommandInfo {
    pub path: PathBuf,
    pub installed: bool,
}

#[tauri::command]
pub fn auto_command_info(state: tauri::State<'_, AppState>) -> AutoCommandInfo {
    let path = rules::auto_script_path(&state.home);
    AutoCommandInfo {
        installed: path.is_file(),
        path,
    }
}

/// True once if Facet was started (or re-invoked) with `--switcher`.
#[tauri::command]
pub fn take_switcher_request(state: tauri::State<'_, AppState>) -> bool {
    state
        .startup_switcher
        .lock()
        .map(|mut v| std::mem::replace(&mut *v, false))
        .unwrap_or(false)
}

/// UTC timestamp for backup names, e.g. `20261026T031500Z-123` (ms suffix
/// keeps two saves in one second apart).
pub fn stamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z-{:03}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        now.subsec_millis()
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlState {
    pub current: control::Current,
    /// A managed policy file exists; it wins over these settings and is
    /// never edited by Facet.
    pub managed_path: Option<PathBuf>,
}

#[tauri::command]
pub fn control_read(state: tauri::State<'_, AppState>, id: String) -> Result<ControlState> {
    let account = state.store.get(&id)?;
    let managed = PathBuf::from(control::MANAGED_SETTINGS_LINUX);
    Ok(ControlState {
        current: control::read_current(&account.code_dir)?,
        managed_path: managed.is_file().then_some(managed),
    })
}

fn control_context(state: &AppState, account: &Account) -> control::Context {
    let others = state.store.load().unwrap_or_default();
    control::Context {
        live_sessions: registry::live_sessions(&account.code_dir, registry::pid_alive).len(),
        other_account_dirs: others
            .iter()
            .filter(|a| a.id != account.id)
            .flat_map(|a| std::iter::once(a.code_dir.clone()).chain(a.desktop_dir.clone()))
            .collect(),
    }
}

#[tauri::command]
pub fn control_policy() -> control::Policy {
    control::policy()
}

#[tauri::command]
pub fn control_preview(
    state: tauri::State<'_, AppState>,
    id: String,
    ops: Vec<control::Op>,
) -> Result<control::Preview> {
    let account = state.store.get(&id)?;
    control::preview(&account.code_dir, &ops, &control_context(&state, &account))
}

#[tauri::command]
pub fn control_apply(
    state: tauri::State<'_, AppState>,
    id: String,
    ops: Vec<control::Op>,
    base_hash: String,
) -> Result<control::Applied> {
    let account = state.store.get(&id)?;
    control::apply(
        &account.code_dir,
        &ops,
        &base_hash,
        &control::backup_dir(&state.home, &id),
        &stamp(),
    )
}

#[tauri::command]
pub fn control_backups(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<control::Backup>> {
    state.store.get(&id)?;
    Ok(control::list_backups(&control::backup_dir(
        &state.home,
        &id,
    )))
}

#[tauri::command]
pub fn control_restore_preview(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
) -> Result<control::Preview> {
    let account = state.store.get(&id)?;
    control::restore_preview(
        &account.code_dir,
        &control::backup_dir(&state.home, &id),
        &name,
        &control_context(&state, &account),
    )
}

#[tauri::command]
pub fn control_restore(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    base_hash: String,
) -> Result<control::Applied> {
    let account = state.store.get(&id)?;
    control::restore(
        &account.code_dir,
        &control::backup_dir(&state.home, &id),
        &name,
        &base_hash,
        &stamp(),
    )
}

/// What applies to an account, optionally inside a project folder.
#[tauri::command]
pub fn control_effective(
    state: tauri::State<'_, AppState>,
    id: String,
    project: Option<PathBuf>,
) -> Result<crate::effective::Effective> {
    let account = state.store.get(&id)?;
    crate::effective::effective(
        &account.code_dir,
        project.as_deref(),
        &crate::effective::managed_path(),
    )
}

// ---------------------------------------------------------------- Studio

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioItem {
    #[serde(flatten)]
    pub item: studio::Item,
    pub placements: Vec<studio::Placement>,
}

fn library(state: &AppState) -> studio::Library {
    studio::Library::new(studio::library_root(&state.home))
}

fn studio_backups(state: &AppState, account_id: &str) -> PathBuf {
    control::backup_dir(&state.home, account_id).join("studio")
}

#[tauri::command]
pub fn studio_list(state: tauri::State<'_, AppState>) -> Result<Vec<StudioItem>> {
    let accounts = state.store.load()?;
    let lib = library(&state);
    Ok(lib
        .list()
        .into_iter()
        .map(|item| {
            let text = fs::read_to_string(&item.path).unwrap_or_default();
            let placements = accounts
                .iter()
                .map(|a| studio::Placement {
                    account_id: a.id.clone(),
                    state: studio::placement_state(&text, &a.code_dir, item.kind, &item.name),
                })
                .collect();
            StudioItem { item, placements }
        })
        .collect())
}

#[tauri::command]
pub fn studio_read(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
) -> Result<String> {
    library(&state).read(kind, &name)
}

#[tauri::command]
pub fn studio_lint(kind: studio::Kind, name: String, text: String) -> studio::Lint {
    studio::lint(kind, &name, &text)
}

#[tauri::command]
pub fn studio_template(
    kind: studio::Kind,
    name: String,
    description: String,
    body: String,
) -> Result<String> {
    studio::validate_name(&name)?;
    Ok(studio::template(kind, &name, &description, &body))
}

#[tauri::command]
pub fn studio_save(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
    text: String,
) -> Result<studio::Item> {
    library(&state).save(kind, &name, &text)
}

/// Remove from the library only; copies placed in accounts stay.
#[tauri::command]
pub fn studio_remove(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
) -> Result<()> {
    library(&state).remove(kind, &name)
}

#[tauri::command]
pub fn studio_place_preview(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
    account_id: String,
) -> Result<studio::PlacePreview> {
    let account = state.store.get(&account_id)?;
    let text = library(&state).read(kind, &name)?;
    let live = registry::live_sessions(&account.code_dir, registry::pid_alive).len();
    studio::place_preview(&text, &account.code_dir, kind, &name, live)
}

#[tauri::command]
pub fn studio_place(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
    account_id: String,
    base_hash: String,
    risk_ack: bool,
) -> Result<Option<PathBuf>> {
    let account = state.store.get(&account_id)?;
    let text = library(&state).read(kind, &name)?;
    studio::place(
        &text,
        &account.code_dir,
        kind,
        &name,
        &base_hash,
        risk_ack,
        &studio_backups(&state, &account_id),
        &stamp(),
    )
}

#[tauri::command]
pub fn studio_unplace(
    state: tauri::State<'_, AppState>,
    kind: studio::Kind,
    name: String,
    account_id: String,
) -> Result<PathBuf> {
    let account = state.store.get(&account_id)?;
    studio::unplace(
        &account.code_dir,
        kind,
        &name,
        &studio_backups(&state, &account_id),
        &stamp(),
    )
}

/// Skills, agents and commands already in an account (not `skills/synced`).
#[tauri::command]
pub fn studio_account_items(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<Vec<studio::Item>> {
    let account = state.store.get(&account_id)?;
    Ok(studio::list_items(&account.code_dir, true))
}

/// Copy an account's item into the library (secret scan applies).
#[tauri::command]
pub fn studio_import(
    state: tauri::State<'_, AppState>,
    account_id: String,
    kind: studio::Kind,
    name: String,
) -> Result<studio::Item> {
    let account = state.store.get(&account_id)?;
    let text = studio::read_account_item(&account.code_dir, kind, &name)?;
    library(&state).save(kind, &name, &text)
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamp_has_sortable_utc_shape() {
        let s = super::stamp();
        assert_eq!(s.len(), "20261026T031500Z-123".len(), "{s}");
        assert!(s.starts_with("20"), "{s}");
        assert_eq!(&s[8..9], "T");
        // Same calendar day as the system clock (UTC).
        let out = std::process::Command::new("date")
            .args(["-u", "+%Y%m%d"])
            .output()
            .unwrap();
        let today = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert_eq!(&s[..8], today);
    }
}

/// Usage and API-equivalent cost per account for the last `days` days,
/// from transcript metadata only (PRD M3, M6).
#[tauri::command]
pub async fn usage_report(
    state: tauri::State<'_, AppState>,
    days: u32,
) -> Result<usage::UsageReport> {
    let accounts: Vec<(String, PathBuf)> = state
        .store
        .load()?
        .into_iter()
        .map(|a| (a.id, a.code_dir))
        .collect();
    let now = now_ms();
    let since = now - i64::from(days.clamp(1, 3650)) * 86_400_000;
    // Async command: runs off the UI thread.
    Ok(state.usage.report(&accounts, since, now))
}

/// Claude Code session ids are UUIDs (8-4-4-4-12 hex). Anything else, such
/// as a file named like a flag, is refused (compliance finding 2).
pub fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Resume a session in its own account and folder: opens a terminal that
/// runs the official `claude --resume <id>` (user-triggered, PRD M6).
#[tauri::command]
pub fn resume_session(
    state: tauri::State<'_, AppState>,
    id: String,
    session_id: String,
    cwd: Option<PathBuf>,
) -> Result<u32> {
    if session_id.is_empty()
        || session_id.len() > 64
        || !session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(Error::Invalid(
            "That does not look like a session id.".into(),
        ));
    }
    let account = state.store.get(&id)?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    let dir = cwd
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| state.home.clone());
    linux::open_terminal_running(
        &state.home,
        &account,
        &dir,
        &[
            claude.path.display().to_string(),
            "--resume".into(),
            session_id,
        ],
    )
}

/// Save a CSV or text export into the Downloads folder (never overwrites).
#[tauri::command]
pub fn export_text(
    state: tauri::State<'_, AppState>,
    name: String,
    text: String,
) -> Result<PathBuf> {
    let ok = !name.is_empty()
        && name.len() <= 80
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        && (name.ends_with(".csv") || name.ends_with(".txt") || name.ends_with(".md"));
    if !ok {
        return Err(Error::Invalid(
            "Export names are plain .csv, .txt or .md file names.".into(),
        ));
    }
    if text.len() > 50 * 1024 * 1024 {
        return Err(Error::Invalid("The export is too large.".into()));
    }
    let dir = downloads_dir(&state.home);
    fs::create_dir_all(&dir)?;
    let (stem, ext) = name.rsplit_once('.').unwrap_or((&name, "txt"));
    for i in 0..1000 {
        let file = if i == 0 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}-{i}.{ext}")
        };
        let path = dir.join(file);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(text.as_bytes())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(Error::Invalid(
        "Too many exports with this name; tidy the Downloads folder.".into(),
    ))
}

/// `XDG_DOWNLOAD_DIR` from `user-dirs.dirs`, else `~/Downloads`.
fn downloads_dir(home: &Path) -> PathBuf {
    let cfg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    if let Ok(text) = fs::read_to_string(cfg.join("user-dirs.dirs")) {
        for line in text.lines() {
            if let Some(v) = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=") {
                let v = v
                    .trim_matches('"')
                    .replace("$HOME", &home.display().to_string());
                let p = PathBuf::from(v);
                if p.is_absolute() {
                    return p;
                }
            }
        }
    }
    home.join("Downloads")
}

#[cfg(test)]
mod resume_tests {
    use super::is_uuid;

    #[test]
    fn only_uuid_session_ids_are_resumed() {
        assert!(is_uuid("6f1c2a3b-4d5e-4f60-8a7b-9c0d1e2f3a4d"));
        assert!(!is_uuid("--dangerously-skip-permissions"));
        assert!(!is_uuid("6f1c2a3b-4d5e-4f60-8a7b-9c0d1e2f3a4"));
        assert!(!is_uuid("6f1c2a3b-4d5e-4f60-8a7b-9c0d1e2f3a4z"));
        assert!(!is_uuid("-f1c2a3b-4d5e-4f60-8a7b-9c0d1e2f3a4b"));
    }
}

/// What one account has installed: plugins (with a read-only safety scan),
/// marketplaces, MCP servers and hooks (PRD M5).
#[tauri::command]
pub async fn ecosystem_inventory(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<crate::ecosystem::Inventory> {
    let account = state.store.get(&id)?;
    Ok(crate::ecosystem::inventory(
        &account.id,
        &account.code_dir,
        &state.home,
    ))
}

/// Safety preview of a folder (plugin, skill pack or repository). Reads
/// files only; nothing is run.
#[tauri::command]
pub async fn ecosystem_scan(path: PathBuf) -> Result<crate::ecosystem::PackageReport> {
    if !path.is_absolute() {
        return Err(Error::Invalid(
            "Use a full folder path, for example ~/Downloads/plugin.".into(),
        ));
    }
    let meta =
        fs::symlink_metadata(&path).map_err(|_| Error::NotFound(path.display().to_string()))?;
    if !meta.is_dir() {
        return Err(Error::Invalid(format!(
            "{} is not a folder.",
            path.display()
        )));
    }
    Ok(crate::ecosystem::scan(&path))
}

/// Remember that the user reviewed this exact version of a plugin.
#[tauri::command]
pub fn ecosystem_mark_reviewed(
    state: tauri::State<'_, AppState>,
    id: String,
    plugin_id: String,
    hash: Option<String>,
) -> Result<()> {
    state.store.get(&id)?;
    let mut r = crate::ecosystem::load_reviewed(&state.home, &id);
    match hash {
        Some(h) if h.len() == 16 && h.chars().all(|c| c.is_ascii_hexdigit()) => {
            r.insert(plugin_id, h);
        }
        Some(_) => return Err(Error::Invalid("That is not a scan fingerprint.".into())),
        None => {
            r.remove(&plugin_id);
        }
    }
    crate::ecosystem::save_reviewed(&state.home, &id, &r)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvents {
    pub events: Vec<usage::TimelineEvent>,
    /// More events than the limit; the oldest were left out.
    pub truncated: bool,
}

/// Prompts, model calls and tool runs between two times (Timeline, PRD M2).
#[tauri::command]
pub async fn timeline_events(
    state: tauri::State<'_, AppState>,
    since_ms: i64,
    until_ms: i64,
) -> Result<TimelineEvents> {
    let accounts: Vec<(String, PathBuf)> = state
        .store
        .load()?
        .into_iter()
        .map(|a| (a.id, a.code_dir))
        .collect();
    let (events, truncated) =
        state
            .usage
            .events(&accounts, since_ms, until_ms.max(since_ms), 20_000);
    Ok(TimelineEvents { events, truncated })
}

fn find_session(
    state: &AppState,
    account_id: &str,
    session_id: &str,
) -> Result<usage::SessionSummary> {
    let accounts: Vec<(String, PathBuf)> = state
        .store
        .load()?
        .into_iter()
        .filter(|a| a.id == account_id)
        .map(|a| (a.id, a.code_dir))
        .collect();
    let now = now_ms();
    state
        .usage
        .report(&accounts, now - 365 * 86_400_000, now)
        .accounts
        .into_iter()
        .flat_map(|a| a.sessions)
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| Error::NotFound(format!("session {session_id}")))
}

/// Draft a handoff from a session in one account to another account. Not
/// saved yet: the user reads and may edit it first.
#[tauri::command]
pub async fn bridge_draft(
    state: tauri::State<'_, AppState>,
    from_id: String,
    session_id: String,
    to_id: String,
    note: String,
    include_git: bool,
) -> Result<crate::bridge::Handoff> {
    if from_id == to_id {
        return Err(Error::Invalid(
            "Pick a different account to hand off to.".into(),
        ));
    }
    if note.chars().count() > crate::bridge::MAX_NOTE {
        return Err(Error::Invalid(
            "The note can be up to 8,000 characters.".into(),
        ));
    }
    let from = state.store.get(&from_id)?;
    let to = state.store.get(&to_id)?;
    let session = find_session(&state, &from_id, &session_id)?;
    let git = if include_git {
        session
            .project
            .as_deref()
            .map(Path::new)
            .filter(|p| p.is_dir())
            .and_then(crate::bridge::git_state)
    } else {
        None
    };
    let st = stamp();
    let text = crate::bridge::compose(&crate::bridge::HandoffInput {
        from_name: &from.name,
        to_name: &to.name,
        session: &session,
        note: &note,
        git: git.as_ref(),
        stamp: &format!(
            "{}-{}-{} {}:{} UTC",
            &st[0..4],
            &st[4..6],
            &st[6..8],
            &st[9..11],
            &st[11..13]
        ),
    });
    Ok(crate::bridge::Handoff {
        name: format!("{}-{}-to-{}.md", &st[..15], from.id, to.id),
        path: String::new(),
        text,
    })
}

/// Save a (possibly edited) handoff privately in Facet's data folder.
#[tauri::command]
pub fn bridge_save(
    state: tauri::State<'_, AppState>,
    name: String,
    text: String,
) -> Result<crate::bridge::Handoff> {
    if !crate::bridge::valid_name(&name) {
        return Err(Error::Invalid("That is not a handoff file name.".into()));
    }
    if text.len() > 1024 * 1024 {
        return Err(Error::Invalid(
            "The handoff is over 1 MB; shorten the note or leave out the changes.".into(),
        ));
    }
    if let Some(what) = studio::find_secret(&text) {
        return Err(Error::Invalid(format!(
            "The handoff looks like it holds a secret ({what}). Remove it, then save."
        )));
    }
    let path = crate::bridge::save(&state.home, &name, &text)?;
    Ok(crate::bridge::Handoff {
        name,
        path: path.display().to_string(),
        text,
    })
}

#[tauri::command]
pub fn bridge_list(state: tauri::State<'_, AppState>) -> Vec<crate::bridge::HandoffFile> {
    crate::bridge::list(&state.home)
}

#[tauri::command]
pub fn bridge_remove(state: tauri::State<'_, AppState>, name: String) -> Result<()> {
    crate::bridge::remove(&state.home, &name)
}

/// Open a terminal as the receiving account in `cwd`, starting the official
/// claude with a prompt that points at the handoff file (user-triggered).
#[tauri::command]
pub fn bridge_open(
    state: tauri::State<'_, AppState>,
    to_id: String,
    name: String,
    cwd: Option<PathBuf>,
) -> Result<u32> {
    if !crate::bridge::valid_name(&name) {
        return Err(Error::Invalid("That is not a handoff file name.".into()));
    }
    let account = state.store.get(&to_id)?;
    let path = crate::bridge::handoff_dir(&state.home).join(&name);
    if !fs::symlink_metadata(&path).is_ok_and(|m| m.is_file()) {
        return Err(Error::NotFound(name));
    }
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    let dir = cwd
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| state.home.clone());
    linux::open_terminal_running(
        &state.home,
        &account,
        &dir,
        &[
            claude.path.display().to_string(),
            "--".into(),
            crate::bridge::opening_prompt(&path),
        ],
    )
}

/// Add an MCP server to one account with the official `claude mcp add`.
#[tauri::command]
pub async fn mcp_add(
    state: tauri::State<'_, AppState>,
    id: String,
    input: crate::mcp::McpAdd,
) -> Result<String> {
    let account = state.store.get(&id)?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    let args = crate::mcp::add_args(&input)?;
    let cwd = input
        .folder
        .clone()
        .filter(|f| f.is_dir())
        .unwrap_or_else(|| state.home.clone());
    crate::mcp::run(&claude.path, &account.code_dir, &state.home, &cwd, &args)
}

/// Remove an MCP server from one account with the official `claude mcp remove`.
#[tauri::command]
pub async fn mcp_remove(
    state: tauri::State<'_, AppState>,
    id: String,
    name: String,
    scope: String,
    folder: Option<PathBuf>,
) -> Result<String> {
    let account = state.store.get(&id)?;
    let claude = linux::find_claude_code()
        .ok_or_else(|| Error::Unavailable("Claude Code is not installed.".into()))?;
    let args = crate::mcp::remove_args(&name, &scope)?;
    let cwd = folder
        .filter(|f| f.is_dir())
        .unwrap_or_else(|| state.home.clone());
    crate::mcp::run(&claude.path, &account.code_dir, &state.home, &cwd, &args)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinksState {
    pub settings: crate::browser::LinkSettings,
    pub browsers: Vec<crate::browser::Browser>,
    /// Suggested profile per account (signed-in email matches).
    pub suggestions: BTreeMap<String, crate::browser::LinkTarget>,
    pub current_default: Option<String>,
    pub recent: Vec<crate::browser::RecentLink>,
}

/// Browsers, profiles, per-account link targets and recent routed links.
#[tauri::command]
pub fn links_state(state: tauri::State<'_, AppState>) -> Result<LinksState> {
    let browsers = crate::browser::detect(&state.home);
    let accounts = state.store.load()?;
    let suggestions = accounts
        .iter()
        .filter_map(|a| {
            a.expected_email
                .as_deref()
                .and_then(|e| crate::browser::suggest(&browsers, e))
                .map(|t| (a.id.clone(), t))
        })
        .collect();
    Ok(LinksState {
        settings: crate::browser::load(&state.home),
        browsers,
        suggestions,
        current_default: crate::browser::current_default(),
        recent: crate::browser::recent(&state.home),
    })
}

/// Set where an account's links open (`None` = the fallback).
#[tauri::command]
pub fn links_set_account(
    state: tauri::State<'_, AppState>,
    id: String,
    target: Option<crate::browser::LinkTarget>,
) -> Result<crate::browser::LinkSettings> {
    state.store.get(&id)?;
    let mut s = crate::browser::load(&state.home);
    match target {
        Some(t) => {
            check_target(&state.home, &t)?;
            s.accounts.insert(id, t);
        }
        None => {
            s.accounts.remove(&id);
        }
    }
    crate::browser::save(&state.home, &s)?;
    Ok(s)
}

/// Set where links from unknown windows open.
#[tauri::command]
pub fn links_set_fallback(
    state: tauri::State<'_, AppState>,
    target: Option<crate::browser::LinkTarget>,
) -> Result<crate::browser::LinkSettings> {
    if let Some(t) = &target {
        check_target(&state.home, t)?;
    }
    let mut s = crate::browser::load(&state.home);
    s.fallback = target;
    crate::browser::save(&state.home, &s)?;
    Ok(s)
}

fn check_target(home: &Path, t: &crate::browser::LinkTarget) -> Result<()> {
    let browsers = crate::browser::detect(home);
    let b = browsers
        .iter()
        .find(|b| b.id == t.browser)
        .ok_or_else(|| Error::Invalid(format!("{} is not installed.", t.browser)))?;
    if let Some(p) = &t.profile {
        if !b.profiles.iter().any(|x| &x.dir == p) {
            return Err(Error::Invalid(format!("{} has no profile {p}.", b.name)));
        }
    }
    Ok(())
}

/// Make Facet's link opener the default browser (backs up mimeapps.list).
#[tauri::command]
pub fn links_enable(state: tauri::State<'_, AppState>) -> Result<crate::browser::LinkSettings> {
    let exe = std::env::current_exe()?;
    let backups = control::backup_dir(&state.home, "system");
    crate::browser::enable(&state.home, &exe, &backups, &stamp())
}

/// Give the default browser back.
#[tauri::command]
pub fn links_disable(state: tauri::State<'_, AppState>) -> Result<crate::browser::LinkSettings> {
    crate::browser::disable(&state.home)
}

/// Open a test page in an account's browser profile (user-triggered).
#[tauri::command]
pub fn links_test(state: tauri::State<'_, AppState>, id: Option<String>) -> Result<()> {
    let accounts = state.store.load()?;
    let s = crate::browser::load(&state.home);
    let target = id
        .as_ref()
        .and_then(|i| s.accounts.get(i))
        .or(s.fallback.as_ref())
        .cloned()
        .ok_or_else(|| Error::Invalid("Pick a browser profile first.".into()))?;
    let browsers = crate::browser::detect(&state.home);
    let b = browsers
        .iter()
        .find(|b| b.id == target.browser)
        .ok_or_else(|| Error::Invalid(format!("{} is not installed.", target.browser)))?;
    let _ = accounts;
    let args = crate::browser::argv(b, target.profile.as_deref(), "https://example.com/");
    let mut cmd = crate::env::command(&args[0]);
    cmd.args(&args[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    linux::spawn_detached(cmd)?;
    Ok(())
}
