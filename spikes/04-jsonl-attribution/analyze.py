#!/usr/bin/env python3
"""Spike 4: what do Claude Code transcripts contain, structurally?

Reads <config_dir>/projects/**/*.jsonl READ-ONLY and reports field names, record types,
timestamp precision, token-usage fields and how well tool_use ids join to tool_result ids.
It never prints message text, prompts, tool inputs or outputs.

Usage: python3 analyze.py [CONFIG_DIR] [--max-files N]
       CONFIG_DIR defaults to ~/.claude
"""
import json, os, sys, glob, collections, statistics
from datetime import datetime

cfg = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "~/.claude")
max_files = int(sys.argv[sys.argv.index("--max-files") + 1]) if "--max-files" in sys.argv else 200
files = sorted(glob.glob(os.path.join(cfg, "projects", "**", "*.jsonl"), recursive=True), key=os.path.getmtime, reverse=True)[:max_files]
if not files:
    sys.exit(f"No transcripts under {cfg}/projects")

types = collections.Counter()
top_keys = collections.Counter()
msg_keys = collections.Counter()
usage_keys = collections.Counter()
content_block_types = collections.Counter()
ts_formats = collections.Counter()
tool_use_ids, tool_result_ids = set(), set()
assistant_with_usage = assistant_total = 0
per_file_lines = []
gaps_ms = []
has_session = has_cwd = has_uuid = has_parent = 0
records = 0

for f in files:
    n = 0
    last_ts = None
    with open(f, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                r = json.loads(line)
            except json.JSONDecodeError:
                types["<invalid json>"] += 1
                continue
            n += 1
            records += 1
            t = r.get("type", "<none>")
            types[t] += 1
            top_keys.update(r.keys())
            has_session += "sessionId" in r
            has_cwd += "cwd" in r
            has_uuid += "uuid" in r
            has_parent += "parentUuid" in r
            ts = r.get("timestamp")
            if isinstance(ts, str):
                frac = ts.split(".")[1].rstrip("Z") if "." in ts else ""
                ts_formats[f"ISO, {len(frac)} fractional digits"] += 1
                try:
                    dt = datetime.fromisoformat(ts.replace("Z", "+00:00")).timestamp() * 1000
                    if last_ts is not None and dt >= last_ts:
                        gaps_ms.append(dt - last_ts)
                    last_ts = dt
                except ValueError:
                    pass
            msg = r.get("message")
            if isinstance(msg, dict):
                msg_keys.update(msg.keys())
                if msg.get("role") == "assistant" or t == "assistant":
                    assistant_total += 1
                    if isinstance(msg.get("usage"), dict):
                        assistant_with_usage += 1
                        usage_keys.update(msg["usage"].keys())
                content = msg.get("content")
                if isinstance(content, list):
                    for b in content:
                        if isinstance(b, dict):
                            bt = b.get("type", "<none>")
                            content_block_types[bt] += 1
                            if bt == "tool_use" and "id" in b:
                                tool_use_ids.add(b["id"])
                            if bt == "tool_result" and "tool_use_id" in b:
                                tool_result_ids.add(b["tool_use_id"])
    per_file_lines.append(n)

def table(title, counter, limit=25):
    print(f"\n## {title}")
    for k, v in counter.most_common(limit):
        print(f"- `{k}`: {v}")

print(f"# Transcript structure report\n\nConfig dir: `{cfg}` · files scanned: {len(files)} · records: {records}")
print(f"Lines per file: median {statistics.median(per_file_lines):.0f}, max {max(per_file_lines)}")
table("Record types (`type`)", types)
table("Top-level keys", top_keys)
table("`message` keys", msg_keys)
table("Content block types", content_block_types)
table("Timestamp format", ts_formats)
print("\n## Token usage on assistant messages")
print(f"- assistant records: {assistant_total}, with `usage`: {assistant_with_usage} ({(assistant_with_usage/assistant_total*100 if assistant_total else 0):.0f}%)")
table("`usage` keys", usage_keys)
joined = len(tool_use_ids & tool_result_ids)
print("\n## tool_use ↔ tool_result join")
print(f"- tool_use ids: {len(tool_use_ids)}, tool_result ids: {len(tool_result_ids)}, joined: {joined} ({(joined/len(tool_use_ids)*100 if tool_use_ids else 0):.0f}%)")
print("\n## Linkage fields present (share of records)")
for k, v in [("sessionId", has_session), ("cwd", has_cwd), ("uuid", has_uuid), ("parentUuid", has_parent)]:
    print(f"- `{k}`: {v/records*100:.0f}%")
if gaps_ms:
    print(f"\n## Time between consecutive records\n- median {statistics.median(gaps_ms)/1000:.1f} s, p95 {sorted(gaps_ms)[int(len(gaps_ms)*.95)]/1000:.1f} s")
