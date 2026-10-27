//! Open links in each account's own browser profile (D-022).
//!
//! When the user turns it on, Facet registers a small link opener as the
//! default web browser. A Claude window or Claude Code terminal that opens a
//! link starts `facet --open-url <url>`; the opener inherits that process's
//! environment, finds the account from `FACET_ACCOUNT` or
//! `CLAUDE_CONFIG_DIR` (names only, never values of other variables), and
//! starts the browser with that account's profile. Facet reads only each
//! browser profile's display name and signed-in email, never cookies, keys or
//! passwords. Links are never sent anywhere else.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::adapter::linux::which;
use crate::env;
use crate::error::{Error, Result};
use crate::profile::Account;

pub const OPENER_ID: &str = "facet-links.desktop";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    /// Folder name, e.g. "Profile 10", or the Firefox profile name.
    pub dir: String,
    pub name: String,
    pub emails: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    pub id: String,
    pub name: String,
    pub exe: PathBuf,
    pub profiles: Vec<BrowserProfile>,
}

/// Where an account's links open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkTarget {
    pub browser: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

struct Family {
    id: &'static str,
    name: &'static str,
    config: &'static str,
    exes: &'static [&'static str],
}

const CHROMIUM: &[Family] = &[
    Family {
        id: "chrome",
        name: "Google Chrome",
        config: ".config/google-chrome",
        exes: &["google-chrome-stable", "google-chrome"],
    },
    Family {
        id: "chromium",
        name: "Chromium",
        config: ".config/chromium",
        exes: &["chromium", "chromium-browser"],
    },
    Family {
        id: "brave",
        name: "Brave",
        config: ".config/BraveSoftware/Brave-Browser",
        exes: &["brave-browser", "brave"],
    },
    Family {
        id: "edge",
        name: "Microsoft Edge",
        config: ".config/microsoft-edge",
        exes: &["microsoft-edge", "microsoft-edge-stable"],
    },
    Family {
        id: "vivaldi",
        name: "Vivaldi",
        config: ".config/vivaldi",
        exes: &["vivaldi", "vivaldi-stable"],
    },
];

/// Only these two fields of a Chromium profile's `Preferences` are read.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Prefs {
    profile: PrefsProfile,
    account_info: Vec<PrefsAccount>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PrefsProfile {
    name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PrefsAccount {
    email: Option<String>,
}

fn chromium_profiles(dir: &Path) -> Vec<BrowserProfile> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let folder = e.file_name().to_string_lossy().into_owned();
        if !(folder == "Default"
            || folder
                .strip_prefix("Profile ")
                .is_some_and(|n| n.chars().all(|c| c.is_ascii_digit())))
        {
            continue;
        }
        let prefs = e.path().join("Preferences");
        let Ok(meta) = fs::symlink_metadata(&prefs) else {
            continue;
        };
        if !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
            continue;
        }
        let p: Prefs = fs::read_to_string(&prefs)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        out.push(BrowserProfile {
            name: p
                .profile
                .name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| folder.clone()),
            emails: p
                .account_info
                .into_iter()
                .filter_map(|a| a.email)
                .filter(|e| e.contains('@'))
                .collect(),
            dir: folder,
        });
    }
    out.sort_by_key(|p| {
        (
            p.dir != "Default",
            p.dir
                .strip_prefix("Profile ")
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0),
        )
    });
    out
}

fn firefox_profiles(home: &Path) -> Vec<BrowserProfile> {
    let Ok(ini) = fs::read_to_string(home.join(".mozilla/firefox/profiles.ini")) else {
        return Vec::new();
    };
    ini.lines()
        .filter_map(|l| l.trim().strip_prefix("Name="))
        .map(|n| BrowserProfile {
            dir: n.to_string(),
            name: n.to_string(),
            emails: Vec::new(),
        })
        .collect()
}

/// Installed browsers and their profiles.
pub fn detect(home: &Path) -> Vec<Browser> {
    let mut out = Vec::new();
    for f in CHROMIUM {
        let Some(exe) = f.exes.iter().find_map(|e| which(e)) else {
            continue;
        };
        out.push(Browser {
            id: f.id.into(),
            name: f.name.into(),
            exe,
            profiles: chromium_profiles(&home.join(f.config)),
        });
    }
    if let Some(exe) = which("firefox") {
        out.push(Browser {
            id: "firefox".into(),
            name: "Firefox".into(),
            exe,
            profiles: firefox_profiles(home),
        });
    }
    out
}

