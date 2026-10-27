//! Bridge v1 (PRD M8, D-010): a handoff file the user writes from one
//! account's session and opens in another account. Facet never sends
//! messages between sessions and never reads message text: the handoff
//! holds session metadata, the project's git changes (secrets redacted) and
//! the user's own note.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::ecosystem::redact;
use crate::env;
use crate::error::{Error, Result};
use crate::studio;
use crate::usage::SessionSummary;

pub const MAX_DIFF: usize = 120 * 1024;
pub const MAX_NOTE: usize = 8000;

pub fn handoff_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("facet/handoffs")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GitState {
    pub branch: Option<String>,
    /// `git status --porcelain` lines (paths only).
    pub files: Vec<String>,
    pub diff_stat: String,
    pub diff: String,
    pub diff_truncated: bool,
    /// Lines hidden because they looked like secrets.
    pub redacted_lines: usize,
}

fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = env::command("git");
    // A cloned repo's own config must not run anything: no fsmonitor hook,
    // no external diff, no textconv filters, no pager.
    cmd.args([
        "-c",
        "core.quotepath=off",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.untrackedCache=false",
        "-c",
        "diff.external=",
        "--no-pager",
    ])
    .args(args)
    .current_dir(cwd)
    .env("GIT_TERMINAL_PROMPT", "0")
    .env("GIT_OPTIONAL_LOCKS", "0")
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE")
    .env_remove("GIT_CONFIG_PARAMETERS")
    .env_remove("GIT_CONFIG_COUNT")
    .env_remove("GIT_EXTERNAL_DIFF")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    // Own process group, so a timeout also stops anything git started.
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
    let child = cmd.spawn().ok()?;
    let out = wait_with_timeout(child, Duration::from_secs(10))?;
    String::from_utf8(out).ok()
}

fn wait_with_timeout(mut child: std::process::Child, limit: Duration) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let buf = reader.join().ok()?;
                return status.success().then_some(buf);
            }
            Ok(None) if start.elapsed() < limit => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                // SAFETY: plain kill(2) on the child's own process group.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                return None;
            }
        }
    }
}

/// The project's uncommitted changes, without following anything outside it.
pub fn git_state(cwd: &Path) -> Option<GitState> {
    git(cwd, &["rev-parse", "--is-inside-work-tree"]).filter(|s| s.trim() == "true")?;
    let branch = git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let files: Vec<String> = git(
        cwd,
        &["status", "--porcelain=v1", "--untracked-files=normal"],
    )
    .unwrap_or_default()
    .lines()
    .map(|l| l.to_string())
    .take(500)
    .collect();
    // Repository config that can make diff run programs (filters, textconv,
    // diff drivers): leave the diff out rather than risk running them.
    let risky = git(
        cwd,
        &[
            "config",
            "--local",
            "--includes",
            "--get-regexp",
            r"^(filter\..*|diff\..*\.(textconv|command)|diff\.external|core\.fsmonitor)$",
        ],
    )
    // Facet's own -c overrides above show as "command line:"; only the
    // repository's, user's and system settings count.
    .is_some_and(|s| {
        s.lines()
            .any(|l| !l.trim().is_empty() && !l.starts_with("command line:"))
    });
    if risky {
        return Some(GitState {
            branch,
            files,
            diff_stat: "(The diff was left out: this repository's git settings run filters or programs when diffing.)\n".into(),
            diff: String::new(),
            diff_truncated: false,
            redacted_lines: 0,
        });
    }
    let diff_stat = git(
        cwd,
        &[
            "diff",
            "HEAD",
            "--stat",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
        ],
    )
    .unwrap_or_default();
    let raw = git(
        cwd,
        &[
            "diff",
            "HEAD",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
        ],
    )
    .unwrap_or_default();
    let (diff, redacted_lines) = redact_diff(&raw);
    let truncated = diff.len() > MAX_DIFF;
    let diff = if truncated {
        let mut cut = MAX_DIFF;
        while !diff.is_char_boundary(cut) {
            cut -= 1;
        }
        diff[..cut].to_string()
    } else {
        diff
    };
    Some(GitState {
        branch,
        files,
        diff_stat,
        diff,
        diff_truncated: truncated,
        redacted_lines,
    })
}

/// Files that usually hold secrets: shown by name only.
pub fn is_secret_file(path: &str) -> bool {
    let name = path
        .trim_matches('"')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    name.starts_with(".env")
        || name.contains("credential")
        || name.contains("secret")
        || [
            ".pem",
            ".key",
            ".p12",
            ".pfx",
            ".keystore",
            ".jks",
            ".tfvars",
            ".kdbx",
            ".gpg",
        ]
        .iter()
        .any(|e| name.ends_with(e))
        || [
            ".netrc",
            ".npmrc",
            ".pypirc",
            ".pgpass",
            "id_rsa",
            "id_ed25519",
            "id_ecdsa",
            "id_dsa",
            ".htpasswd",
            "auth.json",
        ]
        .contains(&name.as_str())
}

