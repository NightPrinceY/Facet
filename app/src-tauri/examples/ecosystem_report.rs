//! What each account has installed, with risk levels, without the UI.
//! `cargo run --example ecosystem_report`. Prints names and counts only.
use facet_lib::ecosystem::inventory;
use std::path::PathBuf;

fn main() {
    let home = PathBuf::from(std::env::var("HOME").expect("HOME"));
    for (id, dir) in [
        ("personal", home.join(".claude")),
        ("work", home.join(".claude-work")),
    ] {
        let t = std::time::Instant::now();
        let inv = inventory(id, &dir, &home);
        println!(
            "{id}: {} plugins, {} marketplaces, {} MCP servers, {} hooks, {} skills, {} agents, {} commands, problems {:?} ({:?})",
            inv.plugins.len(), inv.marketplaces.len(), inv.mcp.len(), inv.hooks.len(), inv.skills, inv.agents, inv.commands, inv.problems, t.elapsed()
        );
        for p in &inv.plugins {
            let r = p.report.as_ref();
            println!(
                "  plugin {} enabled={:?} risk={:?} skills={} hooks={} mcp={} bin={}",
                p.id,
                p.enabled,
                r.map(|r| r.risk),
                r.map_or(0, |r| r.skills.len()),
                r.map_or(0, |r| r.hooks.len()),
                r.map_or(0, |r| r.mcp.len()),
                r.map_or(0, |r| r.bin.len())
            );
        }
        for m in &inv.mcp {
            println!(
                "  mcp {} ({}, {}) notes={}",
                m.name,
                m.scope,
                m.transport,
                m.notes.len()
            );
        }
        for m in &inv.marketplaces {
            println!(
                "  marketplace {} ({}) pinned={}",
                m.name, m.source, m.pinned
            );
        }
    }
}
