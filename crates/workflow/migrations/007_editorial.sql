CREATE TABLE feed_entries (
 guid TEXT PRIMARY KEY, app_id TEXT NOT NULL, release_id TEXT NOT NULL, name TEXT NOT NULL,
 summary TEXT NOT NULL, version TEXT NOT NULL, maker_ids TEXT NOT NULL, first_delivered_at INTEGER NOT NULL,
 latest_revision TEXT NOT NULL REFERENCES revisions(id), UNIQUE(app_id,release_id)
);
CREATE TRIGGER feed_identity_immutable BEFORE UPDATE OF guid,app_id,release_id,first_delivered_at ON feed_entries BEGIN SELECT RAISE(ABORT,'immutable_feed_identity'); END;
CREATE TRIGGER feed_history_no_delete BEFORE DELETE ON feed_entries BEGIN SELECT RAISE(ABORT,'immutable_feed_history'); END;
CREATE TABLE revision_context (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), catalogue_snapshot TEXT NOT NULL,
 apps TEXT NOT NULL, makers TEXT NOT NULL
);
CREATE TRIGGER revision_context_immutable BEFORE UPDATE ON revision_context BEGIN SELECT RAISE(ABORT,'immutable_revision_context'); END;
CREATE TRIGGER revision_context_no_delete BEFORE DELETE ON revision_context BEGIN SELECT RAISE(ABORT,'immutable_revision_context'); END;
PRAGMA user_version=7;