/// Hide diff lines that look like secrets, and whole hunks of env and key files.
pub fn redact_diff(raw: &str) -> (String, usize) {
    let mut out = String::with_capacity(raw.len());
    let mut hidden = 0usize;
    let mut secret_file = false;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let path = rest.rsplit_once(" b/").map(|(_, p)| p).unwrap_or(rest);
            secret_file = is_secret_file(path);
            out.push_str(line);
            out.push('\n');
            if secret_file {
                out.push_str("(contents hidden: this file usually holds secrets)\n");
            }
            continue;
        }
        let body = matches!(line.as_bytes().first(), Some(b'+' | b'-' | b' '))
            && !line.starts_with("+++")
            && !line.starts_with("---");
        if body && (secret_file || studio::find_secret(line).is_some() || redact(line) != line) {
            hidden += 1;
            if !secret_file {
                out.push_str(&line[..1]);
                out.push_str("‹line hidden: looked like a secret›\n");
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    (out, hidden)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Handoff {
    pub name: String,
    pub path: String,
    pub text: String,
}

pub struct HandoffInput<'a> {
    pub from_name: &'a str,
    pub to_name: &'a str,
    pub session: &'a SessionSummary,
    pub note: &'a str,
    pub git: Option<&'a GitState>,
    pub stamp: &'a str,
}

fn fence(s: &str) -> String {
    let mut f = "```".to_string();
    while s.contains(&f) {
        f.push('`');
    }
    f
}

/// The handoff text. Plain Markdown so any Claude session can read it.
pub fn compose(h: &HandoffInput) -> String {
    let s = h.session;
    let mut t = String::new();
    t.push_str(&format!(
        "# Handoff from {} to {}\n\n",
        h.from_name, h.to_name
    ));
    t.push_str("Written with Facet (unofficial, not affiliated with Anthropic) when you asked for it. It holds the project's state and your note, not the conversation.\n\n");
    t.push_str("## Where the work is\n\n");
    t.push_str(&format!(
        "- Folder: `{}`\n",
        s.project.as_deref().unwrap_or("unknown")
    ));
    if let Some(b) = h
        .git
        .and_then(|g| g.branch.as_deref())
        .or(s.branch.as_deref())
    {
        t.push_str(&format!("- Branch: `{b}`\n"));
    }
    t.push_str(&format!(
        "- Session in {}: `{}` ({} prompts, {} model replies)\n",
        h.from_name, s.session_id, s.prompts, s.turns
    ));
    if !s.models.is_empty() {
        t.push_str(&format!("- Models used: {}\n", s.models.join(", ")));
    }
    t.push_str(&format!("- Written: {}\n\n", h.stamp));
    let note = h.note.trim();
    t.push_str("## Note\n\n");
    if note.is_empty() {
        t.push_str("(No note.)\n\n");
    } else {
        t.push_str(note);
        t.push_str("\n\n");
    }
    if let Some(g) = h.git {
        t.push_str("## Changed files\n\n");
        if g.files.is_empty() {
            t.push_str("Nothing uncommitted.\n\n");
        } else {
            let f = fence(&g.files.join("\n"));
            t.push_str(&format!("{f}\n{}\n{f}\n\n", g.files.join("\n")));
        }
        if !g.diff.trim().is_empty() {
            t.push_str("## Uncommitted changes\n\n");
            if !g.diff_stat.trim().is_empty() {
                let f = fence(&g.diff_stat);
                t.push_str(&format!("{f}\n{}{f}\n\n", g.diff_stat));
            }
            let f = fence(&g.diff);
            t.push_str(&format!("{f}diff\n{}{f}\n\n", g.diff));
            if g.diff_truncated {
                t.push_str("(The diff was cut at 120 KB.)\n\n");
            }
            if g.redacted_lines > 0 {
                t.push_str(&format!(
                    "({} line(s) were hidden because they looked like secrets.)\n\n",
                    g.redacted_lines
                ));
            }
        }
    }
    t.push_str("## For the session that reads this\n\n");
    t.push_str("Read the note and the changes above, look at the files in the folder, then continue the work. Ask before undoing anything listed here.\n");
    t
}

pub fn save(home: &Path, name: &str, text: &str) -> Result<PathBuf> {
    let dir = handoff_dir(home);
    crate::control::ensure_private_dir(&dir)?;
    let path = dir.join(name);
    crate::control::write_new_private(&path, text)?;
    Ok(path)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffFile {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub modified_ms: i64,
    pub title: String,
}

pub fn list(home: &Path) -> Vec<HandoffFile> {
    let mut out: Vec<HandoffFile> = fs::read_dir(handoff_dir(home))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let meta = fs::symlink_metadata(e.path()).ok()?;
            if !meta.is_file() || !e.file_name().to_string_lossy().ends_with(".md") {
                return None;
            }
            let head = fs::read_to_string(e.path())
                .ok()?
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches("# ")
                .to_string();
            Some(HandoffFile {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path().display().to_string(),
                bytes: meta.len(),
                modified_ms: meta
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_millis() as i64,
                title: head,
            })
        })
        .collect();
    out.sort_by_key(|h| std::cmp::Reverse(h.modified_ms));
    out
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".md")
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