/// The profile whose signed-in email matches the account, if any.
pub fn suggest(browsers: &[Browser], email: &str) -> Option<LinkTarget> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return None;
    }
    browsers.iter().find_map(|b| {
        b.profiles
            .iter()
            .find(|p| p.emails.iter().any(|e| e.to_ascii_lowercase() == email))
            .map(|p| LinkTarget {
                browser: b.id.clone(),
                profile: Some(p.dir.clone()),
            })
    })
}

// ---------- settings ----------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinkSettings {
    #[serde(default)]
    pub enabled: bool,
    /// Links from windows Facet cannot tie to an account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<LinkTarget>,
    /// The default browser before Facet took over, restored when turned off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_default: Option<String>,
    /// Per-account targets.
    #[serde(default)]
    pub accounts: BTreeMap<String, LinkTarget>,
}

pub fn settings_path(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("facet/links.toml")
}

pub fn load(home: &Path) -> LinkSettings {
    fs::read_to_string(settings_path(home))
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(home: &Path, s: &LinkSettings) -> Result<()> {
    let p = settings_path(home);
    if let Some(d) = p.parent() {
        fs::create_dir_all(d)?;
    }
    let text = toml::to_string_pretty(s).map_err(|e| Error::Invalid(e.to_string()))?;
    crate::control::replace_atomically(
        &p,
        &format!("# Facet: which browser profile opens each account's links.\n{text}"),
    )
}

// ---------- routing ----------

/// The account a link came from, from the opener's own environment.
pub fn account_for(
    facet_account: Option<&str>,
    config_dir: Option<&Path>,
    accounts: &[Account],
    home: &Path,
) -> Option<String> {
    if let Some(id) = facet_account.filter(|id| accounts.iter().any(|a| a.id == *id)) {
        return Some(id.to_string());
    }
    // Without CLAUDE_CONFIG_DIR the link may come from any default-folder
    // session or from an app that is not Claude at all, so it stays unassigned
    // and goes to the fallback profile.
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let dir = config_dir?;
    let dir = if dir.is_relative() {
        home.join(dir)
    } else {
        dir.to_path_buf()
    };
    accounts
        .iter()
        .find(|a| canon(&a.code_dir) == canon(&dir))
        .map(|a| a.id.clone())
}

pub fn valid_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() < 16 * 1024
        && !url.chars().any(|c| c.is_control() || c == ' ')
}

/// Host of a URL, for the recent-links list (never the path or query).
pub fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    host.rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(host)
        .to_string()
}

/// Arguments for the browser, the URL last and never parsed as a flag.
pub fn argv(browser: &Browser, profile: Option<&str>, url: &str) -> Vec<String> {
    let mut a = vec![browser.exe.display().to_string()];
    match (browser.id.as_str(), profile) {
        ("firefox", Some(p)) => {
            a.push("-P".into());
            a.push(p.into());
        }
        (_, Some(p)) => a.push(format!("--profile-directory={p}")),
        _ => {}
    }
    a.push(url.to_string());
    a
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecentLink {
    pub ms: i64,
    pub host: String,
    pub account_id: Option<String>,
    pub browser: Option<String>,
    pub profile: Option<String>,
}

pub fn recent_path(home: &Path) -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"))
        .join("facet/links.json")
}

