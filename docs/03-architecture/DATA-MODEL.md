# Data model

SQLite, WAL mode, one file at `<app-data>/facet.sqlite`. Times are Unix milliseconds (UTC). Money is stored as micro-dollars (integer) to avoid float drift.

```sql
-- accounts: no secrets ever
CREATE TABLE account (
  id            TEXT PRIMARY KEY,          -- slug, e.g. 'work'
  name          TEXT NOT NULL,
  color         TEXT NOT NULL,             -- token name, e.g. 'facet-2'
  plan          TEXT,
  desktop_dir   TEXT,                      -- Claude Desktop --user-data-dir
  code_dir      TEXT,                      -- CLAUDE_CONFIG_DIR
  expected_email TEXT,                     -- for sign-in verification, user-entered
  tint          INTEGER NOT NULL DEFAULT 1,
  capture_prompts INTEGER NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL
);

CREATE TABLE session (                     -- a "conversation"
  id          TEXT PRIMARY KEY,            -- session.id from Claude Code
  account_id  TEXT NOT NULL REFERENCES account(id),
  project     TEXT,                        -- working directory
  title       TEXT,
  started_at  INTEGER, ended_at INTEGER,
  cost_micros INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE task (                        -- one user prompt and what followed
  id          TEXT PRIMARY KEY,            -- prompt.id if present, else session.id + seq
  session_id  TEXT NOT NULL REFERENCES session(id),
  account_id  TEXT NOT NULL,
  started_at  INTEGER NOT NULL, ended_at INTEGER,
  prompt_chars INTEGER,
  prompt_text TEXT,                        -- NULL unless capture is on for the account
  cost_micros INTEGER NOT NULL DEFAULT 0,
  calls       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE event (
  id          INTEGER PRIMARY KEY,
  ts          INTEGER NOT NULL,
  account_id  TEXT NOT NULL,
  session_id  TEXT,
  task_id     TEXT,
  type        TEXT NOT NULL,               -- prompt | api | tool | decision | error
  model       TEXT,
  tok_in INTEGER, tok_out INTEGER, tok_cache_read INTEGER, tok_cache_write INTEGER,
  cost_micros INTEGER,
  duration_ms INTEGER,
  tool        TEXT, target TEXT,           -- e.g. 'Bash', 'npm test'
  ok          INTEGER,
  out_tokens  INTEGER,                     -- size of tool output
  source      TEXT NOT NULL,               -- otel | transcript
  raw_ref     TEXT                         -- pointer for debugging, never content
);
CREATE INDEX event_ts ON event(ts);
CREATE INDEX event_acc_ts ON event(account_id, ts);
CREATE INDEX event_task ON event(task_id);

-- rollups for fast zooming
CREATE TABLE rollup_minute (
  account_id TEXT NOT NULL, minute INTEGER NOT NULL,     -- ts / 60000
  calls INTEGER, tools INTEGER, tok_total INTEGER, cost_micros INTEGER,
  PRIMARY KEY (account_id, minute)
);
CREATE TABLE rollup_hour (
  account_id TEXT NOT NULL, hour INTEGER NOT NULL,
  calls INTEGER, tools INTEGER, tok_total INTEGER, cost_micros INTEGER,
  PRIMARY KEY (account_id, hour)
);

CREATE TABLE budget (
  id INTEGER PRIMARY KEY, account_id TEXT, client TEXT NOT NULL,
  folder_glob TEXT, monthly_limit_micros INTEGER NOT NULL, alert_pct INTEGER DEFAULT 80
);

CREATE TABLE bookmark (id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, account_id TEXT, note TEXT);

CREATE TABLE backup (
  id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL, account_id TEXT,
  file TEXT NOT NULL, reason TEXT NOT NULL, path TEXT NOT NULL   -- encrypted archive
);

CREATE TABLE ecosystem_item (
  id TEXT PRIMARY KEY, name TEXT, kind TEXT, source TEXT, version TEXT,
  preview_json TEXT NOT NULL, risk TEXT, inspected_at INTEGER
);
CREATE TABLE ecosystem_install (
  item_id TEXT REFERENCES ecosystem_item(id), account_id TEXT REFERENCES account(id),
  installed_at INTEGER, PRIMARY KEY (item_id, account_id)
);
```

## Query patterns
- Timeline at span ≥ 2 days: `rollup_hour`. 2 h–2 days: `rollup_minute` + `task`. < 2 h: `event` by `(account_id, ts)` range.
- Most expensive tasks in range: `SELECT task_id, SUM(cost_micros) FROM event WHERE type='api' AND ts BETWEEN ? AND ? GROUP BY task_id ORDER BY 2 DESC LIMIT 10`.
- Tool-run attribution: for each tool event, the next `api` event with the same `task_id` and `ts >` tool end.

## Retention
A daily job deletes `event` rows older than the retention setting (default 90 days) and keeps rollups. Prompt text is deleted with its task.
