//! Studio v1 (PRD M11): a local library of skills, subagents and commands,
//! a linter, and placement into accounts with drift detection (D-017).
//!
//! Formats follow docs/03-architecture/EXTENSION-SURFACE.md. Rules:
//! - the library never holds secrets: every save and every placement is
//!   scanned, and refused on token or key patterns;
//! - an account copy that holds a secret is never shown, backed up or
//!   replaced; the user removes the secret by hand (as D-015 for settings);
//! - placing an item that runs commands (hooks, `!`cmd``, MCP servers, broad
//!   shell access) needs an explicit acknowledgement, checked in the core;
//! - account files are never read or written through symlinks, never inside
//!   `skills/synced/` (owned by Claude Code) and never in managed folders;
//!   anything replaced or removed is backed up first (0700/0600, last 30);
//! - nothing here runs a skill, a hook or the `claude` binary.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

use crate::control::{
    diff, ensure_private_dir, hash, prune_dir, replace_atomically, write_new_private, DiffLine,
    MANAGED_DIRS,
};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Skill,
    Agent,
    Command,
}

impl Kind {
    /// Path of an item relative to a root (library, account folder, or a
    /// project's `.claude`).
    pub fn rel_path(self, name: &str) -> PathBuf {
        match self {
            Kind::Skill => PathBuf::from("skills").join(name).join("SKILL.md"),
            Kind::Agent => PathBuf::from("agents").join(format!("{name}.md")),
            Kind::Command => PathBuf::from("commands").join(format!("{name}.md")),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Skill => "skill",
            Kind::Agent => "agent",
            Kind::Command => "command",
        }
    }
}

// ---------------------------------------------------------------- linting

const SKILL_KEYS: &[&str] = &[
    "name",
    "description",
    "when_to_use",
    "argument-hint",
    "arguments",
    "disable-model-invocation",
    "user-invocable",
    "allowed-tools",
    "disallowed-tools",
    "model",
    "effort",
    "context",
    "agent",
    "background",
    "hooks",
    "paths",
    "shell",
    "metadata",
    "license",
    "compatibility",
];
const PORTABLE_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];
const AGENT_KEYS: &[&str] = &[
    "name",
    "description",
    "tools",
    "disallowedTools",
    "model",
    "permissionMode",
    "maxTurns",
    "autoCompactWindow",
    "skills",
    "mcpServers",
    "hooks",
    "memory",
    "background",
    "omitClaudeMd",
    "effort",
    "isolation",
    "color",
    "initialPrompt",
    "experimental",
];
const COMMAND_KEYS: &[&str] = &[
    "description",
    "argument-hint",
    "allowed-tools",
    "model",
    "disable-model-invocation",
];
/// Description plus `when_to_use` are cut at this many characters in the listing.
pub const DESC_BUDGET: usize = 1536;
const BODY_LINES: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Risk,
    Warn,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lint {
    pub findings: Vec<Finding>,
    /// Characters of description + when_to_use, against `DESC_BUDGET`.
    pub description_chars: usize,
    pub description: String,
}

impl Lint {
    pub fn has(&self, s: Severity) -> bool {
        self.findings.iter().any(|f| f.severity == s)
    }
}

/// Split `---` frontmatter from the body.
pub fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    let body = rest[end + 4..].trim_start_matches(['\r', '\n']);
    Some((fm, body))
}

/// Minimal reading of top-level `key: value` lines (enough for linting;
/// not a YAML parser). Continuation lines are appended to the last key.
fn frontmatter_keys(fm: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut keys: Vec<(String, String)> = Vec::new();
    let mut problems = Vec::new();
    for (n, line) in fm.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t', '-']) {
            if let Some(last) = keys.last_mut() {
                last.1.push('\n');
                last.1.push_str(line);
            }
            continue;
        }
        match line.split_once(':') {
            Some((k, v))
                if !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
            {
                if keys.iter().any(|(x, _)| x == k) {
                    problems.push(format!("line {}: duplicate key '{k}'", n + 2));
                }
                keys.push((k.to_string(), v.trim().to_string()));
            }
            _ => problems.push(format!("line {}: not a 'key: value' line", n + 2)),
        }
    }
    (keys, problems)
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

fn scalar_text(raw: &str) -> String {
    let (first, rest) = raw.split_once('\n').unwrap_or((raw, ""));
    if matches!(first.trim(), "|" | ">" | "|-" | ">-" | "") {
        rest.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        unquote(first)
    }
}

/// Checks that apply to the whole file, with or without frontmatter.
pub(crate) fn whole_file_checks(text: &str, f: &mut Vec<Finding>) {
    let risk = |f: &mut Vec<Finding>, m: &str| {
        f.push(Finding {
            severity: Severity::Risk,
            message: m.into(),
        })
    };
    if text.contains("!`") || text.lines().any(|l| l.trim_start().starts_with("```!")) {
        risk(f, "Runs shell commands when it loads (!`command`).");
    }
    let lower = text.to_ascii_lowercase();
    if ["curl", "wget"].iter().any(|c| {
        lower.lines().any(|l| {
            l.contains(c)
                && (l.contains("| sh")
                    || l.contains("|sh")
                    || l.contains("| bash")
                    || l.contains("|bash"))
        })
    }) {
        risk(
            f,
            "Downloads and runs a script (curl or wget piped into a shell).",
        );
    }
    if let Some(what) = find_secret(text) {
        f.push(Finding {
            severity: Severity::Error,
            message: format!(
                "Looks like a secret ({what}). Use an environment variable reference instead."
            ),
        });
    }
}

