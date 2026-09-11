CREATE TABLE commerce_model (version INTEGER PRIMARY KEY, body TEXT NOT NULL, report_digest TEXT NOT NULL, actor TEXT NOT NULL REFERENCES users(id), recorded_at INTEGER NOT NULL);
CREATE TRIGGER commerce_model_immutable_update BEFORE UPDATE ON commerce_model BEGIN SELECT RAISE(ABORT,'immutable commercial evidence'); END;
CREATE TRIGGER commerce_model_immutable_delete BEFORE DELETE ON commerce_model BEGIN SELECT RAISE(ABORT,'retain commercial evidence'); END;
INSERT INTO metadata VALUES('commerce_paused','1');
PRAGMA user_version=9;
