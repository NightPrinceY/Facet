//! Claude Code's session registry: `<config>/sessions/<pid>.json`.
//!
//! Parsed with an allow-list (compliance review 2026-10-26): every other field
//! is dropped, and the `<pid>.*.key` files next to them are never opened.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The only registry fields Facet keeps. Everything else (session ids,
/// names, folders, socket paths) is dropped until a feature needs it
/// (compliance review 2026-10-26, data minimisation; D-010 keeps the native
/// messaging path in the Lab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSession {
    pub pid: u32,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    /// `cli`, `claude-desktop`, `claude-vscode`, `sdk-cli`, …
    #[serde(default)]
    pub entrypoint: Option<String>,
}

/// Parse one registry file's text, keeping allow-listed fields only.
pub fn parse_entry(text: &str) -> Option<LiveSession> {
    // Deserialising into the struct drops unknown keys (no `deny_unknown_fields`
    // and no catch-all map), which is the allow-list.
    serde_json::from_str::<LiveSession>(text).ok()
}

/// Live sessions registered in one account folder. Skips stale files whose
/// process has exited.
pub fn live_sessions(code_dir: &Path, is_alive: impl Fn(u32) -> bool) -> Vec<LiveSession> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(code_dir.join("sessions")) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Only `<pid>.json`. `<pid>.<hash>.key` is never opened.
        let Some(file) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = file.strip_suffix(".json") else {
            continue;
        };
        if stem.parse::<u32>().is_err() {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if let Some(s) = parse_entry(&text) {
            if is_alive(s.pid) {
                out.push(s);
            }
        }
    }
    out.sort_by_key(|s| s.pid);
    out
}

pub fn pid_alive(pid: u32) -> bool {
    Path::new("/proc").join(pid.to_string()).exists()
}

/// The executable of a process, from the `/proc/<pid>/exe` link. The command
/// line is not read: `claude -p "…"` would put prompt text there.
pub fn program_of(proc_root: &Path, pid: u32) -> Option<PathBuf> {
    fs::read_link(proc_root.join(pid.to_string()).join("exe")).ok()
}

/// A Desktop-launched Code session registered in the wrong account folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MisplacedSession {
    pub pid: u32,
    /// Account whose Desktop started the session (its bundled `claude-code`).
    pub desktop_account: String,
    /// Account whose folder the session is writing into.
    pub folder_account: String,
}

/// Desktop's Code tab signs in with the Desktop's account but writes into
/// `CLAUDE_CONFIG_DIR` (default `~/.claude`). A session whose program lives
/// under account A's Desktop folder but is registered in account B's Claude
/// Code folder is mixing two accounts (spike 1, test 2).
pub fn misplaced_desktop_sessions(
    accounts: &[(String, Option<PathBuf>, PathBuf)], // (id, desktop_dir, code_dir)
    sessions_by_account: &[(String, Vec<LiveSession>)],
    program: impl Fn(u32) -> Option<PathBuf>,
) -> Vec<MisplacedSession> {
    let mut out = Vec::new();
    for (folder_account, sessions) in sessions_by_account {
        for s in sessions
            .iter()
            .filter(|s| s.entrypoint.as_deref() == Some("claude-desktop"))
        {
            let Some(prog) = program(s.pid) else { continue };
            let owner = accounts.iter().find(|(_, desktop, _)| {
                desktop
                    .as_ref()
                    .is_some_and(|d| prog.starts_with(d.join("claude-code")))
            });
            if let Some((desktop_account, _, _)) = owner {
                if desktop_account != folder_account {
                    out.push(MisplacedSession {
                        pid: s.pid,
                        desktop_account: desktop_account.clone(),
                        folder_account: folder_account.clone(),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"pid":440478,"sessionId":"s-1","cwd":"/home/u","startedAt":1,"procStart":"x","version":"2.1.308","peerProtocol":1,"peerFeatures":["notify_idle"],"kind":"interactive","entrypoint":"claude-desktop","hostSessionId":"h","pidDomain":"linux:abc","messagingSocketPath":"/run/user/1000/cc-socks/440478.sock","name":"Dual","status":"idle","bridgeSessionId":"b"}"#;

    #[test]
    fn keeps_only_allow_listed_fields() {
        let s = parse_entry(SAMPLE).unwrap();
        assert_eq!(s.pid, 440478);
        assert_eq!(s.entrypoint.as_deref(), Some("claude-desktop"));
        let back = serde_json::to_string(&s).unwrap();
        for dropped in [
            "hostSessionId",
            "bridgeSessionId",
            "pidDomain",
            "procStart",
            "peerFeatures",
            "messagingSocketPath",
            "sessionId",
            "cwd",
        ] {
            assert!(!back.contains(dropped), "{dropped} leaked");
        }
    }

    #[test]
    fn reads_json_files_only_never_key_files_and_skips_dead_pids() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("sessions");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("440478.json"), SAMPLE).unwrap();
        fs::write(dir.join("999.json"), r#"{"pid":999}"#).unwrap();
        // A key file with JSON inside must still be ignored.
        fs::write(dir.join("440478.deadbeef.key"), r#"{"pid":1}"#).unwrap();
        fs::write(dir.join("notes.json"), r#"{"pid":2}"#).unwrap();
        let live = live_sessions(tmp.path(), |pid| pid != 999);
        assert_eq!(live.iter().map(|s| s.pid).collect::<Vec<_>>(), vec![440478]);
    }

    #[test]
    fn finds_work_desktop_sessions_in_personal_folder() {
        let accounts = vec![
            (
                "personal".to_string(),
                Some(PathBuf::from("/h/.config/Claude")),
                PathBuf::from("/h/.claude"),
            ),
            (
                "work".to_string(),
                Some(PathBuf::from("/h/.config/Claude-Work")),
                PathBuf::from("/h/.claude-work"),
            ),
        ];
        let mut ok = parse_entry(SAMPLE).unwrap();
        ok.pid = 1;
        let mut bad = ok.clone();
        bad.pid = 2;
        let mut cli = ok.clone();
        cli.pid = 3;
        cli.entrypoint = Some("cli".into());
        let sessions = vec![("personal".to_string(), vec![ok, bad, cli])];
        let program = |pid: u32| match pid {
            1 => Some(PathBuf::from(
                "/h/.config/Claude/claude-code/2.1.308/x/claude",
            )),
            2 => Some(PathBuf::from(
                "/h/.config/Claude-Work/claude-code/2.1.308/x/claude",
            )),
            _ => Some(PathBuf::from("/usr/bin/claude")),
        };
        let found = misplaced_desktop_sessions(&accounts, &sessions, program);
        assert_eq!(
            found,
            vec![MisplacedSession {
                pid: 2,
                desktop_account: "work".into(),
                folder_account: "personal".into()
            }]
        );
    }
}
