//! Accounts ("profiles") and their store. Holds no secrets, ever: only names,
//! colours and folder paths (DATA-MODEL.md `account`).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub const FACET_COLORS: u8 = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Slug used in file names and launchers, e.g. `work`.
    pub id: String,
    pub name: String,
    /// Facet colour 1..=6 (`--facet-N`).
    pub color: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// Claude Desktop `--user-data-dir`. `None` = this account has no Desktop instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_dir: Option<PathBuf>,
    /// `CLAUDE_CONFIG_DIR` for Claude Code and for the Desktop instance's Code tab.
    pub code_dir: PathBuf,
    /// Typed by the user; compared with `claude auth status` after sign-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_email: Option<String>,
    /// Tint the Facet window in this account's colour when it is active.
    #[serde(default = "yes")]
    pub tint: bool,
    /// Run the Desktop instance under XWayland so it can get its own dock icon.
    #[serde(default)]
    pub x11_identity: bool,
    pub created_at: i64,
    /// Up to three characters or one emoji shown instead of the initial.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    /// File name of the account picture in Facet's avatars folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Free notes for the user's own reference (client, billing, purpose).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// What the menu and dock icon of this account's Desktop app shows.
    #[serde(default)]
    pub launcher_icon: LauncherIcon,
    /// Folder that "Open terminal" starts in. `None` = home.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_dir: Option<PathBuf>,
    /// Name shown in the dock and app menu. `None` = "<name> · Facet".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher_name: Option<String>,
}

/// Launcher and dock icon style.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LauncherIcon {
    /// Facet's own square in the account colour (initial, badge or picture).
    /// The default: no Anthropic artwork, nothing that implies endorsement.
    #[default]
    Avatar,
    /// Opt-in: the Claude app icon installed on this computer with the
    /// account's badge in a corner, composed locally (rule 8 note in D-018).
    ClaudeBadge,
    /// Opt-in: the Claude app's own installed icon, unchanged (the launcher
    /// names the system icon `claude-desktop`; Facet ships no artwork).
    Claude,
}

pub const MAX_NOTES: usize = 2000;

/// A badge is short and printable: up to three characters, or one emoji
/// sequence (which may be several code points).
pub fn validate_badge(b: &str) -> Result<String> {
    let b = b.trim();
    if b.chars().any(|c| c.is_control()) || b.len() > 32 {
        return Err(Error::Invalid(
            "Use up to three letters or one emoji for the badge.".into(),
        ));
    }
    let visible = b
        .chars()
        .filter(|c| !matches!(*c as u32, 0x200D | 0xFE0E | 0xFE0F | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F))
        .count();
    let has_emoji = b.chars().any(|c| (c as u32) >= 0x2190);
    if visible == 0 || (!has_emoji && visible > 3) || (has_emoji && visible > 4) {
        return Err(Error::Invalid(
            "Use up to three letters or one emoji for the badge.".into(),
        ));
    }
    Ok(b.to_string())
}

fn yes() -> bool {
    true
}

/// What the UI sends to create or update an account.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub id: Option<String>,
    pub name: String,
    pub color: Option<u8>,
    pub plan: Option<String>,
    pub with_desktop: bool,
    pub desktop_dir: Option<PathBuf>,
    pub code_dir: Option<PathBuf>,
    pub expected_email: Option<String>,
    pub tint: Option<bool>,
    pub x11_identity: Option<bool>,
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub launcher_icon: Option<LauncherIcon>,
    #[serde(default)]
    pub terminal_dir: Option<PathBuf>,
    #[serde(default)]
    pub launcher_name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreFile {
    #[serde(default)]
    account: Vec<Account>,
}

/// Turn a display name into an id: lowercase ASCII letters, digits and dashes.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    out.chars().take(32).collect()
}

/// Ids that would make `claude-<id>` clash with official command names or
/// with Facet's own `claude-auto` (compliance review finding 5).
pub const RESERVED_IDS: [&str; 6] = ["desktop", "code", "auto", "cli", "app", "facet"];

pub fn validate_id(id: &str) -> Result<()> {
    if RESERVED_IDS.contains(&id) {
        return Err(Error::Invalid(format!(
            "\"{id}\" is reserved because claude-{id} could hide another command. Pick another name."
        )));
    }
    let ok = !id.is_empty()
        && id.len() <= 32
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "Account id \"{id}\" must be 1–32 lowercase letters, digits or dashes."
        )))
    }
}

