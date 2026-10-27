//! Environment for every process Facet starts (rule 6).
//!
//! The three override variables replace a profile's sign-in (spike 8), so they
//! are removed from every child. `CLAUDE_CONFIG_DIR` is always set explicitly so
//! Claude Code, including the Code tab of a Desktop instance, uses the right
//! account folder (spike 1, test 2).

use std::ffi::{OsStr, OsString};
use std::io::BufRead;
use std::path::Path;
use std::process::Command;

/// Variables that override a signed-in profile. Never passed to a child.
pub const OVERRIDE_VARS: [&str; 3] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// Also flagged by Health when found in Facet's own environment or shell files
/// (CVE-2026-21852 used a base-URL override). Not removed from children,
/// because Facet never sets it and removing it would hide the problem.
pub const RISKY_VARS: [&str; 1] = ["ANTHROPIC_BASE_URL"];

/// Also reported when found in shell files: exporting it there makes every
/// terminal use one account's folder.
pub const SHELL_SCAN_EXTRA: [&str; 1] = ["CLAUDE_CONFIG_DIR"];

/// The only way Facet starts a process (a test rejects `Command::new`
/// elsewhere): the override variables are removed from every child, Claude
/// or not, so rule 6 holds literally and is easy to audit.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    for name in OVERRIDE_VARS {
        cmd.env_remove(name);
    }
    cmd
}

/// Is `dir` Claude Code's default folder (`~/.claude`)? For that folder
/// `CLAUDE_CONFIG_DIR` must be *unset*, not set: when it is set, Claude Code
/// keeps its state in `$CLAUDE_CONFIG_DIR/.claude.json` instead of
/// `~/.claude.json`, so sessions would lose the account's MCP servers and
/// project trust (D-016).
pub fn is_default_config_dir(dir: &Path, home: &Path) -> bool {
    let clean = |p: &Path| p.to_string_lossy().trim_end_matches('/').to_string();
    clean(dir) == clean(&home.join(".claude"))
}

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(Into::into)
        .unwrap_or_else(|| "/".into())
}

/// Point a (scrubbed) command at an account's folder.
pub fn prepare(cmd: &mut Command, config_dir: &Path) {
    prepare_with_home(cmd, config_dir, &home());
}

pub fn prepare_with_home(cmd: &mut Command, config_dir: &Path, home: &Path) {
    for name in OVERRIDE_VARS {
        cmd.env_remove(name);
    }
    if is_default_config_dir(config_dir, home) {
        cmd.env_remove("CLAUDE_CONFIG_DIR");
    } else {
        cmd.env("CLAUDE_CONFIG_DIR", config_dir);
    }
}

/// The shell line that selects an account's folder in scripts Facet writes.
pub fn config_dir_shell_line(config_dir: &Path, home: &Path) -> String {
    if is_default_config_dir(config_dir, home) {
        "unset CLAUDE_CONFIG_DIR".into()
    } else {
        format!(
            "export CLAUDE_CONFIG_DIR='{}'",
            config_dir.to_string_lossy().replace('\'', "'\\''")
        )
    }
}

/// Names of override or risky variables present in an environment.
/// Values are never read: only `is_some()` on the lookup.
pub fn present_override_names<F>(lookup: F) -> Vec<&'static str>
where
    F: Fn(&str) -> Option<OsString>,
{
    OVERRIDE_VARS
        .iter()
        .chain(RISKY_VARS.iter())
        .copied()
        .filter(|name| lookup(name).is_some())
        .collect()
}

/// Names of override, risky or folder variables mentioned in a shell file.
/// Read line by line; each line is dropped after the check and only names are
/// returned, so values never leave this function.
pub fn names_in_shell_reader(reader: impl BufRead) -> Vec<&'static str> {
    let watched: Vec<&'static str> = OVERRIDE_VARS
        .iter()
        .chain(RISKY_VARS.iter())
        .chain(SHELL_SCAN_EXTRA.iter())
        .copied()
        .collect();
    let mut seen = vec![false; watched.len()];
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let code = line.split('#').next().unwrap_or("");
        for (i, name) in watched.iter().enumerate() {
            if seen[i] || !contains_word(code, name) {
                continue;
            }
            // The folder variable matters only when the whole shell gets it
            // (an alias like `alias claude-work='CLAUDE_CONFIG_DIR=… claude'`
            // sets it for one command and is fine). Sign-in overrides count
            // on any mention.
            if SHELL_SCAN_EXTRA.contains(name) {
                let t = code.trim_start();
                let sets_shell = t.starts_with(&format!("{name}="))
                    || ["export ", "declare -x ", "set -gx ", "set -x ", "setenv "]
                        .iter()
                        .any(|p| t.starts_with(p) && contains_word(t, name));
                if !sets_shell {
                    continue;
                }
            }
            seen[i] = true;
        }
    }
    watched
        .into_iter()
        .zip(seen)
        .filter_map(|(n, s)| s.then_some(n))
        .collect()
}

