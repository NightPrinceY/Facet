#!/usr/bin/env python3
"""PreToolUse hook: enforce rule 2 (never read sign-in data) on every tool.

settings.json denies the Read tool on credential paths, but Bash, Grep, Edit
and others could still reach them. This hook blocks any tool call whose path
or command touches sign-in data. Exit code 2 blocks the call and shows the
reason to Claude.
"""
import json
import re
import sys

# Sign-in data: Claude Code credentials, session keys, Electron cookies and
# storage, OS keychains, and the override token variables.
SECRET_PATTERNS = [
    r"\.cred",
    r"Local State",
    r"Keychains/",
    r"\bcmdkey\b",
    r"^\s*(env|printenv|set)\s*($|\|)",
    r"sessions/[^\s/]*\.key\b",
    r"\bCookies(-journal)?\b",
    r"Local Storage",
    r"Session Storage",
    r"Login Data",
    r"\bsecret-tool\b",
    r"\bgnome-keyring\b",
    r"\bkwallet",
    r"\bsecurity\s+find-(generic|internet)-password",
    r"keyrings/",
    # Printing a token's value: $VAR or ${VAR} anywhere, or printenv VAR.
    # Naming the variable (env -u VAR, VAR=fake, labels) is fine.
    r"\$\{?(ANTHROPIC_API_KEY|ANTHROPIC_AUTH_TOKEN|CLAUDE_CODE_OAUTH_TOKEN|CLAUDE_CODE_MESSAGING_TOKEN)\b",
    r"\bprintenv\s+(ANTHROPIC_API_KEY|ANTHROPIC_AUTH_TOKEN|CLAUDE_CODE_OAUTH_TOKEN|CLAUDE_CODE_MESSAGING_TOKEN)\b",
    # Another process's environment holds injected tokens: allow names only.
    r"/proc/\S*/environ(?![^\n]*\|\s*cut\s+-d\s*=?\s*-f\s*1\b)",
]
SECRET_RE = re.compile("|".join(SECRET_PATTERNS), re.IGNORECASE | re.MULTILINE)

# Fields that hold a path or command, per tool. Content being written is not
# scanned, so docs may still mention these file names.
FIELDS = ("file_path", "path", "pattern", "command", "notebook_path", "glob")


def main() -> int:
    try:
        event = json.load(sys.stdin)
    except json.JSONDecodeError:
        print("Blocked: guard-secrets could not parse the hook input (fails closed).", file=sys.stderr)
        return 2
    tool_input = event.get("tool_input") or {}
    haystack = " ".join(str(tool_input.get(f, "")) for f in FIELDS)
    match = SECRET_RE.search(haystack)
    if not match:
        return 0
    print(
        f"Blocked by Facet rule 2 (never read sign-in data): '{match.group(0)}' "
        "looks like credentials, session keys, cookies or a keychain. "
        "List folder names only, or ask the user. To write docs that mention "
        "these names, use the Write or Edit tool instead of a shell heredoc.",
        file=sys.stderr,
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
