//! Read-only view of an account's Claude Code `settings.json` for Health.
//! Writing settings (with schema, diff, backup and atomic write) is Phase 2.

use serde_json::Value;
use std::fs;
use std::path::Path;

/// Claude Code deletes transcripts older than this many days unless
/// `cleanupPeriodDays` says otherwise (EXTENSION-SURFACE.md).
pub const DEFAULT_CLEANUP_DAYS: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsRead {
    Missing,
    Invalid(String),
    Ok(Value),
}

pub fn read(code_dir: &Path) -> SettingsRead {
    match fs::read_to_string(code_dir.join("settings.json")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SettingsRead::Missing,
        Err(e) => SettingsRead::Invalid(e.to_string()),
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(v) if v.is_object() => SettingsRead::Ok(v),
            Ok(_) => SettingsRead::Invalid("top level is not an object".into()),
            Err(e) => SettingsRead::Invalid(e.to_string()),
        },
    }
}

/// Effective transcript retention in days: `cleanupPeriodDays` when set,
/// otherwise Claude Code's default of 30.
pub fn cleanup_days(settings: &SettingsRead) -> i64 {
    match settings {
        SettingsRead::Ok(v) => v
            .get("cleanupPeriodDays")
            .and_then(Value::as_i64)
            .unwrap_or(DEFAULT_CLEANUP_DAYS),
        _ => DEFAULT_CLEANUP_DAYS,
    }
}

/// Names of env keys set in settings that override sign-in (values unread).
pub fn override_env_names(settings: &SettingsRead) -> Vec<String> {
    let SettingsRead::Ok(v) = settings else {
        return Vec::new();
    };
    let Some(env) = v.get("env").and_then(Value::as_object) else {
        return Vec::new();
    };
    crate::env::OVERRIDE_VARS
        .iter()
        .chain(crate::env::RISKY_VARS.iter())
        .filter(|name| env.contains_key(**name))
        .map(|s| s.to_string())
        .collect()
}

pub fn has_api_key_helper(settings: &SettingsRead) -> bool {
    matches!(settings, SettingsRead::Ok(v) if v.get("apiKeyHelper").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, text: &str) {
        fs::write(dir.join("settings.json"), text).unwrap();
    }

    #[test]
    fn missing_and_invalid_settings() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read(tmp.path()), SettingsRead::Missing);
        assert_eq!(cleanup_days(&read(tmp.path())), 30);
        write(tmp.path(), "{ nope");
        assert!(matches!(read(tmp.path()), SettingsRead::Invalid(_)));
        write(tmp.path(), "[]");
        assert!(matches!(read(tmp.path()), SettingsRead::Invalid(_)));
    }

    #[test]
    fn retention_and_override_names() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            r#"{"cleanupPeriodDays": 365, "env": {"ANTHROPIC_BASE_URL": "http://x", "OTHER": "1"}, "apiKeyHelper": "x"}"#,
        );
        let s = read(tmp.path());
        assert_eq!(cleanup_days(&s), 365);
        assert_eq!(
            override_env_names(&s),
            vec!["ANTHROPIC_BASE_URL".to_string()]
        );
        assert!(has_api_key_helper(&s));
    }
}