pub fn names_in_shell_text(text: &str) -> Vec<&'static str> {
    names_in_shell_reader(text.as_bytes())
}

fn contains_word(haystack: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    haystack.match_indices(word).any(|(i, _)| {
        let before = haystack[..i].chars().next_back();
        let after = haystack[i + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn child_env(cmd: &Command) -> HashMap<String, Option<String>> {
        cmd.get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect()
    }

    #[test]
    fn prepare_removes_overrides_and_sets_config_dir() {
        let mut cmd = Command::new("true");
        cmd.env("ANTHROPIC_API_KEY", "fake").env("KEEP_ME", "1");
        prepare_with_home(
            &mut cmd,
            Path::new("/home/u/.claude-work"),
            Path::new("/home/u"),
        );
        let env = child_env(&cmd);
        for name in OVERRIDE_VARS {
            assert_eq!(env.get(name), Some(&None), "{name} must be removed");
        }
        assert_eq!(
            env.get("CLAUDE_CONFIG_DIR"),
            Some(&Some("/home/u/.claude-work".into()))
        );
        assert_eq!(env.get("KEEP_ME"), Some(&Some("1".into())));
    }

    #[test]
    fn present_names_never_needs_values() {
        let found = present_override_names(|name| {
            (name == "ANTHROPIC_AUTH_TOKEN" || name == "ANTHROPIC_BASE_URL")
                .then(|| OsString::from("x"))
        });
        assert_eq!(found, vec!["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_BASE_URL"]);
    }

    #[test]
    fn shell_text_scan_matches_whole_names_outside_comments() {
        let text = "export ANTHROPIC_API_KEY=abc\n# CLAUDE_CODE_OAUTH_TOKEN=old\nMY_ANTHROPIC_AUTH_TOKEN_X=1\n";
        assert_eq!(names_in_shell_text(text), vec!["ANTHROPIC_API_KEY"]);
    }

    #[test]
    fn default_folder_unsets_the_variable_instead_of_setting_it() {
        let home = Path::new("/home/u");
        let mut cmd = Command::new("true");
        prepare_with_home(&mut cmd, Path::new("/home/u/.claude/"), home);
        assert_eq!(child_env(&cmd).get("CLAUDE_CONFIG_DIR"), Some(&None));
        let mut cmd = Command::new("true");
        prepare_with_home(&mut cmd, Path::new("/home/u/.claude-work"), home);
        assert_eq!(
            child_env(&cmd).get("CLAUDE_CONFIG_DIR"),
            Some(&Some("/home/u/.claude-work".into()))
        );
        assert_eq!(
            config_dir_shell_line(Path::new("/home/u/.claude"), home),
            "unset CLAUDE_CONFIG_DIR"
        );
        assert_eq!(
            config_dir_shell_line(Path::new("/home/u/it's"), home),
            "export CLAUDE_CONFIG_DIR='/home/u/it'\\''s'"
        );
    }

    #[test]
    fn command_scrubs_every_child() {
        let cmd = command("dpkg-query");
        let env = child_env(&cmd);
        for name in OVERRIDE_VARS {
            assert_eq!(env.get(name), Some(&None), "{name} must be removed");
        }
        assert!(!env.contains_key("CLAUDE_CONFIG_DIR"));
    }

    /// Rule 6 audit: no file starts a process except through `env::command`.
    #[test]
    fn no_command_new_outside_env() {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut offenders = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(src)];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("env.rs") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    let code = text.split("#[cfg(test)]").next().unwrap_or("");
                    if code.contains("Command::new(") {
                        offenders.push(p.display().to_string());
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "use env::command in: {offenders:?}");
    }

    #[test]
    fn shell_scan_reports_folder_variable_only_when_the_shell_gets_it() {
        assert_eq!(
            names_in_shell_text("export CLAUDE_CONFIG_DIR=~/.claude-work"),
            vec!["CLAUDE_CONFIG_DIR"]
        );
        assert_eq!(
            names_in_shell_text("CLAUDE_CONFIG_DIR=~/.claude-work"),
            vec!["CLAUDE_CONFIG_DIR"]
        );
        assert_eq!(
            names_in_shell_text("set -gx CLAUDE_CONFIG_DIR ~/.claude-work"),
            vec!["CLAUDE_CONFIG_DIR"]
        );
        assert!(names_in_shell_text(
            "alias claude-work='CLAUDE_CONFIG_DIR=/h/.claude-work claude'"
        )
        .is_empty());
        // Sign-in overrides count even inside an alias.
        assert_eq!(
            names_in_shell_text("alias c='ANTHROPIC_API_KEY=x claude'"),
            vec!["ANTHROPIC_API_KEY"]
        );
    }

    #[test]
    fn shell_text_scan_finds_unset_lines_too() {
        // Mentioning the name at all is worth showing; the user decides.
        assert_eq!(
            names_in_shell_text("unset ANTHROPIC_BASE_URL"),
            vec!["ANTHROPIC_BASE_URL"]
        );
        assert!(names_in_shell_text("alias ll='ls -l'").is_empty());
    }
}
