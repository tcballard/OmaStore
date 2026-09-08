CREATE TABLE publications (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), repository TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL DEFAULT 'waiting_approval', intake_state TEXT NOT NULL DEFAULT 'queued',
 issue_number INTEGER, pr_number INTEGER, attached_pr INTEGER, attached_issue INTEGER, rebase_requested INTEGER NOT NULL DEFAULT 0, predecessor_sha TEXT, branch TEXT, base_sha TEXT, head_sha TEXT,
 base_registry TEXT, expected_registry TEXT, expected_files TEXT, merged_sha TEXT, delivered_revision TEXT,
 error TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
INSERT INTO publications(revision_id,created_at,updated_at) SELECT id,submitted_at,submitted_at FROM revisions;
INSERT OR IGNORE INTO jobs(id,revision_id,kind,state,due_at)
 SELECT 'intake-'||id,id,'intake','queued',submitted_at FROM revisions;
CREATE TABLE github_deliveries (
 id TEXT PRIMARY KEY, digest TEXT NOT NULL, event TEXT NOT NULL, payload TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'queued', created_at INTEGER NOT NULL
);
CREATE TABLE publication_events (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, revision_id TEXT NOT NULL REFERENCES revisions(id),
 action TEXT NOT NULL, detail TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE TRIGGER publication_events_immutable BEFORE UPDATE ON publication_events BEGIN SELECT RAISE(ABORT,'immutable_publication_event'); END;
CREATE TRIGGER publication_events_no_delete BEFORE DELETE ON publication_events BEGIN SELECT RAISE(ABORT,'immutable_publication_event'); END;
CREATE TABLE entity_owners (
 kind TEXT NOT NULL, entity_id TEXT NOT NULL, owner TEXT NOT NULL REFERENCES users(id),
 revision_id TEXT NOT NULL REFERENCES revisions(id), PRIMARY KEY(kind,entity_id)
);
CREATE TABLE github_checks (
 repository TEXT NOT NULL, pr_number INTEGER NOT NULL, head_sha TEXT NOT NULL, conclusion TEXT NOT NULL,
 check_id INTEGER NOT NULL, checked_at INTEGER NOT NULL, PRIMARY KEY(repository,pr_number)
);
CREATE TABLE worker_locks (name TEXT PRIMARY KEY, token TEXT NOT NULL, expires_at INTEGER NOT NULL);
PRAGMA user_version=5;