/// Default folders for a new account. The first account on a machine usually
/// already lives in the Claude defaults (`~/.config/Claude`, `~/.claude`).
pub fn default_dirs(home: &Path, id: &str, use_claude_defaults: bool) -> (PathBuf, PathBuf) {
    if use_claude_defaults {
        (home.join(".config/Claude"), home.join(".claude"))
    } else {
        let title: String = id
            .split('-')
            .map(|part| {
                let mut c = part.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("-");
        (
            home.join(format!(".config/Claude-{title}")),
            home.join(format!(".claude-{id}")),
        )
    }
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Vec<Account>> {
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                let file: StoreFile = toml::from_str(&text).map_err(|e| {
                    Error::Invalid(format!("{} is not valid: {e}", self.path.display()))
                })?;
                Ok(file.account)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Atomic write: temp file in the same folder, fsync, rename.
    fn save_all(&self, accounts: &[Account]) -> Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| Error::Invalid("store has no folder".into()))?;
        fs::create_dir_all(dir)?;
        let text = toml::to_string_pretty(&StoreFile {
            account: accounts.to_vec(),
        })
        .map_err(|e| Error::Invalid(e.to_string()))?;
        let tmp = self.path.with_extension("toml.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            // Owner-only: the file holds expected sign-in emails (personal data).
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(fs::Permissions::from_mode(0o600))?;
            f.write_all(b"# Facet accounts. No secrets are stored here.\n")?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Create or update. Never creates or deletes the account's folders.
    pub fn upsert(&self, input: AccountInput, home: &Path, now_ms: i64) -> Result<Account> {
        let mut accounts = self.load()?;
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(Error::Invalid("Give the account a name.".into()));
        }
        let existing_idx = input
            .id
            .as_ref()
            .and_then(|id| accounts.iter().position(|a| &a.id == id));
        let id = match (&input.id, existing_idx) {
            (Some(id), Some(_)) => id.clone(),
            (Some(id), None) => id.clone(),
            (None, _) => slugify(&name),
        };
        validate_id(&id)?;
        if existing_idx.is_none() && accounts.iter().any(|a| a.id == id) {
            return Err(Error::Invalid(format!(
                "An account called \"{id}\" already exists."
            )));
        }

        let first = accounts.is_empty() && existing_idx.is_none();
        let (def_desktop, def_code) = default_dirs(home, &id, first);
        let code_dir = input.code_dir.clone().unwrap_or(def_code);
        let desktop_dir = if input.with_desktop {
            Some(input.desktop_dir.clone().unwrap_or(def_desktop))
        } else {
            None
        };
        let color = match input.color {
            Some(c) if (1..=FACET_COLORS).contains(&c) => c,
            Some(c) => {
                return Err(Error::Invalid(format!(
                    "Colour {c} is not one of the six facets."
                )))
            }
            None => first_unused_color(&accounts),
        };

        // Two accounts must never share a folder: that would mix sign-ins.
        for other in accounts
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != existing_idx)
            .map(|(_, a)| a)
        {
            if same_path(&other.code_dir, &code_dir) {
                return Err(Error::Invalid(format!(
                    "{} already uses {}. Each account needs its own Claude Code folder.",
                    other.name,
                    code_dir.display()
                )));
            }
            if let (Some(a), Some(b)) = (&other.desktop_dir, &desktop_dir) {
                if same_path(a, b) {
                    return Err(Error::Invalid(format!(
                        "{} already uses {}. Each account needs its own Desktop folder.",
                        other.name,
                        b.display()
                    )));
                }
            }
        }
        let email = input
            .expected_email
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty());
        if let Some(e) = &email {
            if !e.contains('@') {
                return Err(Error::Invalid(format!(
                    "\"{e}\" does not look like an email address."
                )));
            }
        }

        let prev = existing_idx.map(|i| accounts[i].clone());
        let badge = match input.badge.as_deref().map(str::trim) {
            Some("") | None => None,
            Some(b) => Some(validate_badge(b)?),
        };
        let notes = input
            .notes
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        if notes
            .as_ref()
            .is_some_and(|n| n.chars().count() > MAX_NOTES)
        {
            return Err(Error::Invalid(format!(
                "Notes can be up to {MAX_NOTES} characters."
            )));
        }
        let terminal_dir = input.terminal_dir.filter(|d| !d.as_os_str().is_empty());
        if let Some(d) = &terminal_dir {
            if !d.is_absolute() {
                return Err(Error::Invalid(format!(
                    "The terminal folder must be a full path, not {}.",
                    d.display()
                )));
            }
        }
        let launcher_name = input
            .launcher_name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());
        if launcher_name
            .as_ref()
            .is_some_and(|n| n.chars().count() > 64 || n.chars().any(char::is_control))
        {
            return Err(Error::Invalid(
                "The dock name can be up to 64 characters on one line.".into(),
            ));
        }
        let account = Account {
            id: id.clone(),
            name,
            color,
            plan: input
                .plan
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty()),
            desktop_dir,
            code_dir,
            expected_email: email,
            tint: input.tint.unwrap_or(true),
            x11_identity: input.x11_identity.unwrap_or(!first),
            created_at: existing_idx
                .map(|i| accounts[i].created_at)
                .unwrap_or(now_ms),
            badge,
            image: prev.as_ref().and_then(|p| p.image.clone()),
            notes,
            launcher_icon: input
                .launcher_icon
                .or(prev.as_ref().map(|p| p.launcher_icon))
                .unwrap_or_default(),
            terminal_dir,
            launcher_name,
        };
        match existing_idx {
            Some(i) => accounts[i] = account.clone(),
            None => accounts.push(account.clone()),
        }
        self.save_all(&accounts)?;
        Ok(account)
    }

    /// Forget an account. Its folders, sign-in and transcripts stay untouched.
    pub fn remove(&self, id: &str) -> Result<()> {
        let mut accounts = self.load()?;
        let before = accounts.len();
        accounts.retain(|a| a.id != id);
        if accounts.len() == before {
            return Err(Error::NotFound(format!("account {id}")));
        }
        self.save_all(&accounts)
    }

    /// Put accounts in the given order. Ids not listed keep their relative
    /// order after the listed ones.
    pub fn reorder(&self, ids: &[String]) -> Result<Vec<Account>> {
        let mut accounts = self.load()?;
        let pos = |a: &Account| ids.iter().position(|i| i == &a.id).unwrap_or(usize::MAX);
        accounts.sort_by_key(pos);
        self.save_all(&accounts)?;
        Ok(accounts)
    }

    /// Change an account's id (used in `claude-<id>`, launchers and rules).
    /// Only the store entry changes here; callers move what hangs off the id.
    pub fn rename_id(&self, old: &str, new: &str) -> Result<Account> {
        validate_id(new)?;
        let mut accounts = self.load()?;
        if accounts.iter().any(|a| a.id == new) {
            return Err(Error::Invalid(format!(
                "An account called \"{new}\" already exists."
            )));
        }
        let acc = accounts
            .iter_mut()
            .find(|a| a.id == old)
            .ok_or_else(|| Error::NotFound(format!("account {old}")))?;
        acc.id = new.to_string();
        let out = acc.clone();
        self.save_all(&accounts)?;
        Ok(out)
    }

    /// Record (or clear) the picture file name.
    pub fn set_image(&self, id: &str, image: Option<String>) -> Result<Account> {
        let mut accounts = self.load()?;
        let acc = accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| Error::NotFound(format!("account {id}")))?;
        acc.image = image;
        let out = acc.clone();
        self.save_all(&accounts)?;
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<Account> {
        self.load()?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| Error::NotFound(format!("account {id}")))
    }
}

