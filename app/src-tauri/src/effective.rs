//! Effective settings (PRD M4): what actually applies to an account in a
//! project after Claude Code merges its layers. Read-only; secret-looking
//! values are hidden (D-015). Managed policy is shown, never written.
//!
//! Precedence (docs/03-architecture/EXTENSION-SURFACE.md): managed >
//! project local (`.claude/settings.local.json`) > project
//! (`.claude/settings.json`) > account (`<config>/settings.json`). List
//! permissions (`allow`, `ask`, `deny`, `additionalDirectories`) are
//! combined across layers instead of overridden.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::control::{redacted, MANAGED_SETTINGS_LINUX};
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    /// `managed`, `project-local`, `project` or `account`.
    pub name: &'static str,
    pub path: PathBuf,
    pub exists: bool,
    /// Set when the file exists but cannot be used (invalid JSON, not an object).
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Dotted key, e.g. `permissions.defaultMode`.
    pub key: String,
    /// Effective value (secret-looking env values hidden).
    pub value: Value,
    /// Layer the value comes from; for combined lists, every contributing layer.
    pub sources: Vec<&'static str>,
    /// Lower layers that also set this key but lose.
    pub overridden: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Effective {
    pub layers: Vec<Layer>,
    pub entries: Vec<Entry>,
}

const COMBINED: &[&str] = &[
    "permissions.allow",
    "permissions.ask",
    "permissions.deny",
    "permissions.additionalDirectories",
];

fn read_layer(name: &'static str, path: PathBuf) -> (Layer, Option<Value>) {
    match fs::read_to_string(&path) {
        Err(_) => (
            Layer {
                name,
                path,
                exists: false,
                problem: None,
            },
            None,
        ),
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(v) if v.is_object() => (
                Layer {
                    name,
                    path,
                    exists: true,
                    problem: None,
                },
                Some(redacted(&v)),
            ),
            Ok(_) => (
                Layer {
                    name,
                    path,
                    exists: true,
                    problem: Some("not a JSON object".into()),
                },
                None,
            ),
            Err(e) => (
                Layer {
                    name,
                    path,
                    exists: true,
                    problem: Some(format!("invalid JSON at line {}", e.line())),
                },
                None,
            ),
        },
    }
}

/// Leaf values keyed by dotted path. Objects recurse; arrays and scalars are leaves.
fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(map) if !map.is_empty() => {
            for (k, child) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, child, out);
            }
        }
        _ if !prefix.is_empty() => out.push((prefix.to_string(), v.clone())),
        _ => {}
    }
}

/// Merge layers given highest precedence first.
pub fn merge(layers: &[(&'static str, Value)]) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for (name, settings) in layers {
        let mut leaves = Vec::new();
        flatten("", settings, &mut leaves);
        for (key, value) in leaves {
            match entries.iter_mut().find(|e| e.key == key) {
                None => entries.push(Entry {
                    key,
                    value,
                    sources: vec![name],
                    overridden: vec![],
                }),
                Some(e) if COMBINED.contains(&e.key.as_str()) => {
                    if let (Some(have), Some(more)) = (e.value.as_array_mut(), value.as_array()) {
                        for item in more {
                            if !have.contains(item) {
                                have.push(item.clone());
                            }
                        }
                        e.sources.push(name);
                    }
                }
                Some(e) => e.overridden.push(name),
            }
        }
    }
    entries.sort_by(|a, b| a.key.cmp(&b.key));
    entries
}

pub fn effective(code_dir: &Path, project: Option<&Path>, managed: &Path) -> Result<Effective> {
    let mut specs: Vec<(&'static str, PathBuf)> = vec![("managed", managed.to_path_buf())];
    if let Some(p) = project {
        if !p.is_dir() {
            return Err(Error::Invalid(format!("{} is not a folder.", p.display())));
        }
        specs.push(("project-local", p.join(".claude/settings.local.json")));
        specs.push(("project", p.join(".claude/settings.json")));
    }
    specs.push(("account", code_dir.join("settings.json")));
    let mut layers = Vec::new();
    let mut values = Vec::new();
    for (name, path) in specs {
        let (layer, value) = read_layer(name, path);
        layers.push(layer);
        if let Some(v) = value {
            values.push((name, v));
        }
    }
    Ok(Effective {
        layers,
        entries: merge(&values),
    })
}

pub fn managed_path() -> PathBuf {
    PathBuf::from(MANAGED_SETTINGS_LINUX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn precedence_and_combined_lists() {
        let entries = merge(&[
            (
                "managed",
                json!({"permissions": {"deny": ["Bash(rm -rf *)"]}}),
            ),
            (
                "project",
                json!({"model": "sonnet", "permissions": {"allow": ["Bash(npm test)"], "defaultMode": "plan"}}),
            ),
            (
                "account",
                json!({"model": "opus", "permissions": {"allow": ["Bash(git status)", "Bash(npm test)"]}, "cleanupPeriodDays": 365}),
            ),
        ]);
        let get = |k: &str| entries.iter().find(|e| e.key == k).unwrap();
        assert_eq!(get("model").value, json!("sonnet"));
        assert_eq!(get("model").sources, ["project"]);
        assert_eq!(get("model").overridden, ["account"]);
        assert_eq!(
            get("permissions.allow").value,
            json!(["Bash(npm test)", "Bash(git status)"])
        );
        assert_eq!(get("permissions.allow").sources, ["project", "account"]);
        assert_eq!(get("permissions.deny").sources, ["managed"]);
        assert_eq!(get("cleanupPeriodDays").sources, ["account"]);
    }

    #[test]
    fn reads_files_hides_secrets_and_reports_bad_layers() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join("code");
        let project = tmp.path().join("proj");
        fs::create_dir_all(code.clone()).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::write(
            code.join("settings.json"),
            r#"{"env": {"GITHUB_TOKEN": "ghp_x", "A": "1"}}"#,
        )
        .unwrap();
        fs::write(project.join(".claude/settings.json"), "{ nope").unwrap();
        let eff = effective(&code, Some(&project), &tmp.path().join("managed.json")).unwrap();
        assert_eq!(
            eff.layers.iter().map(|l| l.name).collect::<Vec<_>>(),
            ["managed", "project-local", "project", "account"]
        );
        assert!(!eff.layers[0].exists);
        assert!(eff.layers[2]
            .problem
            .as_deref()
            .unwrap()
            .contains("invalid JSON"));
        let token = eff
            .entries
            .iter()
            .find(|e| e.key == "env.GITHUB_TOKEN")
            .unwrap();
        assert_ne!(token.value, json!("ghp_x"));
        assert!(effective(
            &code,
            Some(&tmp.path().join("missing")),
            &tmp.path().join("m")
        )
        .is_err());
    }
}