/// Lint one item. `name` is the folder or file name it lives under.
pub fn lint(kind: Kind, name: &str, text: &str) -> Lint {
    let mut f = Vec::new();
    let push = |f: &mut Vec<Finding>, s: Severity, m: String| {
        f.push(Finding {
            severity: s,
            message: m,
        })
    };
    whole_file_checks(text, &mut f);
    let Some((fm, body)) = split_frontmatter(text) else {
        if kind != Kind::Command {
            push(&mut f, Severity::Error, "No frontmatter: the file must start with a --- block holding name and description.".into());
        }
        f.sort_by_key(|x| x.severity);
        return Lint {
            findings: f,
            description_chars: 0,
            description: String::new(),
        };
    };
    let (keys, problems) = frontmatter_keys(fm);
    for p in problems {
        push(&mut f, Severity::Error, p);
    }
    let allowed = match kind {
        Kind::Skill => SKILL_KEYS,
        Kind::Agent => AGENT_KEYS,
        Kind::Command => COMMAND_KEYS,
    };
    for (k, _) in &keys {
        if !allowed.contains(&k.as_str())
            && !(kind == Kind::Command && SKILL_KEYS.contains(&k.as_str()))
        {
            push(
                &mut f,
                Severity::Warn,
                format!("Unknown key '{k}': Claude Code ignores it without saying so."),
            );
        }
    }
    let get = |k: &str| keys.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str());
    let desc_raw = get("description").unwrap_or("");
    let first = desc_raw.split('\n').next().unwrap_or("");
    if kind != Kind::Command && desc_raw.trim().is_empty() {
        push(
            &mut f,
            Severity::Error,
            "Missing description. Claude decides when to use this from the description.".into(),
        );
    }
    if desc_raw.contains('\n') {
        push(
            &mut f,
            Severity::Warn,
            "Multi-line description: prefer one line; some tools read block text badly.".into(),
        );
    }
    if first.contains(": ") && !first.starts_with(['"', '\'', '|', '>']) {
        push(
            &mut f,
            Severity::Error,
            "The description contains ': ' without quotes, which breaks YAML. Wrap it in quotes."
                .into(),
        );
    }
    let description = scalar_text(desc_raw);
    let total = description.chars().count()
        + get("when_to_use")
            .map(scalar_text)
            .unwrap_or_default()
            .chars()
            .count();
    if kind == Kind::Skill {
        if total > DESC_BUDGET {
            push(&mut f, Severity::Warn, format!("Description and when_to_use are {total} characters; Claude Code cuts them at {DESC_BUDGET}. Put the trigger words first."));
        } else if total > 0 && total < 40 {
            push(
                &mut f,
                Severity::Warn,
                "The description is very short. Say what it does and when to use it.".into(),
            );
        }
        let declared = get("name").map(unquote).unwrap_or_else(|| name.to_string());
        if declared != name {
            push(
                &mut f,
                Severity::Warn,
                format!("name '{declared}' differs from its folder '{name}'."),
            );
        }
        let extra: Vec<&str> = keys
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !PORTABLE_KEYS.contains(k))
            .collect();
        if !extra.is_empty() {
            push(
                &mut f,
                Severity::Info,
                format!(
                    "Works in Claude Code; claude.ai and other Agent Skills apps reject: {}.",
                    extra.join(", ")
                ),
            );
        }
    }
    if kind == Kind::Agent {
        match get("name").map(unquote) {
            None => push(&mut f, Severity::Error, "A subagent needs a name.".into()),
            Some(n) if n.contains(':') || n.starts_with('-') || n.len() > 256 => push(
                &mut f,
                Severity::Error,
                format!("'{n}' is not a valid subagent name."),
            ),
            _ => {}
        }
        if let Some(mode) = get("permissionMode").map(unquote) {
            if matches!(mode.as_str(), "bypassPermissions" | "dontAsk" | "auto") {
                push(
                    &mut f,
                    Severity::Risk,
                    format!("permissionMode {mode} lets this agent act without asking."),
                );
            }
        }
        if get("mcpServers").is_some() {
            push(
                &mut f,
                Severity::Risk,
                "Starts its own MCP servers, which run programs on this computer.".into(),
            );
        }
    }
    for key in ["allowed-tools", "tools"] {
        if let Some(v) = get(key) {
            let v = v.replace(' ', "");
            let broad = v
                .split([',', '\n', '[', ']', '-'])
                .map(|t| t.trim_matches(['"', '\'']))
                .any(|t| t == "Bash" || t == "Bash(*)" || t == "Bash(*:*)");
            if broad {
                push(
                    &mut f,
                    Severity::Risk,
                    format!("{key} grants any shell command (Bash without a pattern)."),
                );
            }
        }
    }
    if get("shell").is_some() {
        push(
            &mut f,
            Severity::Info,
            "Sets which shell runs its commands.".into(),
        );
    }
    let lines = body.lines().count();
    if lines > BODY_LINES {
        push(&mut f, Severity::Warn, format!("The body is {lines} lines; move detail into linked files (aim for under {BODY_LINES})."));
    }
    if get("hooks").is_some() {
        push(
            &mut f,
            Severity::Risk,
            "Defines hooks, which run commands on Claude Code events.".into(),
        );
    }
    f.sort_by_key(|x| x.severity);
    Lint {
        findings: f,
        description_chars: total,
        description,
    }
}

