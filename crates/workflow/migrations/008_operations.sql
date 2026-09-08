CREATE TABLE operation_weeks (week_start INTEGER PRIMARY KEY, week_end INTEGER NOT NULL, overdue INTEGER NOT NULL, sampled_at INTEGER NOT NULL);
CREATE TABLE release_evidence (gate TEXT PRIMARY KEY, outcome TEXT NOT NULL, report_url TEXT NOT NULL, report_digest TEXT NOT NULL, actor TEXT NOT NULL REFERENCES users(id), recorded_at INTEGER NOT NULL);
CREATE TRIGGER first_finding_response AFTER INSERT ON findings BEGIN
 UPDATE revisions SET first_response_at=COALESCE(first_response_at,NEW.created_at) WHERE id=NEW.revision_id;
END;
UPDATE revisions SET first_response_at=(SELECT MIN(created_at) FROM finding_history WHERE revision_id=revisions.id)
 WHERE (first_response_at IS NULL OR first_response_at>(SELECT MIN(created_at) FROM finding_history WHERE revision_id=revisions.id)) AND EXISTS(SELECT 1 FROM finding_history WHERE revision_id=revisions.id);
PRAGMA user_version=8;
