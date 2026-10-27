//! Which layer each setting comes from, for a config folder and a project.
//!
//!   cargo run --example effective_report -- ~/.claude /path/to/project
//!
//! Read-only. Prints keys and sources, not values.

fn main() {
    let mut args = std::env::args().skip(1);
    let code = std::path::PathBuf::from(args.next().expect("config folder"));
    let project = args.next().map(std::path::PathBuf::from);
    let eff = facet_lib::effective::effective(
        &code,
        project.as_deref(),
        &facet_lib::effective::managed_path(),
    )
    .expect("read");
    for l in &eff.layers {
        println!(
            "layer {:<14} exists={} problem={:?}",
            l.name, l.exists, l.problem
        );
    }
    for e in &eff.entries {
        println!(
            "{:<40} from {:?}{}",
            e.key,
            e.sources,
            if e.overridden.is_empty() {
                String::new()
            } else {
                format!("  (overrides {:?})", e.overridden)
            }
        );
    }
}