pub fn recent(home: &Path) -> Vec<RecentLink> {
    fs::read_to_string(recent_path(home))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn remember(home: &Path, link: RecentLink) {
    let mut list = recent(home);
    list.insert(0, link);
    list.truncate(30);
    let p = recent_path(home);
    if let Some(d) = p.parent() {
        let _ = crate::control::ensure_private_dir(d);
    }
    let _ =
        crate::control::replace_atomically(&p, &serde_json::to_string(&list).unwrap_or_default());
}

/// What `facet --open-url` does: pick the account, then the browser.
pub fn open(url: &str, home: &Path, accounts: &[Account], now_ms: i64) -> Result<()> {
    if !valid_url(url) {
        return Err(Error::Invalid(
            "Facet opens only http and https links.".into(),
        ));
    }
    let settings = load(home);
    let facet_account = std::env::var("FACET_ACCOUNT").ok();
    let config_dir = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from);
    let account = account_for(
        facet_account.as_deref(),
        config_dir.as_deref(),
        accounts,
        home,
    );
    let target = account
        .as_ref()
        .and_then(|id| settings.accounts.get(id))
        .or(settings.fallback.as_ref())
        .cloned();
    let browsers = detect(home);
    let chosen = target.as_ref().and_then(|t| {
        browsers
            .iter()
            .find(|b| b.id == t.browser)
            .map(|b| (b, t.profile.clone()))
    });
    remember(
        home,
        RecentLink {
            ms: now_ms,
            host: host_of(url),
            account_id: account.clone(),
            browser: chosen.as_ref().map(|(b, _)| b.id.clone()),
            profile: chosen.as_ref().and_then(|(_, p)| p.clone()),
        },
    );
    let args = match chosen {
        Some((b, p)) => argv(b, p.as_deref(), url),
        None => previous_default_argv(home, &settings, &browsers, url)?,
    };
    // The opener inherits the Claude process's environment, which can hold
    // sign-in tokens: env::command removes them before the browser starts.
    let mut cmd = env::command(&args[0]);
    cmd.args(&args[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::adapter::linux::spawn_detached(cmd)?;
    Ok(())
}

/// The browser that was the default before Facet, started directly (never
/// through xdg-open, which would come back to Facet).
fn previous_default_argv(
    home: &Path,
    s: &LinkSettings,
    browsers: &[Browser],
    url: &str,
) -> Result<Vec<String>> {
    if let Some(id) = s.previous_default.as_deref().filter(|id| *id != OPENER_ID) {
        if let Some(path) = desktop_file(home, id) {
            if let Some(gio) = which("gio") {
                return Ok(vec![
                    gio.display().to_string(),
                    "launch".into(),
                    path.display().to_string(),
                    url.into(),
                ]);
            }
        }
    }
    browsers
        .first()
        .map(|b| argv(b, None, url))
        .ok_or_else(|| Error::Unavailable("No browser found to open the link.".into()))
}

fn desktop_file(home: &Path, id: &str) -> Option<PathBuf> {
    if id.contains('/') || !id.ends_with(".desktop") {
        return None;
    }
    let mut dirs = vec![home.join(".local/share/applications")];
    let data =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(
        data.split(':')
            .filter(|d| !d.is_empty())
            .map(|d| Path::new(d).join("applications")),
    );
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.into_iter().map(|d| d.join(id)).find(|p| p.is_file())
}

// ---------- turning it on and off ----------

pub fn opener_entry(facet_exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Facet link opener\nComment=Opens each Claude account's links in its own browser profile (unofficial, not affiliated with Anthropic)\nExec={} --open-url %u\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\nNoDisplay=true\nTerminal=false\nCategories=Network;WebBrowser;\n",
        if facet_exe.display().to_string().contains(' ') { format!("\"{}\"", facet_exe.display()) } else { facet_exe.display().to_string() }
    )
}

fn xdg_settings(args: &[&str]) -> Result<String> {
    let out = env::command("xdg-settings")
        .args(args)
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        return Err(Error::Unavailable(format!(
            "xdg-settings {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn current_default() -> Option<String> {
    xdg_settings(&["get", "default-web-browser"])
        .ok()
        .filter(|s| !s.is_empty())
}

/// Make Facet's opener the default browser. Backs up `mimeapps.list` and
/// remembers the previous default so turning it off restores it.
pub fn enable(home: &Path, facet_exe: &Path, backups: &Path, stamp: &str) -> Result<LinkSettings> {
    let mut s = load(home);
    let prev = current_default();
    if prev.as_deref() != Some(OPENER_ID) {
        s.previous_default = prev;
    }
    let mime = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("mimeapps.list");
    if let Ok(text) = fs::read_to_string(&mime) {
        crate::control::ensure_private_dir(backups)?;
        crate::control::write_new_private(&backups.join(format!("{stamp}-mimeapps.list")), &text)?;
    }
    let apps = home.join(".local/share/applications");
    fs::create_dir_all(&apps)?;
    crate::control::replace_atomically(&apps.join(OPENER_ID), &opener_entry(facet_exe))?;
    let _ = env::command("update-desktop-database")
        .arg(&apps)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    xdg_settings(&["set", "default-web-browser", OPENER_ID])?;
    // xdg-settings also claims local HTML files and about:/unknown links;
    // Facet opens only web links, so those go back to the previous browser.
    if let Some(prev) = s.previous_default.as_deref().filter(|p| *p != OPENER_ID) {
        let _ = env::command("xdg-mime")
            .args([
                "default",
                prev,
                "text/html",
                "x-scheme-handler/about",
                "x-scheme-handler/unknown",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    s.enabled = true;
    save(home, &s)?;
    Ok(s)
}

/// Give the default back to the browser that had it.
pub fn disable(home: &Path) -> Result<LinkSettings> {
    let mut s = load(home);
    if current_default().as_deref() == Some(OPENER_ID) {
        let back = s
            .previous_default
            .clone()
            .filter(|p| p != OPENER_ID && desktop_file(home, p).is_some());
        match back {
            Some(id) => {
                xdg_settings(&["set", "default-web-browser", &id])?;
            }
            None => return Err(Error::Invalid("Facet does not know which browser was the default. Pick one in your system settings, then turn this off again.".into())),
        }
    }
    s.enabled = false;
    save(home, &s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(id: &str, code: &str) -> Account {
        Account {
            id: id.into(),
            name: id.into(),
            color: 1,
            plan: None,
            desktop_dir: None,
            code_dir: code.into(),
            expected_email: None,
            tint: true,
            x11_identity: false,
            created_at: 0,
            badge: None,
            image: None,
            notes: None,
            launcher_icon: Default::default(),
            terminal_dir: None,
            launcher_name: None,
        }
    }

    #[test]
    fn finds_the_account_from_the_environment() {
        let accounts = vec![
            acc("personal", "/h/.claude"),
            acc("work", "/h/.claude-work"),
        ];
        let h = Path::new("/h");
        assert_eq!(
            account_for(Some("work"), None, &accounts, h).as_deref(),
            Some("work")
        );
        assert_eq!(
            account_for(
                Some("ghost"),
                Some(Path::new("/h/.claude-work")),
                &accounts,
                h
            )
            .as_deref(),
            Some("work")
        );
        assert_eq!(
            account_for(None, Some(Path::new("/h/.claude")), &accounts, h).as_deref(),
            Some("personal")
        );
        assert_eq!(account_for(None, None, &accounts, h), None);
    }

    #[test]
    fn urls_and_arguments_are_safe() {
        assert!(valid_url("https://claude.ai/login?x=1"));
        assert!(!valid_url("file:///etc/passwd"));
        assert!(!valid_url("--disable-web-security"));
        assert!(!valid_url("https://a b"));
        assert_eq!(
            host_of("https://user:pw@claude.ai/oauth?code=SECRET"),
            "claude.ai"
        );
        let chrome = Browser {
            id: "chrome".into(),
            name: "Chrome".into(),
            exe: "/usr/bin/google-chrome".into(),
            profiles: vec![],
        };
        assert_eq!(
            argv(&chrome, Some("Profile 10"), "https://x.example"),
            [
                "/usr/bin/google-chrome",
                "--profile-directory=Profile 10",
                "https://x.example"
            ]
        );
        let ff = Browser {
            id: "firefox".into(),
            name: "Firefox".into(),
            exe: "/usr/bin/firefox".into(),
            profiles: vec![],
        };
        assert_eq!(
            argv(&ff, Some("work"), "https://x.example"),
            ["/usr/bin/firefox", "-P", "work", "https://x.example"]
        );
    }

    #[test]
    fn reads_only_profile_names_and_emails() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("Profile 10");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("Preferences"), r#"{"profile":{"name":"Work"},"account_info":[{"email":"you@company.example","gaia":"123"}],"os_crypt":{"encrypted_key":"SECRETKEY"}}"#).unwrap();
        fs::create_dir_all(d.path().join("Default")).unwrap();
        fs::write(d.path().join("Default/Preferences"), "{}").unwrap();
        fs::create_dir_all(d.path().join("System Profile")).unwrap();
        let found = chromium_profiles(d.path());
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].dir, "Default");
        assert_eq!(
            found[1],
            BrowserProfile {
                dir: "Profile 10".into(),
                name: "Work".into(),
                emails: vec!["you@company.example".into()]
            }
        );
        assert!(!format!("{found:?}").contains("SECRET"));
        let browsers = vec![Browser {
            id: "chrome".into(),
            name: "Chrome".into(),
            exe: "/x".into(),
            profiles: found,
        }];
        assert_eq!(
            suggest(&browsers, "You@Company.example"),
            Some(LinkTarget {
                browser: "chrome".into(),
                profile: Some("Profile 10".into())
            })
        );
        assert_eq!(suggest(&browsers, "other@x.com"), None);
    }

    #[test]
    fn opener_entry_points_back_at_facet() {
        let e = opener_entry(Path::new("/usr/bin/facet"));
        assert!(e.contains("Exec=/usr/bin/facet --open-url %u"));
        assert!(e.contains("x-scheme-handler/https"));
        assert!(e.contains("NoDisplay=true"));
    }
}
