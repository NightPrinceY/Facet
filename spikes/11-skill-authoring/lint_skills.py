#!/usr/bin/env python3
"""Lint Claude Code skills and subagents (prototype of Facet Studio's linter).

Usage: python3 lint_skills.py <dir> [<dir> ...]

Each <dir> is scanned for skills/<name>/SKILL.md, commands/*.md and
agents/**/*.md (a config dir, a project's .claude dir, or a plugin). Read-only.
Prints findings per file; never prints skill bodies. Rules follow
docs/03-architecture/EXTENSION-SURFACE.md (checked 2026-10-26).
"""
import re
import sys
from pathlib import Path

SKILL_KEYS = {
    "name", "description", "when_to_use", "argument-hint", "arguments",
    "disable-model-invocation", "user-invocable", "allowed-tools",
    "disallowed-tools", "model", "effort", "context", "agent", "background",
    "hooks", "paths", "shell", "metadata", "license", "compatibility",
}
PORTABLE_KEYS = {"name", "description", "license", "compatibility", "metadata", "allowed-tools"}
AGENT_KEYS = {
    "name", "description", "tools", "disallowedTools", "model", "permissionMode",
    "maxTurns", "autoCompactWindow", "skills", "mcpServers", "hooks", "memory",
    "background", "omitClaudeMd", "effort", "isolation", "color", "initialPrompt",
    "experimental",
}
COMMAND_KEYS = {"description", "argument-hint", "allowed-tools", "model", "disable-model-invocation"}
DESC_BUDGET = 1536
BODY_LINES = 500
SHELL_INJECTION = re.compile(r"!`[^`]+`|^```!", re.MULTILINE)
LINK = re.compile(r"\]\((?!https?:|#|mailto:)([^)\s]+)\)")


def split_frontmatter(text):
    if not text.startswith("---\n"):
        return None, text
    end = text.find("\n---", 4)
    if end == -1:
        return None, text
    return text[4:end], text[end + 4:].lstrip("\n")


def parse_keys(fm):
    """Minimal YAML reading: top-level keys and raw values. No PyYAML needed."""
    keys, problems, current = {}, [], None
    for n, line in enumerate(fm.split("\n"), 2):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line[0] in " \t-":
            if current:
                keys[current] += "\n" + line
            continue
        m = re.match(r"^([A-Za-z_][\w-]*):(.*)$", line)
        if not m:
            problems.append(f"line {n}: not a 'key: value' line")
            continue
        current = m.group(1)
        if current in keys:
            problems.append(f"line {n}: duplicate key '{current}'")
        keys[current] = m.group(2).strip()
    return keys, problems


def scalar(v):
    v = v.strip()
    if len(v) >= 2 and v[0] == v[-1] and v[0] in "'\"":
        v = v[1:-1]
    return v


def lint(path, kind, root):
    text = path.read_text(encoding="utf-8", errors="replace")
    fm, body = split_frontmatter(text)
    out = []
    if fm is None:
        if kind == "command":
            return out  # commands may omit frontmatter
        return ["error: no YAML frontmatter (file must start with '---')"]
    keys, problems = parse_keys(fm)
    out += [f"error: {p}" for p in problems]
    allowed = {"skill": SKILL_KEYS, "agent": AGENT_KEYS, "command": COMMAND_KEYS | SKILL_KEYS}[kind]
    for k in keys:
        if k not in allowed:
            out.append(f"warn: unknown key '{k}' (Claude Code ignores it silently)")

    desc_raw = keys.get("description", "")
    if kind in ("skill", "agent") and not desc_raw:
        out.append("error: missing 'description' (routing depends on it)")
    if desc_raw in ("|", ">", "|-", ">-") or "\n" in desc_raw:
        out.append("warn: multi-line description; prefer one line (some tools mis-read block scalars)")
    first, _, rest = desc_raw.partition("\n")
    if first in ("|", ">", "|-", ">-", ""):
        desc = " ".join(l.strip() for l in rest.split("\n") if l.strip())
    else:
        desc = scalar(first)
    if desc_raw and ": " in first and first[:1] not in "'\"|>":
        out.append("error: unquoted ': ' in description breaks YAML; wrap it in quotes")
    total = len(desc) + len(scalar(keys.get("when_to_use", "")))
    if kind == "skill":
        if total > DESC_BUDGET:
            out.append(f"warn: description + when_to_use is {total} chars; listing truncates at {DESC_BUDGET}")
        elif total and total < 40:
            out.append(f"warn: description is only {total} chars; say what it does and when to use it")
        name = scalar(keys.get("name", path.parent.name))
        if name != path.parent.name:
            out.append(f"warn: name '{name}' differs from folder '{path.parent.name}'")
        if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", name):
            out.append(f"warn: name '{name}' is not lowercase-kebab-case")
        extra = set(keys) - PORTABLE_KEYS
        if extra:
            out.append("info: not portable to claude.ai / Agent Skills clients: " + ", ".join(sorted(extra)))
    if kind == "agent":
        name = scalar(keys.get("name", ""))
        if not name:
            out.append("error: subagent needs 'name'")
        elif ":" in name or name.startswith("-") or len(name) > 256:
            out.append(f"error: invalid subagent name '{name}'")
        if scalar(keys.get("permissionMode", "")) in ("bypassPermissions", "dontAsk", "auto"):
            out.append("risk: permissionMode lets this agent act without asking")

    lines = body.count("\n") + 1
    if lines > BODY_LINES:
        out.append(f"warn: body is {lines} lines; move detail into linked files (target < {BODY_LINES})")
    if SHELL_INJECTION.search(body):
        out.append("risk: runs shell commands when loaded (!`cmd` injection)")
    if "hooks" in keys:
        out.append("risk: defines hooks in frontmatter")
    for target in LINK.findall(body):
        target = target.split("#")[0].replace("${CLAUDE_SKILL_DIR}/", "")
        resolved = (path.parent / target).resolve()
        if not target or "$" in target or not resolved.is_relative_to(path.parent.resolve()):
            continue  # never probe paths outside the skill folder
        if not resolved.exists():
            out.append(f"warn: broken relative link '{target}'")
    return out


def targets(root):
    for p in sorted(root.glob("skills/*/SKILL.md")):
        yield p, "skill"
    for p in sorted(root.glob("commands/**/*.md")):
        yield p, "command"
    for p in sorted(root.glob("agents/**/*.md")):
        yield p, "agent"


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 64
    counts = {"error": 0, "warn": 0, "risk": 0, "info": 0}
    files = 0
    for arg in argv[1:]:
        root = Path(arg).expanduser()
        for path, kind in targets(root):
            files += 1
            findings = lint(path, kind, root)
            rel = path.relative_to(root)
            print(f"{'OK ' if not findings else '   '} {kind:7} {root}/{rel}")
            for f in findings:
                counts[f.split(":")[0]] += 1
                print(f"        {f}")
    print(f"\n{files} files · " + " · ".join(f"{v} {k}" for k, v in counts.items()))
    return 1 if counts["error"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
