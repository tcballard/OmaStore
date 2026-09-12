//! Private local observations and the durable operation ledger. No inventory upload.
use crate::{
    planner::Plan,
    platform::{self, Host, Result},
};
use omastore_catalogue::{Catalogue, InstallRoute, ReleaseIdentity};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(feature = "development-catalogue")]
use std::collections::BTreeMap;

pub struct Store {
    pub connection: Connection,
    pub demo: bool,
}
fn db<T>(result: rusqlite::Result<T>) -> Result<T> {
    result.map_err(|_| "local_database_unavailable")
}
pub fn directory(demo: bool) -> Result<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".local/state")))
        .ok_or("local_state_unavailable")?;
    if !base.is_absolute() {
        return Err("local_state_unavailable");
    }
    Ok(base
        .join(if demo { "omastore-sample" } else { "omastore" })
        .join("native"))
}
pub fn private_directory(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err("unsafe_local_path");
    }
    for ancestor in path.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("unsafe_local_path");
        }
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| "local_state_unavailable")?;
    let metadata = fs::metadata(path).map_err(|_| "local_state_unavailable")?;
    let uid = fs::metadata("/proc/self")
        .map_err(|_| "local_state_unavailable")?
        .uid();
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err("unsafe_local_permissions");
    }
    Ok(())
}
impl Store {
    pub fn open(demo: bool) -> Result<Self> {
        Self::at(&directory(demo)?, demo)
    }
    pub fn at(directory: &Path, demo: bool) -> Result<Self> {
        private_directory(directory)?;
        let path = directory.join("library.db");
        for suffix in ["library.db", "library.db-wal", "library.db-shm"] {
            if fs::symlink_metadata(directory.join(suffix))
                .is_ok_and(|m| !m.is_file() || m.mode() & 0o077 != 0)
            {
                return Err("unsafe_local_database");
            }
        }
        if !path.exists() {
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&path)
                .map_err(|_| "local_state_unavailable")?;
        }
        let connection = db(Connection::open(path))?;
        db(connection.busy_timeout(Duration::from_secs(3)))?;
        let schema: i64 = db(connection.query_row("PRAGMA user_version", [], |r| r.get(0)))?;
        if schema > 5 {
            return Err("local_schema_upgrade_required");
        }
        let has_meta: bool = db(connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='local_meta')",
            [],
            |r| r.get(0),
        ))?;
        if has_meta {
            let existing: Option<String> = db(connection
                .query_row("SELECT value FROM local_meta WHERE key='mode'", [], |r| {
                    r.get(0)
                })
                .optional())?;
            if existing.as_deref() != Some(if demo { "sample" } else { "system" }) {
                return Err("local_mode_mismatch");
            }
        }
        db(connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS local_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS installed_apps(id TEXT PRIMARY KEY,name TEXT NOT NULL,package TEXT NOT NULL,repository TEXT NOT NULL,observed_version TEXT NOT NULL,present INTEGER NOT NULL,last_seen_at INTEGER NOT NULL,preexisting INTEGER NOT NULL DEFAULT 1,source_state TEXT NOT NULL,catalogue_release TEXT NOT NULL,catalogue_version TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS setup_refs(setup_id TEXT NOT NULL,revision TEXT NOT NULL,app_id TEXT NOT NULL,added_at INTEGER NOT NULL,PRIMARY KEY(setup_id,app_id));
            CREATE TABLE IF NOT EXISTS operations(id TEXT PRIMARY KEY,plan_json TEXT NOT NULL,state TEXT NOT NULL,version INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS operation_events(sequence INTEGER PRIMARY KEY AUTOINCREMENT,operation_id TEXT NOT NULL REFERENCES operations(id),state TEXT NOT NULL,code TEXT NOT NULL,at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS sample_packages(name TEXT PRIMARY KEY,version TEXT NOT NULL);
"))?;
        let mode = if demo { "sample" } else { "system" };
        let stored: Option<String> = db(connection
            .query_row("SELECT value FROM local_meta WHERE key='mode'", [], |r| {
                r.get(0)
            })
            .optional())?;
        if stored.as_deref().is_some_and(|v| v != mode) {
            return Err("local_mode_mismatch");
        }
        db(connection.execute(
            "INSERT OR IGNORE INTO local_meta(key,value) VALUES('mode',?1)",
            [mode],
        ))?;
        crate::lifecycle::migrate(&connection)?;
        crate::settings::migrate(&connection)?;
        crate::settings_apply::migrate(&connection)?;
        crate::remixes::migrate(&connection)?;
        Ok(Self { connection, demo })
    }
    pub fn observe(&mut self, c: &Catalogue, h: &Host, now: i64) -> Result<()> {
        if h.simulated != self.demo {
            return Err("local_mode_mismatch");
        }
        if h.state != "supported" || h.locked {
            return Ok(());
        }
        let tx = db(self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate))?;
        let previous: Vec<(String, String)> = {
            let mut q = db(tx.prepare("SELECT id,package FROM installed_apps"))?;
            let rows = db(q.query_map([], |r| Ok((r.get(0)?, r.get(1)?))))?;
            db(rows.collect())?
        };
        for (id, package) in previous {
            match h.installed.get(&package) {
                Some(version) => {
                    db(tx.execute("UPDATE installed_apps SET present=1,observed_version=?2,last_seen_at=?3,source_state=CASE WHEN present=0 THEN 'reappeared' WHEN observed_version=?2 THEN source_state ELSE 'external_change' END WHERE id=?1",params![id,version,now]))?;
                }
                None => {
                    db(tx.execute("UPDATE installed_apps SET present=0,last_seen_at=?2,source_state='absent' WHERE id=?1",params![id,now]))?;
                }
            }
        }
        for app in &c.apps {
            let release = app.current_release();
            if let InstallRoute::ArchPackage {
                repository,
                package,
                ..
            } = &release.route
            {
                if let Some(installed) = h.installed.get(package) {
                    let package_version = match &release.identity {
                        ReleaseIdentity::RepositoryPackage { version, .. } => version,
                        _ => &release.version,
                    };
                    db(tx.execute("INSERT INTO installed_apps(id,name,package,repository,observed_version,present,last_seen_at,preexisting,source_state,catalogue_release,catalogue_version) VALUES(?1,?2,?3,?4,?5,1,?6,1,'observed_locally',?7,?8) ON CONFLICT(id) DO UPDATE SET name=excluded.name,package=excluded.package,repository=excluded.repository,observed_version=excluded.observed_version,present=1,last_seen_at=excluded.last_seen_at,catalogue_release=excluded.catalogue_release,catalogue_version=excluded.catalogue_version",params![app.id,app.name,package,repository,installed,now,release.id,package_version]))?;
                }
            }
        }
        db(tx.execute("INSERT INTO local_meta(key,value) VALUES('last_observation',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[now.to_string()]))?;
        db(tx.commit())
    }
    pub fn save_plan(&mut self, plan: &Plan) -> Result<()> {
        if plan.simulated != self.demo {
            return Err("local_mode_mismatch");
        }
        let tx = db(self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate))?;
        db(tx.execute("DELETE FROM operation_events WHERE operation_id IN (SELECT id FROM operations WHERE state='planned' AND created_at<?1)",[plan.created_at-7*86400]))?;
        db(tx.execute(
            "DELETE FROM operations WHERE state='planned' AND created_at<?1",
            [plan.created_at - 7 * 86400],
        ))?;
        let pending: i64 = db(tx.query_row(
            "SELECT count(*) FROM operations WHERE state='planned'",
            [],
            |r| r.get(0),
        ))?;
        let exists: bool = db(tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM operations WHERE id=?1)",
            [&plan.digest],
            |r| r.get(0),
        ))?;
        if pending >= 100 && !exists {
            return Err("too_many_local_proposals");
        }
        let text = serde_json::to_string(plan).map_err(|_| "plan_invalid")?;
        let inserted=db(tx.execute("INSERT OR IGNORE INTO operations(id,plan_json,state,created_at,updated_at) VALUES(?1,?2,'planned',?3,?3)",params![plan.digest,text,plan.created_at]))?;
        if inserted == 1 {
            db(tx.execute("INSERT INTO operation_events(operation_id,state,code,at) VALUES(?1,'planned','proposal_saved',?2)",params![plan.digest,plan.created_at]))?;
        }
        db(tx.commit())
    }
    pub fn plan(&self, id: &str) -> Result<Plan> {
        if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("invalid_operation_id");
        }
        let raw: String = db(self
            .connection
            .query_row("SELECT plan_json FROM operations WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional())?
        .ok_or("operation_not_found")?;
        let plan: Plan = serde_json::from_str(&raw).map_err(|_| "local_plan_invalid")?;
        let mut check = plan.clone();
        check.digest.clear();
        if plan.digest != id || platform::hash(&check) != id || plan.simulated != self.demo {
            return Err("local_plan_invalid");
        }
        Ok(plan)
    }
    #[cfg(feature = "development-catalogue")]
    pub fn sample_packages(&self) -> Result<BTreeMap<String, String>> {
        if !self.demo {
            return Err("sample_mode_required");
        }
        let mut q = db(self
            .connection
            .prepare("SELECT name,version FROM sample_packages ORDER BY name"))?;
        let rows = db(q.query_map([], |r| Ok((r.get(0)?, r.get(1)?))))?;
        db(rows.collect())
    }
    pub fn view(&self, offset: u32) -> Result<Value> {
        if offset > 20000 {
            return Err("invalid_offset");
        }
        let mut q=db(self.connection.prepare("SELECT id,name,package,repository,observed_version,present,last_seen_at,preexisting,source_state,catalogue_release,catalogue_version FROM installed_apps ORDER BY lower(name),id LIMIT 30 OFFSET ?1"))?;
        let rows:Vec<Value>=db(db(q.query_map([offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"package":r.get::<_,String>(2)?,"repository":r.get::<_,String>(3)?,"installedVersion":r.get::<_,String>(4)?,"present":r.get::<_,bool>(5)?,"observedAt":r.get::<_,i64>(6)?,"preexisting":r.get::<_,bool>(7)?,"sourceState":r.get::<_,String>(8)?,"catalogueReleaseId":r.get::<_,String>(9)?,"catalogueVersion":r.get::<_,String>(10)?}))))?.collect())?;
        let mut q=db(self.connection.prepare("SELECT id,state,version,created_at,updated_at FROM operations ORDER BY updated_at DESC,id LIMIT 50"))?;
        let operations:Vec<Value>=db(db(q.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?,"version":r.get::<_,i64>(2)?,"createdAt":r.get::<_,i64>(3)?,"updatedAt":r.get::<_,i64>(4)?}))))?.collect())?;
        let total: u32 =
            db(self
                .connection
                .query_row("SELECT count(*) FROM installed_apps", [], |r| r.get(0)))?;
        let sequence: i64 = db(self.connection.query_row(
            "SELECT coalesce(max(sequence),0) FROM operation_events",
            [],
            |r| r.get(0),
        ))?;
        let observed: Option<String> = db(self
            .connection
            .query_row(
                "SELECT value FROM local_meta WHERE key='last_observation'",
                [],
                |r| r.get(0),
            )
            .optional())?;
        Ok(
            json!({"schemaVersion":1,"items":rows,"total":total,"offset":offset,"nextOffset":if offset+30<total{Some(offset+30)}else{None},"operations":operations,"lastSequence":sequence,"observedAt":observed.and_then(|s|s.parse::<i64>().ok()),"simulated":self.demo,"notice":"Installed versions are local observations. They do not update catalogue test evidence or prove the repository of an external installation. Proposals do not mean installed."}),
        )
    }
    pub fn events(&self, id: &str, after: i64) -> Result<Value> {
        self.plan(id)?;
        let mut q=db(self.connection.prepare("SELECT sequence,state,code,at FROM operation_events WHERE operation_id=?1 AND sequence>?2 ORDER BY sequence LIMIT 100"))?;
        let rows:Vec<Value>=db(db(q.query_map(params![id,after],|r|Ok(json!({"sequence":r.get::<_,i64>(0)?,"state":r.get::<_,String>(1)?,"code":r.get::<_,String>(2)?,"at":r.get::<_,i64>(3)?}))))?.collect())?;
        Ok(json!({"id":id,"events":rows}))
    }
    pub fn launchers(&self, id: &str) -> Result<Vec<String>> {
        if !omastore_catalogue::token(id) {
            return Err("invalid_id");
        }
        let package: Option<String> = db(self
            .connection
            .query_row(
                "SELECT package FROM installed_apps WHERE id=?1 AND present=1",
                [id],
                |r| r.get(0),
            )
            .optional())?;
        let package = package.ok_or("installed_app_not_found")?;
        if !platform::package_token(&package) {
            return Err("invalid_package_target");
        }
        if self.demo {
            return Ok(vec!["fictional.desktop".into()]);
        }
        let installed = platform::read_command(
            "/usr/bin/pacman",
            &["-Q".into(), "--".into(), package.clone()],
        )?;
        if installed.code != 0 || !platform::installed(&installed.text)?.contains_key(&package) {
            return Err("installed_app_not_found");
        }
        let output =
            platform::read_command("/usr/bin/pacman", &["-Qlq".into(), "--".into(), package])?;
        if output.code != 0 {
            return Err("launcher_unavailable");
        }
        let mut entries = Vec::new();
        for line in output.text.lines() {
            let Some(name) = line.strip_prefix("/usr/share/applications/") else {
                continue;
            };
            if !name.ends_with(".desktop")
                || !platform::package_token(name)
                || platform::trusted_program(line).is_err()
            {
                continue;
            }
            if fs::metadata(line)
                .map_err(|_| "launcher_unavailable")?
                .len()
                > 65536
            {
                continue;
            }
            let bytes = fs::read(line).map_err(|_| "launcher_unavailable")?;
            if bytes.len() > 65536 {
                continue;
            }
            let text = String::from_utf8(bytes).map_err(|_| "launcher_unavailable")?;
            let mut section = false;
            let mut application = false;
            let mut hidden = false;
            for field in text.lines() {
                if field.starts_with('[') {
                    section = field == "[Desktop Entry]";
                }
                if section {
                    application |= field == "Type=Application";
                    hidden |= field == "Hidden=true" || field == "NoDisplay=true";
                }
            }
            if application && !hidden && entries.len() < 20 {
                entries.push(name.into());
            }
        }
        entries.sort();
        entries.dedup();
        Ok(entries)
    }
    pub fn launch(&self, id: &str, desktop: &str) -> Result<Value> {
        if !self.launchers(id)?.iter().any(|s| s == desktop) {
            return Err("launcher_unavailable");
        }
        if self.demo {
            return Ok(json!({"requested":true,"simulated":true}));
        }
        platform::trusted_program("/usr/bin/gio")?;
        let path = Path::new("/usr/share/applications").join(desktop);
        // GIO reads an exact system-owned desktop file, not an author command or a user override.
        let mut command = std::process::Command::new("/usr/bin/gio");
        command
            .args(["launch"])
            .arg(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.env_clear().env("PATH", "/usr/bin:/bin");
        for name in [
            "HOME",
            "XDG_RUNTIME_DIR",
            "DBUS_SESSION_BUS_ADDRESS",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "XDG_CURRENT_DESKTOP",
            "LANG",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let mut child = command.spawn().map_err(|_| "launcher_unavailable")?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(json!({"requested":true,"simulated":false}))
    }
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    #[test]
    fn observations_survive_restart_and_do_not_rewrite_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("native");
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let selection = crate::planner::Selection::App {
            id: "demo-fieldnotes".into(),
        };
        let now = 1_800_000_000;
        let (mut host, status, p) =
            crate::planner::sample(&c, &selection, BTreeMap::new(), now).unwrap();
        let plan =
            crate::planner::build(&c, selection, &host, &status, Ok(p.clone()), now).unwrap();
        let mut store = Store::at(&path, true).unwrap();
        store.save_plan(&plan).unwrap();
        store.save_plan(&plan).unwrap();
        assert_eq!(store.view(0).unwrap()["lastSequence"], 1);
        assert!(store.view(0).unwrap()["items"]
            .as_array()
            .unwrap()
            .is_empty());
        host.installed
            .insert(p[0].name.clone(), p[0].version.clone());
        store.observe(&c, &host, now).unwrap();
        drop(store);
        let mut store = Store::at(&path, true).unwrap();
        assert_eq!(store.plan(&plan.digest).unwrap().digest, plan.digest);
        assert_eq!(
            store.view(0).unwrap()["items"][0]["installedVersion"],
            p[0].version
        );
        host.installed.insert(p[0].name.clone(), "2.0-1".into());
        store.observe(&c, &host, now + 10).unwrap();
        let item = store.view(0).unwrap()["items"][0].clone();
        assert_eq!(item["installedVersion"], "2.0-1");
        assert_eq!(item["catalogueVersion"], p[0].version);
        assert_eq!(item["sourceState"], "external_change");
        host.state = "unknown".into();
        host.installed.clear();
        store.observe(&c, &host, now + 20).unwrap();
        assert_eq!(store.view(0).unwrap()["items"][0]["present"], true);
        host.state = "supported".into();
        host.locked = true;
        store.observe(&c, &host, now + 21).unwrap();
        assert_eq!(store.view(0).unwrap()["items"][0]["present"], true);
        host.locked = false;
        let mut changed = c.clone();
        let app = changed
            .apps
            .iter_mut()
            .find(|a| a.id == "demo-fieldnotes")
            .unwrap();
        let release = app
            .releases
            .iter_mut()
            .find(|r| r.id == app.current_release_id)
            .unwrap();
        if let InstallRoute::ArchPackage { package, .. } = &mut release.route {
            *package = "replacement-package".into();
        }
        host.installed
            .insert("replacement-package".into(), "3-1".into());
        store.observe(&changed, &host, now + 22).unwrap();
        let item = store.view(0).unwrap()["items"][0].clone();
        assert_eq!(item["package"], "replacement-package");
        assert_eq!(item["installedVersion"], "3-1");
        assert!(Store::at(&path, false).is_err());
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(Store::at(&link, true).is_err());
    }
}
