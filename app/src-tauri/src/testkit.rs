//! Test helpers. Fake secrets are built at run time, so the source holds no
//! key-shaped literal for secret scanners to report.

/// `prefix` followed by `n` x's, for example `fake("ghp_", 36)`.
pub fn fake(prefix: &str, n: usize) -> String {
    format!("{prefix}{}", "x".repeat(n))
}
