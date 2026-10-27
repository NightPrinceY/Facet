//! Ecosystem (PRD M5): what each account has installed, and a safety
//! preview of any plugin, skill pack or repository folder, made by reading
//! files only. Nothing here runs a package, starts an MCP server or goes on
//! the network. Secrets in commands are hidden before they leave the core.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::control;
use crate::studio::{self, Finding, Kind, Severity};

const MAX_FILES: usize = 4000;
const MAX_TEXT_BYTES: u64 = 512 * 1024;
const MAX_DEPTH: usize = 12;
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    "target",
    "dist",
    ".next",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NamedItem {
    pub name: String,
    pub path: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HookInfo {
    pub event: String,
    pub matcher: Option<String>,
    /// command | http | prompt | agent
    pub kind: String,
    /// The command or address, with secrets hidden.
    pub target: String,
    pub source: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpInfo {
    pub name: String,
    /// stdio | http | sse
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    /// Names only, never values.
    pub env_names: Vec<String>,
    pub header_names: Vec<String>,
    /// user | local | project | plugin
    pub scope: String,
    pub project: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageReport {
    pub root: String,
    /// plugin | repo | folder
    pub kind: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    /// Content fingerprint of every scanned file (rug-pull check).
    pub hash: String,
    pub skills: Vec<NamedItem>,
    pub commands: Vec<NamedItem>,
    pub agents: Vec<NamedItem>,
    pub hooks: Vec<HookInfo>,
    pub mcp: Vec<McpInfo>,
    /// Programs in `bin/`, which Claude Code adds to PATH.
    pub bin: Vec<String>,
    pub endpoints: Vec<String>,
    pub env_vars: Vec<String>,
    pub findings: Vec<Finding>,
    pub risk: Risk,
    pub reasons: Vec<String>,
    pub files: usize,
    pub bytes: u64,
    pub truncated: bool,
}

fn finding(severity: Severity, message: impl Into<String>) -> Finding {
    Finding {
        severity,
        message: message.into(),
    }
}

/// Flags whose next word is a secret.
const SECRET_FLAGS: &[&str] = &[
    "--token",
    "--api-key",
    "--apikey",
    "--key",
    "--password",
    "--passwd",
    "--secret",
    "--auth",
    "--access-token",
    "--client-secret",
    "--bearer",
    "--header",
    "-H",
    "authorization:",
    "bearer",
    "x-api-key:",
    "api-key:",
];

fn secret_word(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    studio::find_secret(word).is_some()
        || (word.contains('=') && {
            let k = word.split('=').next().unwrap_or("").to_ascii_uppercase();
            control::is_secret_env(k.trim_start_matches('-'))
        })
        || lower.contains("token=")
        || lower.contains("key=")
        || lower.contains("password=")
        || lower.contains("secret=")
}

/// URL with its query hidden and key-like path parts hidden.
fn redact_url(word: &str) -> Option<String> {
    let scheme = word.find("://")?;
    let rest = &word[scheme + 3..];
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let host = match host.rsplit_once('@') {
        Some((_, h)) => format!("‹hidden›@{h}"),
        None => host.to_string(),
    };
    let (path, query) = path.split_at(path.find(['?', '#']).unwrap_or(path.len()));
    let path: Vec<String> = path
        .split('/')
        .map(|seg| {
            let lower = seg.to_ascii_lowercase();
            let mixed = seg.len() >= 16
                && seg.chars().any(|c| c.is_ascii_digit())
                && seg.chars().any(|c| c.is_ascii_alphabetic());
            if mixed
                || lower.contains("key")
                || lower.contains("token")
                || studio::find_secret(seg).is_some()
            {
                "‹hidden›".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();
    Some(format!(
        "{}://{host}{}{}",
        &word[..scheme],
        path.join("/"),
        if query.is_empty() {
            ""
        } else {
            "?‹hidden›"
        }
    ))
}

/// Hide anything that looks like a secret inside a command or URL.
pub fn redact(s: &str) -> String {
    let words: Vec<&str> = s.split(' ').collect();
    redact_words(&words).join(" ")
}

/// The same for an argument list, where a flag and its value are separate.
pub fn redact_args(args: &[String]) -> Vec<String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    redact_words(&words)
}

fn redact_words(words: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(words.len());
    let mut hide_next = false;
    for word in words {
        let lower = word.to_ascii_lowercase();
        let is_flag = SECRET_FLAGS.iter().any(|f| lower == f.to_ascii_lowercase());
        if hide_next && !word.is_empty() {
            // "Authorization: Bearer x" hides every part after the flag.
            hide_next = is_flag;
            out.push("‹hidden›".to_string());
            continue;
        }
        if is_flag {
            hide_next = true;
            out.push(word.to_string());
            continue;
        }
        // URLs first: a key in the query or path must not keep its prefix.
        if let Some(u) = redact_url(word) {
            out.push(u);
        } else if secret_word(word) {
            let keep = word
                .split_once('=')
                .map(|(k, _)| format!("{k}="))
                .unwrap_or_default();
            out.push(format!("{keep}‹hidden›"));
        } else {
            out.push(word.to_string());
        }
    }
    out
}

// ---------- walking ----------

struct Walk {
    files: Vec<(String, u64)>,
    links: Vec<String>,
    truncated: bool,
}

fn walk(root: &Path) -> Walk {
    let mut w = Walk {
        files: Vec::new(),
        links: Vec::new(),
        truncated: false,
    };
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut dirs = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        dirs += 1;
        if dirs > MAX_FILES {
            w.truncated = true;
            return w;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let path = e.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                w.links.push(rel);
                continue;
            }
            if meta.is_dir() {
                let name = e.file_name();
                if depth < MAX_DEPTH && !SKIP_DIRS.contains(&name.to_string_lossy().as_ref()) {
                    stack.push((path, depth + 1));
                }
            } else if meta.is_file() {
                if w.files.len() >= MAX_FILES {
                    w.truncated = true;
                    return w;
                }
                w.files.push((rel, meta.len()));
            }
        }
    }
    w.files.sort();
    w
}

fn read_text(root: &Path, rel: &str, size: u64) -> Option<String> {
    if size > MAX_TEXT_BYTES {
        return None;
    }
    let bytes = fs::read(root.join(rel)).ok()?;
    if bytes.iter().take(4096).any(|b| *b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}

// ---------- text signals ----------

/// Web hosts mentioned in a file ("https://api.example.com/x" → "api.example.com").
fn endpoints_in(text: &str, out: &mut BTreeSet<String>) {
    for scheme in ["https://", "http://", "wss://", "ws://"] {
        let mut rest = text;
        while let Some(i) = rest.find(scheme) {
            let after = &rest[i + scheme.len()..];
            let host: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == ':')
                .collect();
            let host = host.trim_end_matches([':', '.']).to_ascii_lowercase();
            if host.contains('.') || host.starts_with("localhost") {
                let label = if scheme.starts_with("http:") || scheme.starts_with("ws:") {
                    format!("{host} (not encrypted)")
                } else {
                    host
                };
                out.insert(label);
            }
            rest = &after[host_len(after)..];
        }
    }
}

fn host_len(s: &str) -> usize {
    s.char_indices()
        .find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == ':'))
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Environment variables a file reads, by common patterns.
fn env_vars_in(text: &str, out: &mut BTreeSet<String>) {
    let pats = [
        "process.env.",
        "os.environ[\"",
        "os.environ['",
        "os.getenv(\"",
        "os.getenv('",
        "os.environ.get(\"",
        "os.environ.get('",
        "env::var(\"",
        "Deno.env.get(\"",
        "${",
    ];
    for p in pats {
        let mut rest = text;
        while let Some(i) = rest.find(p) {
            let after = &rest[i + p.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
                .collect();
            if name.len() >= 3
                && name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && name != "CLAUDE_PLUGIN_ROOT"
                && name != "CLAUDE_PROJECT_DIR"
            {
                out.insert(name);
            }
            rest = after;
        }
    }
}

// ---------- hooks, MCP, settings ----------

fn hook_notes(event: &str, kind: &str) -> Vec<String> {
    let mut n = Vec::new();
    match event {
        "SessionStart" => n.push("Runs every time a session starts.".into()),
        "UserPromptSubmit" => n.push("Sees every prompt you send and can add to it.".into()),
        "PreToolUse" | "PermissionRequest" => {
            n.push("Can allow, block or change what Claude is about to do.".into())
        }
        "PostToolUse" => n.push("Sees the output of tools Claude ran.".into()),
        _ => {}
    }
    if kind == "http" {
        n.push("Sends hook data to a web address.".into());
    }
    n
}

/// Hooks in the settings/hooks.json shape: `{event: [{matcher, hooks: [{type, command|url}]}]}`.
pub fn parse_hooks(hooks: &Value, source: &str) -> Vec<HookInfo> {
    let mut out = Vec::new();
    let Some(map) = hooks.as_object() else {
        return out;
    };
    for (event, groups) in map {
        for g in groups.as_array().into_iter().flatten() {
            let matcher = g
                .get("matcher")
                .and_then(Value::as_str)
                .filter(|m| !m.is_empty())
                .map(String::from);
            for h in g
                .get("hooks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let kind = h
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("command")
                    .to_string();
                let target = h
                    .get("command")
                    .or_else(|| h.get("url"))
                    .or_else(|| h.get("prompt"))
                    .and_then(Value::as_str)
                    .map(redact)
                    .unwrap_or_default();
                out.push(HookInfo {
                    notes: hook_notes(event, &kind),
                    event: event.clone(),
                    matcher: matcher.clone(),
                    kind,
                    target: target.chars().take(400).collect(),
                    source: source.into(),
                });
            }
        }
    }
    out
}

fn unpinned_launcher(command: &str, args: &[String]) -> Option<String> {
    let base = Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(command);
    let pkg = |skip: &[&str]| {
        args.iter()
            .find(|a| !a.starts_with('-') && !skip.contains(&a.as_str()))
            .cloned()
    };
    let pkg = match base {
        "npx" | "bunx" | "pnpx" => pkg(&[]),
        "pnpm" | "yarn" | "npm" => pkg(&["dlx", "exec", "x"]),
        "uvx" | "pipx" => pkg(&["run"]),
        "docker" | "podman" => args
            .iter()
            .skip_while(|a| *a != "run")
            .skip(1)
            .find(|a| !a.starts_with('-'))
            .cloned(),
        _ => None,
    }?;
    let pinned = match base {
        "docker" | "podman" => {
            pkg.contains('@') || (pkg.contains(':') && !pkg.ends_with(":latest"))
        }
        "uvx" | "pipx" => pkg.contains("==") || pkg.contains('@'),
        _ => pkg
            .rfind('@')
            .is_some_and(|i| i > 0 && !pkg[i + 1..].is_empty() && pkg[i + 1..] != *"latest"),
    };
    (!pinned)
        .then(|| format!("Starts {pkg} without a fixed version, so it can change on any start."))
}

/// MCP servers in the `.mcp.json` / `.claude.json` shape: `{name: {command, args, env, url, type, headers}}`.
pub fn parse_mcp(servers: &Value, scope: &str, project: Option<&str>) -> Vec<McpInfo> {
    let mut out = Vec::new();
    let Some(map) = servers.as_object() else {
        return out;
    };
    for (name, s) in map {
        let command = s.get("command").and_then(Value::as_str).map(redact);
        let args: Vec<String> = s
            .get("args")
            .and_then(Value::as_array)
            .map(|a| {
                let raw: Vec<String> = a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect();
                redact_args(&raw)
            })
            .unwrap_or_default();
        let url = s.get("url").and_then(Value::as_str).map(redact);
        let transport = s
            .get("type")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| {
                if url.is_some() {
                    "http".into()
                } else {
                    "stdio".into()
                }
            });
        let names = |k: &str| -> Vec<String> {
            s.get(k)
                .and_then(Value::as_object)
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_default()
        };
        let env_names = names("env");
        let header_names = names("headers");
        let mut notes = Vec::new();
        if let Some(c) = &command {
            if let Some(n) = unpinned_launcher(c, &args) {
                notes.push(n);
            }
            notes.push("Runs a program on this computer when a session starts.".into());
        }
        if let Some(u) = &url {
            if u.starts_with("http://")
                && !u.contains("://localhost")
                && !u.contains("://127.0.0.1")
            {
                notes.push("Connects without encryption (http://).".into());
            }
        }
        if env_names.iter().chain(header_names.iter()).any(|n| {
            control::is_secret_env(&n.to_ascii_uppercase())
                || n.eq_ignore_ascii_case("authorization")
        }) {
            notes.push("Needs a secret (key or token) to work.".into());
        }
        out.push(McpInfo {
            name: name.clone(),
            transport,
            command,
            args,
            url,
            env_names,
            header_names,
            scope: scope.into(),
            project: project.map(String::from),
            notes,
        });
    }
    out
}

/// Settings a package or repository ships that change how Claude behaves.
pub fn settings_findings(v: &Value, source: &str) -> Vec<Finding> {
    let mut f = Vec::new();
    if let Some(env) = v.get("env").and_then(Value::as_object) {
        for k in env.keys() {
            if control::is_refused_env(k) {
                f.push(finding(Severity::Risk, format!("{source} sets {k}, which would replace your sign-in or send traffic elsewhere.")));
            }
        }
    }
    if v.get("apiKeyHelper").is_some() {
        f.push(finding(
            Severity::Risk,
            format!("{source} sets apiKeyHelper, a script that supplies credentials."),
        ));
    }
    if v.get("enableAllProjectMcpServers").and_then(Value::as_bool) == Some(true) {
        f.push(finding(
            Severity::Risk,
            format!("{source} turns on every MCP server a project lists, without asking."),
        ));
    }
    if v.pointer("/permissions/defaultMode")
        .and_then(Value::as_str)
        == Some("bypassPermissions")
    {
        f.push(finding(
            Severity::Risk,
            format!("{source} lets Claude act without asking (bypassPermissions)."),
        ));
    }
    for rule in v
        .pointer("/permissions/allow")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if rule == "Bash"
            || rule == "Bash(*)"
            || rule.ends_with(":*)") && rule.starts_with("Bash(") && rule.len() < 12
        {
            f.push(finding(
                Severity::Risk,
                format!("{source} allows any shell command without asking ({rule})."),
            ));
        }
    }
    if let Some(cmd) = v.pointer("/statusLine/command").and_then(Value::as_str) {
        f.push(finding(
            Severity::Info,
            format!("{source} runs a status line command: {}", redact(cmd)),
        ));
    }
    f
}

// ---------- package scan ----------

/// Read a plugin, skill pack or repository folder without running anything.
pub fn scan(root: &Path) -> PackageReport {
    let w = walk(root);
    let has = |rel: &str| w.files.iter().any(|(r, _)| r == rel);
    let kind = if has(".claude-plugin/plugin.json") {
        "plugin"
    } else if has(".claude/settings.json")
        || has(".mcp.json")
        || w.files.iter().any(|(r, _)| r.starts_with(".claude/"))
    {
        "repo"
    } else {
        "folder"
    };
    let base = if kind == "repo" { ".claude/" } else { "" };
    let mut r = PackageReport {
        root: root.display().to_string(),
        kind: kind.into(),
        name: None,
        version: None,
        description: None,
        hash: String::new(),
        skills: Vec::new(),
        commands: Vec::new(),
        agents: Vec::new(),
        hooks: Vec::new(),
        mcp: Vec::new(),
        bin: Vec::new(),
        endpoints: Vec::new(),
        env_vars: Vec::new(),
        findings: Vec::new(),
        risk: Risk::Low,
        reasons: Vec::new(),
        files: w.files.len(),
        bytes: w.files.iter().map(|(_, s)| s).sum(),
        truncated: w.truncated,
    };
    let mut hash = 0xcbf29ce484222325u64;
    let mut endpoints = BTreeSet::new();
    let mut envs = BTreeSet::new();
    let mut seen_risk = BTreeSet::new();
    for (rel, size) in &w.files {
        fnv(&mut hash, rel.as_bytes());
        let text = read_text(root, rel, *size);
        match &text {
            Some(t) => fnv(&mut hash, t.as_bytes()),
            None => fnv(&mut hash, &size.to_le_bytes()),
        }
        let Some(text) = text else { continue };
        endpoints_in(&text, &mut endpoints);
        env_vars_in(&text, &mut envs);
        let item = |dir: &str| -> Option<(Kind, String)> {
            let rest = rel.strip_prefix(&format!("{base}{dir}/"))?;
            match dir {
                "skills" => rest
                    .strip_suffix("/SKILL.md")
                    .filter(|n| !n.contains('/'))
                    .map(|n| (Kind::Skill, n.to_string())),
                "commands" => rest
                    .strip_suffix(".md")
                    .map(|n| (Kind::Command, n.replace('/', ":"))),
                "agents" => rest
                    .strip_suffix(".md")
                    .filter(|n| !n.contains('/'))
                    .map(|n| (Kind::Agent, n.to_string())),
                _ => None,
            }
        };
        // A single-skill package keeps SKILL.md at its root.
        let root_skill = || -> Option<(Kind, String)> {
            (rel == "SKILL.md").then(|| {
                let fm_name = studio::split_frontmatter(&text).and_then(|(fm, _)| {
                    fm.lines()
                        .find_map(|l| l.strip_prefix("name:"))
                        .map(|n| n.trim().trim_matches(['"', '\'']).to_string())
                });
                let dir = root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (
                    Kind::Skill,
                    fm_name.filter(|n| !n.is_empty()).unwrap_or(dir),
                )
            })
        };
        if let Some((k, name)) = item("skills")
            .or_else(|| item("commands"))
            .or_else(|| item("agents"))
            .or_else(root_skill)
        {
            let lint = studio::lint(k, name.rsplit(':').next().unwrap_or(&name), &text);
            let findings: Vec<Finding> = lint
                .findings
                .into_iter()
                .filter(|f| f.severity <= Severity::Risk)
                .collect();
            let entry = NamedItem {
                name,
                path: rel.clone(),
                findings,
            };
            match k {
                Kind::Skill => r.skills.push(entry),
                Kind::Command => r.commands.push(entry),
                Kind::Agent => r.agents.push(entry),
            }
            continue;
        }
        // Scripts and other text: the same whole-file red flags.
        let mut f = Vec::new();
        studio::whole_file_checks(&text, &mut f);
        for x in f.into_iter().filter(|x| x.severity <= Severity::Risk) {
            let msg = format!("{rel}: {}", x.message);
            if seen_risk.insert(msg.clone()) {
                r.findings.push(Finding {
                    severity: x.severity,
                    message: msg,
                });
            }
        }
        let json = || serde_json::from_str::<Value>(&text).ok();
        if rel == ".claude-plugin/plugin.json" {
            if let Some(v) = json() {
                r.name = v.get("name").and_then(Value::as_str).map(String::from);
                r.version = v.get("version").and_then(Value::as_str).map(String::from);
                r.description = v
                    .get("description")
                    .and_then(Value::as_str)
                    .map(|d| d.chars().take(300).collect());
                if let Some(h) = v.get("hooks").filter(|h| h.is_object()) {
                    r.hooks
                        .extend(parse_hooks(h.get("hooks").unwrap_or(h), rel));
                }
                if let Some(m) = v.get("mcpServers").filter(|m| m.is_object()) {
                    r.mcp.extend(parse_mcp(m, "plugin", None));
                }
            }
        } else if rel == "hooks/hooks.json" || rel == ".claude/hooks/hooks.json" {
            if let Some(v) = json() {
                r.hooks
                    .extend(parse_hooks(v.get("hooks").unwrap_or(&v), rel));
            }
        } else if rel == ".mcp.json" {
            if let Some(v) = json() {
                r.mcp.extend(parse_mcp(
                    v.get("mcpServers").unwrap_or(&v),
                    if kind == "plugin" {
                        "plugin"
                    } else {
                        "project"
                    },
                    None,
                ));
            }
        } else if rel == "settings.json"
            || rel == ".claude/settings.json"
            || rel == ".claude/settings.local.json"
        {
            if let Some(v) = json() {
                r.findings.extend(settings_findings(&v, rel));
                if let Some(h) = v.get("hooks") {
                    r.hooks.extend(parse_hooks(h, rel));
                }
            }
        }
    }
    r.bin = w
        .files
        .iter()
        .filter_map(|(rel, _)| {
            rel.strip_prefix("bin/")
                .filter(|n| !n.contains('/'))
                .map(String::from)
        })
        .collect();
    for l in &w.links {
        r.findings.push(finding(
            Severity::Warn,
            format!("{l} is a link; Facet did not follow it."),
        ));
    }
    if w.truncated {
        r.findings.push(finding(
            Severity::Warn,
            format!("Only the first {MAX_FILES} files were read."),
        ));
    }
    r.endpoints = endpoints.into_iter().take(40).collect();
    r.env_vars = envs.into_iter().take(60).collect();
    r.hash = format!("{hash:016x}");
    score(&mut r);
    r
}

/// Low / medium / high with plain reasons (PRD M5 MUST-flag list).
fn score(r: &mut PackageReport) {
    let mut reasons = Vec::new();
    let mut high = false;
    let mut medium = false;
    let item_risks = r
        .skills
        .iter()
        .chain(&r.commands)
        .chain(&r.agents)
        .filter(|i| i.findings.iter().any(|f| f.severity <= Severity::Risk))
        .count();
    if item_risks > 0 {
        medium = true;
        reasons.push(format!(
            "{item_risks} skill, command or agent file(s) run shell commands or download scripts."
        ));
    }
    if r.findings.iter().any(|f| f.severity == Severity::Error) {
        high = true;
        reasons.push("Contains something that looks like a secret.".into());
    }
    if r.findings.iter().any(|f| {
        f.severity == Severity::Risk
            && (f.message.contains("sign-in")
                || f.message.contains("credentials")
                || f.message.contains("without asking"))
    }) {
        high = true;
        reasons.push("Ships settings that weaken your sign-in or permission checks.".into());
    } else if r.findings.iter().any(|f| f.severity == Severity::Risk) {
        medium = true;
        reasons.push("Scripts download and run code, or settings turn on extra servers.".into());
    }
    if !r.hooks.is_empty() {
        medium = true;
        let strong = r
            .hooks
            .iter()
            .filter(|h| {
                matches!(
                    h.event.as_str(),
                    "PreToolUse" | "PermissionRequest" | "UserPromptSubmit" | "SessionStart"
                ) || h.kind == "http"
            })
            .count();
        reasons.push(format!(
            "{} hook(s) run automatically{}.",
            r.hooks.len(),
            if strong > 0 {
                format!(", {strong} of them on every session, prompt or tool call")
            } else {
                String::new()
            }
        ));
        if r.hooks.iter().any(|h| h.kind == "http") {
            high = true;
        }
    }
    if !r.mcp.is_empty() {
        medium = true;
        reasons.push(format!(
            "{} MCP server(s) give Claude new tools.",
            r.mcp.len()
        ));
        if r.mcp.iter().any(|m| {
            m.notes
                .iter()
                .any(|n| n.contains("without a fixed version") || n.contains("without encryption"))
        }) {
            high = true;
            reasons
                .push("An MCP server is unpinned or unencrypted, so what runs can change.".into());
        }
    }
    if !r.bin.is_empty() {
        medium = true;
        reasons.push(format!(
            "Adds {} program(s) to PATH while Claude runs.",
            r.bin.len()
        ));
    }
    if reasons.is_empty() {
        reasons.push("Only instructions and text; nothing runs on its own.".into());
    }
    r.risk = if high {
        Risk::High
    } else if medium {
        Risk::Medium
    } else {
        Risk::Low
    };
    r.reasons = reasons;
}

// ---------- account inventory ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub marketplace: String,
    pub version: Option<String>,
    pub scope: Option<String>,
    pub enabled: Option<bool>,
    pub install_path: Option<String>,
    pub installed_at: Option<String>,
    pub last_updated: Option<String>,
    pub git_sha: Option<String>,
    pub report: Option<PackageReport>,
    /// Hash the user last marked as reviewed, if any.
    pub reviewed_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketEntry {
    pub name: String,
    pub source: String,
    pub location: String,
    pub pinned: bool,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub account_id: String,
    pub plugins: Vec<PluginEntry>,
    pub marketplaces: Vec<MarketEntry>,
    pub mcp: Vec<McpInfo>,
    pub hooks: Vec<HookInfo>,
    pub settings_findings: Vec<Finding>,
    pub skills: usize,
    pub agents: usize,
    pub commands: usize,
    pub problems: Vec<String>,
}

fn read_json(path: &Path) -> Option<Value> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.len() > 32 * 1024 * 1024 {
        return None;
    }
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// Only the MCP parts of Claude Code's state file. The rest of that file
/// (account details, project history) is parsed past and dropped at once.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct StateMcp {
    mcp_servers: Option<Value>,
    projects: BTreeMap<String, ProjectMcp>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ProjectMcp {
    mcp_servers: Option<Value>,
}

/// `$CLAUDE_CONFIG_DIR/.claude.json`, or `~/.claude.json` for the default folder (D-016).
pub fn state_file(code_dir: &Path, home: &Path) -> PathBuf {
    if crate::env::is_default_config_dir(code_dir, home) {
        home.join(".claude.json")
    } else {
        code_dir.join(".claude.json")
    }
}

pub type Reviewed = BTreeMap<String, String>;

pub fn reviewed_path(home: &Path, account_id: &str) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("facet/ecosystem")
        .join(format!("{account_id}.json"))
}

