//! Add and remove MCP servers per account through the official
//! `claude mcp add|remove` (PRD M4: Facet never writes Claude Code's state
//! file itself). Runs scrubbed, as the account, when the user asks.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::control;
use crate::ecosystem::redact;
use crate::env;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAdd {
    pub name: String,
    /// user | local | project
    pub scope: String,
    /// stdio | http | sse
    pub transport: String,
    /// stdio: the program; http/sse: the URL.
    pub command_or_url: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Project folder for local and project scope.
    pub folder: Option<PathBuf>,
}

fn valid_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 64
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && !n.starts_with('-')
}

fn valid_env_name(n: &str) -> bool {
    let mut c = n.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

fn check_scope(scope: &str, folder: &Option<PathBuf>) -> Result<()> {
    match scope {
        "user" => Ok(()),
        "local" | "project" => match folder {
            Some(f) if f.is_absolute() && f.is_dir() => Ok(()),
            _ => Err(Error::Invalid(
                "Pick the project folder this server belongs to.".into(),
            )),
        },
        _ => Err(Error::Invalid("Scope is user, local or project.".into())),
    }
}

/// The argument list for `claude mcp add`, in an order where the variadic
/// `-e` and `-H` options cannot swallow the name or URL.
pub fn add_args(a: &McpAdd) -> Result<Vec<String>> {
    if !valid_name(&a.name) {
        return Err(Error::Invalid(
            "Name the server with letters, digits, - or _ (up to 64).".into(),
        ));
    }
    check_scope(&a.scope, &a.folder)?;
    let target = a.command_or_url.trim();
    if target.is_empty() || target.starts_with('-') {
        return Err(Error::Invalid(
            "Give the program to run or the server's address.".into(),
        ));
    }
    for (k, v) in &a.env {
        if !valid_env_name(k) {
            return Err(Error::Invalid(format!(
                "\"{k}\" is not a valid variable name."
            )));
        }
        if control::is_refused_env(k) {
            return Err(Error::Invalid(format!("Facet does not pass {k} to an MCP server: it would carry the account's sign-in or redirect its traffic.")));
        }
        if v.contains('\n') {
            return Err(Error::Invalid(format!("{k} must be one line.")));
        }
    }
    for (k, v) in &a.headers {
        if k.is_empty()
            || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            || v.contains('\n')
        {
            return Err(Error::Invalid(format!("\"{k}\" is not a valid header.")));
        }
    }
    let mut out = vec![
        "mcp".to_string(),
        "add".into(),
        "--scope".into(),
        a.scope.clone(),
    ];
    match a.transport.as_str() {
        "stdio" => {
            if !a.headers.is_empty() {
                return Err(Error::Invalid(
                    "Headers are only for http and sse servers.".into(),
                ));
            }
            out.push(a.name.clone());
            for (k, v) in &a.env {
                out.push("-e".into());
                out.push(format!("{k}={v}"));
            }
            out.push("--".into());
            out.push(target.to_string());
            out.extend(a.args.iter().cloned());
        }
        "http" | "sse" => {
            if !(target.starts_with("https://") || target.starts_with("http://")) {
                return Err(Error::Invalid(
                    "The address must start with https://.".into(),
                ));
            }
            if !a.env.is_empty() || !a.args.is_empty() {
                return Err(Error::Invalid(
                    "Variables and arguments are only for stdio servers.".into(),
                ));
            }
            out.push("--transport".into());
            out.push(a.transport.clone());
            out.push(a.name.clone());
            out.push(target.to_string());
            for (k, v) in &a.headers {
                out.push("--header".into());
                out.push(format!("{k}: {v}"));
            }
        }
        _ => return Err(Error::Invalid("Transport is stdio, http or sse.".into())),
    }
    Ok(out)
}

pub fn remove_args(name: &str, scope: &str) -> Result<Vec<String>> {
    if !valid_name(name) {
        return Err(Error::Invalid("That is not an MCP server name.".into()));
    }
    if !matches!(scope, "user" | "local" | "project") {
        return Err(Error::Invalid("Scope is user, local or project.".into()));
    }
    Ok(vec![
        "mcp".into(),
        "remove".into(),
        "--scope".into(),
        scope.into(),
        name.into(),
    ])
}

/// Run the official claude as the account. Output is redacted before it is
/// returned, because Claude Code may echo what it saved.
pub fn run(
    claude: &Path,
    code_dir: &Path,
    home: &Path,
    cwd: &Path,
    args: &[String],
) -> Result<String> {
    let mut cmd = env::command(claude);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    env::prepare_with_home(&mut cmd, code_dir, home);
    let mut child = cmd.spawn()?;
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Unavailable(
                "claude mcp did not finish in 30 seconds.".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let read = |r: Option<std::process::ChildStdout>| -> String {
        use std::io::Read;
        let mut s = String::new();
        if let Some(mut r) = r {
            let _ = r.read_to_string(&mut s);
        }
        s
    };
    let out = read(child.stdout.take());
    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        use std::io::Read;
        let _ = e.read_to_string(&mut err);
    }
    let text: String = format!("{out}{err}")
        .lines()
        .map(redact)
        .collect::<Vec<_>>()
        .join("\n");
    if status.success() {
        Ok(text)
    } else {
        Err(Error::Invalid(if text.trim().is_empty() {
            "claude mcp reported an error.".into()
        } else {
            text
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> McpAdd {
        McpAdd {
            name: "db".into(),
            scope: "user".into(),
            transport: "stdio".into(),
            command_or_url: "npx".into(),
            args: vec!["-y".into(), "db-mcp@1.2.0".into()],
            env: vec![("DB_URL".into(), "postgres://x".into())],
            headers: vec![],
            folder: None,
        }
    }

    #[test]
    fn stdio_args_keep_the_name_before_variadic_env() {
        let a = add_args(&base()).unwrap();
        assert_eq!(
            a,
            [
                "mcp",
                "add",
                "--scope",
                "user",
                "db",
                "-e",
                "DB_URL=postgres://x",
                "--",
                "npx",
                "-y",
                "db-mcp@1.2.0"
            ]
        );
    }

    #[test]
    fn http_args_put_headers_last() {
        let a = add_args(&McpAdd {
            transport: "http".into(),
            command_or_url: "https://mcp.example.com/mcp".into(),
            args: vec![],
            env: vec![],
            headers: vec![("Authorization".into(), "Bearer t".into())],
            ..base()
        })
        .unwrap();
        assert_eq!(
            a,
            [
                "mcp",
                "add",
                "--scope",
                "user",
                "--transport",
                "http",
                "db",
                "https://mcp.example.com/mcp",
                "--header",
                "Authorization: Bearer t"
            ]
        );
    }

    #[test]
    fn refuses_bad_input() {
        assert!(add_args(&McpAdd {
            name: "-x".into(),
            ..base()
        })
        .is_err());
        assert!(add_args(&McpAdd {
            env: vec![("ANTHROPIC_API_KEY".into(), "x".into())],
            ..base()
        })
        .is_err());
        assert!(
            add_args(&McpAdd {
                scope: "local".into(),
                ..base()
            })
            .is_err(),
            "local needs a folder"
        );
        assert!(add_args(&McpAdd {
            transport: "http".into(),
            command_or_url: "ftp://x".into(),
            env: vec![],
            args: vec![],
            ..base()
        })
        .is_err());
        assert!(add_args(&McpAdd {
            command_or_url: "--evil".into(),
            ..base()
        })
        .is_err());
        assert!(remove_args("db", "everywhere").is_err());
        assert_eq!(
            remove_args("db", "local").unwrap(),
            ["mcp", "remove", "--scope", "local", "db"]
        );
    }

    #[test]
    fn run_scrubs_and_redacts() {
        let d = tempfile::tempdir().unwrap();
        let fake = d.path().join("claude");
        std::fs::write(&fake, "#!/bin/sh\necho \"args: $*\"\necho \"key set: ${ANTHROPIC_API_KEY:-none}\"\necho \"dir: $CLAUDE_CONFIG_DIR\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let out = run(
            &fake,
            Path::new("/h/.claude-work"),
            Path::new("/h"),
            d.path(),
            &[
                "mcp".into(),
                "add".into(),
                "x".into(),
                "-e".into(),
                crate::testkit::fake("TOKEN=ghp_", 36),
            ],
        )
        .unwrap();
        assert!(out.contains("key set: none"));
        assert!(out.contains("dir: /h/.claude-work"));
        assert!(!out.contains("ghp_"), "{out}");
    }
}
