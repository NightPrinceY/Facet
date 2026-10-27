//! Usage and cost from Claude Code transcripts (PRD M3 Costs, M6 Sessions,
//! Today). Read-only. Reads only metadata: timestamps, ids, model, token
//! counts, tool names, folder and branch. Message text is skipped by the
//! parser and never stored (rule 9).
//!
//! Cost is the API-equivalent price from `PRICES` (official pricing page,
//! checked 2026-10-26, `docs/01-research/notes/model_pricing.md`). Checked
//! against Claude Code's own `cost-state` lines on this machine: 99.5% over
//! 42 sessions.

use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

pub const PRICES_CHECKED: &str = "2026-10-26";
pub const PRICES_SOURCE: &str = "https://platform.claude.com/docs/en/about-claude/pricing";

/// USD per million tokens. Most specific prefix first.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub prefix: &'static str,
    pub input: f64,
    pub output: f64,
    pub cache_write_5m: f64,
    pub cache_write_1h: f64,
    pub cache_read: f64,
}

const fn p(prefix: &'static str, input: f64, output: f64, w5: f64, w1: f64, read: f64) -> Price {
    Price {
        prefix,
        input,
        output,
        cache_write_5m: w5,
        cache_write_1h: w1,
        cache_read: read,
    }
}

pub const PRICES: &[Price] = &[
    p("claude-fable-5-1", 10.0, 50.0, 12.5, 20.0, 0.25),
    p("claude-fable-5", 10.0, 50.0, 12.5, 20.0, 1.0),
    p("claude-mythos-5-1", 10.0, 50.0, 12.5, 20.0, 0.25),
    p("claude-mythos-5", 10.0, 50.0, 12.5, 20.0, 1.0),
    p("claude-opus-5-5", 4.0, 20.0, 5.0, 8.0, 0.2),
    p("claude-opus-5", 5.0, 25.0, 6.25, 10.0, 0.5),
    p("claude-opus-4-8", 5.0, 25.0, 6.25, 10.0, 0.5),
    p("claude-opus-4-7", 5.0, 25.0, 6.25, 10.0, 0.5),
    p("claude-opus-4-6", 5.0, 25.0, 6.25, 10.0, 0.5),
    p("claude-opus-4-5", 5.0, 25.0, 6.25, 10.0, 0.5),
    p("claude-opus-4-1", 15.0, 75.0, 18.75, 30.0, 1.5),
    p("claude-opus-4", 15.0, 75.0, 18.75, 30.0, 1.5),
    p("claude-sonnet-5-5", 2.0, 10.0, 2.5, 4.0, 0.1),
    p("claude-sonnet-5", 2.0, 10.0, 2.5, 4.0, 0.2),
    p("claude-sonnet-4-6", 3.0, 15.0, 3.75, 6.0, 0.3),
    p("claude-sonnet-4-5", 3.0, 15.0, 3.75, 6.0, 0.3),
    p("claude-sonnet-4", 3.0, 15.0, 3.75, 6.0, 0.3),
    p("claude-haiku-5-5", 0.1, 0.5, 0.125, 0.2, 0.01),
    p("claude-haiku-4-5", 1.0, 5.0, 1.25, 2.0, 0.1),
    p("claude-3-5-haiku", 0.8, 4.0, 1.0, 1.6, 0.08),
];
pub const WEB_SEARCH_PER_1K: f64 = 10.0;

pub fn price_for(model: &str) -> Option<&'static Price> {
    PRICES.iter().find(|p| model.starts_with(p.prefix))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub web_searches: u64,
}

impl Tokens {
    fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.web_searches += o.web_searches;
    }
}

/// One model reply's usage, as written in the transcript.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
    pub web_searches: u64,
    pub fast: bool,
}

impl Usage {
    pub fn tokens(&self) -> Tokens {
        Tokens {
            input: self.input,
            output: self.output,
            cache_read: self.cache_read,
            cache_write: self.cache_write_5m + self.cache_write_1h,
            web_searches: self.web_searches,
        }
    }
}