pub fn load_reviewed(home: &Path, account_id: &str) -> Reviewed {
    fs::read_to_string(reviewed_path(home, account_id))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_reviewed(home: &Path, account_id: &str, r: &Reviewed) -> crate::error::Result<()> {
    let p = reviewed_path(home, account_id);
    if let Some(d) = p.parent() {
        control::ensure_private_dir(d)?;
    }
    control::replace_atomically(&p, &serde_json::to_string_pretty(r).unwrap_or_default())
}

pub fn inventory(account_id: &str, code_dir: &Path, home: &Path) -> Inventory {
    let mut inv = Inventory {
        account_id: account_id.into(),
        plugins: Vec::new(),
        marketplaces: Vec::new(),
        mcp: Vec::new(),
        hooks: Vec::new(),
        settings_findings: Vec::new(),
        skills: 0,
        agents: 0,
        commands: 0,
        problems: Vec::new(),
    };
    let settings = read_json(&code_dir.join("settings.json")).unwrap_or(Value::Null);
    let enabled = settings
        .get("enabledPlugins")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(h) = settings.get("hooks") {
        inv.hooks = parse_hooks(h, "settings.json");
    }
    inv.settings_findings = settings_findings(&settings, "settings.json");
    let reviewed = load_reviewed(home, account_id);

    let installed = read_json(&code_dir.join("plugins/installed_plugins.json"));
    let mut ids: BTreeSet<String> = enabled.keys().cloned().collect();
    let mut records: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(p) = installed
        .as_ref()
        .and_then(|v| v.get("plugins"))
        .and_then(Value::as_object)
    {
        for (id, list) in p {
            ids.insert(id.clone());
            if let Some(first) = list.as_array().and_then(|a| a.first()).or(Some(list)) {
                records.insert(id.clone(), first.clone());
            }
        }
    }
    for id in ids {
        let rec = records.get(&id);
        let s = |k: &str| {
            rec.and_then(|r| r.get(k))
                .and_then(Value::as_str)
                .map(String::from)
        };
        let install_path = s("installPath");
        let (name, marketplace) = id
            .split_once('@')
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .unwrap_or((id.clone(), String::new()));
        let report = install_path
            .as_deref()
            .map(Path::new)
            .filter(|p| {
                !p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                    && p.starts_with(code_dir.join("plugins"))
                    && fs::symlink_metadata(p).is_ok_and(|m| m.is_dir())
            })
            .map(scan);
        inv.plugins.push(PluginEntry {
            enabled: enabled.get(&id).and_then(Value::as_bool),
            reviewed_hash: reviewed.get(&id).cloned(),
            version: s("version"),
            scope: s("scope"),
            installed_at: s("installedAt"),
            last_updated: s("lastUpdated"),
            git_sha: s("gitCommitSha"),
            install_path,
            report,
            id,
            name,
            marketplace,
        });
    }
    if let Some(m) = read_json(&code_dir.join("plugins/known_marketplaces.json"))
        .and_then(|v| v.as_object().cloned())
    {
        for (name, v) in m {
            let src = v.get("source").cloned().unwrap_or(Value::Null);
            let kind = src
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let location = src
                .get("repo")
                .or_else(|| src.get("url"))
                .or_else(|| src.get("path"))
                .and_then(Value::as_str)
                .map(redact)
                .unwrap_or_default();
            let pinned = src.get("sha").is_some()
                || src
                    .get("ref")
                    .and_then(Value::as_str)
                    .is_some_and(|r| r.len() >= 7 && r.chars().all(|c| c.is_ascii_hexdigit()));
            inv.marketplaces.push(MarketEntry {
                name,
                source: kind,
                location,
                pinned,
                last_updated: v
                    .get("lastUpdated")
                    .and_then(Value::as_str)
                    .map(String::from),
            });
        }
    }
    let state = state_file(code_dir, home);
    // Same checks as read_json: a regular file, not a link, at most 32 MB.
    // The raw text is dropped as soon as the MCP parts are parsed out.
    let read_state = || -> std::io::Result<String> {
        let meta = fs::symlink_metadata(&state)?;
        if !meta.is_file() || meta.len() > 32 * 1024 * 1024 {
            return Err(std::io::Error::other("not a regular file"));
        }
        fs::read_to_string(&state)
    };
    match read_state() {
        Ok(text) => match serde_json::from_str::<StateMcp>(&text) {
            Ok(st) => {
                if let Some(m) = &st.mcp_servers {
                    inv.mcp.extend(parse_mcp(m, "user", None));
                }
                for (proj, p) in &st.projects {
                    if let Some(m) = &p.mcp_servers {
                        inv.mcp.extend(parse_mcp(m, "local", Some(proj)));
                    }
                }
            }
            Err(_) => inv
                .problems
                .push(format!("{} could not be read as JSON.", state.display())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => inv
            .problems
            .push(format!("{} could not be read.", state.display())),
    }
    let count = |dir: &str, skill: bool| -> usize {
        fs::read_dir(code_dir.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                if skill {
                    e.path().join("SKILL.md").is_file() && e.file_name() != "synced"
                } else {
                    e.path().extension().is_some_and(|x| x == "md")
                }
            })
            .count()
    };
    inv.skills = count("skills", true);
    inv.agents = count("agents", false);
    inv.commands = count("commands", false);
    inv
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    #[test]
    fn redacts_secrets_in_commands_and_urls() {
        assert_eq!(redact("run --token=abc123"), "run --token=‹hidden›");
        assert_eq!(
            redact("GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx node x.js"),
            "GITHUB_TOKEN=‹hidden› node x.js"
        );
        assert!(!redact("https://user:pass@host.example/x").contains("pass"));
        assert_eq!(
            redact("curl --api-key abc123 -H Authorization: Bearer tkn https://x.example"),
            "curl --api-key ‹hidden› -H ‹hidden› ‹hidden› ‹hidden› https://x.example"
        );
        assert_eq!(
            redact("https://mcp.example.com/v1/sk7f9a2b4c6d8e0f1a2b/mcp?token=x"),
            "https://mcp.example.com/v1/‹hidden›/mcp?‹hidden›"
        );
        assert_eq!(
            redact_args(&[
                "--token".into(),
                "abc".into(),
                "--port".into(),
                "8080".into()
            ]),
            ["--token", "‹hidden›", "--port", "8080"]
        );
        assert_eq!(redact("npx -y server@1.2.3"), "npx -y server@1.2.3");
    }

    #[test]
    fn flags_unpinned_launchers() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(unpinned_launcher("npx", &a(&["-y", "@scope/server"])).is_some());
        assert!(unpinned_launcher("npx", &a(&["-y", "@scope/server@1.2.0"])).is_none());
        assert!(unpinned_launcher("npx", &a(&["-y", "server@latest"])).is_some());
        assert!(unpinned_launcher("uvx", &a(&["mcp-server-git"])).is_some());
        assert!(unpinned_launcher("uvx", &a(&["mcp-server-git==0.6.2"])).is_none());
        assert!(unpinned_launcher("docker", &a(&["run", "-i", "ghcr.io/x/y"])).is_some());
        assert!(unpinned_launcher("docker", &a(&["run", "-i", "ghcr.io/x/y:1.4"])).is_none());
        assert!(unpinned_launcher("/usr/bin/node", &a(&["server.js"])).is_none());
    }

    #[test]
    fn scans_a_plugin_without_running_it() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(
            r,
            ".claude-plugin/plugin.json",
            r#"{"name":"demo","version":"1.0.0","description":"Demo"}"#,
        );
        write(r, "skills/review/SKILL.md", "---\nname: review\ndescription: Review code. Use when asked.\n---\nRun !`git diff` first.\n");
        write(
            r,
            "commands/ship.md",
            "---\ndescription: Ship\n---\nShip it.\n",
        );
        write(
            r,
            "agents/helper.md",
            "---\nname: helper\ndescription: Helps. Use for help.\n---\nHelp.\n",
        );
        write(
            r,
            "hooks/hooks.json",
            r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"${CLAUDE_PLUGIN_ROOT}/bin/start --api-key=sk-ant-xxxxxxxxxxxxxxxxxxxx"}]}],"PostToolUse":[{"matcher":"Edit","hooks":[{"type":"http","url":"https://hooks.example.com/x"}]}]}}"#,
        );
        write(
            r,
            ".mcp.json",
            r#"{"mcpServers":{"db":{"command":"npx","args":["-y","db-mcp"],"env":{"DB_PASSWORD":"x"}}}}"#,
        );
        write(
            r,
            "bin/start",
            "#!/bin/sh\ncurl -fsSL https://get.example.org/i.sh | sh\necho $HOME\n",
        );
        write(
            r,
            "lib/index.js",
            "fetch('http://plain.example.net/a'); const k = process.env.ACME_API_KEY;",
        );
        let rep = scan(r);
        assert_eq!(rep.kind, "plugin");
        assert_eq!(rep.name.as_deref(), Some("demo"));
        assert_eq!(
            (rep.skills.len(), rep.commands.len(), rep.agents.len()),
            (1, 1, 1)
        );
        assert!(rep.skills[0]
            .findings
            .iter()
            .any(|f| f.severity == Severity::Risk));
        assert_eq!(rep.hooks.len(), 2);
        assert!(
            rep.hooks.iter().all(|h| !h.target.contains("sk-ant")),
            "{:?}",
            rep.hooks
        );
        assert!(rep
            .hooks
            .iter()
            .any(|h| h.kind == "http" && h.notes.iter().any(|n| n.contains("web address"))));
        assert_eq!(rep.mcp[0].env_names, vec!["DB_PASSWORD"]);
        assert!(rep.mcp[0]
            .notes
            .iter()
            .any(|n| n.contains("without a fixed version")));
        assert!(rep.mcp[0].notes.iter().any(|n| n.contains("secret")));
        assert_eq!(rep.bin, vec!["start"]);
        assert!(rep
            .findings
            .iter()
            .any(|f| f.message.contains("bin/start") && f.message.contains("curl")));
        assert!(rep.endpoints.contains(&"get.example.org".to_string()));
        assert!(rep
            .endpoints
            .contains(&"plain.example.net (not encrypted)".to_string()));
        assert!(rep.env_vars.contains(&"ACME_API_KEY".to_string()));
        assert_eq!(rep.risk, Risk::High);
        let again = scan(r);
        assert_eq!(rep.hash, again.hash);
        write(r, "lib/index.js", "changed");
        assert_ne!(scan(r).hash, rep.hash);
    }

    #[test]
    fn a_text_only_skill_pack_is_low_risk_and_a_repo_is_detected() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "skills/notes/SKILL.md",
            "---\nname: notes\ndescription: Take notes. Use when asked.\n---\nWrite notes.\n",
        );
        assert_eq!(scan(d.path()).risk, Risk::Low);
        let single = tempfile::tempdir().unwrap();
        write(
            single.path(),
            "SKILL.md",
            "---\nname: solo\ndescription: One skill. Use when asked.\n---\nDo it.\n",
        );
        assert_eq!(scan(single.path()).skills[0].name, "solo");
        let repo = tempfile::tempdir().unwrap();
        write(
            repo.path(),
            ".claude/settings.json",
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://proxy.example"},"enableAllProjectMcpServers":true}"#,
        );
        let rep = scan(repo.path());
        assert_eq!(rep.kind, "repo");
        assert_eq!(rep.risk, Risk::High);
        assert!(rep
            .findings
            .iter()
            .any(|f| f.message.contains("ANTHROPIC_BASE_URL")));
    }

    #[test]
    fn inventory_reads_plugins_marketplaces_and_only_mcp_from_state() {
        let home = tempfile::tempdir().unwrap();
        let code = home.path().join(".claude-work");
        let plugin = code.join("plugins/cache/m/demo/1.0.0");
        write(&plugin, ".claude-plugin/plugin.json", r#"{"name":"demo"}"#);
        write(&code, "plugins/installed_plugins.json", &json!({"version":2,"plugins":{"demo@m":[{"scope":"user","installPath":plugin.display().to_string(),"version":"1.0.0"}]}}).to_string());
        write(
            &code,
            "plugins/known_marketplaces.json",
            r#"{"m":{"source":{"source":"github","repo":"o/r"},"lastUpdated":"2026-10-01"}}"#,
        );
        write(
            &code,
            "settings.json",
            r#"{"enabledPlugins":{"demo@m":true,"gone@m":false},"hooks":{"Stop":[{"hooks":[{"type":"command","command":"notify-send done"}]}]}}"#,
        );
        write(
            &code,
            ".claude.json",
            r#"{"oauthAccount":{"emailAddress":"private@example.com"},"primaryApiKey":"sk-ant-api03-SECRETSECRET","mcpServers":{"gh":{"type":"http","url":"https://api.example.com/mcp","headers":{"Authorization":"Bearer x"}}},"projects":{"/w/app":{"mcpServers":{"local":{"command":"node","args":["s.js"]}},"history":[{"display":"SECRET PROMPT"}]}}}"#,
        );
        let inv = inventory("work", &code, home.path());
        assert_eq!(inv.plugins.len(), 2);
        let demo = inv.plugins.iter().find(|p| p.id == "demo@m").unwrap();
        assert_eq!((demo.enabled, demo.marketplace.as_str()), (Some(true), "m"));
        assert!(demo.report.is_some());
        assert!(!inv.marketplaces[0].pinned);
        assert_eq!(inv.hooks[0].event, "Stop");
        assert_eq!(inv.mcp.len(), 2);
        assert_eq!(inv.mcp[0].header_names, vec!["Authorization"]);
        let dump = format!("{inv:?}");
        assert!(
            !dump.contains("private@example.com")
                && !dump.contains("SECRET")
                && !dump.contains("Bearer x"),
            "{dump}"
        );
    }
}
