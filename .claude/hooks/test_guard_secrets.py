#!/usr/bin/env python3
"""Tests for guard-secrets.py. Run: python3 -I .claude/hooks/test_guard_secrets.py

Cases live here rather than on the command line so the hook does not block
the test run itself.
"""
import json
import subprocess
import sys
from pathlib import Path

HOOK = Path(__file__).with_name("guard-secrets.py")
V = "ANTHROPIC_API_KEY"  # assembled below so this file reads as plainly as possible

BLOCK = [
    {"command": "cat ~/.claude/.credentials.json"},
    {"command": "cat ~/.claude/.cred*"},
    {"command": "cat ~/.claude/sessions/440478.abc.key"},
    {"file_path": "/home/y/.config/Claude-Work/Cookies"},
    {"path": "/home/y/.config/Claude/Local Storage", "pattern": "x"},
    {"file_path": "/home/y/.config/Claude/Local State"},
    {"command": "echo $" + V},
    {"command": "echo ${CLAUDE_CODE_OAUTH_TOKEN:0:5}"},
    {"command": "printenv ANTHROPIC_AUTH_TOKEN"},
    {"command": "cat /proc/123/environ"},
    {"command": "tr '\\0' '\\n' < /proc/123/environ | grep TOKEN"},
    {"command": "env"},
    {"command": "printenv | sort"},
    {"command": "secret-tool lookup service x"},
]
ALLOW = [
    {"command": "ls ~/.claude/sessions"},
    {"command": "tr '\\0' '\\n' < /proc/123/environ | cut -d= -f1 | sort"},
    {"command": "echo \"== 1: fake " + V + "\""},
    {"command": "env -u " + V + " " + V + "=sk-ant-invalid-facet-test claude -p hi"},
    {"command": "grep -nE " + V + " ~/.bashrc | cut -d= -f1"},
    {"command": "set -euo pipefail; ls"},
    {"command": "python3 -I spikes/11-skill-authoring/lint_skills.py .claude"},
    {"file_path": "docs/x.md", "content": "never read .credentials.json"},
]


def run(tool_input):
    event = json.dumps({"tool_name": "Bash", "tool_input": tool_input})
    return subprocess.run([sys.executable, "-I", str(HOOK)], input=event, text=True,
                          capture_output=True).returncode


def main():
    failures = 0
    for case in BLOCK:
        if run(case) != 2:
            failures += 1
            print("SHOULD BLOCK:", case)
    for case in ALLOW:
        if run(case) != 0:
            failures += 1
            print("SHOULD ALLOW:", case)
    bad = subprocess.run([sys.executable, "-I", str(HOOK)], input="not json", text=True,
                         capture_output=True).returncode
    if bad != 2:
        failures += 1
        print("SHOULD FAIL CLOSED on bad input")
    total = len(BLOCK) + len(ALLOW) + 1
    print(f"{total - failures}/{total} passed")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
