//! Health checks (PRD M9, first set for Phase 1).
//!
//! `evaluate` is a pure function over facts gathered by `gather`, so every
//! rule is unit-tested. Health never reads the environment of Claude
//! processes (compliance review 2026-10-26): it uses Facet's own environment
//! (names only), shell files (names only), command lines, the session registry
//! and `claude auth status`.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::adapter::{linux, ClaudeInstall, DesktopProcess, ShortcutSupport};
use crate::auth::{self, AuthStatus};
use crate::env;
use crate::profile::Account;
use crate::registry::{self, MisplacedSession};
use crate::settings::{self, SettingsRead};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Bad,
    Warn,
    Info,
    Ok,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub status: Status,
    pub title: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// The detail holds personal data (a signed-in email). Any future log,
    /// export or crash report must redact it (compliance review finding 14).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub personal: bool,
}

impl Check {
    fn new(
        id: impl Into<String>,
        status: Status,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status,
            title: title.into(),
            detail: detail.into(),
            fix: None,
            account_id: None,
            personal: false,
        }
    }
    fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
    fn account(mut self, id: &str) -> Self {
        self.account_id = Some(id.to_string());
        self
    }
    fn personal(mut self) -> Self {
        self.personal = true;
        self
    }
}

pub struct AccountFacts {
    pub account: Account,
    pub code_dir_exists: bool,
    pub settings: SettingsRead,
    /// `None` when the sign-in check was not run or failed.
    pub auth: Option<Result<AuthStatus, String>>,
}