/// Token and key patterns the library refuses to store. Every occurrence is
/// checked; `${VAR}` references are fine.
pub fn find_secret(text: &str) -> Option<&'static str> {
    // Literal prefixes followed by token characters.
    const PREFIXES: &[(&str, &str, usize)] = &[
        ("sk-ant-", "an Anthropic key", 8),
        ("ghp_", "a GitHub token", 20),
        ("gho_", "a GitHub token", 20),
        ("ghu_", "a GitHub token", 20),
        ("ghs_", "a GitHub token", 20),
        ("ghr_", "a GitHub token", 20),
        ("github_pat_", "a GitHub token", 20),
        ("glpat-", "a GitLab token", 16),
        ("xoxb-", "a Slack token", 10),
        ("xoxp-", "a Slack token", 10),
        ("xoxa-", "a Slack token", 10),
        ("xoxe-", "a Slack token", 10),
        ("xapp-", "a Slack token", 10),
        ("sk-proj-", "an OpenAI key", 16),
        ("sk_live_", "a Stripe key", 16),
        ("rk_live_", "a Stripe key", 16),
        ("npm_", "an npm token", 30),
        ("hf_", "a Hugging Face token", 30),
        ("AIza", "a Google key", 30),
    ];
    let token_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    for (prefix, what, min) in PREFIXES {
        for (i, _) in text.match_indices(prefix) {
            let before = text[..i].chars().next_back();
            if before.is_some_and(token_char) && !prefix.starts_with("sk-ant") {
                continue;
            }
            let tail = text[i + prefix.len()..]
                .chars()
                .take_while(|c| token_char(*c))
                .count();
            if tail >= *min {
                return Some(what);
            }
        }
    }
    // AWS access keys: AKIA or ASIA + 16 upper-case letters or digits.
    for prefix in ["AKIA", "ASIA"] {
        for (i, _) in text.match_indices(prefix) {
            let tail: String = text[i + 4..].chars().take(16).collect();
            if tail.len() == 16
                && tail
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                return Some("an AWS key");
            }
        }
    }
    if text.contains("-----BEGIN") && text.contains("PRIVATE KEY") {
        return Some("a private key");
    }
    // JWTs: header.payload starting with eyJ.
    if text.match_indices("eyJ").any(|(i, _)| {
        text[i..]
            .chars()
            .take_while(|c| token_char(*c) || *c == '.')
            .filter(|c| *c == '.')
            .count()
            >= 2
    }) {
        return Some("a JSON Web Token");
    }
    // Header or assignment with a literal value (not a ${VAR} reference).
    let lower = text.to_ascii_lowercase();
    for (needle, what) in [
        ("authorization:", "an Authorization header"),
        ("x-api-key:", "an API key header"),
        ("aws_secret_access_key", "an AWS secret"),
        ("bearer ", "a bearer token"),
    ] {
        for (i, _) in lower.match_indices(needle) {
            let mut rest = text[i + needle.len()..].trim_start_matches([' ', '=', ':', '"', '\'']);
            // Skip an auth scheme word ("Bearer abc…") to reach the value.
            for scheme in ["Bearer ", "bearer ", "Basic ", "basic ", "Token ", "token "] {
                if let Some(r) = rest.strip_prefix(scheme) {
                    rest = r.trim_start();
                }
            }
            let value: String = rest
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                .collect();
            if value.len() >= 16
                && !value.contains("${")
                && !value.starts_with('$')
                && !value.starts_with('<')
            {
                return Some(what);
            }
        }
    }
    // user:password@ in URLs.
    if let Some(i) = text.find("://") {
        let rest = &text[i + 3..];
        let host_part: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '/')
            .collect();
        if let Some((cred, _)) = host_part.split_once('@') {
            if cred.contains(':') && !cred.contains("${") {
                return Some("a password in a URL");
            }
        }
    }
    None
}

// ---------------------------------------------------------------- names and templates

pub fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-');
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "\"{name}\" must be 1–64 lowercase letters, digits or dashes, for example review-pr."
        )))
    }
}

fn yaml_quote(s: &str) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "\"{}\"",
        one_line.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// A new item from the form fields. Description stays on one line.
