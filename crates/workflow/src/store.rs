use crate::{digest, Error, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

#[derive(Clone)]
pub struct Store {
    db: Arc<Mutex<Connection>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Actor {
    pub id: String,
    pub login: String,
    pub roles: Vec<String>,
}
impl Actor {
    pub fn require(&self, role: &str) -> Result<()> {
        if self.roles.iter().any(|r| r == role) {
            Ok(())
        } else {
            Err(Error::new(403, "role_required"))
        }
    }
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_mode(path, false)
    }
    #[cfg(feature = "development-workflow")]
    pub fn development(path: &Path) -> Result<Self> {
        Self::open_mode(path, true)
    }
    fn open_mode(path: &Path, development: bool) -> Result<Self> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
            }
            if std::fs::symlink_metadata(parent)?.file_type().is_symlink() {
                return Err(Error::new(500, "unsafe_storage_path"));
            }
        }
        match std::fs::symlink_metadata(path) {
            Ok(m) if !m.is_file() || m.permissions().mode() & 0o077 != 0 => {
                return Err(Error::new(500, "private_database_required"))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)?;
            }
            Err(e) => return Err(e.into()),
        }
        Self::from_connection(Connection::open(path)?, development)
    }
    pub fn memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?, false)
    }
    fn from_connection(mut c: Connection, development: bool) -> Result<Self> {
        c.busy_timeout(Duration::from_secs(3))?;
        c.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: u32 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 9 {
            return Err(Error::new(500, "unsupported_database_version"));
        }
        if version > 0 {
            let expected = if development {
                "development"
            } else {
                "production"
            };
            let actual: Option<String> = c
                .query_row(
                    "SELECT value FROM metadata WHERE key='environment'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if actual.as_deref() != Some(expected) {
                return Err(Error::new(500, "database_environment_mismatch"));
            }
        }
        if version == 0 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/001_identity.sql"))?;
            t.commit()?;
        }
        let environment = if development {
            "development"
        } else {
            "production"
        };
        c.execute(
            "INSERT OR IGNORE INTO metadata VALUES('environment',?1)",
            [environment],
        )?;
        let actual: String = c.query_row(
            "SELECT value FROM metadata WHERE key='environment'",
            [],
            |r| r.get(0),
        )?;
        if actual != environment {
            return Err(Error::new(500, "database_environment_mismatch"));
        }
        if version < 2 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/002_drafts.sql"))?;
            t.commit()?;
        }
        if version < 3 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/003_checks.sql"))?;
            t.commit()?;
        }
        if version < 4 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/004_review.sql"))?;
            t.commit()?;
        }
        if version < 5 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/005_publication.sql"))?;
            t.commit()?;
        }
        if version < 6 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/006_monitoring.sql"))?;
            t.commit()?;
        }
        if version < 7 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/007_editorial.sql"))?;
            t.commit()?;
        }
        if version < 8 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/008_operations.sql"))?;
            t.commit()?;
        }
        if version < 9 {
            let t = c.transaction()?;
            t.execute_batch(include_str!("../migrations/009_commerce_model.sql"))?;
            t.commit()?;
        }
        Ok(Self {
            db: Arc::new(Mutex::new(c)),
        })
    }
    pub(crate) fn connection(&self) -> Result<MutexGuard<'_, Connection>> {
        self.db
            .lock()
            .map_err(|_| Error::new(500, "storage_unavailable"))
    }
    pub(crate) fn transaction<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut c = self.connection()?;
        let t = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let value = f(&t)?;
        t.commit()?;
        Ok(value)
    }
    pub fn actor(&self, token: &str, now: i64) -> Result<Actor> {
        if token.len() != 64 {
            return Err(Error::new(401, "sign_in_required"));
        }
        let c = self.connection()?;
        let row = c.query_row("SELECT u.id,u.login FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=?1 AND s.expires_at>?2 AND s.revoked_at IS NULL AND u.active=1", params![digest(token),now], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
        let (id, login) = row.ok_or(Error::new(401, "session_expired"))?;
        let roles = roles(&c, &id)?;
        Ok(Actor { id, login, roles })
    }
    pub fn revoke_session(&self, token: &str, now: i64) -> Result<()> {
        self.connection()?.execute(
            "UPDATE sessions SET revoked_at=?2 WHERE token_hash=?1",
            params![digest(token), now],
        )?;
        Ok(())
    }
    /// Local operator command. Not a public bootstrap or role-grant endpoint.
    pub fn set_role(&self, user: &str, role: &str, enabled: bool, now: i64) -> Result<()> {
        if !["author", "reviewer", "maintainer", "operator", "editor"].contains(&role) {
            return Err(Error::new(422, "invalid_role"));
        }
        self.transaction(|t| {
            if enabled {
                t.execute(
                    "INSERT OR IGNORE INTO roles(user_id,role) VALUES(?1,?2)",
                    params![user, role],
                )?;
            } else {
                t.execute(
                    "DELETE FROM roles WHERE user_id=?1 AND role=?2",
                    params![user, role],
                )?;
            }
            audit(
                t,
                "local_operator",
                "role_changed",
                user,
                now,
                &json!({"role":role,"enabled":enabled}),
            )
        })
    }
    pub fn check_rate(&self, bucket: &str, now: i64, limit: u32) -> Result<()> {
        self.transaction(|t| {
            let window = now / 60;
            t.execute("INSERT INTO rate_limits VALUES(?1,?2,1) ON CONFLICT(bucket) DO UPDATE SET count=CASE WHEN window=excluded.window THEN count+1 ELSE 1 END,window=excluded.window",params![bucket,window])?;
            let n:u32=t.query_row("SELECT count FROM rate_limits WHERE bucket=?1",[bucket],|r|r.get(0))?;
            if n>limit { return Err(Error::new(429,"rate_limited")); } Ok(())
        })
    }
    pub fn claims(&self, actor: &Actor, now: i64) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        let mut s=c.prepare("SELECT target,proof_url,verified_at,expires_at,revoked_at FROM claims WHERE user_id=?1 ORDER BY target")?;
        let rows=s.query_map([&actor.id],|r|{let expires:i64=r.get(3)?; let revoked:Option<i64>=r.get(4)?;Ok(json!({"target":r.get::<_,String>(0)?,"proofUrl":r.get::<_,String>(1)?,"verifiedAt":r.get::<_,i64>(2)?,"expiresAt":expires,"active":expires>now&&revoked.is_none()}))})?;
        Ok(Value::Array(
            rows.collect::<std::result::Result<Vec<_>, _>>()?,
        ))
    }
    pub fn admin_users(&self) -> Result<Value> {
        let c = self.connection()?;
        let mut s = c.prepare("SELECT id,login,active FROM users ORDER BY id")?;
        let rows=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"login":r.get::<_,String>(1)?,"active":r.get::<_,bool>(2)?})))?;
        Ok(json!(rows.collect::<std::result::Result<Vec<_>, _>>()?))
    }
}
pub(crate) fn roles(c: &Connection, user: &str) -> Result<Vec<String>> {
    let mut s = c.prepare("SELECT role FROM roles WHERE user_id=?1 ORDER BY role")?;
    let values = s
        .query_map([user], |r| r.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(values)
}
pub(crate) fn recheck(c: &Connection, actor: &Actor, role: &str) -> Result<()> {
    let valid:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM users u JOIN roles r ON r.user_id=u.id WHERE u.id=?1 AND u.active=1 AND r.role=?2)",params![actor.id,role],|r|r.get(0))?;
    if valid {
        Ok(())
    } else {
        Err(Error::new(403, "role_revoked"))
    }
}
pub(crate) fn audit(
    t: &Transaction<'_>,
    actor: &str,
    action: &str,
    target: &str,
    now: i64,
    detail: &Value,
) -> Result<()> {
    t.execute(
        "INSERT INTO audit(actor,action,target,at,detail) VALUES(?1,?2,?3,?4,?5)",
        params![actor, action, target, now, detail.to_string()],
    )?;
    Ok(())
}
