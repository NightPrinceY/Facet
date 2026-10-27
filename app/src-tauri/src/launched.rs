//! Desktop instances Facet started, kept across Facet restarts and shared with
//! `facet --launch` (compliance review finding 18). Each entry stores the
//! process start time from `/proc/<pid>/stat`, so a reused pid is never
//! mistaken for an instance Facet launched.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Launch {
    pub pid: u32,
    /// Field 22 of `/proc/<pid>/stat`: start time in clock ticks since boot.
    pub start_ticks: u64,
}

/// Start time of a process, or `None` if it is gone.
pub fn start_ticks(proc_root: &Path, pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(proc_root.join(pid.to_string()).join("stat")).ok()?;
    // The command name (field 2) is in parentheses and may contain spaces.
    let after = &stat[stat.rfind(')')? + 2..];
    // `after` starts at field 3, so field 22 is index 19.
    after.split_whitespace().nth(19)?.parse().ok()
}

pub struct LaunchFile {
    path: PathBuf,
}

impl LaunchFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Entries whose process is still the one Facet started.
    pub fn load(&self, proc_root: &Path) -> BTreeMap<String, Launch> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        let all: BTreeMap<String, Launch> = serde_json::from_str(&text).unwrap_or_default();
        all.into_iter()
            .filter(|(_, l)| start_ticks(proc_root, l.pid) == Some(l.start_ticks))
            .collect()
    }

    pub fn save(&self, map: &BTreeMap<String, Launch>) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("json.tmp");
        if let Ok(text) = serde_json::to_string_pretty(map) {
            if fs::write(&tmp, text).is_ok() {
                let _ = fs::rename(&tmp, &self.path);
            }
        }
    }
}

/// `$XDG_STATE_HOME/facet/launched.json`, default `~/.local/state/facet/`.
pub fn default_path(home: &Path) -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"))
        .join("facet/launched.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_proc(root: &Path, pid: u32, ticks: u64) {
        let dir = root.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        // Command name with a space and parentheses, as the kernel allows.
        let mut fields = vec!["S".to_string(); 19];
        fields.push(ticks.to_string());
        fields.push("0".into());
        fs::write(
            dir.join("stat"),
            format!("{pid} (claude (x) y) {}", fields.join(" ")),
        )
        .unwrap();
    }

    #[test]
    fn reads_start_time_after_odd_command_names() {
        let tmp = tempfile::tempdir().unwrap();
        fake_proc(tmp.path(), 42, 123456);
        assert_eq!(start_ticks(tmp.path(), 42), Some(123456));
        assert_eq!(start_ticks(tmp.path(), 43), None);
    }

    #[test]
    fn keeps_only_entries_whose_process_is_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let proc_root = tmp.path().join("proc");
        fake_proc(&proc_root, 10, 500);
        fake_proc(&proc_root, 11, 999); // pid reused by another process
        let file = LaunchFile::new(tmp.path().join("state/launched.json"));
        let mut map = BTreeMap::new();
        map.insert(
            "work".to_string(),
            Launch {
                pid: 10,
                start_ticks: 500,
            },
        );
        map.insert(
            "acme".to_string(),
            Launch {
                pid: 11,
                start_ticks: 700,
            },
        );
        map.insert(
            "gone".to_string(),
            Launch {
                pid: 12,
                start_ticks: 1,
            },
        );
        file.save(&map);
        let loaded = file.load(&proc_root);
        assert_eq!(loaded.keys().collect::<Vec<_>>(), vec!["work"]);
    }
}