pub struct Facts {
    pub desktop: Option<ClaudeInstall>,
    pub code: Option<ClaudeInstall>,
    pub facet_env_overrides: Vec<&'static str>,
    pub shell_hits: Vec<(PathBuf, Vec<&'static str>)>,
    pub running: Vec<DesktopProcess>,
    pub launched_by_facet: BTreeSet<u32>,
    pub misplaced: Vec<MisplacedSession>,
    pub accounts: Vec<AccountFacts>,
    pub shortcut: ShortcutSupport,
    /// `~/.claude/.claude.json` next to `~/.claude.json` (D-016).
    pub stray_state: Option<PathBuf>,
}

pub fn evaluate(f: &Facts) -> Vec<Check> {
    let mut out = Vec::new();
    let any_desktop = f.accounts.iter().any(|a| a.account.desktop_dir.is_some());

    match &f.desktop {
        Some(d) if d.official => out.push(Check::new(
            "desktop-install",
            Status::Ok,
            "Claude Desktop is installed",
            format!("{}{}", d.path.display(), d.version.as_ref().map(|v| format!(" · version {v} ({})", d.channel)).unwrap_or_default()),
        )),
        Some(d) => out.push(
            Check::new(
                "desktop-install",
                Status::Warn,
                "Claude Desktop is installed, but not confirmed as the official build",
                format!("{} ({}). Its package is not maintained by Anthropic, or it was not installed as a package.", d.path.display(), d.channel),
            )
            .fix("Install Claude Desktop from Anthropic's own download. Facet launches only the official app."),
        ),
        None => out.push(
            Check::new(
                "desktop-install",
                if any_desktop { Status::Bad } else { Status::Info },
                "Claude Desktop was not found",
                "Facet looked in /usr/lib/claude-desktop, /opt/Claude and /usr/bin.",
            )
            .fix("Install the official Claude Desktop for Linux from Anthropic."),
        ),
    }
    match &f.code {
        Some(c) if !c.official => out.push(
            Check::new(
                "code-install",
                Status::Warn,
                "The claude command is not confirmed as the official Claude Code",
                format!("{} does not lead to the official install folders. It may be a wrapper script.", c.path.display()),
            )
            .fix("Install Claude Code with Anthropic's installer or npm package, or make sure the first claude on PATH is the official one."),
        ),
        Some(c) => out.push(Check::new(
            "code-install",
            Status::Ok,
            "Claude Code is installed",
            format!(
                "{}{}",
                c.path.display(),
                c.version
                    .as_ref()
                    .map(|v| format!(" · version {v}"))
                    .unwrap_or_default()
            ),
        )),
        None => out.push(
            Check::new(
                "code-install",
                Status::Warn,
                "Claude Code was not found on PATH",
                "Terminals and sign-in checks need the claude command.",
            )
            .fix("Install Claude Code with the official installer, then reopen Facet."),
        ),
    }

    if f.facet_env_overrides.is_empty() {
        out.push(Check::new(
            "env-facet",
            Status::Ok,
            "No override variables where Facet runs",
            "None of the sign-in override variables are set in Facet's environment.",
        ));
    } else {
        out.push(
            Check::new(
                "env-facet",
                Status::Bad,
                "Override variables are set where Facet runs",
                format!(
                    "{} {} set. Facet removes them from the processes it starts, but a terminal you open yourself would use {} instead of the account's sign-in.",
                    f.facet_env_overrides.join(", "),
                    if f.facet_env_overrides.len() == 1 { "is" } else { "are" },
                    if f.facet_env_overrides.len() == 1 { "it" } else { "them" },
                ),
            )
            .fix("Remove them from your shell files or login environment, then log out and back in."),
        );
    }
    for (file, names) in &f.shell_hits {
        out.push(
            Check::new(
                format!("env-file:{}", file.display()),
                Status::Warn,
                format!("{} sets variables that change which account is used", file.display()),
                format!(
                    "{} found (names only). Facet removes them from the processes it starts and from bash terminals it opens, but other shells and terminals you open yourself read this file and set them again.",
                    names.join(", ")
                ),
            )
            .fix("Delete those lines unless you need them on purpose, or use the claude-<account> commands, which remove them each time."),
        );
    }

    for a in &f.accounts {
        account_checks(a, f, &mut out);
    }

    for m in &f.misplaced {
        let name = |id: &str| {
            f.accounts
                .iter()
                .find(|a| a.account.id == id)
                .map(|a| a.account.name.clone())
                .unwrap_or_else(|| id.to_string())
        };
        out.push(
            Check::new(
                format!("misplaced:{}", m.pid),
                Status::Bad,
                format!("{}'s Desktop is writing Code sessions into {}'s folder", name(&m.desktop_account), name(&m.folder_account)),
                format!(
                    "Session {} signs in as {} but keeps its transcripts, settings and skills with {}. Costs and skills mix between the two accounts.",
                    m.pid, name(&m.desktop_account), name(&m.folder_account)
                ),
            )
            .fix(format!("Quit {}'s Desktop app and start it from Facet, which sets the right folder.", name(&m.desktop_account)))
            .account(&m.desktop_account),
        );
    }

    for p in &f.running {
        let known = f.accounts.iter().any(|a| {
            a.account.desktop_dir.as_ref().is_some_and(|d| {
                d.to_string_lossy().trim_end_matches('/')
                    == p.data_dir.to_string_lossy().trim_end_matches('/')
            })
        });
        if !known {
            out.push(
                Check::new(
                    format!("unknown-instance:{}", p.pid),
                    Status::Info,
                    "A Claude Desktop instance is running that no Facet account uses",
                    format!("Folder {} (process {}).", p.data_dir.display(), p.pid),
                )
                .fix("Add it as an account if it is yours: Accounts › Add account › Use existing folders."),
            );
        }
    }

    if let Some(p) = &f.stray_state {
        out.push(
            Check::new(
                "stray-state",
                Status::Info,
                "A second Claude Code state file exists for the default account",
                format!(
                    "{} sits next to ~/.claude.json. It appears when something starts Claude Code with CLAUDE_CONFIG_DIR=~/.claude; such sessions do not see this account's MCP servers or project trust. Facet never starts sessions that way.",
                    p.display()
                ),
            )
            .fix("If nothing you use sets CLAUDE_CONFIG_DIR=~/.claude, you can delete that file; Facet leaves it alone."),
        );
    }

    out.push(match f.shortcut {
        ShortcutSupport::Portal => Check::new("shortcut", Status::Ok, "Global shortcut available", "The desktop offers the GlobalShortcuts portal, so the quick switcher can have a real shortcut."),
        ShortcutSupport::CliOnly => Check::new("shortcut", Status::Info, "No global shortcut portal", "Bind a custom shortcut in your desktop settings to run facet --switcher."),
    });

    out.sort_by_key(|c| c.status);
    out
}

fn account_checks(a: &AccountFacts, f: &Facts, out: &mut Vec<Check>) {
    let acc = &a.account;
    let id = acc.id.as_str();
    if !a.code_dir_exists {
        out.push(
            Check::new(format!("code-dir:{id}"), Status::Info, format!("{} has not used Claude Code yet", acc.name), format!("{} does not exist yet.", acc.code_dir.display()))
                .fix("Open a terminal as this account and run claude to sign in; Claude Code creates the folder.")
                .account(id),
        );
        return;
    }
    match &a.settings {
        SettingsRead::Invalid(e) => out.push(
            Check::new(
                format!("settings:{id}"),
                Status::Bad,
                format!("{}'s settings.json cannot be read", acc.name),
                e.clone(),
            )
            .fix("Open the file and fix the JSON. Claude Code ignores a broken file.")
            .account(id),
        ),
        _ => {
            let names = settings::override_env_names(&a.settings);
            if !names.is_empty() {
                out.push(
                    Check::new(format!("settings-env:{id}"), Status::Bad, format!("{}'s settings override its sign-in", acc.name), format!("settings.json sets {} in env.", names.join(", ")))
                        .fix("Remove those keys from env unless you use an API key for this account on purpose.")
                        .account(id),
                );
            }
            if settings::has_api_key_helper(&a.settings) {
                out.push(
                    Check::new(format!("settings-helper:{id}"), Status::Warn, format!("{} uses apiKeyHelper", acc.name), "A helper script supplies credentials instead of the account's sign-in. Facet never edits it.")
                        .account(id),
                );
            }
            let days = settings::cleanup_days(&a.settings);
            if days <= settings::DEFAULT_CLEANUP_DAYS {
                out.push(
                    Check::new(
                        format!("retention:{id}"),
                        Status::Warn,
                        format!("Claude Code deletes {}'s transcripts after {days} days", acc.name),
                        "Older sessions disappear from Claude Code, and from Facet's Timeline and Costs.",
                    )
                    .fix("Raise cleanupPeriodDays in this account's settings.json (Facet's Control view will do it with a backup in Phase 2).")
                    .account(id),
                );
            }
        }
    }
    match &a.auth {
        Some(Ok(s)) if !s.logged_in => out.push(
            Check::new(
                format!("auth:{id}"),
                Status::Warn,
                format!("{} is not signed in to Claude Code", acc.name),
                "claude auth status reports no sign-in for this folder.",
            )
            .fix("Open a terminal as this account and run claude to sign in.")
            .account(id),
        ),
        Some(Ok(s)) => {
            let folder_ok = s
                .config_directory
                .as_ref()
                .is_none_or(|d| Path::new(d) == acc.code_dir);
            match (auth::email_matches(s, acc.expected_email.as_deref()), folder_ok) {
                (_, false) => out.push(
                    Check::new(format!("auth:{id}"), Status::Bad, format!("{} reports a different folder", acc.name), format!("Claude Code says it uses {}.", s.config_directory.clone().unwrap_or_default()))
                        .account(id),
                ),
                (Some(false), _) => out.push(
                    Check::new(
                        format!("auth:{id}"),
                        Status::Bad,
                        format!("{} is signed in as someone else", acc.name),
                        format!("Expected {}, signed in as {}.", acc.expected_email.clone().unwrap_or_default(), s.email.clone().unwrap_or_default()),
                    )
                    .fix("Sign out in a terminal for this account (claude auth logout) and sign in with the right email.")
                    .account(id)
                    .personal(),
                ),
                _ => out.push(
                    Check::new(
                        format!("auth:{id}"),
                        Status::Ok,
                        format!("{} is signed in", acc.name),
                        [s.email.clone(), s.org_name.clone(), s.subscription_type.clone().map(|t| format!("plan {t}"))]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" · "),
                    )
                    .account(id)
                    .personal(),
                ),
            }
        }
        Some(Err(e)) => out.push(
            Check::new(
                format!("auth:{id}"),
                Status::Info,
                format!("Could not check {}'s sign-in", acc.name),
                e.clone(),
            )
            .account(id),
        ),
        None => {}
    }
    if let Some(p) = linux::instance_for(acc, &f.running) {
        if !f.launched_by_facet.contains(&p.pid)
            && !f.misplaced.iter().any(|m| m.desktop_account == acc.id)
        {
            out.push(
                Check::new(
                    format!("instance:{id}"),
                    Status::Info,
                    format!("{}'s Desktop was not started by Facet", acc.name),
                    "Facet cannot confirm which folder its Code tab uses until a Code session starts. It will warn if sessions land in the wrong account.",
                )
                .account(id),
            );
        }
    }
}

