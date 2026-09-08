CREATE TABLE monitored_apps (
 app_id TEXT PRIMARY KEY, payload TEXT NOT NULL, material_digest TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 1,
 due_at INTEGER NOT NULL, lease_token TEXT, lease_until INTEGER,
 last_attempt INTEGER, last_success INTEGER, last_error TEXT, observation TEXT,
 baseline_owner TEXT, route_available INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE release_candidates (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, source_version TEXT NOT NULL, observation TEXT NOT NULL,
 compatibility TEXT NOT NULL DEFAULT 'unknown', detected_at INTEGER NOT NULL, UNIQUE(app_id,source_version)
);
CREATE TABLE distribution_holds (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, code TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'open',
 created_by TEXT NOT NULL, created_at INTEGER NOT NULL, resolved_by TEXT, resolved_at INTEGER, resolution TEXT
);
CREATE TABLE integrity_reports (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, reporter TEXT NOT NULL REFERENCES users(id), kind TEXT NOT NULL,
 message TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'open', created_at INTEGER NOT NULL, resolved_at INTEGER
);
CREATE TABLE distribution_appeals (
 id TEXT PRIMARY KEY, app_id TEXT NOT NULL, author TEXT NOT NULL REFERENCES users(id), message TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'open', created_at INTEGER NOT NULL, resolved_at INTEGER
);
CREATE TABLE monitor_events (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, event_key TEXT NOT NULL UNIQUE, app_id TEXT NOT NULL,
 action TEXT NOT NULL, detail TEXT NOT NULL, actor TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE TRIGGER monitor_events_immutable BEFORE UPDATE ON monitor_events BEGIN SELECT RAISE(ABORT,'immutable_monitor_event'); END;
CREATE TRIGGER monitor_events_no_delete BEFORE DELETE ON monitor_events WHEN OLD.action!='upstream_observation_unavailable' BEGIN SELECT RAISE(ABORT,'immutable_monitor_event'); END;
PRAGMA user_version=6;
