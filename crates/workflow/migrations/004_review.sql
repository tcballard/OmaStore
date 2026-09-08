CREATE TABLE review_decisions (
 id TEXT PRIMARY KEY, revision_id TEXT NOT NULL REFERENCES revisions(id), actor TEXT NOT NULL REFERENCES users(id),
 decision TEXT NOT NULL, reason TEXT NOT NULL, candidate_digest TEXT NOT NULL, policy TEXT NOT NULL,
 created_at INTEGER NOT NULL, UNIQUE(revision_id,actor,decision)
);
CREATE TRIGGER decisions_immutable BEFORE UPDATE ON review_decisions
 BEGIN SELECT RAISE(ABORT,'immutable_review_decision'); END;
CREATE TABLE approvals (
 id TEXT PRIMARY KEY, revision_id TEXT NOT NULL UNIQUE REFERENCES revisions(id), candidate_digest TEXT NOT NULL,
 payload TEXT NOT NULL, payload_digest TEXT NOT NULL, policy TEXT NOT NULL, approvers TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE TRIGGER approvals_immutable BEFORE UPDATE ON approvals
 BEGIN SELECT RAISE(ABORT,'immutable_approval'); END;
CREATE TRIGGER decisions_no_delete BEFORE DELETE ON review_decisions
 BEGIN SELECT RAISE(ABORT,'immutable_review_decision'); END;
CREATE TRIGGER approvals_no_delete BEFORE DELETE ON approvals
 BEGIN SELECT RAISE(ABORT,'immutable_approval'); END;
CREATE TRIGGER evidence_no_delete BEFORE DELETE ON runtime_evidence
 BEGIN SELECT RAISE(ABORT,'immutable_runtime_evidence'); END;
CREATE TABLE finding_history (
 sequence INTEGER PRIMARY KEY AUTOINCREMENT, revision_id TEXT NOT NULL,
 actor TEXT NOT NULL, check_name TEXT NOT NULL, tool_version TEXT NOT NULL,
 result TEXT NOT NULL, code TEXT NOT NULL, detail TEXT NOT NULL, private INTEGER NOT NULL, created_at INTEGER NOT NULL
);
INSERT INTO finding_history(revision_id,actor,check_name,tool_version,result,code,detail,private,created_at)
 SELECT revision_id,actor,check_name,tool_version,result,code,detail,private,created_at FROM findings;
CREATE TRIGGER finding_recorded AFTER INSERT ON findings BEGIN
 INSERT INTO finding_history(revision_id,actor,check_name,tool_version,result,code,detail,private,created_at)
 VALUES(NEW.revision_id,NEW.actor,NEW.check_name,NEW.tool_version,NEW.result,NEW.code,NEW.detail,NEW.private,NEW.created_at); END;
CREATE TRIGGER finding_rechecked AFTER UPDATE ON findings BEGIN
 INSERT INTO finding_history(revision_id,actor,check_name,tool_version,result,code,detail,private,created_at)
 VALUES(NEW.revision_id,NEW.actor,NEW.check_name,NEW.tool_version,NEW.result,NEW.code,NEW.detail,NEW.private,NEW.created_at); END;
CREATE TRIGGER finding_history_no_update BEFORE UPDATE ON finding_history BEGIN SELECT RAISE(ABORT,'immutable_finding_history'); END;
CREATE TRIGGER finding_history_no_delete BEFORE DELETE ON finding_history BEGIN SELECT RAISE(ABORT,'immutable_finding_history'); END;
PRAGMA user_version=4;
