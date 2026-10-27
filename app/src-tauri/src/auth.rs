//! Which account a profile is signed into, from the official
//! `claude auth status --json` (documented CLI; prints status, not
//! credentials). Facet never reads `~/.claude.json` or credential files.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use crate::env;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub logged_in: bool,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub org_name: Option<String>,
    #[serde(default)]
    pub subscription_type: Option<String>,
    /// Folder Claude Code reports using; must equal the account's `code_dir`.
    #[serde(default)]
    pub config_directory: Option<String>,
}

/// Keep only the fields above (`orgId` and others are dropped).
pub fn parse(json: &str) -> Result<AuthStatus> {
    serde_json::from_str(json)
        .map_err(|e| Error::Invalid(format!("Unexpected output from claude auth status: {e}")))
}

/// Does the signed-in email match what the user expects? Case-insensitive.
pub fn email_matches(status: &AuthStatus, expected: Option<&str>) -> Option<bool> {
    let expected = expected?;
    let got = status.email.as_deref()?;
    Some(got.trim().eq_ignore_ascii_case(expected.trim()))
}

pub fn check(claude: &Path, code_dir: &Path, timeout: Duration) -> Result<AuthStatus> {
    let mut cmd = env::command(claude);
    cmd.args(["auth", "status", "--json"]);
    env::prepare(&mut cmd, code_dir);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn()?;
    let started = std::time::Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Err(Error::Unavailable(
                "claude auth status did not answer in time.".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let out = child.wait_with_output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    parse(text.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_fields_and_drops_the_rest() {
        let s = parse(r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","email":"Me@Example.com","orgId":"o-1","orgName":"Acme","subscriptionType":"max","configDirectory":"/h/.claude-work","projectsDirectory":"/h/.claude-work/projects"}"#).unwrap();
        assert!(s.logged_in);
        assert_eq!(s.subscription_type.as_deref(), Some("max"));
        assert!(!serde_json::to_string(&s).unwrap().contains("o-1"));
        assert_eq!(email_matches(&s, Some("me@example.com ")), Some(true));
        assert_eq!(email_matches(&s, Some("other@example.com")), Some(false));
        assert_eq!(email_matches(&s, None), None);
    }

    #[test]
    fn signed_out_has_no_email() {
        let s = parse(r#"{"loggedIn":false}"#).unwrap();
        assert!(!s.logged_in);
        assert_eq!(email_matches(&s, Some("a@b.c")), None);
        assert!(parse("not json").is_err());
    }
}
