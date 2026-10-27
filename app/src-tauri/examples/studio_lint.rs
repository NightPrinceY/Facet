//! Lint the skills, agents and commands in a folder with Studio's linter.
//!
//!   cargo run --example studio_lint -- ~/.claude
//!
//! Read-only. Prints findings, never file contents.

fn main() {
    let root = std::env::args().nth(1).expect("folder");
    let items = facet_lib::studio::list_items(std::path::Path::new(&root), true);
    let mut counts = std::collections::BTreeMap::new();
    for i in &items {
        for f in &i.lint {
            *counts
                .entry(format!("{:?}", f.severity).to_lowercase())
                .or_insert(0) += 1;
            if f.severity != facet_lib::studio::Severity::Info {
                println!(
                    "{:?} {:<40} {:?}: {}",
                    i.kind, i.name, f.severity, f.message
                );
            }
        }
    }
    println!("{} items · {:?}", items.len(), counts);
}