/// API-equivalent USD for one reply, or `None` for an unknown model.
pub fn cost(model: &str, u: &Usage) -> Option<f64> {
    let p = price_for(model)?;
    let prompt = u.input + u.cache_read + u.cache_write_5m + u.cache_write_1h;
    let mut mult = 1.0;
    // Haiku 5.5: prompts over 100K tokens pay 5x on every rate.
    if p.prefix == "claude-haiku-5-5" && prompt > 100_000 {
        mult *= 5.0;
    }
    // Fast mode (Opus only) costs twice the standard rates.
    if u.fast {
        mult *= 2.0;
    }
    let tokens = u.input as f64 * p.input
        + u.output as f64 * p.output
        + u.cache_write_5m as f64 * p.cache_write_5m
        + u.cache_write_1h as f64 * p.cache_write_1h
        + u.cache_read as f64 * p.cache_read;
    Some(tokens / 1e6 * mult + u.web_searches as f64 * WEB_SEARCH_PER_1K / 1000.0)
}

// ---------- tolerant line parser (only the fields Facet uses) ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Line {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    version: Option<String>,
    entrypoint: Option<String>,
    request_id: Option<String>,
    is_meta: Option<bool>,
    message: Option<Msg>,
    #[serde(rename = "totalCostUSD")]
    total_cost_usd: Option<f64>,
    start_time: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Msg {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawUsage>,
    content: Content,
}

/// Content blocks keep only their type and tool name; text is skipped.
#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Blocks(Vec<Block>),
    Other(IgnoredAny),
}

impl Default for Content {
    fn default() -> Self {
        Content::Other(IgnoredAny)
    }
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    name: Option<String>,
    /// tool_use id
    #[serde(default)]
    id: Option<String>,
    /// tool_result → the tool_use it answers
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    is_error: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
    cache_creation: Option<RawCacheCreation>,
    server_tool_use: Option<RawServerTools>,
    speed: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawCacheCreation {
    ephemeral_5m_input_tokens: Option<u64>,
    ephemeral_1h_input_tokens: Option<u64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawServerTools {
    web_search_requests: u64,
}

impl RawUsage {
    fn usage(&self) -> Usage {
        let (w5, w1) = match &self.cache_creation {
            Some(c)
                if c.ephemeral_5m_input_tokens.is_some()
                    || c.ephemeral_1h_input_tokens.is_some() =>
            {
                (
                    c.ephemeral_5m_input_tokens.unwrap_or(0),
                    c.ephemeral_1h_input_tokens.unwrap_or(0),
                )
            }
            _ => (self.cache_creation_input_tokens, 0),
        };
        Usage {
            input: self.input_tokens,
            output: self.output_tokens,
            cache_read: self.cache_read_input_tokens,
            cache_write_5m: w5,
            cache_write_1h: w1,
            web_searches: self
                .server_tool_use
                .as_ref()
                .map_or(0, |s| s.web_search_requests),
            fast: self.speed.as_deref() == Some("fast"),
        }
    }
}

/// `2026-10-26T08:12:33.123Z` → milliseconds since 1970 (UTC). Offsets
/// other than `Z` are honoured.
pub fn parse_ts(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        std::str::from_utf8(&b[r]).ok()?.parse().ok()
    };
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut i = 19;
    let mut ms = 0i64;
    if b.get(i) == Some(&b'.') {
        let start = i + 1;
        i = start;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let frac = std::str::from_utf8(&b[start..i]).ok()?;
        let padded: String = frac.chars().chain("000".chars()).take(3).collect();
        ms = padded.parse().ok()?;
    }
    let offset_min = match b.get(i) {
        Some(b'Z') | None => 0,
        Some(sign @ (b'+' | b'-')) => {
            let oh = num(i + 1..i + 3)?;
            let om = num(i + 4..i + 6).or_else(|| num(i + 3..i + 5)).unwrap_or(0);
            let v = oh * 60 + om;
            if *sign == b'+' {
                v
            } else {
                -v
            }
        }
        _ => return None,
    };
    // Days from civil (Howard Hinnant).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(((days * 24 + h) * 60 + mi - offset_min) * 60_000 + se * 1000 + ms)
}

// ---------- per-session digest ----------

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub account_id: String,
    pub session_id: String,
    /// Folder Claude Code ran in (`cwd` of the first line).
    pub project: Option<String>,
    pub branch: Option<String>,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub models: Vec<String>,
    pub entrypoint: Option<String>,
    pub version: Option<String>,
    /// Messages the user typed (not tool results, not system notes).
    pub prompts: u32,
    /// Model replies (deduplicated by message id).
    pub turns: u32,
    pub tools: BTreeMap<String, u32>,
    pub tokens: Tokens,
    pub cost: f64,
    /// Models with no known price; their tokens are counted but not priced.
    pub unpriced_models: Vec<String>,
    /// Claude Code's own figure from `cost-state` lines, when present.
    pub reported_cost: Option<f64>,
    pub subagents: u32,
    pub bytes: u64,
}