pub fn template(kind: Kind, name: &str, description: &str, body: &str) -> String {
    let body = if body.trim().is_empty() {
        match kind {
            Kind::Skill => "Steps Claude should follow when this skill applies:\n\n1. \n2. \n3. \n\nKeep this file short; link longer reference material from files in this folder.\n",
            Kind::Agent => "You are a focused helper. Describe its job, what it may change, and what it returns.\n",
            Kind::Command => "What to do with $ARGUMENTS:\n\n1. \n2. \n",
        }
        .to_string()
    } else {
        format!("{}\n", body.trim_end())
    };
    match kind {
        Kind::Skill | Kind::Agent => format!(
            "---\nname: {name}\ndescription: {}\n---\n\n{body}",
            yaml_quote(description)
        ),
        Kind::Command => format!(
            "---\ndescription: {}\n---\n\n{body}",
            yaml_quote(description)
        ),
    }
}

// ---------------------------------------------------------------- safe file access

/// Read a file that must be a regular file reached without symlinks below
/// `root` (each component is checked; the last open uses O_NOFOLLOW).
/// `Ok(None)` if it does not exist.
pub fn read_no_links(root: &Path, rel: &Path) -> Result<Option<String>> {
    let mut cur = root.to_path_buf();
    let parts: Vec<Component> = rel.components().collect();
    for (i, part) in parts.iter().enumerate() {
        let Component::Normal(seg) = part else {
            return Err(Error::Invalid("Unexpected path.".into()));
        };
        cur.push(seg);
        match fs::symlink_metadata(&cur) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
            Ok(m) if m.file_type().is_symlink() => {
                return Err(Error::Invalid(format!(
                    "{} is a link. Facet does not read or write through links.",
                    cur.display()
                )));
            }
            Ok(m) if i + 1 < parts.len() && !m.is_dir() => return Ok(None),
            Ok(_) => {}
        }
    }
    let mut f = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&cur)?;
    let mut text = String::new();
    f.read_to_string(&mut text)?;
    Ok(Some(text))
}