pub fn first_unused_color(accounts: &[Account]) -> u8 {
    (1..=FACET_COLORS)
        .find(|c| !accounts.iter().any(|a| a.color == *c))
        .unwrap_or_else(|| (accounts.len() as u8 % FACET_COLORS) + 1)
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    norm(a) == norm(b)
}

/// A folder pair found on disk that looks like an existing hand-made setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedSetup {
    pub suggested_id: String,
    pub suggested_name: String,
    pub desktop_dir: Option<PathBuf>,
    pub code_dir: Option<PathBuf>,
}

/// `client-acme` → `Client Acme`.
pub fn display_name(id: &str) -> String {
    id.split('-')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Look for `~/.config/Claude*` and `~/.claude*` folders (names only).
pub fn detect_existing(home: &Path) -> Vec<DetectedSetup> {
    let mut found: Vec<DetectedSetup> = Vec::new();
    let mut push = |id: String, desktop: Option<PathBuf>, code: Option<PathBuf>| {
        if let Some(entry) = found.iter_mut().find(|d| d.suggested_id == id) {
            if desktop.is_some() {
                entry.desktop_dir = desktop;
            }
            if code.is_some() {
                entry.code_dir = code;
            }
        } else {
            let name = if id == "personal" {
                "Personal".to_string()
            } else {
                display_name(&id)
            };
            found.push(DetectedSetup {
                suggested_id: id,
                suggested_name: name,
                desktop_dir: desktop,
                code_dir: code,
            });
        }
    };
    if let Ok(entries) = fs::read_dir(home.join(".config")) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for n in names {
            if n == "Claude" {
                push("personal".into(), Some(home.join(".config/Claude")), None);
            } else if let Some(rest) = n.strip_prefix("Claude-") {
                let id = slugify(rest);
                if !id.is_empty() {
                    push(id, Some(home.join(".config").join(&n)), None);
                }
            }
        }
    }
    if let Ok(entries) = fs::read_dir(home) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for n in names {
            if n == ".claude" {
                push("personal".into(), None, Some(home.join(".claude")));
            } else if let Some(rest) = n.strip_prefix(".claude-") {
                let id = slugify(rest);
                if !id.is_empty() {
                    push(id, None, Some(home.join(&n)));
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> AccountInput {
        AccountInput {
            id: None,
            name: name.into(),
            color: None,
            plan: None,
            with_desktop: true,
            desktop_dir: None,
            code_dir: None,
            expected_email: None,
            tint: None,
            x11_identity: None,
            badge: None,
            notes: None,
            launcher_icon: None,
            terminal_dir: None,
            launcher_name: None,
        }
    }

    #[test]
    fn slugify_makes_safe_ids() {
        assert_eq!(slugify("Client A (Acme)"), "client-a-acme");
        assert_eq!(slugify("  Work  "), "work");
        assert_eq!(slugify("عمل"), "");
    }

    #[test]
    fn first_account_uses_claude_defaults_later_ones_get_their_own() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let store = Store::new(home.join("cfg/profiles.toml"));
        let a = store.upsert(input("Personal"), home, 1).unwrap();
        assert_eq!(a.code_dir, home.join(".claude"));
        assert_eq!(a.desktop_dir, Some(home.join(".config/Claude")));
        assert_eq!(a.color, 1);
        assert!(!a.x11_identity);
        let b = store.upsert(input("Client Acme"), home, 2).unwrap();
        assert_eq!(b.id, "client-acme");
        assert_eq!(b.code_dir, home.join(".claude-client-acme"));
        assert_eq!(b.desktop_dir, Some(home.join(".config/Claude-Client-Acme")));
        assert_eq!(b.color, 2);
        assert!(b.x11_identity);
        assert_eq!(store.load().unwrap().len(), 2);
    }

    #[test]
    fn rejects_shared_folders_and_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let store = Store::new(home.join("profiles.toml"));
        store.upsert(input("Work"), home, 1).unwrap();
        let mut clash = input("Other");
        clash.code_dir = Some(home.join(".claude"));
        let err = store.upsert(clash, home, 2).unwrap_err().to_string();
        assert!(err.contains("own Claude Code folder"), "{err}");
        let err = store
            .upsert(input("work"), home, 3)
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"), "{err}");
    }

    #[test]
    fn update_keeps_created_at_and_validates_email_and_colour() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let store = Store::new(home.join("profiles.toml"));
        let a = store.upsert(input("Work"), home, 10).unwrap();
        let mut edit = input("Work (Acme)");
        edit.id = Some(a.id.clone());
        edit.code_dir = Some(a.code_dir.clone());
        edit.expected_email = Some(" me@example.com ".into());
        edit.color = Some(4);
        let b = store.upsert(edit, home, 99).unwrap();
        assert_eq!(b.created_at, 10);
        assert_eq!(b.expected_email.as_deref(), Some("me@example.com"));
        assert_eq!(b.name, "Work (Acme)");
        let mut bad = input("X");
        bad.expected_email = Some("nope".into());
        assert!(store.upsert(bad, home, 1).is_err());
        let mut bad = input("Y");
        bad.color = Some(9);
        assert!(store.upsert(bad, home, 1).is_err());
    }

    #[test]
    fn reserved_ids_and_private_file_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let store = Store::new(home.join("profiles.toml"));
        let err = store
            .upsert(input("Desktop"), home, 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("reserved"), "{err}");
        store.upsert(input("Personal"), home, 1).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(store.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn remove_forgets_but_never_touches_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        fs::create_dir_all(home.join(".claude")).unwrap();
        let store = Store::new(home.join("profiles.toml"));
        let a = store.upsert(input("Personal"), home, 1).unwrap();
        store.remove(&a.id).unwrap();
        assert!(store.load().unwrap().is_empty());
        assert!(home.join(".claude").exists());
        assert!(store.remove("ghost").is_err());
    }

    #[test]
    fn store_file_has_no_secret_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let store = Store::new(home.join("profiles.toml"));
        store.upsert(input("Personal"), home, 1).unwrap();
        let text = fs::read_to_string(store.path()).unwrap().to_lowercase();
        for word in ["token", "password", "cookie", "credential", "oauth"] {
            assert!(!text.contains(word), "store must not mention {word}");
        }
    }

    #[test]
    fn detects_hand_made_setup_by_folder_names() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        for d in [
            ".config/Claude",
            ".config/Claude-Work",
            ".claude",
            ".claude-work",
            ".claude-client-b",
            ".config/Other",
        ] {
            fs::create_dir_all(home.join(d)).unwrap();
        }
        let found = detect_existing(home);
        let ids: Vec<&str> = found.iter().map(|d| d.suggested_id.as_str()).collect();
        assert_eq!(ids, vec!["personal", "work", "client-b"]);
        let work = &found[1];
        assert_eq!(work.desktop_dir, Some(home.join(".config/Claude-Work")));
        assert_eq!(work.code_dir, Some(home.join(".claude-work")));
        assert_eq!(found[2].desktop_dir, None);
        assert_eq!(found[2].suggested_name, "Client B");
    }

    #[test]
    fn badges_notes_reorder_and_rename() {
        assert!(validate_badge("AB").is_ok());
        assert!(validate_badge("🚀").is_ok());
        assert!(validate_badge("👩‍💻").is_ok());
        assert!(validate_badge("ABCD").is_err());
        assert!(validate_badge("a\nb").is_err());
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("p.toml"));
        let home = Path::new("/h");
        let mut i = input("Work");
        i.badge = Some(" W1 ".into());
        i.notes = Some("Client Acme".into());
        let w = store.upsert(i, home, 1).unwrap();
        assert_eq!(w.badge.as_deref(), Some("W1"));
        assert_eq!(w.launcher_icon, LauncherIcon::Avatar);
        store.upsert(input("Personal"), home, 2).unwrap();
        let order = store.reorder(&["personal".into(), "work".into()]).unwrap();
        assert_eq!(order[0].id, "personal");
        let mut bad = input("X");
        bad.terminal_dir = Some("relative/dir".into());
        assert!(store.upsert(bad, home, 3).is_err());
        assert!(store.rename_id("work", "personal").is_err());
        assert!(store.rename_id("work", "code").is_err());
        let r = store.rename_id("work", "acme").unwrap();
        assert_eq!(
            (r.id.as_str(), r.name.as_str(), r.created_at),
            ("acme", "Work", 1)
        );
        assert_eq!(store.load().unwrap()[1].id, "acme");
        // An edit keeps the picture, which is set separately.
        store.set_image("acme", Some("acme-1.png".into())).unwrap();
        let mut e = input("Work (Acme)");
        e.id = Some("acme".into());
        e.code_dir = Some(r.code_dir.clone());
        e.desktop_dir = r.desktop_dir.clone();
        assert_eq!(
            store.upsert(e, home, 9).unwrap().image.as_deref(),
            Some("acme-1.png")
        );
    }
}
