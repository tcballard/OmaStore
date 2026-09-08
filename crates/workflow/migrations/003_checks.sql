CREATE TABLE runtime_evidence (
 id TEXT PRIMARY KEY, revision_id TEXT NOT NULL REFERENCES revisions(id),
 actor TEXT NOT NULL REFERENCES users(id), app_id TEXT NOT NULL, release_id TEXT NOT NULL,
 candidate_digest TEXT NOT NULL, body TEXT NOT NULL, created_at INTEGER NOT NULL,
 UNIQUE(revision_id,app_id,release_id,actor)
);
CREATE TRIGGER runtime_evidence_immutable BEFORE UPDATE ON runtime_evidence
 BEGIN SELECT RAISE(ABORT,'immutable_runtime_evidence'); END;
CREATE TABLE job_events (
 id INTEGER PRIMARY KEY AUTOINCREMENT, job_id TEXT NOT NULL REFERENCES jobs(id),
 event TEXT NOT NULL, code TEXT NOT NULL, created_at INTEGER NOT NULL
);
PRAGMA user_version=3;
