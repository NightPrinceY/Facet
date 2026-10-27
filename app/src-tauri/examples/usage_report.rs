//! Usage and cost per account from transcripts, without the UI.
//! `cargo run --example usage_report [-- days]`. Prints totals only.
use facet_lib::usage::Cache;
use std::path::PathBuf;

fn main() {
    let home = PathBuf::from(std::env::var("HOME").expect("HOME"));
    let days: i64 = std::env::args()
        .nth(1)
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);
    let accounts = vec![
        ("personal".to_string(), home.join(".claude")),
        ("work".to_string(), home.join(".claude-work")),
    ];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let cache = Cache::default();
    let t = std::time::Instant::now();
    let r = cache.report(&accounts, now - days * 86_400_000, now);
    let first = t.elapsed();
    let t2 = std::time::Instant::now();
    let _ = cache.report(&accounts, now - days * 86_400_000, now);
    println!("read in {first:?}, cached refresh {:?}", t2.elapsed());
    for a in &r.accounts {
        let cost: f64 = a.sessions.iter().map(|s| s.cost).sum();
        let reported: f64 = a.sessions.iter().filter_map(|s| s.reported_cost).sum();
        let matched: f64 = a
            .sessions
            .iter()
            .filter(|s| s.reported_cost.is_some())
            .map(|s| s.cost)
            .sum();
        let turns: u32 = a.sessions.iter().map(|s| s.turns).sum();
        let prompts: u32 = a.sessions.iter().map(|s| s.prompts).sum();
        println!(
            "{}: {} sessions, {} prompts, {} replies, estimate ${:.2}; sessions with Claude Code's own figure: ${:.2} vs estimate ${:.2}; hours {}",
            a.account_id, a.sessions.len(), prompts, turns, cost, reported, matched, a.hours.len()
        );
    }
}
