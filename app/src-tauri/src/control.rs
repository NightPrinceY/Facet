//! Control (PRD M4, Phase 2): edit an account's Claude Code `settings.json`
//! safely (D-015).
//!
//! - Show before change: `preview` returns a diff of the raw file against the
//!   new text; `restore_preview` does the same for a backup.
//! - Rule 2: a file that holds a sign-in value (provider switch, base URL,
//!   API key or token in `env`) is never backed up, copied or edited; the
//!   user removes it by hand. Secret-looking values are never sent to the
//!   UI and are hidden in diffs.
//! - Rule 7: every replace is preceded by a backup (owner-only, fsynced,
//!   last `KEEP_BACKUPS` kept), the write is atomic, and it is refused if the
//!   file changed since the preview. Symlinked settings files and managed
//!   policy folders are never written.
//! - Edits are per key; unknown keys and key order are preserved.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// One edit: set (or, with `value: null`, remove) the key at `path`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Op {
    pub path: Vec<String>,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Str,
    Bool,
    PosInt,
    StrList,
    Enum(&'static [&'static str]),
}

/// Keys Facet edits, with their type (docs/03-architecture/EXTENSION-SURFACE.md).
const KNOWN: &[(&[&str], Kind)] = &[
    (&["model"], Kind::Str),
    (&["outputStyle"], Kind::Str),
    (&["cleanupPeriodDays"], Kind::PosInt),
    (&["autoMemoryEnabled"], Kind::Bool),
    (&["disableAllHooks"], Kind::Bool),
    (&["enableAllProjectMcpServers"], Kind::Bool),
    (&["spinnerTipsEnabled"], Kind::Bool),
    (&["disableSkillShellExecution"], Kind::Bool),
    (
        &["crossSessionInbound"],
        Kind::Enum(&["accept", "hold", "refuse"]),
    ),
    (
        &["permissions", "defaultMode"],
        Kind::Enum(&[
            "default",
            "acceptEdits",
            "plan",
            "auto",
            "dontAsk",
            "bypassPermissions",
        ]),
    ),
    (&["permissions", "allow"], Kind::StrList),
    (&["permissions", "ask"], Kind::StrList),
    (&["permissions", "deny"], Kind::StrList),
    (&["permissions", "additionalDirectories"], Kind::StrList),
];

/// Top-level keys Facet never writes: they produce credentials.
const REFUSED_KEYS: &[&str] = &[
    "apiKeyHelper",
    "awsAuthRefresh",
    "awsCredentialExport",
    "otelHeadersHelper",
];

/// `env` names that switch the account away from its sign-in, redirect
/// traffic, carry credentials, or disable TLS checks. A settings file holding
/// any of them is not edited or backed up by Facet (rule 2), and Facet never
/// sets them. Names from Claude Code's environment-variable docs; re-check
/// with /surface-check.
pub const REFUSED_ENV: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_CUSTOM_HEADERS",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "ANTHROPIC_BEDROCK_BASE_URL",
    "ANTHROPIC_VERTEX_BASE_URL",
    "ANTHROPIC_FOUNDRY_BASE_URL",
    "ANTHROPIC_FOUNDRY_RESOURCE",
    "ANTHROPIC_FOUNDRY_API_KEY",
    "CLAUDE_CODE_SKIP_BEDROCK_AUTH",
    "CLAUDE_CODE_SKIP_VERTEX_AUTH",
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "CLAUDE_CODE_CLIENT_KEY_PASSPHRASE",
    "NODE_TLS_REJECT_UNAUTHORIZED",
];

/// `env` names that are legitimate (corporate networks) but change where
/// traffic goes or what runs; the preview warns about them.
pub const WARN_ENV: &[&str] = &[
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "NO_PROXY",
    "NODE_EXTRA_CA_CERTS",
    "CLAUDE_CODE_CLIENT_CERT",
    "CLAUDE_CODE_CLIENT_KEY",
    "CLAUDE_CODE_SHELL_PREFIX",
    "OTEL_LOG_USER_PROMPTS",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_EXPORTER_OTLP_HEADERS",
];

fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

pub fn is_refused_env(name: &str) -> bool {
    REFUSED_ENV.contains(&upper(name).as_str())
}

/// Values of these names are hidden from the UI and from diffs.
pub fn is_secret_env(name: &str) -> bool {
    let n = upper(name);
    is_refused_env(&n)
        || [
            "KEY",
            "TOKEN",
            "SECRET",
            "PASSWORD",
            "PASSPHRASE",
            "HEADERS",
            "CREDENTIAL",
            "COOKIE",
            "AUTH",
        ]
        .iter()
        .any(|w| n.contains(w))
}

pub const HIDDEN: &str = "•••• hidden by Facet";

