//! Folder rules (PRD M7): pick the account from the folder you are in.
//!
//! A rule maps a folder to an account; the most specific (longest) folder
//! wins. Facet writes the rules into a self-contained POSIX script,
//! `~/.local/bin/claude-auto`, which runs the official `claude` in the
//! matching account. The real `claude` is never replaced or shadowed (on many
//! machines it lives in `~/.local/bin/claude` itself).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::adapter::linux::shell_quote;
use crate::env;
use crate::error::{Error, Result};
use crate::profile::Account;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRule {
    /// Absolute folder; the rule covers it and everything below it.
    pub folder: PathBuf,
    pub account_id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RulesFile {
    #[serde(default)]
    rule: Vec<FolderRule>,
}

pub struct RuleStore {
    path: PathBuf,
}

impl RuleStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Vec<FolderRule>> {
        match fs::read_to_string(&self.path) {
            Ok(text) => toml::from_str::<RulesFile>(&text)
                .map(|f| f.rule)
                .map_err(|e| Error::Invalid(format!("{} is not valid: {e}", self.path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Validate and save all rules atomically.
    pub fn save(
        &self,
        rules: &[FolderRule],
        accounts: &[Account],
        home: &Path,
    ) -> Result<Vec<FolderRule>> {
        let mut clean: Vec<FolderRule> = Vec::new();
        for r in rules {
            let folder = normalize(&r.folder, home)?;
            if !accounts.iter().any(|a| a.id == r.account_id) {
                return Err(Error::Invalid(format!(
                    "No account called \"{}\".",
                    r.account_id
                )));
            }
            if clean.iter().any(|c| c.folder == folder) {
                return Err(Error::Invalid(format!(
                    "{} has two rules. Keep one.",
                    folder.display()
                )));
            }
            clean.push(FolderRule {
                folder,
                account_id: r.account_id.clone(),
            });
        }
        clean.sort_by(|a, b| a.folder.cmp(&b.folder));
        let dir = self
            .path
            .parent()
            .ok_or_else(|| Error::Invalid("rules file has no folder".into()))?;
        fs::create_dir_all(dir)?;
        let text = toml::to_string_pretty(&RulesFile {
            rule: clean.clone(),
        })
        .map_err(|e| Error::Invalid(e.to_string()))?;
        let tmp = self.path.with_extension("toml.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(b"# Facet folder rules: the longest matching folder picks the account.\n")?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(clean)
    }
}

/// Expand `~`, require an absolute path, drop trailing slashes and `.` parts.
pub fn normalize(folder: &Path, home: &Path) -> Result<PathBuf> {
    let text = folder.to_string_lossy();
    let expanded = if text == "~" {
        home.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/") {
        home.join(rest)
    } else {
        folder.to_path_buf()
    };
    if !expanded.is_absolute() {
        return Err(Error::Invalid(format!(
            "Use a full folder path, for example ~/work (got \"{text}\")."
        )));
    }
    let mut out = PathBuf::from("/");
    for part in expanded.components() {
        match part {
            std::path::Component::Normal(p) => out.push(p),
            std::path::Component::ParentDir => {
                return Err(Error::Invalid(format!(
                    "\"{text}\" contains \"..\"; use the folder's real path."
                )))
            }
            _ => {}
        }
    }
    if out == Path::new("/") {
        return Err(Error::Invalid(
            "A rule for / would cover everything; pick a narrower folder.".into(),
        ));
    }
    Ok(out)
}

/// The rule for `cwd`: the longest folder that contains it.
pub fn match_folder<'a>(rules: &'a [FolderRule], cwd: &Path) -> Option<&'a FolderRule> {
    rules
        .iter()
        .filter(|r| cwd.starts_with(&r.folder))
        .max_by_key(|r| r.folder.components().count())
}

/// Generate `claude-auto`. Rules become a `case` on `$PWD`, most specific
/// first. With no match it prints the fallback hint and exits, rather than
/// guessing an account.
pub fn auto_script(
    rules: &[FolderRule],
    accounts: &[Account],
    claude: &Path,
    home: &Path,
) -> String {
    let mut sorted: Vec<&FolderRule> = rules.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.folder.components().count()));
    let mut cases = String::new();
    for r in sorted {
        let Some(acc) = accounts.iter().find(|a| a.id == r.account_id) else {
            continue;
        };
        let folder = sh_glob_escape(&r.folder.to_string_lossy());
        cases.push_str(&format!(
            "  {folder}|{folder}/*) FACET_DIR={dir}; FACET_ACCOUNT={id} ;;\n",
            // The default folder (~/.claude) must leave CLAUDE_CONFIG_DIR unset (D-016).
            dir = if env::is_default_config_dir(&acc.code_dir, home) {
                "''".to_string()
            } else {
                shell_quote(&acc.code_dir.to_string_lossy())
            },
            id = shell_quote(&acc.id),
        ));
    }
    format!(
        "#!/bin/sh\n# Written by Facet. Runs the official Claude Code in the account whose folder rule\n# matches the current folder (longest folder wins). Edit rules in Facet, not here.\nunset {vars}\ncase \"$PWD\" in\n{cases}  *) echo \"claude-auto: no Facet folder rule covers $PWD. Add one in Facet › Folder rules, or use claude-<account>.\" >&2; exit 64 ;;\nesac\nif [ -n \"$FACET_DIR\" ]; then CLAUDE_CONFIG_DIR=\"$FACET_DIR\"; export CLAUDE_CONFIG_DIR; else unset CLAUDE_CONFIG_DIR; fi\nunset FACET_DIR\nexport FACET_ACCOUNT\nexec {claude} \"$@\"\n",
        vars = env::OVERRIDE_VARS.join(" "),
        claude = shell_quote(&claude.to_string_lossy()),
    )
}

/// Quote a path for a `case` pattern: glob characters must match literally.
fn sh_glob_escape(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

pub fn auto_script_path(home: &Path) -> PathBuf {
    home.join(".local/bin/claude-auto")
}

/// Write `claude-auto`; refuses to overwrite a file Facet did not write.
pub fn install_auto_script(
    home: &Path,
    rules: &[FolderRule],
    accounts: &[Account],
    claude: &Path,
) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = auto_script_path(home);
    if let Ok(existing) = fs::read_to_string(&path) {
        if !existing.contains("Written by Facet") {
            return Err(Error::Invalid(format!(
                "{} already exists and was not written by Facet. Rename or remove it first.",
                path.display()
            )));
        }
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("facet-tmp");
    fs::write(&tmp, auto_script(rules, accounts, claude, home))?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(&tmp, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn acc(id: &str, code: &str) -> Account {
        Account {
            id: id.into(),
            name: id.into(),
            color: 1,
            plan: None,
            desktop_dir: None,
            code_dir: PathBuf::from(code),
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

    fn rule(folder: &str, id: &str) -> FolderRule {
        FolderRule {
            folder: PathBuf::from(folder),
            account_id: id.into(),
        }
    }

    #[test]
    fn normalize_expands_home_and_rejects_bad_paths() {
        let home = Path::new("/home/u");
        assert_eq!(
            normalize(Path::new("~/work/"), home).unwrap(),
            PathBuf::from("/home/u/work")
        );
        assert_eq!(
            normalize(Path::new("/srv/./client"), home).unwrap(),
            PathBuf::from("/srv/client")
        );
        assert!(normalize(Path::new("work"), home).is_err());
        assert!(normalize(Path::new("/srv/../etc"), home).is_err());
        assert!(normalize(Path::new("/"), home).is_err());
    }

    #[test]
    fn longest_folder_wins_and_siblings_do_not_match() {
        let rules = vec![
            rule("/home/u/work", "work"),
            rule("/home/u/work/acme", "acme"),
        ];
        assert_eq!(
            match_folder(&rules, Path::new("/home/u/work/acme/app"))
                .unwrap()
                .account_id,
            "acme"
        );
        assert_eq!(
            match_folder(&rules, Path::new("/home/u/work/other"))
                .unwrap()
                .account_id,
            "work"
        );
        assert!(match_folder(&rules, Path::new("/home/u/workshop")).is_none());
        assert!(match_folder(&rules, Path::new("/home/u")).is_none());
    }

    #[test]
    fn store_validates_accounts_and_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let store = RuleStore::new(tmp.path().join("rules.toml"));
        let accounts = vec![acc("work", "/h/.claude-work")];
        let home = Path::new("/home/u");
        assert!(store
            .save(&[rule("~/x", "ghost")], &accounts, home)
            .is_err());
        assert!(store
            .save(
                &[rule("~/x", "work"), rule("/home/u/x/", "work")],
                &accounts,
                home
            )
            .is_err());
        let saved = store.save(&[rule("~/x", "work")], &accounts, home).unwrap();
        assert_eq!(saved, vec![rule("/home/u/x", "work")]);
        assert_eq!(store.load().unwrap(), saved);
    }

    /// Run the generated script with `sh` from different folders, using a
    /// fake `claude` that prints its environment.
    #[test]
    fn generated_script_picks_account_and_scrubs_env() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let work = root.join("work dir");
        let acme = work.join("acme");
        let other = root.join("other");
        for d in [&acme, &other] {
            fs::create_dir_all(d).unwrap();
        }
        let fake = root.join("fake-claude");
        fs::write(&fake, "#!/bin/sh\necho \"dir=$CLAUDE_CONFIG_DIR acct=$FACET_ACCOUNT key=${ANTHROPIC_API_KEY:-none} args=$*\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let accounts = vec![
            acc("work", "/h/.claude-work"),
            acc("acme", "/h/.claude-acme"),
        ];
        let rules = vec![
            FolderRule {
                folder: work.clone(),
                account_id: "work".into(),
            },
            FolderRule {
                folder: acme.clone(),
                account_id: "acme".into(),
            },
        ];
        let script = root.join("claude-auto");
        fs::write(
            &script,
            auto_script(&rules, &accounts, &fake, Path::new("/nonexistent-home")),
        )
        .unwrap();
        let run = |cwd: &Path| {
            Command::new("sh")
                .arg(&script)
                .arg("--version")
                .current_dir(cwd)
                .env("PWD", cwd)
                .env("ANTHROPIC_API_KEY", "fake-from-shell")
                .output()
                .unwrap()
        };
        let out = run(&acme);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "dir=/h/.claude-acme acct=acme key=none args=--version"
        );
        let out = run(&work);
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "dir=/h/.claude-work acct=work key=none args=--version"
        );
        let out = run(&other);
        assert_eq!(out.status.code(), Some(64));
        assert!(String::from_utf8_lossy(&out.stderr).contains("no Facet folder rule"));
    }

    #[test]
    fn default_folder_account_runs_with_the_variable_unset() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let work = tmp.path().join("w");
        fs::create_dir_all(&work).unwrap();
        let fake = tmp.path().join("fake");
        fs::write(
            &fake,
            "#!/bin/sh\necho \"dir=${CLAUDE_CONFIG_DIR-unset} acct=$FACET_ACCOUNT\"\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let personal = acc("personal", &home.join(".claude").to_string_lossy());
        let script = tmp.path().join("auto");
        fs::write(
            &script,
            auto_script(
                &[FolderRule {
                    folder: work.clone(),
                    account_id: "personal".into(),
                }],
                &[personal],
                &fake,
                &home,
            ),
        )
        .unwrap();
        let out = Command::new("sh")
            .arg(&script)
            .current_dir(&work)
            .env("PWD", &work)
            .env("CLAUDE_CONFIG_DIR", "/stale")
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            "dir=unset acct=personal"
        );
    }

    #[test]
    fn glob_characters_in_folders_match_literally() {
        let tmp = tempfile::tempdir().unwrap();
        let star = tmp.path().join("a*b");
        let plain = tmp.path().join("axxb");
        fs::create_dir_all(&star).unwrap();
        fs::create_dir_all(&plain).unwrap();
        let fake = tmp.path().join("fake");
        fs::write(&fake, "#!/bin/sh\necho \"$FACET_ACCOUNT\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        let accounts = vec![acc("s", "/h/s")];
        let script = tmp.path().join("auto");
        fs::write(
            &script,
            auto_script(
                &[FolderRule {
                    folder: star.clone(),
                    account_id: "s".into(),
                }],
                &accounts,
                &fake,
                Path::new("/nonexistent-home"),
            ),
        )
        .unwrap();
        let ok = Command::new("sh")
            .arg(&script)
            .current_dir(&star)
            .env("PWD", &star)
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "s");
        let miss = Command::new("sh")
            .arg(&script)
            .current_dir(&plain)
            .env("PWD", &plain)
            .output()
            .unwrap();
        assert_eq!(miss.status.code(), Some(64), "a*b must not match axxb");
    }

    #[test]
    fn never_overwrites_foreign_claude_auto() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let path = auto_script_path(home);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "#!/bin/sh\necho mine\n").unwrap();
        assert!(install_auto_script(home, &[], &[], Path::new("/usr/bin/claude")).is_err());
    }
}
