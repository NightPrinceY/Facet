#!/usr/bin/env python3
"""Spike 3: a tiny local OTLP/HTTP (JSON) receiver.

Listens on 127.0.0.1:4318 only. Accepts POST /v1/logs and /v1/metrics, appends each payload to
data/otel.jsonl, and prints a one-line summary per record: account (from resource attribute
facet.account), event or metric name, and the attribute KEYS present. It does not print values of
prompt text attributes.

Run:   python3 collector.py            (Ctrl+C to stop)
Then in each test account's settings.json "env", set (see README):
  CLAUDE_CODE_ENABLE_TELEMETRY=1, OTEL_LOGS_EXPORTER=otlp, OTEL_METRICS_EXPORTER=otlp,
  OTEL_EXPORTER_OTLP_PROTOCOL=http/json, OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318,
  OTEL_RESOURCE_ATTRIBUTES=facet.account=<id>
"""
import json, os, sys, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.abspath(__file__))
os.makedirs(os.path.join(HERE, "data"), exist_ok=True)
OUT = os.path.join(HERE, "data", "otel.jsonl")
HIDE = {"prompt", "user_prompt", "prompt.text", "prompt_text", "response"}

def attrs(lst):
    out = {}
    for a in lst or []:
        v = a.get("value", {})
        out[a.get("key")] = next(iter(v.values()), None) if isinstance(v, dict) else v
    return out

class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        ctype = self.headers.get("Content-Type", "")
        if "json" not in ctype:
            print(f"[{self.path}] non-JSON payload ({ctype}, {len(body)} bytes). Set OTEL_EXPORTER_OTLP_PROTOCOL=http/json")
            self.send_response(200); self.end_headers(); return
        data = json.loads(body or b"{}")
        with open(OUT, "a") as fh:
            fh.write(json.dumps({"path": self.path, "received_ns": time.time_ns(), "data": data}) + "\n")
        if self.path.endswith("/v1/logs"):
            for rl in data.get("resourceLogs", []):
                res = attrs(rl.get("resource", {}).get("attributes"))
                acc = res.get("facet.account", "?")
                for sl in rl.get("scopeLogs", []):
                    for lr in sl.get("logRecords", []):
                        a = attrs(lr.get("attributes"))
                        name = a.get("event.name") or (lr.get("body") or {}).get("stringValue", "?")
                        keys = sorted(k for k in a.keys())
                        shown = {k: a[k] for k in ("model", "cost_usd", "input_tokens", "output_tokens", "cache_read_tokens", "cache_creation_tokens", "duration_ms", "tool_name", "success", "session.id", "prompt.id") if k in a and k not in HIDE}
                        print(f"LOG  account={acc:10} event={name:28} {shown}  keys={keys}")
        elif self.path.endswith("/v1/metrics"):
            for rm in data.get("resourceMetrics", []):
                res = attrs(rm.get("resource", {}).get("attributes"))
                acc = res.get("facet.account", "?")
                for sm in rm.get("scopeMetrics", []):
                    for m in sm.get("metrics", []):
                        print(f"MET  account={acc:10} metric={m.get('name')}")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b"{}")

if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 4318
    print(f"Listening on http://127.0.0.1:{port} · writing {OUT}")
    ThreadingHTTPServer(("127.0.0.1", port), H).serve_forever()