/// Shell files to scan (names only).
pub fn shell_files(home: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = [
        ".bashrc",
        ".bash_profile",
        ".profile",
        ".zshrc",
        ".zprofile",
        ".config/fish/config.fish",
    ]
    .iter()
    .map(|f| home.join(f))
    .collect();
    if let Ok(entries) = fs::read_dir(home.join(".config/environment.d")) {
        v.extend(
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "conf")),
        );
    }
    v.push(PathBuf::from("/etc/environment"));
    v
}

pub fn gather(
    home: &Path,
    accounts: &[Account],
    launched_by_facet: BTreeSet<u32>,
    check_auth: bool,
) -> Facts {
    let desktop = linux::find_claude_desktop();
    let code = linux::find_claude_code();
    let facet_env_overrides = env::present_override_names(|n| std::env::var_os(n));
    let shell_hits = shell_files(home)
        .into_iter()
        .filter_map(|p| {
            let file = fs::File::open(&p).ok()?;
            let names = env::names_in_shell_reader(std::io::BufReader::new(file));
            (!names.is_empty()).then_some((p, names))
        })
        .collect();
    let running = linux::running_desktops(Path::new("/proc"), home);
    let triples: Vec<(String, Option<PathBuf>, PathBuf)> = accounts
        .iter()
        .map(|a| (a.id.clone(), a.desktop_dir.clone(), a.code_dir.clone()))
        .collect();
    let sessions: Vec<(String, Vec<registry::LiveSession>)> = accounts
        .iter()
        .map(|a| {
            (
                a.id.clone(),
                registry::live_sessions(&a.code_dir, registry::pid_alive),
            )
        })
        .collect();
    let misplaced = registry::misplaced_desktop_sessions(&triples, &sessions, |pid| {
        registry::program_of(Path::new("/proc"), pid)
    });
    let account_facts = accounts
        .iter()
        .map(|a| AccountFacts {
            account: a.clone(),
            code_dir_exists: a.code_dir.is_dir(),
            settings: settings::read(&a.code_dir),
            auth: match (&code, check_auth && a.code_dir.is_dir()) {
                (Some(c), true) => Some(
                    auth::check(&c.path, &a.code_dir, Duration::from_secs(15))
                        .map_err(|e| e.to_string()),
                ),
                _ => None,
            },
        })
        .collect();
    Facts {
        desktop,
        code,
        facet_env_overrides,
        shell_hits,
        running,
        launched_by_facet,
        misplaced,
        accounts: account_facts,
        shortcut: linux::global_shortcut_support(),
        stray_state: {
            let p = home.join(".claude/.claude.json");
            (p.is_file() && home.join(".claude.json").is_file()).then_some(p)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(id: &str, desktop: Option<&str>, code: &str) -> Account {
        Account {
            id: id.into(),
            name: id[..1].to_uppercase() + &id[1..],
            color: 1,
            plan: None,
            desktop_dir: desktop.map(PathBuf::from),
            code_dir: PathBuf::from(code),
            expected_email: Some("me@example.com".into()),
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

    fn facts(accounts: Vec<AccountFacts>) -> Facts {
        Facts {
            desktop: Some(ClaudeInstall {
                path: "/usr/lib/claude-desktop/claude-desktop".into(),
                version: Some("3.6556.2".into()),
                channel: "deb".into(),
                official: true,
            }),
            code: Some(ClaudeInstall {
                path: "/usr/bin/claude".into(),
                version: Some("2.1.310".into()),
                channel: "path".into(),
                official: true,
            }),
            facet_env_overrides: vec![],
            shell_hits: vec![],
            running: vec![],
            launched_by_facet: BTreeSet::new(),
            misplaced: vec![],
            accounts,
            shortcut: ShortcutSupport::Portal,
            stray_state: None,
        }
    }

    fn ok_settings() -> SettingsRead {
        SettingsRead::Ok(serde_json::json!({"cleanupPeriodDays": 365}))
    }

    fn signed_in(email: &str, dir: &str) -> Option<Result<AuthStatus, String>> {
        Some(Ok(AuthStatus {
            logged_in: true,
            auth_method: Some("claude.ai".into()),
            email: Some(email.into()),
            org_name: None,
            subscription_type: Some("max".into()),
            config_directory: Some(dir.into()),
        }))
    }

    fn find<'a>(checks: &'a [Check], id: &str) -> &'a Check {
        checks.iter().find(|c| c.id == id).unwrap_or_else(|| {
            panic!(
                "no check {id}: {:?}",
                checks.iter().map(|c| &c.id).collect::<Vec<_>>()
            )
        })
    }

    #[test]
    fn healthy_setup_is_all_ok_and_sorted_worst_first() {
        let f = facts(vec![AccountFacts {
            account: acc("work", Some("/h/.config/Claude-Work"), "/h/.claude-work"),
            code_dir_exists: true,
            settings: ok_settings(),
            auth: signed_in("me@example.com", "/h/.claude-work"),
        }]);
        let checks = evaluate(&f);
        assert!(checks.iter().all(|c| c.status == Status::Ok), "{checks:#?}");
        assert_eq!(
            find(&checks, "auth:work").detail,
            "me@example.com · plan max"
        );
    }

    #[test]
    fn flags_overrides_retention_wrong_email_and_folder() {
        let mut f = facts(vec![
            AccountFacts {
                account: acc("work", None, "/h/.claude-work"),
                code_dir_exists: true,
                settings: SettingsRead::Missing,
                auth: signed_in("other@example.com", "/h/.claude-work"),
            },
            AccountFacts {
                account: acc("ci", None, "/h/.claude-ci"),
                code_dir_exists: true,
                settings: ok_settings(),
                auth: signed_in("me@example.com", "/h/.claude"),
            },
        ]);
        f.facet_env_overrides = vec!["ANTHROPIC_API_KEY"];
        f.shell_hits = vec![(PathBuf::from("/h/.bashrc"), vec!["ANTHROPIC_API_KEY"])];
        let checks = evaluate(&f);
        assert_eq!(find(&checks, "env-facet").status, Status::Bad);
        assert_eq!(find(&checks, "env-file:/h/.bashrc").status, Status::Warn);
        assert_eq!(find(&checks, "retention:work").status, Status::Warn);
        assert_eq!(find(&checks, "auth:work").status, Status::Bad);
        assert!(find(&checks, "auth:work").title.contains("someone else"));
        assert!(find(&checks, "auth:ci").title.contains("different folder"));
        assert_eq!(checks[0].status, Status::Bad);
        // Shell-file detail must carry names only.
        assert!(!find(&checks, "env-file:/h/.bashrc").detail.contains('='));
    }

    #[test]
    fn misplaced_desktop_sessions_are_bad_and_name_both_accounts() {
        let mut f = facts(vec![
            AccountFacts {
                account: acc("personal", Some("/h/.config/Claude"), "/h/.claude"),
                code_dir_exists: true,
                settings: ok_settings(),
                auth: None,
            },
            AccountFacts {
                account: acc("work", Some("/h/.config/Claude-Work"), "/h/.claude-work"),
                code_dir_exists: true,
                settings: ok_settings(),
                auth: None,
            },
        ]);
        f.running = vec![DesktopProcess {
            pid: 7,
            data_dir: "/h/.config/Claude-Work".into(),
            x11: true,
        }];
        f.misplaced = vec![MisplacedSession {
            pid: 9,
            desktop_account: "work".into(),
            folder_account: "personal".into(),
        }];
        let checks = evaluate(&f);
        let m = find(&checks, "misplaced:9");
        assert_eq!(m.status, Status::Bad);
        assert_eq!(
            m.title,
            "Work's Desktop is writing Code sessions into Personal's folder"
        );
        // No duplicate "not started by Facet" info when the stronger finding exists.
        assert!(!checks.iter().any(|c| c.id == "instance:work"));
    }

    #[test]
    fn stray_state_file_is_reported() {
        let mut f = facts(vec![]);
        f.stray_state = Some(PathBuf::from("/h/.claude/.claude.json"));
        assert_eq!(find(&evaluate(&f), "stray-state").status, Status::Info);
    }

    #[test]
    fn unknown_instances_and_missing_installs() {
        let mut f = facts(vec![AccountFacts {
            account: acc("work", Some("/h/.config/Claude-Work"), "/h/.claude-work"),
            code_dir_exists: false,
            settings: SettingsRead::Missing,
            auth: None,
        }]);
        f.desktop = None;
        f.code = None;
        f.running = vec![DesktopProcess {
            pid: 5,
            data_dir: "/h/.config/Claude-Old".into(),
            x11: false,
        }];
        f.shortcut = ShortcutSupport::CliOnly;
        let checks = evaluate(&f);
        assert_eq!(find(&checks, "desktop-install").status, Status::Bad);
        assert_eq!(find(&checks, "code-install").status, Status::Warn);
        assert_eq!(find(&checks, "code-dir:work").status, Status::Info);
        assert!(
            !checks.iter().any(|c| c.id == "retention:work"),
            "skip settings checks before the folder exists"
        );
        assert_eq!(find(&checks, "unknown-instance:5").status, Status::Info);
        assert_eq!(find(&checks, "shortcut").status, Status::Info);
    }
}
