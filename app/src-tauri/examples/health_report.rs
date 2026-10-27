//! Run Facet's Health checks against this machine without the UI.
//!
//!   FACET_PROFILES=/tmp/x/profiles.toml cargo run --example health_report [--sign-in]
//!
//! Prints status, id and title. Details of sign-in checks are hidden because
//! they contain the signed-in email.

use std::collections::BTreeSet;

fn main() {
    let home = facet_lib::home_dir();
    let store = facet_lib::profile::Store::new(facet_lib::store_path(&home));
    let accounts = store.load().expect("accounts file");
    let sign_in = std::env::args().any(|a| a == "--sign-in");
    let facts = facet_lib::health::gather(&home, &accounts, BTreeSet::new(), sign_in);
    println!("running Desktop instances: {}", facts.running.len());
    for p in &facts.running {
        let owner = accounts
            .iter()
            .find(|a| facet_lib::adapter::linux::instance_for(a, std::slice::from_ref(p)).is_some())
            .map(|a| a.id.as_str())
            .unwrap_or("(no account)");
        println!("  pid {} x11={} account={owner}", p.pid, p.x11);
    }
    for c in facet_lib::health::evaluate(&facts) {
        let detail = if c.id.starts_with("auth:") {
            "(hidden: contains email)".to_string()
        } else {
            c.detail
        };
        println!(
            "{:<5} {:<28} {}\n      {}",
            format!("{:?}", c.status).to_lowercase(),
            c.id,
            c.title,
            detail
        );
    }
}
