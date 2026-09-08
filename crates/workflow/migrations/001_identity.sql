CREATE TABLE metadata (key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE users (
 id TEXT PRIMARY KEY, login TEXT NOT NULL, active INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL
);
CREATE TABLE roles (
 user_id TEXT NOT NULL REFERENCES users(id), role TEXT NOT NULL,
 PRIMARY KEY(user_id,role), CHECK(role IN ('author','reviewer','maintainer','operator','editor'))
);
CREATE TABLE sessions (
 token_hash TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id),
 expires_at INTEGER NOT NULL, revoked_at INTEGER
);
CREATE TABLE logins (
 id TEXT PRIMARY KEY, state_hash TEXT UNIQUE NOT NULL, desktop_challenge TEXT NOT NULL,
 provider_verifier TEXT NOT NULL, status TEXT NOT NULL, expires_at INTEGER NOT NULL,
 user_id TEXT REFERENCES users(id)
);
CREATE TABLE challenges (
 id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id), target TEXT NOT NULL,
 proof_url TEXT NOT NULL, issuer TEXT NOT NULL, nonce TEXT NOT NULL,
 expires_at INTEGER NOT NULL, consumed_at INTEGER
);
CREATE TABLE claims (
 target TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES users(id),
 proof_url TEXT NOT NULL, verified_at INTEGER NOT NULL, expires_at INTEGER NOT NULL,
 revoked_at INTEGER
);
CREATE TABLE audit (
 seq INTEGER PRIMARY KEY AUTOINCREMENT, actor TEXT NOT NULL, action TEXT NOT NULL,
 target TEXT NOT NULL, at INTEGER NOT NULL, detail TEXT NOT NULL
);
CREATE TRIGGER audit_no_update BEFORE UPDATE ON audit BEGIN SELECT RAISE(ABORT,'append_only'); END;
CREATE TRIGGER audit_no_delete BEFORE DELETE ON audit BEGIN SELECT RAISE(ABORT,'append_only'); END;
CREATE TABLE request_keys (
 actor TEXT NOT NULL, key TEXT NOT NULL, digest TEXT NOT NULL, response TEXT NOT NULL,
 created_at INTEGER NOT NULL, PRIMARY KEY(actor,key)
);
CREATE TABLE rate_limits (bucket TEXT PRIMARY KEY, window INTEGER NOT NULL, count INTEGER NOT NULL);
PRAGMA user_version=1;