fn valid_env_name(name: &str) -> bool {
    let mut c = name.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn kind_of(path: &[String]) -> Option<Kind> {
    KNOWN
        .iter()
        .find(|(p, _)| p.len() == path.len() && p.iter().zip(path).all(|(a, b)| *a == b))
        .map(|(_, k)| *k)
}

fn validate(op: &Op) -> Result<()> {
    if op.path.is_empty() {
        return Err(Error::Invalid("Empty setting name.".into()));
    }
    if REFUSED_KEYS.contains(&op.path[0].as_str()) {
        return Err(Error::Invalid(format!(
            "Facet does not edit {}: it supplies credentials.",
            op.path[0]
        )));
    }
    // Per-plugin switches: ["enabledPlugins", "name@marketplace"] = bool.
    if op.path[0] == "enabledPlugins" {
        let [_, id] = op.path.as_slice() else {
            return Err(Error::Invalid("Switch plugins one at a time.".into()));
        };
        let ok_id = !id.is_empty()
            && id.len() <= 200
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.@:".contains(c));
        if !ok_id {
            return Err(Error::Invalid(format!("\"{id}\" is not a plugin id.")));
        }
        return match &op.value {
            None | Some(Value::Bool(_)) => Ok(()),
            _ => Err(Error::Invalid(format!(
                "enabledPlugins.{id} must be true or false."
            ))),
        };
    }
    // Per-key environment edits: ["env", NAME].
    if op.path[0] == "env" {
        let [_, name] = op.path.as_slice() else {
            return Err(Error::Invalid(
                "Edit environment variables one at a time.".into(),
            ));
        };
        if !valid_env_name(name) {
            return Err(Error::Invalid(format!(
                "\"{name}\" is not a valid variable name (letters, digits and _)."
            )));
        }
        if is_refused_env(name) {
            return Err(Error::Invalid(format!("Facet does not set {name}: it would replace the account's sign-in, redirect its traffic or weaken TLS.")));
        }
        return match &op.value {
            None => Ok(()),
            Some(Value::String(v)) if v != HIDDEN => Ok(()),
            _ => Err(Error::Invalid(format!("env.{name} must be a text value."))),
        };
    }
    let kind = kind_of(&op.path).ok_or_else(|| {
        Error::Invalid(format!(
            "Facet does not edit {} yet. Edit it in the file directly.",
            op.path.join(".")
        ))
    })?;
    let Some(v) = &op.value else { return Ok(()) };
    let bad = |what: &str| Error::Invalid(format!("{} must be {what}.", op.path.join(".")));
    match kind {
        Kind::Str if v.as_str().is_some_and(|s| !s.trim().is_empty()) => Ok(()),
        Kind::Str => Err(bad("a non-empty text")),
        Kind::Bool if v.is_boolean() => Ok(()),
        Kind::Bool => Err(bad("true or false")),
        Kind::PosInt if v.as_i64().is_some_and(|n| n >= 1) => Ok(()),
        Kind::PosInt => Err(bad("a whole number of at least 1")),
        Kind::StrList
            if v.as_array().is_some_and(|a| {
                a.iter()
                    .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty()))
            }) =>
        {
            Ok(())
        }
        Kind::StrList => Err(bad("a list of non-empty texts")),
        Kind::Enum(options) if v.as_str().is_some_and(|s| options.contains(&s)) => Ok(()),
        Kind::Enum(options) => Err(bad(&format!("one of {}", options.join(", ")))),
    }
}

/// Apply ops to a settings object, keeping unknown keys and key order.
pub fn apply_ops(before: &Value, ops: &[Op]) -> Result<Value> {
    let mut root = match before {
        Value::Object(m) => Value::Object(m.clone()),
        Value::Null => Value::Object(Map::new()),
        _ => return Err(Error::Invalid("settings.json is not an object.".into())),
    };
    for op in ops {
        validate(op)?;
        let (last, parents) = op.path.split_last().expect("validated non-empty");
        let mut cur = &mut root;
        for key in parents {
            let obj = cur.as_object_mut().ok_or_else(|| {
                Error::Invalid(format!("{key} is not an object in settings.json."))
            })?;
            cur = obj
                .entry(key.clone())
                .or_insert_with(|| Value::Object(Map::new()));
        }
        let obj = cur.as_object_mut().ok_or_else(|| {
            Error::Invalid(format!(
                "{} is not an object in settings.json.",
                parents.join(".")
            ))
        })?;
        match &op.value {
            Some(v) => {
                obj.insert(last.clone(), v.clone());
            }
            None => {
                obj.shift_remove(last);
            }
        }
        if op.value.is_none() && parents.len() == 1 {
            if let Some(parent) = root.get(&parents[0]).and_then(Value::as_object) {
                if parent.is_empty() {
                    root.as_object_mut()
                        .expect("object")
                        .shift_remove(&parents[0]);
                }
            }
        }
    }
    Ok(root)
}

pub fn pretty(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".into());
    s.push('\n');
    s
}

/// Copy with secret-looking `env` values replaced by `HIDDEN` (for the UI).
pub fn redacted(v: &Value) -> Value {
    let mut out = v.clone();
    if let Some(env) = out.get_mut("env").and_then(Value::as_object_mut) {
        for (k, val) in env.iter_mut() {
            if is_secret_env(k) {
                *val = Value::String(HIDDEN.into());
            }
        }
    }
    out
}