/// Folders an account item may be written to: inside the account folder, not
/// in `skills/synced`, not in a managed-policy folder, no links on the way.
fn check_account_target(code_dir: &Path, rel: &Path) -> Result<()> {
    let real_root = fs::canonicalize(code_dir).unwrap_or_else(|_| code_dir.to_path_buf());
    if MANAGED_DIRS.iter().any(|m| real_root.starts_with(m)) {
        return Err(Error::Invalid(
            "This folder holds managed policy. Facet never writes there.".into(),
        ));
    }
    if rel.starts_with("skills/synced") {
        return Err(Error::Invalid(
            "skills/synced belongs to Claude Code; Facet never writes there.".into(),
        ));
    }
    let mut cur = code_dir.to_path_buf();
    for part in rel.components() {
        cur.push(part);
        if fs::symlink_metadata(&cur).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid(format!(
                "{} is a link. Facet does not write through links.",
                cur.display()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- library

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub kind: Kind,
    pub name: String,
    pub path: PathBuf,
    pub description: String,
    pub hash: String,
    pub lint: Vec<Finding>,
}

pub struct Library {
    pub root: PathBuf,
}

impl Library {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn list(&self) -> Vec<Item> {
        list_items(&self.root, false)
    }

    pub fn read(&self, kind: Kind, name: &str) -> Result<String> {
        validate_name(name)?;
        read_no_links(&self.root, &kind.rel_path(name))?
            .ok_or_else(|| Error::NotFound(format!("{name} in the library")))
    }

    /// Save an item. Refused if it contains a secret; lint problems are shown
    /// but do not block saving (placement refuses items with errors).
    pub fn save(&self, kind: Kind, name: &str, text: &str) -> Result<Item> {
        validate_name(name)?;
        if let Some(what) = find_secret(text) {
            return Err(Error::Invalid(format!("This looks like it contains {what}. The library never stores secrets; use an environment variable reference instead.")));
        }
        let rel = kind.rel_path(name);
        let path = self.root.join(&rel);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        check_account_target(&self.root, &rel)?;
        if path.exists() {
            replace_atomically(&path, text)?;
        } else {
            write_new_private(&path, text)?;
        }
        Ok(item_from(kind, name, &path, text))
    }

    /// Delete from the library only (the item's own file; other files in a
    /// skill folder stay). Copies placed in accounts are not touched.
    pub fn remove(&self, kind: Kind, name: &str) -> Result<()> {
        validate_name(name)?;
        let rel = kind.rel_path(name);
        check_account_target(&self.root, &rel)?;
        let path = self.root.join(rel);
        fs::remove_file(&path)?;
        if kind == Kind::Skill {
            let _ = fs::remove_dir(path.parent().expect("skill folder"));
        }
        Ok(())
    }
}

fn item_from(kind: Kind, name: &str, path: &Path, text: &str) -> Item {
    let l = lint(kind, name, text);
    Item {
        kind,
        name: name.to_string(),
        path: path.to_path_buf(),
        description: l.description,
        hash: hash(text),
        lint: l.findings,
    }
}

/// Items under a root. In an account, `skills/synced` (owned by Claude Code)
/// is skipped. Linked files and folders are skipped everywhere.
pub fn list_items(root: &Path, account: bool) -> Vec<Item> {
    let mut out = Vec::new();
    let mut add = |kind: Kind, name: &str| {
        if validate_name(name).is_err() {
            return;
        }
        let rel = kind.rel_path(name);
        if let Ok(Some(text)) = read_no_links(root, &rel) {
            out.push(item_from(kind, name, &root.join(rel), &text));
        }
    };
    if let Ok(entries) = fs::read_dir(root.join("skills")) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if account && name == "synced" {
                continue;
            }
            add(Kind::Skill, &name);
        }
    }
    for (kind, dir) in [(Kind::Agent, "agents"), (Kind::Command, "commands")] {
        if let Ok(entries) = fs::read_dir(root.join(dir)) {
            for e in entries.flatten() {
                let file = e.file_name().to_string_lossy().into_owned();
                if let Some(name) = file.strip_suffix(".md") {
                    add(kind, name);
                }
            }
        }
    }
    out.sort_by(|a, b| (a.kind as u8, &a.name).cmp(&(b.kind as u8, &b.name)));
    out
}

// ---------------------------------------------------------------- placement

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlacementState {
    /// Not in the account.
    Absent,
    /// Same content as the library.
    InSync,
    /// Present but different (edited in the account, or the library moved on).
    Drifted,
    /// Present but a link, or holding a secret: Facet leaves it alone.
    Untouchable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub account_id: String,
    pub state: PlacementState,
}

pub fn placement_state(lib_text: &str, code_dir: &Path, kind: Kind, name: &str) -> PlacementState {
    match read_no_links(code_dir, &kind.rel_path(name)) {
        Ok(None) => PlacementState::Absent,
        Ok(Some(t)) if t == lib_text => PlacementState::InSync,
        Ok(Some(t)) if find_secret(&t).is_some() => PlacementState::Untouchable,
        Ok(Some(_)) => PlacementState::Drifted,
        Err(_) => PlacementState::Untouchable,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacePreview {
    pub target: PathBuf,
    pub state: PlacementState,
    pub base_hash: String,
    pub diff: Vec<DiffLine>,
    /// What the item does that needs care; placing requires acknowledging these.
    pub risks: Vec<Finding>,
    /// Errors that stop placement (lint errors, secrets).
    pub blockers: Vec<Finding>,
    /// Claude Code sessions of this account running now.
    pub live_sessions: usize,
}

/// Read the account's current copy for a placement decision. Refuses links
/// and copies holding secrets (never shown, backed up or replaced).
fn current_copy(code_dir: &Path, kind: Kind, name: &str) -> Result<Option<String>> {
    let rel = kind.rel_path(name);
    check_account_target(code_dir, &rel)?;
    let current = read_no_links(code_dir, &rel)?;
    if let Some(text) = &current {
        if let Some(what) = find_secret(text) {
            return Err(Error::Invalid(format!(
                "The account's copy of {name} contains {what}. Facet will not show, back up or replace it; remove the secret by hand first."
            )));
        }
    }
    Ok(current)
}

fn gate(lib_text: &str, kind: Kind, name: &str) -> (Vec<Finding>, Vec<Finding>) {
    let l = lint(kind, name, lib_text);
    let risks = l
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Risk)
        .cloned()
        .collect();
    let blockers = l
        .findings
        .into_iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    (risks, blockers)
}

pub fn place_preview(
    lib_text: &str,
    code_dir: &Path,
    kind: Kind,
    name: &str,
    live_sessions: usize,
) -> Result<PlacePreview> {
    validate_name(name)?;
    let current = current_copy(code_dir, kind, name)?;
    let (risks, blockers) = gate(lib_text, kind, name);
    let current = current.unwrap_or_default();
    Ok(PlacePreview {
        state: placement_state(lib_text, code_dir, kind, name),
        base_hash: hash(&current),
        diff: diff(&current, lib_text),
        target: code_dir.join(kind.rel_path(name)),
        risks,
        blockers,
        live_sessions,
    })
}

fn backup_name(stamp: &str, kind: Kind, name: &str) -> String {
    format!("{stamp}-{}-{name}.md", kind.label())
}

fn is_studio_backup(n: &str) -> bool {
    n.ends_with(".md") && n.len() > 21 && n.as_bytes()[8] == b'T'
}

/// Keep this many Studio backups per account.
pub const KEEP_BACKUPS: usize = 30;

fn backup(backup_dir: &Path, stamp: &str, kind: Kind, name: &str, text: &str) -> Result<PathBuf> {
    ensure_private_dir(backup_dir)?;
    let dest = backup_dir.join(backup_name(stamp, kind, name));
    write_new_private(&dest, text)?;
    prune_dir(backup_dir, KEEP_BACKUPS, is_studio_backup);
    Ok(dest)
}

/// Copy the library version into the account. The core re-checks secrets,
/// lint errors and the risk acknowledgement; anything replaced is backed up
/// first; refused if the account file changed since preview.
#[allow(clippy::too_many_arguments)]
pub fn place(
    lib_text: &str,
    code_dir: &Path,
    kind: Kind,
    name: &str,
    base_hash: &str,
    risk_ack: bool,
    backup_dir: &Path,
    stamp: &str,
) -> Result<Option<PathBuf>> {
    validate_name(name)?;
    if let Some(what) = find_secret(lib_text) {
        return Err(Error::Invalid(format!(
            "The library copy contains {what}; remove it before placing."
        )));
    }
    let (risks, blockers) = gate(lib_text, kind, name);
    if let Some(b) = blockers.first() {
        return Err(Error::Invalid(format!("Fix this first: {}", b.message)));
    }
    if !risks.is_empty() && !risk_ack {
        return Err(Error::Invalid("This item runs commands on this computer. Confirm that you understand before placing it.".into()));
    }
    let current = current_copy(code_dir, kind, name)?;
    if hash(current.as_deref().unwrap_or("")) != base_hash {
        return Err(Error::Invalid(
            "The account's copy changed since you reviewed it. Review again.".into(),
        ));
    }
    let saved = match &current {
        Some(text) if text != lib_text => Some(backup(backup_dir, stamp, kind, name, text)?),
        _ => None,
    };
    let rel = kind.rel_path(name);
    let target = code_dir.join(&rel);
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir)?;
    }
    check_account_target(code_dir, &rel)?;
    if current.is_some() {
        replace_atomically(&target, lib_text)?;
    } else {
        write_new_private(&target, lib_text)?;
    }
    Ok(saved)
}