/// One hour of one account and model.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourBucket {
    /// Start of the hour, ms since 1970 UTC.
    pub hour_ms: i64,
    pub model: String,
    pub project: Option<String>,
    pub tokens: Tokens,
    pub cost: f64,
    pub turns: u32,
    pub prompts: u32,
}

/// One thing that happened, for the Timeline (TIMELINE-SPEC.md). Metadata only.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub account_id: String,
    pub session_id: String,
    pub ts: i64,
    /// prompt | call | tool
    pub kind: &'static str,
    /// Tool runs: when the result came back.
    pub end_ts: Option<i64>,
    pub model: Option<String>,
    pub cost: f64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub tool: Option<String>,
    /// Tool runs: false when the tool reported an error.
    pub ok: Option<bool>,
    /// Happened inside a subagent.
    pub subagent: bool,
}

#[derive(Debug, Clone, Default)]
struct Digest {
    summary: SessionSummary,
    hours: Vec<HourBucket>,
    events: Vec<TimelineEvent>,
}

struct Reply {
    model: String,
    usage: Usage,
    ts: i64,
    /// (tool_use id, tool name)
    tools: Vec<(Option<String>, String)>,
    subagent: bool,
}

/// Read one session (main transcript plus its subagent transcripts).
fn digest_session(account_id: &str, main: &Path, subagents: &[PathBuf]) -> Digest {
    let mut s = SessionSummary {
        account_id: account_id.into(),
        session_id: main
            .file_stem()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default(),
        started_ms: i64::MAX,
        ended_ms: 0,
        subagents: subagents.len() as u32,
        ..Default::default()
    };
    let mut replies: HashMap<(String, String), Reply> = HashMap::new();
    // tool_use id → (result time, succeeded)
    let mut results: HashMap<String, (i64, bool)> = HashMap::new();
    let mut prompt_times: Vec<i64> = Vec::new();
    let mut reported: BTreeMap<i64, f64> = BTreeMap::new();
    let files = std::iter::once(main.to_path_buf()).chain(subagents.iter().cloned());
    for (fi, file) in files.enumerate() {
        let Ok(f) = fs::File::open(&file) else {
            continue;
        };
        s.bytes += f.metadata().map(|m| m.len()).unwrap_or(0);
        let mut reader = BufReader::new(f);
        let mut buf = String::new();
        let mut n = 0usize;
        loop {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            n += 1;
            let Ok(line) = serde_json::from_str::<Line>(&buf) else {
                continue;
            };
            let ts = line.timestamp.as_deref().and_then(parse_ts);
            if let Some(t) = ts {
                s.started_ms = s.started_ms.min(t);
                s.ended_ms = s.ended_ms.max(t);
            }
            if fi == 0 {
                if s.project.is_none() {
                    s.project = line.cwd.clone();
                }
                if line.git_branch.as_deref().is_some_and(|b| !b.is_empty()) {
                    s.branch = line.git_branch.clone();
                }
                if line.entrypoint.is_some() && s.entrypoint.is_none() {
                    s.entrypoint = line.entrypoint.clone();
                }
                if line.version.is_some() {
                    s.version = line.version.clone();
                }
                if line.session_id.is_some() && s.session_id.is_empty() {
                    s.session_id = line.session_id.clone().unwrap_or_default();
                }
            }
            match line.kind.as_deref() {
                Some("cost-state") => {
                    if let (Some(c), st) = (line.total_cost_usd, line.start_time) {
                        reported.insert(st.unwrap_or(n as i64), c);
                    }
                }
                Some("assistant") => {
                    let Some(m) = line.message else { continue };
                    let (Some(model), Some(u)) = (m.model, m.usage) else {
                        continue;
                    };
                    if model.starts_with('<') {
                        continue; // "<synthetic>" placeholder replies carry no cost
                    }
                    let key = (
                        m.id.unwrap_or_else(|| format!("line-{fi}-{n}")),
                        line.request_id.unwrap_or_default(),
                    );
                    let usage = u.usage();
                    let tools: Vec<(Option<String>, String)> = match m.content {
                        Content::Blocks(bl) => bl
                            .into_iter()
                            .filter(|b| b.kind == "tool_use")
                            .filter_map(|b| b.name.map(|n| (b.id, n)))
                            .collect(),
                        Content::Other(_) => Vec::new(),
                    };
                    // The same reply is written once per content block; keep the
                    // fullest usage and every tool it called.
                    let entry = replies.entry(key).or_insert(Reply {
                        model: model.clone(),
                        usage,
                        ts: ts.unwrap_or(0),
                        tools: Vec::new(),
                        subagent: fi > 0,
                    });
                    if usage.output >= entry.usage.output {
                        entry.usage = usage;
                    }
                    for t in tools {
                        if !entry.tools.contains(&t) {
                            entry.tools.push(t);
                        }
                    }
                }
                Some("user") => {
                    let content = line.message.map(|m| m.content);
                    if let (Some(Content::Blocks(bl)), Some(t)) = (&content, ts) {
                        for b in bl.iter().filter(|b| b.kind == "tool_result") {
                            if let Some(id) = &b.tool_use_id {
                                results.insert(id.clone(), (t, b.is_error != Some(true)));
                            }
                        }
                    }
                    if fi != 0 || line.is_meta == Some(true) {
                        continue;
                    }
                    let is_prompt = match content {
                        Some(Content::Blocks(bl)) => {
                            bl.iter().any(|b| b.kind == "text")
                                && !bl.iter().any(|b| b.kind == "tool_result")
                        }
                        Some(Content::Other(_)) => true,
                        None => false,
                    };
                    if is_prompt {
                        s.prompts += 1;
                        if let Some(t) = ts {
                            prompt_times.push(t);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if s.started_ms == i64::MAX {
        s.started_ms = 0;
    }
    let mut models = BTreeSet::new();
    let mut unpriced = BTreeSet::new();
    let mut hours: BTreeMap<(i64, String), HourBucket> = BTreeMap::new();
    let mut events: Vec<TimelineEvent> = Vec::new();
    for r in replies.values() {
        s.turns += 1;
        models.insert(r.model.clone());
        for (_, t) in &r.tools {
            *s.tools.entry(t.clone()).or_default() += 1;
        }
        let tk = r.usage.tokens();
        s.tokens.add(&tk);
        let c = match cost(&r.model, &r.usage) {
            Some(c) => c,
            None => {
                unpriced.insert(r.model.clone());
                0.0
            }
        };
        s.cost += c;
        let base = TimelineEvent {
            account_id: account_id.into(),
            session_id: s.session_id.clone(),
            ts: r.ts,
            kind: "call",
            end_ts: None,
            model: Some(r.model.clone()),
            cost: c,
            output_tokens: r.usage.output,
            total_tokens: tk.input + tk.output + tk.cache_read + tk.cache_write,
            tool: None,
            ok: None,
            subagent: r.subagent,
        };
        for (id, name) in &r.tools {
            let res = id.as_ref().and_then(|i| results.get(i));
            events.push(TimelineEvent {
                kind: "tool",
                end_ts: res.map(|(t, _)| *t),
                ok: res.map(|(_, ok)| *ok),
                tool: Some(name.clone()),
                model: None,
                cost: 0.0,
                output_tokens: 0,
                total_tokens: 0,
                ..base.clone()
            });
        }
        events.push(base);
        let hour = r.ts - r.ts.rem_euclid(3_600_000);
        let b = hours
            .entry((hour, r.model.clone()))
            .or_insert_with(|| HourBucket {
                hour_ms: hour,
                model: r.model.clone(),
                project: s.project.clone(),
                ..Default::default()
            });
        b.tokens.add(&tk);
        b.cost += c;
        b.turns += 1;
    }
    for &t in &prompt_times {
        events.push(TimelineEvent {
            account_id: account_id.into(),
            session_id: s.session_id.clone(),
            ts: t,
            kind: "prompt",
            end_ts: None,
            model: None,
            cost: 0.0,
            output_tokens: 0,
            total_tokens: 0,
            tool: None,
            ok: None,
            subagent: false,
        });
        let hour = t - t.rem_euclid(3_600_000);
        if let Some(b) = hours
            .range_mut((hour, String::new())..)
            .next()
            .filter(|(k, _)| k.0 == hour)
        {
            b.1.prompts += 1;
        } else {
            hours.insert(
                (hour, String::new()),
                HourBucket {
                    hour_ms: hour,
                    project: s.project.clone(),
                    prompts: 1,
                    ..Default::default()
                },
            );
        }
    }
    s.models = models.into_iter().collect();
    s.unpriced_models = unpriced.into_iter().collect();
    if !reported.is_empty() {
        s.reported_cost = Some(reported.values().sum());
    }
    events.sort_by(|a, b| a.ts.cmp(&b.ts).then(a.kind.cmp(b.kind)));
    Digest {
        summary: s,
        hours: hours.into_values().collect(),
        events,
    }
}

/// Main transcripts (`projects/<folder>/<session>.jsonl`) with their
/// subagent transcripts (`projects/<folder>/<session>/subagents/*.jsonl`).
pub fn find_sessions(code_dir: &Path) -> Vec<(PathBuf, Vec<PathBuf>)> {
    let mut out = Vec::new();
    let Ok(projects) = fs::read_dir(code_dir.join("projects")) else {
        return out;
    };
    for proj in projects.flatten() {
        let Ok(ft) = proj.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(proj.path()) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "jsonl")
                && e.file_type().is_ok_and(|t| t.is_file())
            {
                let mut subs: Vec<PathBuf> =
                    fs::read_dir(path.with_extension("").join("subagents"))
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
                        .collect();
                subs.sort();
                out.push((path, subs));
            }
        }
    }
    out
}

/// Size and modification time of each file of a session.
type Signature = Vec<(u64, Option<SystemTime>)>;

fn signature(main: &Path, subs: &[PathBuf]) -> Signature {
    std::iter::once(main)
        .chain(subs.iter().map(|p| p.as_path()))
        .map(|p| {
            fs::metadata(p)
                .map(|m| (m.len(), m.modified().ok()))
                .unwrap_or((0, None))
        })
        .collect()
}

/// Remembers digests by file size and time, so a refresh reads only
/// transcripts that changed.
#[derive(Default)]
pub struct Cache {
    inner: Mutex<HashMap<PathBuf, (Signature, Digest)>>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsage {
    pub account_id: String,
    pub sessions: Vec<SessionSummary>,
    pub hours: Vec<HourBucket>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    pub generated_ms: i64,
    pub since_ms: i64,
    pub accounts: Vec<AccountUsage>,
    pub prices_checked: &'static str,
    pub prices_source: &'static str,
    pub prices: &'static [Price],
}

impl Cache {
    /// Digests of sessions touched since `since_ms`, per account, refreshed
    /// only where files changed.
    fn digests(&self, accounts: &[(String, PathBuf)], since_ms: i64) -> Vec<(String, Vec<Digest>)> {
        let mut cache = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for (id, dir) in accounts {
            let mut list = Vec::new();
            for (main, subs) in find_sessions(dir) {
                seen.insert(main.clone());
                // Skip files untouched since the range began.
                let newest = std::iter::once(&main)
                    .chain(subs.iter())
                    .filter_map(|p| fs::metadata(p).ok()?.modified().ok())
                    .max();
                if let Some(t) = newest.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                {
                    if (t.as_millis() as i64) < since_ms {
                        continue;
                    }
                }
                let sig = signature(&main, &subs);
                let digest = match cache.get(&main) {
                    Some((s, d)) if *s == sig && d.summary.account_id == *id => d.clone(),
                    _ => {
                        let d = digest_session(id, &main, &subs);
                        cache.insert(main.clone(), (sig, d.clone()));
                        d
                    }
                };
                if digest.summary.ended_ms < since_ms
                    || digest.summary.turns + digest.summary.prompts == 0
                {
                    continue;
                }
                list.push(digest);
            }
            out.push((id.clone(), list));
        }
        cache.retain(|k, _| seen.contains(k));
        out
    }

    /// Sessions active since `since_ms` for each `(account id, code dir)`.
    pub fn report(
        &self,
        accounts: &[(String, PathBuf)],
        since_ms: i64,
        now_ms: i64,
    ) -> UsageReport {
        let out = self
            .digests(accounts, since_ms)
            .into_iter()
            .map(|(id, list)| {
                let mut acc = AccountUsage {
                    account_id: id,
                    ..Default::default()
                };
                for d in list {
                    acc.hours.extend(
                        d.hours
                            .into_iter()
                            .filter(|h| h.hour_ms + 3_600_000 > since_ms),
                    );
                    acc.sessions.push(d.summary);
                }
                acc.sessions.sort_by_key(|s| std::cmp::Reverse(s.ended_ms));
                acc.hours.sort_by_key(|h| h.hour_ms);
                acc
            })
            .collect();
        UsageReport {
            generated_ms: now_ms,
            since_ms,
            accounts: out,
            prices_checked: PRICES_CHECKED,
            prices_source: PRICES_SOURCE,
            prices: PRICES,
        }
    }

    /// Events between `since_ms` and `until_ms`, oldest first, at most
    /// `limit` (the newest are kept when there are more).
    pub fn events(
        &self,
        accounts: &[(String, PathBuf)],
        since_ms: i64,
        until_ms: i64,
        limit: usize,
    ) -> (Vec<TimelineEvent>, bool) {
        let mut all: Vec<TimelineEvent> = self
            .digests(accounts, since_ms)
            .into_iter()
            .flat_map(|(_, list)| list.into_iter().flat_map(|d| d.events))
            .filter(|e| {
                e.ts >= since_ms && e.ts <= until_ms
                    || e.end_ts.is_some_and(|t| t >= since_ms && e.ts <= until_ms)
            })
            .collect();
        all.sort_by_key(|e| e.ts);
        let truncated = all.len() > limit;
        if truncated {
            all.drain(..all.len() - limit);
        }
        (all, truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn timestamps_parse_to_utc_ms() {
        assert_eq!(parse_ts("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_ts("1970-01-01T00:00:01.5Z"), Some(1500));
        assert_eq!(
            parse_ts("2026-10-26T08:00:00.000Z"),
            Some(1_793_001_600_000)
        );
        assert_eq!(
            parse_ts("2026-10-26T10:00:00+02:00"),
            parse_ts("2026-10-26T08:00:00Z")
        );
        assert_eq!(parse_ts("not a date"), None);
    }

    #[test]
    fn prices_pick_the_most_specific_model() {
        assert_eq!(price_for("claude-opus-5-5-20261001").unwrap().input, 4.0);
        assert_eq!(price_for("claude-opus-5").unwrap().input, 5.0);
        assert_eq!(price_for("claude-opus-4-1-20250805").unwrap().input, 15.0);
        assert!(price_for("gpt-5").is_none());
        let u = Usage {
            input: 1_000_000,
            output: 1_000_000,
            ..Default::default()
        };
        assert!((cost("claude-opus-5-5", &u).unwrap() - 24.0).abs() < 1e-9);
        let fast = Usage { fast: true, ..u };
        assert!((cost("claude-opus-5-5", &fast).unwrap() - 48.0).abs() < 1e-9);
        let big = Usage {
            input: 150_000,
            ..Default::default()
        };
        assert!((cost("claude-haiku-5-5", &big).unwrap() - 0.075).abs() < 1e-9);
        let search = Usage {
            web_searches: 3,
            ..Default::default()
        };
        assert!((cost("claude-sonnet-5-5", &search).unwrap() - 0.03).abs() < 1e-9);
    }

    fn write(path: &Path, lines: &[&str]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
    }

    #[test]
    fn digests_a_session_without_keeping_text() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("projects/-home-you-app/s1.jsonl");
        let usage = r#""usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":100,"cache_creation_input_tokens":20,"cache_creation":{"ephemeral_5m_input_tokens":20,"ephemeral_1h_input_tokens":0}}"#;
        write(
            &main,
            &[
                r#"{"type":"user","timestamp":"2026-10-26T08:00:00Z","sessionId":"s1","cwd":"/home/you/app","gitBranch":"main","entrypoint":"cli","version":"2.1.310","message":{"role":"user","content":"SECRET PROMPT TEXT"}}"#,
                &format!(
                    r#"{{"type":"assistant","timestamp":"2026-10-26T08:00:02Z","requestId":"r1","message":{{"id":"m1","model":"claude-opus-5-5","content":[{{"type":"thinking","thinking":"SECRET"}}],{usage}}}}}"#
                ),
                &format!(
                    r#"{{"type":"assistant","timestamp":"2026-10-26T08:00:03Z","requestId":"r1","message":{{"id":"m1","model":"claude-opus-5-5","content":[{{"type":"tool_use","name":"Bash","input":{{"command":"SECRET"}}}}],{usage}}}}}"#
                ),
                r#"{"type":"user","timestamp":"2026-10-26T08:00:04Z","message":{"role":"user","content":[{"type":"tool_result","content":"SECRET OUTPUT"}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-10-26T09:10:00Z","requestId":"r2","message":{"id":"m2","model":"<synthetic>","content":[],"usage":{"input_tokens":0,"output_tokens":0}}}"#,
                r#"{"type":"cost-state","sessionId":"s1","totalCostUSD":0.01,"startTime":1}"#,
                "not json at all",
            ],
        );
        let sub = dir
            .path()
            .join("projects/-home-you-app/s1/subagents/agent-a.jsonl");
        write(
            &sub,
            &[
                r#"{"type":"assistant","timestamp":"2026-10-26T08:30:00Z","requestId":"r9","message":{"id":"m9","model":"claude-haiku-5-5","content":[{"type":"tool_use","name":"Read"}],"usage":{"input_tokens":1000,"output_tokens":10}}}"#,
            ],
        );
        let found = find_sessions(dir.path());
        assert_eq!(found.len(), 1);
        let d = digest_session("work", &found[0].0, &found[0].1);
        let s = &d.summary;
        assert_eq!((s.prompts, s.turns, s.subagents), (1, 2, 1));
        assert_eq!(s.project.as_deref(), Some("/home/you/app"));
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.tools.get("Bash"), Some(&1));
        assert_eq!(s.tools.get("Read"), Some(&1));
        assert_eq!(s.tokens.input, 1010);
        assert_eq!(s.tokens.cache_read, 100);
        assert_eq!(s.models, vec!["claude-haiku-5-5", "claude-opus-5-5"]);
        assert_eq!(s.reported_cost, Some(0.01));
        let expect = (10.0 * 4.0 + 5.0 * 20.0 + 20.0 * 5.0 + 100.0 * 0.2) / 1e6
            + (1000.0 * 0.1 + 10.0 * 0.5) / 1e6;
        assert!((s.cost - expect).abs() < 1e-12, "{} vs {expect}", s.cost);
        let dump = format!("{s:?}{:?}", d.hours);
        assert!(!dump.contains("SECRET"), "{dump}");
        assert_eq!(d.hours.iter().map(|h| h.prompts).sum::<u32>(), 1);

        let ev = &d.events;
        assert_eq!(ev.iter().filter(|e| e.kind == "prompt").count(), 1);
        assert_eq!(ev.iter().filter(|e| e.kind == "call").count(), 2);
        let bash = ev
            .iter()
            .find(|e| e.tool.as_deref() == Some("Bash"))
            .unwrap();
        assert_eq!(bash.end_ts, None, "the sample result has no tool_use_id");
        assert!(ev
            .iter()
            .any(|e| e.subagent && e.tool.as_deref() == Some("Read")));
        assert!(!format!("{ev:?}").contains("SECRET"));

        let cache = Cache::default();
        let r = cache.report(&[("work".into(), dir.path().to_path_buf())], 0, 0);
        assert_eq!(r.accounts[0].sessions.len(), 1);
        let later = cache.report(
            &[("work".into(), dir.path().to_path_buf())],
            i64::MAX / 2,
            0,
        );
        assert!(later.accounts[0].sessions.is_empty());
        let (events, cut) =
            cache.events(&[("work".into(), dir.path().to_path_buf())], 0, i64::MAX, 2);
        assert!(cut);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn tool_runs_get_their_duration_and_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("projects/p/s2.jsonl");
        write(
            &main,
            &[
                r#"{"type":"assistant","timestamp":"2026-10-26T08:00:00Z","requestId":"r","message":{"id":"m","model":"claude-opus-5-5","content":[{"type":"tool_use","id":"tu1","name":"Bash","input":{"command":"x"}}],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
                r#"{"type":"user","timestamp":"2026-10-26T08:00:02.500Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"tu1","is_error":true,"content":"boom"}]}}"#,
            ],
        );
        let d = digest_session("w", &main, &[]);
        let t = d.events.iter().find(|e| e.kind == "tool").unwrap();
        assert_eq!(t.end_ts.unwrap() - t.ts, 2500);
        assert_eq!(t.ok, Some(false));
        assert_eq!(d.summary.prompts, 0);
    }
}