/// Names of sign-in values present in `env` (case-insensitive).
pub fn sign_in_values(settings: &Value) -> Vec<String> {
    settings
        .get("env")
        .and_then(Value::as_object)
        .map(|env| env.keys().filter(|k| is_refused_env(k)).cloned().collect())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// `same`, `add` or `del`.
    pub kind: &'static str,
    pub text: String,
}

/// Line diff (longest common subsequence). Settings files are small.
pub fn diff(before: &str, after: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            out.push(DiffLine {
                kind: "same",
                text: a[i].into(),
            });
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push(DiffLine {
                kind: "add",
                text: b[j].into(),
            });
            j += 1;
        } else {
            out.push(DiffLine {
                kind: "del",
                text: a[i].into(),
            });
            i += 1;
        }
    }
    out
}

/// FNV-1a: detects that the file changed between preview and save.
pub fn hash(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn settings_path(code_dir: &Path) -> PathBuf {
    code_dir.join("settings.json")
}

/// Folders Facet never writes into (managed and enterprise policy, rule 7).
pub const MANAGED_DIRS: &[&str] = &[
    "/etc/claude-code",
    "/Library/Application Support/ClaudeCode",
    "C:\\Program Files\\ClaudeCode",
];
/// Managed settings file on Linux (marked unverified in EXTENSION-SURFACE.md).
pub const MANAGED_SETTINGS_LINUX: &str = "/etc/claude-code/managed-settings.json";

pub(crate) fn check_writable_location(code_dir: &Path) -> Result<()> {
    let real = fs::canonicalize(code_dir).unwrap_or_else(|_| code_dir.to_path_buf());
    if MANAGED_DIRS.iter().any(|m| real.starts_with(m)) {
        return Err(Error::Invalid(
            "This folder holds managed policy. Facet never edits managed settings.".into(),
        ));
    }
    let path = settings_path(code_dir);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::Invalid(format!(
            "{} is a link to another file (for example from a dotfiles folder). Facet does not write through links; edit the linked file directly.",
            path.display()
        )));
    }
    Ok(())
}

/// The file as read from disk. Only this module sees raw values.
struct Raw {
    path: PathBuf,
    exists: bool,
    text: String,
    settings: Value,
}

fn read_raw(code_dir: &Path) -> Result<Raw> {
    let path = settings_path(code_dir);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let settings: Value = serde_json::from_str(&text).map_err(|e| {
                Error::Invalid(format!("{} is not valid JSON (line {}, column {}). Fix it by hand first; Facet will not overwrite it.", path.display(), e.line(), e.column()))
            })?;
            if !settings.is_object() {
                return Err(Error::Invalid(format!(
                    "{} is not a JSON object.",
                    path.display()
                )));
            }
            Ok(Raw {
                path,
                exists: true,
                text,
                settings,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Raw {
            path,
            exists: false,
            text: String::new(),
            settings: Value::Null,
        }),
        Err(e) => Err(e.into()),
    }
}