pub fn remove(home: &Path, name: &str) -> Result<()> {
    if !valid_name(name) {
        return Err(Error::Invalid("That is not a handoff file.".into()));
    }
    let p = handoff_dir(home).join(name);
    let meta = fs::symlink_metadata(&p).map_err(|_| Error::NotFound(name.into()))?;
    if !meta.is_file() {
        return Err(Error::Invalid("That is not a handoff file.".into()));
    }
    fs::remove_file(p)?;
    Ok(())
}

/// The prompt that starts the receiving session (shown to the user first).
pub fn opening_prompt(path: &Path) -> String {
    format!(
        "Read the handoff file at {} and continue the work it describes.",
        path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> SessionSummary {
        SessionSummary {
            account_id: "work".into(),
            session_id: "2c52f684-4c38-4cb4-aa17-78b4c25c50fd".into(),
            project: Some("/home/u/app".into()),
            branch: Some("main".into()),
            prompts: 3,
            turns: 9,
            models: vec!["claude-opus-5-5".into()],
            ..Default::default()
        }
    }

    #[test]
    fn diff_redaction_hides_secret_lines_and_env_files() {
        let raw = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-let x = 1;\n+let key = \"ghp_abcdefghijklmnopqrstuvwxyz0123456789\";\n+let y = 2;\ndiff --git a/.env.local b/.env.local\n--- a/.env.local\n+++ b/.env.local\n@@ -0,0 +1 @@\n+DB_PASSWORD=hunter2\n";
        let (out, n) = redact_diff(raw);
        assert!(!out.contains("ghp_") && !out.contains("hunter2"), "{out}");
        assert!(out.contains("+let y = 2;"));
        assert!(out.contains("(contents hidden: this file usually holds secrets)"));
        assert_eq!(n, 2);
        for f in [
            ".credentials.json",
            "config/credentials.json",
            "prod.tfvars",
            "secrets.yaml",
            "id_ecdsa",
            "my app/.env",
            ".pgpass",
        ] {
            assert!(is_secret_file(f), "{f}");
        }
        assert!(!is_secret_file("src/main.rs"));
        let (out, _) = redact_diff("diff --git a/my dir/.credentials.json b/my dir/.credentials.json\n+{\"token\":\"abc\"}\n");
        assert!(!out.contains("abc"), "{out}");
    }

    #[test]
    fn compose_has_metadata_note_and_fenced_diff_and_no_conversation() {
        let g = GitState {
            branch: Some("feature/x".into()),
            files: vec![" M src/a.rs".into()],
            diff_stat: " src/a.rs | 2 +-\n".into(),
            diff: "diff --git a/src/a.rs b/src/a.rs\n+```\n".into(),
            diff_truncated: false,
            redacted_lines: 1,
        };
        let t = compose(&HandoffInput {
            from_name: "Work",
            to_name: "Personal",
            session: &session(),
            note: "Finish the login form.",
            git: Some(&g),
            stamp: "2026-10-26 12:00",
        });
        assert!(t.starts_with("# Handoff from Work to Personal"));
        assert!(t.contains("`feature/x`"));
        assert!(t.contains("Finish the login form."));
        assert!(
            t.contains("````diff\n"),
            "fence must outgrow the diff's own backticks: {t}"
        );
        assert!(t.contains("1 line(s) were hidden"));
    }

    #[test]
    fn git_state_reads_a_real_repo_and_saves_privately() {
        let d = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(d.path())
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@example.com")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@example.com")
                .output()
                .unwrap()
        };
        run(&["init", "-q", "-b", "main"]);
        fs::write(d.path().join("a.txt"), "one\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-qm", "init"]);
        fs::write(d.path().join("a.txt"), "two\n").unwrap();
        // A hostile repo config must not run: fsmonitor and an external diff.
        run(&["config", "core.fsmonitor", "touch PWNED_FSMONITOR #"]);
        run(&["config", "diff.external", "touch PWNED_DIFF #"]);
        let g = git_state(d.path()).expect("a git repo");
        assert!(
            !d.path().join("PWNED_FSMONITOR").exists() && !d.path().join("PWNED_DIFF").exists()
        );
        assert!(g.diff.is_empty() && g.diff_stat.contains("left out"));
        run(&["config", "--unset", "core.fsmonitor"]);
        run(&["config", "--unset", "diff.external"]);
        let g = git_state(d.path()).expect("a git repo");
        assert_eq!(g.branch.as_deref(), Some("main"));
        assert!(g.files.iter().any(|f| f.ends_with("a.txt")));
        assert!(g.diff.contains("+two"));
        assert!(git_state(tempfile::tempdir().unwrap().path()).is_none());

        let home = tempfile::tempdir().unwrap();
        std::env::remove_var("XDG_DATA_HOME");
        let p = save(home.path(), "h1.md", "# Handoff from A to B\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(list(home.path())[0].title, "Handoff from A to B");
        assert!(remove(home.path(), "../x.md").is_err());
        remove(home.path(), "h1.md").unwrap();
        assert!(list(home.path()).is_empty());
    }
}
