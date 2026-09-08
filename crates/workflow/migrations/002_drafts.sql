CREATE TABLE drafts (
 id TEXT PRIMARY KEY, owner TEXT NOT NULL REFERENCES users(id), kind TEXT NOT NULL,
 candidate TEXT NOT NULL, version INTEGER NOT NULL DEFAULT 1, base_revision TEXT,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, expiry_notice_at INTEGER,
 archived_at INTEGER, CHECK(kind IN ('app','setup','editorial'))
);
CREATE TABLE revisions (
 id TEXT PRIMARY KEY, draft_id TEXT NOT NULL REFERENCES drafts(id), owner TEXT NOT NULL REFERENCES users(id),
 number INTEGER NOT NULL, candidate TEXT NOT NULL, digest TEXT NOT NULL, state TEXT NOT NULL,
 version INTEGER NOT NULL DEFAULT 1, submitted_at INTEGER NOT NULL, first_response_at INTEGER,
 UNIQUE(draft_id,digest), UNIQUE(draft_id,number)
);
CREATE TRIGGER revision_content_immutable BEFORE UPDATE OF candidate,digest,owner,draft_id,number ON revisions
 BEGIN SELECT RAISE(ABORT,'immutable_candidate'); END;
CREATE TABLE media (
 id TEXT PRIMARY KEY, draft_id TEXT NOT NULL REFERENCES drafts(id), owner TEXT NOT NULL REFERENCES users(id),
 kind TEXT NOT NULL, digest TEXT NOT NULL, content_type TEXT NOT NULL, bytes INTEGER NOT NULL,
 width INTEGER NOT NULL, height INTEGER NOT NULL, duration_ms INTEGER,
 alt TEXT NOT NULL, rights TEXT NOT NULL, created_at INTEGER NOT NULL, public_at INTEGER,
 UNIQUE(draft_id,digest,kind)
);
CREATE TABLE jobs (
 id TEXT PRIMARY KEY, revision_id TEXT REFERENCES revisions(id), kind TEXT NOT NULL,
 state TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, due_at INTEGER NOT NULL,
 lease_token TEXT, lease_until INTEGER, last_error TEXT, UNIQUE(revision_id,kind)
);
CREATE TABLE findings (
 id TEXT PRIMARY KEY, revision_id TEXT NOT NULL REFERENCES revisions(id), actor TEXT NOT NULL,
 check_name TEXT NOT NULL, tool_version TEXT NOT NULL, result TEXT NOT NULL,
 code TEXT NOT NULL, detail TEXT NOT NULL, private INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL, UNIQUE(revision_id,check_name,tool_version,actor)
);
PRAGMA user_version=2;