/// Remove a placed item from an account (backed up first). Only the item's
/// own file is removed; other files in a skill folder stay.
pub fn unplace(
    code_dir: &Path,
    kind: Kind,
    name: &str,
    backup_dir: &Path,
    stamp: &str,
) -> Result<PathBuf> {
    validate_name(name)?;
    let text = current_copy(code_dir, kind, name)?
        .ok_or_else(|| Error::NotFound(format!("{name} in this account")))?;
    let dest = backup(backup_dir, stamp, kind, name, &text)?;
    let target = code_dir.join(kind.rel_path(name));
    fs::remove_file(&target)?;
    if kind == Kind::Skill {
        let _ = fs::remove_dir(target.parent().expect("skill folder"));
    }
    Ok(dest)
}

/// Read an account item for import (no links, secret scan in `Library::save`).
pub fn read_account_item(code_dir: &Path, kind: Kind, name: &str) -> Result<String> {
    validate_name(name)?;
    let rel = kind.rel_path(name);
    if rel.starts_with("skills/synced") {
        return Err(Error::Invalid(
            "skills/synced belongs to Claude Code.".into(),
        ));
    }
    read_no_links(code_dir, &rel)?.ok_or_else(|| Error::NotFound(format!("{name} in this account")))
}

/// `~/.local/share/facet/library` (or `$XDG_DATA_HOME/facet/library`).
pub fn library_root(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("facet/library")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    const GOOD: &str = "---\nname: review-pr\ndescription: \"Review a pull request for bugs and missing tests. Use when the user asks to review a PR or a diff.\"\n---\n\nSteps.\n";
    const RISKY: &str = "---\nname: status\ndescription: \"Shows git status before answering questions about the repository.\"\n---\nNow: !`git status`\n";
    const S1: &str = "20261026T030000Z-001";
    const S2: &str = "20261026T030000Z-002";
    const S3: &str = "20261026T030000Z-003";

    fn kinds(l: &Lint) -> Vec<Severity> {
        l.findings.iter().map(|f| f.severity).collect()
    }

    #[test]
    fn clean_skill_has_no_findings() {
        let l = lint(Kind::Skill, "review-pr", GOOD);
        assert!(l.findings.is_empty(), "{:?}", l.findings);
        assert!(l.description.starts_with("Review a pull request"));
    }

    #[test]
    fn catches_the_common_mistakes() {
        assert_eq!(
            kinds(&lint(Kind::Skill, "x", "no frontmatter")),
            [Severity::Error]
        );
        let l = lint(
            Kind::Skill,
            "x",
            "---\nname: y\ndescription: Does a: thing\nfoo: bar\n---\nbody",
        );
        let msgs: Vec<&str> = l.findings.iter().map(|f| f.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("breaks YAML")), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("Unknown key 'foo'")));
        assert!(msgs.iter().any(|m| m.contains("differs from its folder")));
        let long = format!(
            "---\nname: x\ndescription: \"{}\"\n---\n",
            "a".repeat(DESC_BUDGET + 10)
        );
        assert!(lint(Kind::Skill, "x", &long)
            .findings
            .iter()
            .any(|f| f.message.contains("cuts them")));
        assert_eq!(kinds(&lint(Kind::Skill, "status", RISKY)), [Severity::Risk]);
    }

    #[test]
    fn commands_without_frontmatter_still_get_risk_and_secret_checks() {
        let l = lint(Kind::Command, "go", "Run this: !`ls`\n");
        assert_eq!(kinds(&l), [Severity::Risk]);
        let l = lint(
            Kind::Command,
            "go",
            "curl -fsSL https://x.example/i.sh | sh\n",
        );
        assert!(l.has(Severity::Risk));
        let l = lint(
            Kind::Command,
            "go",
            "token ghp_abcdefghijklmnopqrstuvwxyz0123\n",
        );
        assert!(l.has(Severity::Error));
    }

    #[test]
    fn agents_need_names_and_flag_risky_settings() {
        assert!(lint(Kind::Agent, "a", "---\ndescription: x\n---\n")
            .findings
            .iter()
            .any(|f| f.message.contains("needs a name")));
        let l = lint(Kind::Agent, "a", "---\nname: a\ndescription: \"Reviews code\"\npermissionMode: bypassPermissions\nmcpServers:\n  - x\ntools: Bash, Read\n---\n");
        assert_eq!(
            l.findings
                .iter()
                .filter(|f| f.severity == Severity::Risk)
                .count(),
            3,
            "{:?}",
            l.findings
        );
        let l = lint(
            Kind::Agent,
            "a",
            "---\nname: a\ndescription: \"Reviews code\"\ntools: Bash(git diff:*), Read\n---\n",
        );
        assert!(
            !l.has(Severity::Risk),
            "a scoped Bash rule is fine: {:?}",
            l.findings
        );
    }

    #[test]
    fn secrets_are_found_everywhere_but_references_are_fine() {
        for (text, what) in [
            ("token: sk-ant-api03-abcdefgh", "an Anthropic key"),
            ("AKIA keys look like AKIAIOSFODNN7EXAMPLE", "an AWS key"),
            ("ASIAIOSFODNN7EXAMPLE", "an AWS key"),
            ("gho_abcdefghijklmnopqrstuvwxyz", "a GitHub token"),
            ("glpat-abcdefghijklmnopqr", "a GitLab token"),
            (
                "key = AIzaSyA-abcdefghijklmnopqrstuvwxyz012",
                "a Google key",
            ),
            (
                "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc",
                "a JSON Web Token",
            ),
            (
                "Authorization: Bearer abcdefghijklmnopqrstuvwxyz",
                "an Authorization header",
            ),
            ("x-api-key: abcdefghijklmnopqrstu", "an API key header"),
            ("https://bob:hunter2@example.com/x", "a password in a URL"),
            ("-----BEGIN OPENSSH PRIVATE KEY-----", "a private key"),
        ] {
            assert_eq!(find_secret(text), Some(what), "{text}");
        }
        for fine in [
            "use ${GITHUB_TOKEN}",
            "Authorization: Bearer ${TOKEN}",
            "AKIA is a prefix",
            "the ghp_ prefix",
            "https://example.com/a@b",
            "sk-ant- is the prefix",
        ] {
            assert_eq!(find_secret(fine), None, "{fine}");
        }
    }

    #[test]
    fn template_round_trips_through_lint() {
        for kind in [Kind::Skill, Kind::Agent, Kind::Command] {
            let t = template(
                kind,
                "review-pr",
                "Review a pull request: bugs, tests and naming. Use when asked to review.",
                "",
            );
            assert!(
                !lint(kind, "review-pr", &t).has(Severity::Error),
                "{kind:?}\n{t}"
            );
        }
        let t = template(Kind::Skill, "q", "Say \"hi\"\nthen stop", "");
        assert!(
            t.contains("description: \"Say \\\"hi\\\" then stop\"\n"),
            "{t}"
        );
    }

    #[test]
    fn library_save_list_read_remove_keeps_other_files() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = Library::new(tmp.path().join("lib"));
        lib.save(Kind::Skill, "review-pr", GOOD).unwrap();
        fs::write(
            tmp.path().join("lib/skills/review-pr/reference.md"),
            "notes",
        )
        .unwrap();
        lib.save(
            Kind::Command,
            "ship",
            "---\ndescription: \"Ship it\"\n---\nGo.\n",
        )
        .unwrap();
        lib.save(Kind::Skill, "review-pr", GOOD).unwrap(); // replace in place
        assert_eq!(
            lib.list()
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>(),
            ["review-pr", "ship"]
        );
        assert!(lib.save(Kind::Skill, "../evil", GOOD).is_err());
        assert!(lib
            .save(
                Kind::Skill,
                "x",
                "---\nname: x\ndescription: y\n---\nghp_abcdefghijklmnopqrstuvwx\n"
            )
            .unwrap_err()
            .to_string()
            .contains("never stores secrets"));
        lib.remove(Kind::Skill, "review-pr").unwrap();
        assert!(
            tmp.path()
                .join("lib/skills/review-pr/reference.md")
                .exists(),
            "other files stay"
        );
        assert_eq!(lib.list().len(), 1);
    }

    #[test]
    fn placement_flow_with_backups_and_hash_check() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join(".claude-work");
        let backups = tmp.path().join("backups/studio");
        assert_eq!(
            placement_state(GOOD, &code, Kind::Skill, "review-pr"),
            PlacementState::Absent
        );
        let p = place_preview(GOOD, &code, Kind::Skill, "review-pr", 0).unwrap();
        assert!(p.risks.is_empty() && p.blockers.is_empty());
        assert!(place(
            GOOD,
            &code,
            Kind::Skill,
            "review-pr",
            &p.base_hash,
            false,
            &backups,
            S1
        )
        .unwrap()
        .is_none());
        assert_eq!(
            placement_state(GOOD, &code, Kind::Skill, "review-pr"),
            PlacementState::InSync
        );
        fs::write(code.join("skills/review-pr/SKILL.md"), "edited").unwrap();
        assert_eq!(
            placement_state(GOOD, &code, Kind::Skill, "review-pr"),
            PlacementState::Drifted
        );
        let p = place_preview(GOOD, &code, Kind::Skill, "review-pr", 2).unwrap();
        assert_eq!(p.live_sessions, 2);
        let b = place(
            GOOD,
            &code,
            Kind::Skill,
            "review-pr",
            &p.base_hash,
            false,
            &backups,
            S2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(fs::read_to_string(&b).unwrap(), "edited");
        assert_eq!(
            fs::metadata(&b).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&backups).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let p = place_preview(GOOD, &code, Kind::Skill, "review-pr", 0).unwrap();
        fs::write(code.join("skills/review-pr/SKILL.md"), "again").unwrap();
        assert!(place(
            GOOD,
            &code,
            Kind::Skill,
            "review-pr",
            &p.base_hash,
            false,
            &backups,
            S3
        )
        .is_err());
        let b = unplace(
            &code,
            Kind::Skill,
            "review-pr",
            &backups,
            "20261026T030000Z-004",
        )
        .unwrap();
        assert_eq!(fs::read_to_string(b).unwrap(), "again");
        assert!(!code.join("skills/review-pr").exists());
    }

    #[test]
    fn risky_items_need_acknowledgement_and_errors_block() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join("c");
        let p = place_preview(RISKY, &code, Kind::Skill, "status", 0).unwrap();
        assert_eq!(p.risks.len(), 1);
        let err = place(
            RISKY,
            &code,
            Kind::Skill,
            "status",
            &p.base_hash,
            false,
            tmp.path(),
            S1,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("Confirm"), "{err}");
        place(
            RISKY,
            &code,
            Kind::Skill,
            "status",
            &p.base_hash,
            true,
            tmp.path(),
            S1,
        )
        .unwrap();
        let broken = "no frontmatter";
        let p = place_preview(broken, &code, Kind::Skill, "broken", 0).unwrap();
        assert!(!p.blockers.is_empty());
        assert!(place(
            broken,
            &code,
            Kind::Skill,
            "broken",
            &p.base_hash,
            true,
            tmp.path(),
            S2
        )
        .is_err());
    }

    #[test]
    fn links_synced_and_secret_copies_are_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let code = tmp.path().join("c");
        fs::create_dir_all(code.join("agents")).unwrap();
        fs::create_dir_all(code.join("skills/synced/bucket/pdf")).unwrap();
        fs::write(code.join("skills/synced/bucket/pdf/SKILL.md"), GOOD).unwrap();
        // A linked agent pointing at a sign-in file.
        let secret_file = tmp.path().join("creds.json");
        fs::write(&secret_file, "{\"token\":\"x\"}").unwrap();
        symlink(&secret_file, code.join("agents/sneaky.md")).unwrap();
        assert_eq!(
            placement_state(GOOD, &code, Kind::Agent, "sneaky"),
            PlacementState::Untouchable
        );
        assert!(place_preview(GOOD, &code, Kind::Agent, "sneaky", 0).is_err());
        assert!(unplace(&code, Kind::Agent, "sneaky", tmp.path(), S1).is_err());
        assert!(read_account_item(&code, Kind::Agent, "sneaky").is_err());
        assert!(
            list_items(&code, true).is_empty(),
            "links and synced are not listed"
        );
        // A linked skill folder into synced cannot be written through.
        symlink(
            code.join("skills/synced/bucket/pdf"),
            code.join("skills/pdf"),
        )
        .unwrap();
        assert!(place_preview(GOOD, &code, Kind::Skill, "pdf", 0).is_err());
        assert!(place_preview(GOOD, &code, Kind::Skill, "synced", 0).is_err());
        // An account copy holding a secret is never shown or replaced.
        fs::write(
            code.join("agents/token.md"),
            "---\nname: token\ndescription: x\n---\nghp_abcdefghijklmnopqrstuvwxyz\n",
        )
        .unwrap();
        assert_eq!(
            placement_state(GOOD, &code, Kind::Agent, "token"),
            PlacementState::Untouchable
        );
        let err = place_preview(GOOD, &code, Kind::Agent, "token", 0)
            .unwrap_err()
            .to_string();
        assert!(err.contains("remove the secret by hand"), "{err}");
        assert!(unplace(&code, Kind::Agent, "token", tmp.path(), S2).is_err());
    }

    #[test]
    fn backups_are_pruned() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..(KEEP_BACKUPS + 3) {
            backup(
                tmp.path(),
                &format!("20261026T030000Z-{i:03}"),
                Kind::Skill,
                "x",
                "t",
            )
            .unwrap();
        }
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), KEEP_BACKUPS);
    }
}