fn blocked_message(names: &[String]) -> String {
    format!(
        "settings.json holds a sign-in or provider value ({}). Facet does not copy, back up or edit a file with sign-in data. Remove {} from the file by hand, then open Control again.",
        names.join(", "),
        if names.len() == 1 { "it" } else { "them" }
    )
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Current {
    pub path: PathBuf,
    pub exists: bool,
    /// Parsed settings with secret-looking env values hidden.
    pub settings: Value,
    pub hash: String,
    /// Set when Facet will not edit this file, with the reason and the fix.
    pub blocked: Option<String>,
}

pub fn read_current(code_dir: &Path) -> Result<Current> {
    let raw = read_raw(code_dir)?;
    let names = sign_in_values(&raw.settings);
    let blocked = if !names.is_empty() {
        Some(blocked_message(&names))
    } else {
        check_writable_location(code_dir)
            .err()
            .map(|e| e.to_string())
    };
    Ok(Current {
        path: raw.path,
        exists: raw.exists,
        settings: if names.is_empty() {
            redacted(&raw.settings)
        } else {
            Value::Null
        },
        hash: hash(&raw.text),
        blocked,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub path: PathBuf,
    pub base_hash: String,
    pub diff: Vec<DiffLine>,
    pub changed: bool,
    /// Only formatting differs (Facet rewrites the file in its own layout).
    pub format_only: bool,
    pub warnings: Vec<String>,
}

/// What else the preview needs to warn about, gathered by the caller.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Live Claude Code sessions registered in this account's folder.
    pub live_sessions: usize,
    /// Folders of the other accounts (code and Desktop).
    pub other_account_dirs: Vec<PathBuf>,
}

fn warnings(after: &Value, ctx: &Context) -> Vec<String> {
    let mut w = Vec::new();
    if after
        .pointer("/permissions/defaultMode")
        .and_then(Value::as_str)
        == Some("bypassPermissions")
    {
        w.push("bypassPermissions lets Claude Code run tools without asking. Use it only in a sandbox.".into());
    }
    if after
        .get("enableAllProjectMcpServers")
        .and_then(Value::as_bool)
        == Some(true)
    {
        w.push("Every MCP server a project lists will start without asking, including in repositories you just cloned.".into());
    }
    if after.get("disableAllHooks").and_then(Value::as_bool) == Some(true) {
        w.push("All hooks are off for this account, including safety hooks you rely on.".into());
    }
    if after
        .get("cleanupPeriodDays")
        .and_then(Value::as_i64)
        .is_some_and(|d| d < 30)
    {
        w.push("Transcripts will be deleted sooner than Claude Code's default of 30 days.".into());
    }
    if let Some(env) = after.get("env").and_then(Value::as_object) {
        for (k, v) in env {
            let n = upper(k);
            if n == "OTEL_LOG_USER_PROMPTS" && v.as_str().is_some_and(|s| s != "0" && !s.is_empty())
            {
                w.push("OTEL_LOG_USER_PROMPTS puts your prompt text into telemetry.".into());
            } else if n == "OTEL_EXPORTER_OTLP_ENDPOINT"
                && !v.as_str().is_some_and(|s| {
                    s.contains("://127.0.0.1")
                        || s.contains("://localhost")
                        || s.contains("://[::1]")
                })
            {
                w.push(
                    "Telemetry will leave this computer: the OTLP endpoint is not on localhost."
                        .into(),
                );
            } else if WARN_ENV.contains(&n.as_str()) {
                w.push(format!("{k} changes where Claude Code's traffic goes or what it runs. Keep it only if your network needs it."));
            }
        }
    }
    let rules = ["/permissions/additionalDirectories", "/permissions/allow"]
        .iter()
        .filter_map(|p| after.pointer(p).and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for r in &rules {
        let touches_other = ctx
            .other_account_dirs
            .iter()
            .any(|d| r.contains(d.to_string_lossy().as_ref()));
        if touches_other || r.contains(".credentials") {
            w.push(format!("\"{r}\" reaches another account's folder or sign-in file. That weakens the separation between accounts."));
        }
    }
    if ctx.live_sessions > 0 {
        w.push(format!(
            "{} Claude Code session{} of this account {} running. {} may keep the old settings until restarted, or write the file again.",
            ctx.live_sessions,
            if ctx.live_sessions == 1 { "" } else { "s" },
            if ctx.live_sessions == 1 { "is" } else { "are" },
            if ctx.live_sessions == 1 { "It" } else { "They" },
        ));
    }
    w
}

pub fn preview(code_dir: &Path, ops: &[Op], ctx: &Context) -> Result<Preview> {
    let raw = read_raw(code_dir)?;
    let names = sign_in_values(&raw.settings);
    if !names.is_empty() {
        return Err(Error::Invalid(blocked_message(&names)));
    }
    check_writable_location(code_dir)?;
    let after = apply_ops(&raw.settings, ops)?;
    let after_text = pretty(&after);
    // Diff the raw file against the new text, with secrets hidden on both sides.
    let shown_before = if raw.exists {
        hide_in_text(&raw.text, &raw.settings)
    } else {
        String::new()
    };
    let shown_after = pretty(&redacted(&after));
    let lines = diff(&shown_before, &shown_after);
    let format_only = raw.exists && raw.settings == after && raw.text != after_text;
    Ok(Preview {
        path: raw.path,
        base_hash: hash(&raw.text),
        changed: raw.text != after_text,
        format_only,
        diff: lines,
        warnings: warnings(&after, ctx),
    })
}

/// Hide secret env values inside the raw file text for display.
fn hide_in_text(text: &str, settings: &Value) -> String {
    let mut out = text.to_string();
    if let Some(env) = settings.get("env").and_then(Value::as_object) {
        for (k, v) in env {
            if let (true, Some(s)) = (is_secret_env(k), v.as_str()) {
                if !s.is_empty() {
                    out = out.replace(
                        &serde_json::to_string(s).unwrap_or_default(),
                        &format!("\"{HIDDEN}\""),
                    );
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub path: PathBuf,
    /// `None` when the file did not exist before (nothing to back up).
    pub backup: Option<PathBuf>,
}

/// Keep this many backups per account; older ones are deleted.
pub const KEEP_BACKUPS: usize = 30;

pub(crate) fn ensure_private_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

/// Write `text` to a new owner-only file and fsync it. Never follows a link.
pub(crate) fn write_new_private(path: &Path, text: &str) -> Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    Ok(())
}

fn backup_text(backup_dir: &Path, name: &str, text: &str) -> Result<PathBuf> {
    ensure_private_dir(backup_dir)?;
    let dest = backup_dir.join(name);
    write_new_private(&dest, text)?;
    prune_backups(backup_dir);
    Ok(dest)
}

/// Keep the newest `keep` regular files in `dir` whose names pass `matches`.
pub(crate) fn prune_dir(dir: &Path, keep: usize, matches: impl Fn(&str) -> bool) {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| matches(n))
                .collect()
        })
        .unwrap_or_default();
    names.sort_by(|a, b| b.cmp(a));
    for n in names.iter().skip(keep) {
        let _ = fs::remove_file(dir.join(n));
    }
}

fn prune_backups(backup_dir: &Path) {
    let list = list_backups(backup_dir);
    for b in list.iter().skip(KEEP_BACKUPS) {
        let _ = fs::remove_file(backup_dir.join(&b.name));
    }
}

/// Atomically replace `path` with `text`, keeping its permissions. The temp
/// file is created fresh (never through a link) in the same folder.
pub(crate) fn replace_atomically(path: &Path, text: &str) -> Result<()> {
    let tmp = path.with_file_name(format!(
        "{}.facet-tmp",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    if let Ok(meta) = fs::symlink_metadata(&tmp) {
        if meta.file_type().is_file() {
            fs::remove_file(&tmp)?;
        } else {
            return Err(Error::Invalid(format!(
                "{} exists and is not a regular file; remove it first.",
                tmp.display()
            )));
        }
    }
    let mode = fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(0o600);
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Write the change. `base_hash` must match the file as previewed.
pub fn apply(
    code_dir: &Path,
    ops: &[Op],
    base_hash: &str,
    backup_dir: &Path,
    stamp: &str,
) -> Result<Applied> {
    let raw = read_raw(code_dir)?;
    let names = sign_in_values(&raw.settings);
    if !names.is_empty() {
        return Err(Error::Invalid(blocked_message(&names)));
    }
    check_writable_location(code_dir)?;
    if hash(&raw.text) != base_hash {
        return Err(Error::Invalid(
            "settings.json changed since you reviewed it (Claude Code or an editor wrote it). Review again; your edits are kept.".into(),
        ));
    }
    let after = apply_ops(&raw.settings, ops)?;
    let backup = if raw.exists {
        Some(backup_text(
            backup_dir,
            &format!("{stamp}-settings.json"),
            &raw.text,
        )?)
    } else {
        None
    };
    fs::create_dir_all(code_dir)?;
    // Last check right before the swap.
    let now = fs::read_to_string(&raw.path).unwrap_or_default();
    if hash(&now) != base_hash {
        return Err(Error::Invalid(
            "settings.json changed while saving. Nothing was replaced; review again.".into(),
        ));
    }
    replace_atomically(&raw.path, &pretty(&after))?;
    Ok(Applied {
        path: raw.path,
        backup,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub name: String,
    pub bytes: u64,
}

/// Backup file names Facet writes: `<stamp>[-before-restore]-settings.json`.
pub fn is_backup_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix("-settings.json") else {
        return false;
    };
    let stem = stem.strip_suffix("-before-restore").unwrap_or(stem);
    let b = stem.as_bytes();
    // 20261026T031500Z-123
    b.len() == 20
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'T'
        && b[9..15].iter().all(u8::is_ascii_digit)
        && b[15] == b'Z'
        && b[16] == b'-'
        && b[17..].iter().all(u8::is_ascii_digit)
}

pub fn list_backups(backup_dir: &Path) -> Vec<Backup> {
    let mut out: Vec<Backup> = fs::read_dir(backup_dir)
        .map(|it| {
            it.flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    is_backup_name(&name).then(|| Backup {
                        bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                        name,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

/// Read a backup by name: name pattern checked, must be a regular file (no
/// links), must be a settings object without sign-in values.
fn read_backup(backup_dir: &Path, name: &str) -> Result<(String, Value)> {
    if !is_backup_name(name) {
        return Err(Error::Invalid(
            "That is not one of this account's settings backups.".into(),
        ));
    }
    let path = backup_dir.join(name);
    let meta =
        fs::symlink_metadata(&path).map_err(|_| Error::NotFound(format!("backup {name}")))?;
    if !meta.file_type().is_file() {
        return Err(Error::Invalid(
            "That backup is not a regular file; Facet will not read it.".into(),
        ));
    }
    let text = fs::read_to_string(&path)?;
    let parsed: Value = serde_json::from_str(&text).map_err(|e| {
        Error::Invalid(format!("The backup is not valid JSON (line {}).", e.line()))
    })?;
    if !parsed.is_object() {
        return Err(Error::Invalid(
            "The backup is not a settings object.".into(),
        ));
    }
    let names = sign_in_values(&parsed);
    if !names.is_empty() {
        return Err(Error::Invalid(format!(
            "That backup holds a sign-in or provider value ({}). Facet will not restore it.",
            names.join(", ")
        )));
    }
    Ok((text, parsed))
}

pub fn restore_preview(
    code_dir: &Path,
    backup_dir: &Path,
    name: &str,
    ctx: &Context,
) -> Result<Preview> {
    let raw = read_raw(code_dir)?;
    let names = sign_in_values(&raw.settings);
    if !names.is_empty() {
        return Err(Error::Invalid(blocked_message(&names)));
    }
    check_writable_location(code_dir)?;
    let (text, parsed) = read_backup(backup_dir, name)?;
    let shown_before = if raw.exists {
        hide_in_text(&raw.text, &raw.settings)
    } else {
        String::new()
    };
    let shown_after = hide_in_text(&text, &parsed);
    Ok(Preview {
        path: raw.path,
        base_hash: hash(&raw.text),
        changed: raw.text != text,
        format_only: false,
        diff: diff(&shown_before, &shown_after),
        warnings: warnings(&parsed, ctx),
    })
}

/// Restore a backup. The current file is backed up first, so a restore can
/// itself be undone.
pub fn restore(
    code_dir: &Path,
    backup_dir: &Path,
    name: &str,
    base_hash: &str,
    stamp: &str,
) -> Result<Applied> {
    let raw = read_raw(code_dir)?;
    let names = sign_in_values(&raw.settings);
    if !names.is_empty() {
        return Err(Error::Invalid(blocked_message(&names)));
    }
    check_writable_location(code_dir)?;
    if hash(&raw.text) != base_hash {
        return Err(Error::Invalid(
            "settings.json changed since you reviewed the restore. Review again.".into(),
        ));
    }
    let (text, _) = read_backup(backup_dir, name)?;
    let saved = if raw.exists {
        Some(backup_text(
            backup_dir,
            &format!("{stamp}-before-restore-settings.json"),
            &raw.text,
        )?)
    } else {
        None
    };
    fs::create_dir_all(code_dir)?;
    replace_atomically(&raw.path, &text)?;
    Ok(Applied {
        path: raw.path,
        backup: saved,
    })
}

/// `~/.local/share/facet/backups/<account>/`.
pub fn backup_dir(home: &Path, account_id: &str) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("facet/backups")
        .join(account_id)
}

/// Lists the UI shows next to the environment editor.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub refused_env: Vec<&'static str>,
    pub warn_env: Vec<&'static str>,
    pub hidden_marker: &'static str,
}

pub fn policy() -> Policy {
    Policy {
        refused_env: REFUSED_ENV.to_vec(),
        warn_env: WARN_ENV.to_vec(),
        hidden_marker: HIDDEN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const STAMP1: &str = "20261026T030000Z-001";
    const STAMP2: &str = "20261026T030000Z-002";
    const STAMP3: &str = "20261026T030000Z-003";

    fn op(path: &str, value: Option<Value>) -> Op {
        Op {
            path: path.split('.').map(String::from).collect(),
            value,
        }
    }

    fn ctx() -> Context {
        Context::default()
    }

    #[test]
    fn keeps_unknown_keys_and_order() {
        let before =
            json!({"zeta": 1, "model": "opus", "hooks": {"PreToolUse": []}, "alpha": true});
        let after = apply_ops(
            &before,
            &[
                op("model", Some(json!("sonnet"))),
                op("cleanupPeriodDays", Some(json!(365))),
            ],
        )
        .unwrap();
        let keys: Vec<&String> = after.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            ["zeta", "model", "hooks", "alpha", "cleanupPeriodDays"]
        );
    }

    #[test]
    fn plugin_switches_are_per_plugin_booleans() {
        let before = json!({"enabledPlugins": {"a@m": true, "b@m": true}});
        let p = |id: &str, v: Option<Value>| Op {
            path: vec!["enabledPlugins".into(), id.into()],
            value: v,
        };
        let after = apply_ops(&before, &[p("a@m", Some(json!(false)))]).unwrap();
        assert_eq!(
            after,
            json!({"enabledPlugins": {"a@m": false, "b@m": true}})
        );
        assert!(apply_ops(&before, &[p("a@m", Some(json!("yes")))]).is_err());
        assert!(apply_ops(&before, &[p("../x", Some(json!(true)))]).is_err());
        assert!(apply_ops(
            &before,
            &[Op {
                path: vec!["enabledPlugins".into()],
                value: Some(json!({}))
            }]
        )
        .is_err());
    }

    #[test]
    fn per_key_env_edits_leave_other_values_alone() {
        let before = json!({"env": {"GITHUB_TOKEN": "x1", "OTEL_METRICS_EXPORTER": "otlp"}});
        let after = apply_ops(
            &before,
            &[
                op("env.MY_FLAG", Some(json!("1"))),
                op("env.OTEL_METRICS_EXPORTER", None),
            ],
        )
        .unwrap();
        assert_eq!(
            after,
            json!({"env": {"GITHUB_TOKEN": "x1", "MY_FLAG": "1"}})
        );
        // The hidden marker can never be written back.
        assert!(apply_ops(&before, &[op("env.GITHUB_TOKEN", Some(json!(HIDDEN)))]).is_err());
    }

    #[test]
    fn validation_refuses_credentials_providers_and_bad_names() {
        for (o, needle) in [
            (op("apiKeyHelper", Some(json!("/bin/x"))), "credentials"),
            (op("env.ANTHROPIC_API_KEY", Some(json!("x"))), "sign-in"),
            (op("env.anthropic_auth_token", Some(json!("x"))), "sign-in"),
            (
                op("env.CLAUDE_CODE_USE_BEDROCK", Some(json!("1"))),
                "sign-in",
            ),
            (
                op("env.NODE_TLS_REJECT_UNAUTHORIZED", Some(json!("0"))),
                "TLS",
            ),
            (op("env.CLAUDE_CONFIG_DIR", Some(json!("/x"))), "sign-in"),
            (
                op("env.1BAD", Some(json!("x"))),
                "not a valid variable name",
            ),
            (op("cleanupPeriodDays", Some(json!(0))), "at least 1"),
            (op("crossSessionInbound", Some(json!("maybe"))), "one of"),
            (op("hooks", Some(json!({}))), "does not edit hooks"),
        ] {
            let err = apply_ops(&json!({}), &[o]).unwrap_err().to_string();
            assert!(err.contains(needle), "{err} should mention {needle}");
        }
    }

    #[test]
    fn redaction_hides_secret_looking_values() {
        let s = json!({"env": {"GITHUB_TOKEN": "ghp_x", "MY_PASSWORD": "p", "OTEL_METRICS_EXPORTER": "otlp"}});
        let r = redacted(&s);
        assert_eq!(r["env"]["GITHUB_TOKEN"], json!(HIDDEN));
        assert_eq!(r["env"]["MY_PASSWORD"], json!(HIDDEN));
        assert_eq!(r["env"]["OTEL_METRICS_EXPORTER"], json!("otlp"));
    }

    #[test]
    fn files_with_sign_in_values_are_never_read_out_backed_up_or_edited() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join("c");
        fs::create_dir_all(&code).unwrap();
        fs::write(
            code.join("settings.json"),
            r#"{"env": {"ANTHROPIC_API_KEY": "sk-real"}, "model": "opus"}"#,
        )
        .unwrap();
        let cur = read_current(&code).unwrap();
        assert!(cur
            .blocked
            .as_deref()
            .unwrap()
            .contains("ANTHROPIC_API_KEY"));
        assert_eq!(
            cur.settings,
            Value::Null,
            "nothing from the file goes to the UI"
        );
        assert!(preview(&code, &[op("model", Some(json!("x")))], &ctx()).is_err());
        let backups = tmp.path().join("b");
        assert!(apply(
            &code,
            &[op("model", Some(json!("x")))],
            &cur.hash,
            &backups,
            STAMP1
        )
        .is_err());
        assert!(!backups.exists(), "no backup was written");
    }

    #[test]
    fn diff_uses_raw_text_and_hides_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("settings.json"),
            "{\"model\":\"opus\",\"env\":{\"GITHUB_TOKEN\":\"ghp_secret\"}}",
        )
        .unwrap();
        let p = preview(tmp.path(), &[op("model", Some(json!("sonnet")))], &ctx()).unwrap();
        let all: String = p.diff.iter().map(|l| l.text.clone()).collect();
        assert!(!all.contains("ghp_secret"), "{all}");
        assert!(all.contains(HIDDEN));
        // Reformat-only detection.
        let p = preview(tmp.path(), &[], &ctx()).unwrap();
        assert!(p.format_only && p.changed);
    }

    #[test]
    fn preview_then_apply_backs_up_privately_and_atomically() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join(".claude-work");
        fs::create_dir_all(&code).unwrap();
        fs::write(
            code.join("settings.json"),
            "{\n  \"model\": \"opus\",\n  \"custom\": [1, 2]\n}\n",
        )
        .unwrap();
        fs::set_permissions(
            code.join("settings.json"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        let ops = vec![op("cleanupPeriodDays", Some(json!(365)))];
        let p = preview(&code, &ops, &ctx()).unwrap();
        let backups = tmp.path().join("backups/work");
        let done = apply(&code, &ops, &p.base_hash, &backups, STAMP1).unwrap();
        let backup = done.backup.unwrap();
        assert!(fs::read_to_string(&backup)
            .unwrap()
            .contains("\"custom\": [1, 2]"));
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&backups).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(code.join("settings.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640,
            "mode kept"
        );
        let now: Value =
            serde_json::from_str(&fs::read_to_string(code.join("settings.json")).unwrap()).unwrap();
        assert_eq!(now["cleanupPeriodDays"], json!(365));
        assert_eq!(list_backups(&backups).len(), 1);
    }

    #[test]
    fn apply_refuses_when_file_changed_after_preview() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("settings.json"), "{}").unwrap();
        let ops = vec![op("model", Some(json!("sonnet")))];
        let p = preview(tmp.path(), &ops, &ctx()).unwrap();
        fs::write(tmp.path().join("settings.json"), "{\"model\": \"haiku\"}").unwrap();
        let err = apply(
            tmp.path(),
            &ops,
            &p.base_hash,
            &tmp.path().join("b"),
            STAMP1,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("changed since you reviewed it"));
    }

    #[test]
    fn symlinked_settings_and_managed_folders_are_never_written() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("dotfiles-settings.json");
        fs::write(&real, "{}").unwrap();
        let code = tmp.path().join("c");
        fs::create_dir_all(&code).unwrap();
        std::os::unix::fs::symlink(&real, code.join("settings.json")).unwrap();
        let cur = read_current(&code).unwrap();
        assert!(cur.blocked.as_deref().unwrap().contains("link"));
        assert!(preview(&code, &[op("model", Some(json!("x")))], &ctx()).is_err());
        assert!(check_writable_location(Path::new("/etc/claude-code/x")).is_err());
    }

    #[test]
    fn invalid_json_is_never_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("settings.json"), "{ broken").unwrap();
        let err = preview(tmp.path(), &[op("model", Some(json!("x")))], &ctx())
            .unwrap_err()
            .to_string();
        assert!(err.contains("not valid JSON"));
        assert_eq!(
            fs::read_to_string(tmp.path().join("settings.json")).unwrap(),
            "{ broken"
        );
    }

    #[test]
    fn restore_previews_checks_names_links_and_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join("code");
        let backups = tmp.path().join("backups");
        let p = preview(&code, &[op("model", Some(json!("sonnet")))], &ctx()).unwrap();
        assert!(apply(
            &code,
            &[op("model", Some(json!("sonnet")))],
            &p.base_hash,
            &backups,
            STAMP1
        )
        .unwrap()
        .backup
        .is_none());
        let p2 = preview(&code, &[op("model", Some(json!("opus")))], &ctx()).unwrap();
        let b = apply(
            &code,
            &[op("model", Some(json!("opus")))],
            &p2.base_hash,
            &backups,
            STAMP2,
        )
        .unwrap()
        .backup
        .unwrap();
        let name = b.file_name().unwrap().to_string_lossy().into_owned();
        let rp = restore_preview(&code, &backups, &name, &ctx()).unwrap();
        assert!(rp
            .diff
            .iter()
            .any(|l| l.kind == "add" && l.text.contains("sonnet")));
        restore(&code, &backups, &name, &rp.base_hash, STAMP3).unwrap();
        let now: Value =
            serde_json::from_str(&fs::read_to_string(code.join("settings.json")).unwrap()).unwrap();
        assert_eq!(now["model"], json!("sonnet"));
        assert!(list_backups(&backups)
            .iter()
            .any(|b| b.name.contains("before-restore")));
        // Names outside the pattern, and links, are refused.
        assert!(restore_preview(&code, &backups, "../x-settings.json", &ctx()).is_err());
        let link = format!("{STAMP3}-settings.json").replace("003", "009");
        std::os::unix::fs::symlink(code.join("settings.json"), backups.join(&link)).unwrap();
        assert!(restore_preview(&code, &backups, &link, &ctx()).is_err());
        // A backup holding a sign-in value is refused.
        let bad = STAMP3.replace("003", "008") + "-settings.json";
        fs::write(
            backups.join(&bad),
            r#"{"env":{"ANTHROPIC_AUTH_TOKEN":"t"}}"#,
        )
        .unwrap();
        assert!(restore_preview(&code, &backups, &bad, &ctx())
            .unwrap_err()
            .to_string()
            .contains("sign-in"));
    }

    #[test]
    fn backups_are_pruned_to_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..(KEEP_BACKUPS + 5) {
            backup_text(
                tmp.path(),
                &format!("20261026T030000Z-{i:03}-settings.json"),
                "{}",
            )
            .unwrap();
        }
        let left = list_backups(tmp.path());
        assert_eq!(left.len(), KEEP_BACKUPS);
        assert_eq!(
            left[0].name,
            format!("20261026T030000Z-{:03}-settings.json", KEEP_BACKUPS + 4),
            "newest kept"
        );
    }

    #[test]
    fn warnings_cover_risky_values_other_accounts_and_live_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Context {
            live_sessions: 2,
            other_account_dirs: vec![PathBuf::from("/h/.claude-work")],
        };
        let p = preview(
            tmp.path(),
            &[
                op("permissions.defaultMode", Some(json!("bypassPermissions"))),
                op("env.OTEL_LOG_USER_PROMPTS", Some(json!("1"))),
                op(
                    "env.OTEL_EXPORTER_OTLP_ENDPOINT",
                    Some(json!("https://otel.example.com")),
                ),
                op("env.HTTPS_PROXY", Some(json!("http://proxy:3128"))),
                op(
                    "permissions.additionalDirectories",
                    Some(json!(["/h/.claude-work"])),
                ),
            ],
            &ctx,
        )
        .unwrap();
        let all = p.warnings.join("\n");
        for needle in [
            "without asking",
            "prompt text",
            "leave this computer",
            "HTTPS_PROXY",
            "another account",
            "2 Claude Code sessions",
        ] {
            assert!(all.contains(needle), "missing {needle}: {all}");
        }
    }

    #[test]
    fn backup_names() {
        assert!(is_backup_name("20261026T031500Z-123-settings.json"));
        assert!(is_backup_name(
            "20261026T031500Z-123-before-restore-settings.json"
        ));
        assert!(!is_backup_name("../20261026T031500Z-123-settings.json"));
        assert!(!is_backup_name("evil-settings.json"));
    }
}
