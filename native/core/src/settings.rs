//! Read-only settings proposals. Application and desktop settings use separate consent.
use crate::{
    library::Store,
    platform::{self, Result},
};
use omastore_catalogue::{settings::Choice, setups::SettingReference};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub const VERIFIED_SETTING_RELEASES: &[&str] = &[];
pub fn db<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|_| "local_database_unavailable")
}
pub fn migrate(c: &rusqlite::Connection) -> Result<()> {
    db(c.execute_batch("CREATE TABLE IF NOT EXISTS settings_plans(id TEXT PRIMARY KEY,body TEXT NOT NULL,created_at INTEGER NOT NULL);"))?;
    let v: i64 = db(c.query_row("PRAGMA user_version", [], |r| r.get(0)))?;
    if v < 3 {
        db(c.execute_batch("PRAGMA user_version=3;"))?;
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub settings: Vec<SettingReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Effect {
    pub reference: SettingReference,
    pub choice: Choice,
    pub before: Value,
    pub desired: Value,
    pub scope: String,
    pub restoration: String,
    pub state: String,
    pub reason: String,
    pub fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub request: Request,
    pub simulated: bool,
    pub host_version: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub effects: Vec<Effect>,
    pub material_digest: String,
    pub digest: String,
    pub can_apply: bool,
    pub notice: String,
}
pub struct Environment {
    pub root: PathBuf,
    pub simulated: bool,
    pub version: String,
    pub supported: bool,
}
pub struct Snapshot {
    pub value: Value,
    pub fingerprint: String,
    pub mode: u32,
}
pub fn directory(store: &Store) -> Result<PathBuf> {
    store
        .connection
        .path()
        .and_then(|s| Path::new(s).parent())
        .map(PathBuf::from)
        .ok_or("local_state_unavailable")
}
pub fn environment(store: &Store) -> Result<Environment> {
    if store.demo {
        return Ok(Environment {
            root: directory(store)?.join("desktop-fixture"),
            simulated: true,
            version: "fictional".into(),
            supported: true,
        });
    }
    let host = platform::probe();
    let root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or("local_state_unavailable")?;
    Ok(Environment {
        root,
        simulated: false,
        version: host.omarchy_version.unwrap_or_else(|| "unknown".into()),
        supported: host.state == "supported" && !host.locked,
    })
}
pub fn path(env: &Environment, choice: &Choice) -> PathBuf {
    env.root.join(match choice {
        Choice::Theme { .. } => {
            if env.simulated {
                "theme.json"
            } else {
                ".local/state/omarchy/current/theme.name"
            }
        }
        Choice::ClockPlacement { .. } => {
            if env.simulated {
                "shell.json"
            } else {
                ".config/omarchy/shell.json"
            }
        }
    })
}
pub fn safe_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err("unsafe_settings_path");
    }
    for p in path.ancestors() {
        if fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("unsafe_settings_path");
        }
    }
    Ok(())
}
pub fn open_parent(path: &Path) -> Result<fs::File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    if !path.is_absolute() {
        return Err("unsafe_settings_path");
    }
    let mut dir = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open("/")
        .map_err(|_| "settings_read_failed")?;
    for component in path.parent().ok_or("unsafe_settings_path")?.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::Normal(part) => {
                let name = std::ffi::CString::new(part.as_encoded_bytes())
                    .map_err(|_| "unsafe_settings_path")?;
                let fd = unsafe {
                    libc::openat(
                        dir.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
                if fd < 0 {
                    return Err("unsafe_settings_path");
                }
                dir = unsafe { fs::File::from_raw_fd(fd) };
            }
            _ => return Err("unsafe_settings_path"),
        }
    }
    Ok(dir)
}
pub fn snapshot(env: &Environment, choice: &Choice) -> Result<Snapshot> {
    let path = path(env, choice);
    safe_path(&path)?;
    let existed = path.try_exists().map_err(|_| "settings_read_failed")?;
    let (body, mode, identity) = if existed {
        use std::os::fd::{AsRawFd, FromRawFd};
        let dir = open_parent(&path)?;
        let name = std::ffi::CString::new(
            path.file_name()
                .ok_or("unsafe_settings_path")?
                .as_encoded_bytes(),
        )
        .map_err(|_| "unsafe_settings_path")?;
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err("settings_read_failed");
        }
        let mut f = unsafe { fs::File::from_raw_fd(fd) };
        let m = f.metadata().map_err(|_| "settings_read_failed")?;
        if !m.is_file()
            || m.len() > 128 * 1024
            || m.mode() & 0o022 != 0
            || m.uid() != unsafe { libc::geteuid() }
        {
            return Err("unsafe_settings_file");
        }
        let mut body = Vec::new();
        f.by_ref()
            .take(128 * 1024 + 1)
            .read_to_end(&mut body)
            .map_err(|_| "settings_read_failed")?;
        if body.len() > 128 * 1024 {
            return Err("settings_file_too_large");
        }
        (
            body,
            m.mode() & 0o777,
            json!({"device":m.dev(),"inode":m.ino(),"mode":m.mode()}),
        )
    } else if env.simulated {
        let value = match choice {
            Choice::Theme { .. } => json!({"theme":"sample-ink"}),
            Choice::ClockPlacement { .. } => {
                json!({"version":1,"bar":{"layout":{"left":[{"id":"omarchy.menu"}],"center":[{"id":"omarchy.indicators"},{"id":"omarchy.clock","format":"HH:mm"}],"right":[{"id":"omarchy.power"}]}},"plugins":[],"fixtureNote":"fictional desktop"})
            }
        };
        (
            serde_json::to_vec(&value).map_err(|_| "settings_unavailable")?,
            0o600,
            Value::Null,
        )
    } else {
        return Err("settings_not_observed");
    };
    let value = if !env.simulated && matches!(choice, Choice::Theme { .. }) {
        let name = std::str::from_utf8(&body)
            .map_err(|_| "malformed_settings")?
            .trim();
        if !omastore_catalogue::token(name) {
            return Err("malformed_settings");
        }
        json!({"theme":name})
    } else {
        serde_json::from_slice(&body).map_err(|_| "malformed_settings")?
    };
    let fingerprint = platform::hash(
        &json!({"bytes":platform::digest(&body),"identity":identity,"existed":existed,"version":env.version,"simulated":env.simulated}),
    );
    Ok(Snapshot {
        value,
        fingerprint,
        mode,
    })
}
pub fn layout(value: &Value) -> Result<&Value> {
    if value["version"] != 1
        || value["bar"]["id"]
            .as_str()
            .is_some_and(|id| id != "omarchy.bar")
    {
        return Err("unsupported_bar_configuration");
    }
    let layout = &value["bar"]["layout"];
    for section in ["left", "center", "right"] {
        let entries = layout[section].as_array().ok_or("malformed_bar_layout")?;
        if entries.len() > 128
            || entries
                .iter()
                .any(|e| !e.is_object() || !e["id"].is_string())
        {
            return Err("malformed_bar_layout");
        }
    }
    Ok(layout)
}
pub fn clock(value: &Value) -> Result<Value> {
    let layout = layout(value)?;
    let mut locations = Vec::new();
    for section in ["left", "center", "right"] {
        for (index, entry) in layout[section].as_array().unwrap().iter().enumerate() {
            if entry["id"] == "omarchy.clock" {
                locations.push(json!({"section":section,"index":index}));
            }
        }
    }
    if locations.len() != 1 {
        return Err("existing_single_clock_required");
    }
    Ok(locations.remove(0))
}
pub fn build(request: Request, env: &Environment, now: i64) -> Result<Plan> {
    if request.settings.is_empty()
        || request.settings.len() > 2
        || request
            .settings
            .iter()
            .map(|s| &s.adapter)
            .collect::<BTreeSet<_>>()
            .len()
            != request.settings.len()
    {
        return Err("select_supported_settings");
    }
    let definitions = omastore_catalogue::settings::adapters(env.simulated);
    let mut effects = Vec::new();
    for reference in &request.settings {
        let choice = omastore_catalogue::settings::resolve(reference, env.simulated)?;
        let desired = match &choice {
            Choice::Theme { theme_id } => json!(theme_id),
            Choice::ClockPlacement { section, index } => json!({"section":section,"index":index}),
        };
        let observed = snapshot(env, &choice).and_then(|s| {
            let before = match choice {
                Choice::Theme { .. } => json!(s.value["theme"]
                    .as_str()
                    .filter(|v| omastore_catalogue::token(v))
                    .ok_or("malformed_settings")?),
                Choice::ClockPlacement { .. } => clock(&s.value)?,
            };
            Ok((s, before))
        });
        let (before, fingerprint, mut state, mut reason) = match observed {
            Ok((s, before)) => {
                let same = before == desired;
                (
                    before,
                    s.fingerprint,
                    if same { "noop" } else { "change" }.into(),
                    if same {
                        "Already has the selected value"
                    } else {
                        "Review the exact setting change"
                    }
                    .into(),
                )
            }
            Err(code) => (
                Value::Null,
                String::new(),
                "unsupported".into(),
                String::from(code),
            ),
        };
        if !env.supported {
            state = "unsupported".into();
            reason = "Current Omarchy environment is unavailable".into();
        }
        if !env.simulated
            && !VERIFIED_SETTING_RELEASES.contains(&env.version.as_str())
            && state == "change"
        {
            state = "unverified".into();
            reason =
                "This Omarchy version has no recorded settings adapter acceptance report".into();
        }
        let d = definitions["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == reference.adapter)
            .unwrap();
        effects.push(Effect {
            reference: reference.clone(),
            choice,
            before,
            desired,
            scope: d["scope"].as_str().unwrap().into(),
            restoration: d["restoration"].as_str().unwrap().into(),
            state,
            reason,
            fingerprint,
        });
    }
    let material_digest = platform::hash(
        &json!({"request":request,"effects":effects,"version":env.version,"simulated":env.simulated}),
    );
    let can_apply = effects
        .iter()
        .all(|e| ["change", "noop"].contains(&e.state.as_str()))
        && effects.iter().any(|e| e.state == "change");
    let mut p=Plan{schema_version:1,request,simulated:env.simulated,host_version:env.version.clone(),created_at:now,expires_at:now+600,effects,material_digest,digest:String::new(),can_apply,notice:if env.simulated{"Fictional settings diff. No real desktop configuration was read or changed."}else{"Read-only proposal. Application installation and desktop settings have separate consent."}.into()};
    p.digest = platform::hash(&p);
    Ok(p)
}
pub fn save(store: &Store, plan: &Plan) -> Result<Value> {
    db(store.connection.execute(
        "DELETE FROM settings_plans WHERE created_at<?1 AND NOT EXISTS(SELECT 1 FROM setting_steps s WHERE s.plan_id=settings_plans.id)",
        [plan.created_at - 7 * 86400],
    ))?;
    let count: i64 =
        db(store
            .connection
            .query_row("SELECT count(*) FROM settings_plans p WHERE NOT EXISTS(SELECT 1 FROM setting_steps s WHERE s.plan_id=p.id)", [], |r| r.get(0)))?;
    if count >= 100 {
        return Err("settings_plan_limit");
    }
    db(store.connection.execute(
        "INSERT OR IGNORE INTO settings_plans VALUES(?1,?2,?3)",
        params![
            plan.digest,
            serde_json::to_string(plan).map_err(|_| "settings_unavailable")?,
            plan.created_at
        ],
    ))?;
    serde_json::to_value(plan).map_err(|_| "settings_unavailable")
}
pub fn stored(store: &Store, id: &str) -> Result<Plan> {
    let raw: Option<String> = db(store
        .connection
        .query_row("SELECT body FROM settings_plans WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional())?;
    let mut p: Plan = serde_json::from_str(&raw.ok_or("settings_plan_not_found")?)
        .map_err(|_| "settings_plan_invalid")?;
    let digest = std::mem::take(&mut p.digest);
    if platform::hash(&p) != digest || id != digest || p.simulated != store.demo {
        return Err("settings_plan_invalid");
    }
    p.digest = digest;
    Ok(p)
}
pub fn request(store: &Store, method: &str, params: Value, now: i64) -> Result<Value> {
    match method {
        "settings.adapters" if params == json!({}) => {
            Ok(omastore_catalogue::settings::adapters(store.demo))
        }
        "settings.preview" => {
            let p: Request =
                serde_json::from_value(params).map_err(|_| "invalid_settings_request")?;
            save(store, &build(p, &environment(store)?, now)?)
        }
        "settings.get" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Lookup {
                id: String,
            }
            let p: Lookup =
                serde_json::from_value(params).map_err(|_| "invalid_settings_request")?;
            serde_json::to_value(stored(store, &p.id)?).map_err(|_| "settings_unavailable")
        }
        _ => crate::settings_apply::request(store, method, params, now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn choice() -> SettingReference {
        SettingReference {
            adapter: "omarchy-clock-placement".into(),
            value_id: "right-start".into(),
            revision: "1".into(),
        }
    }
    #[test]
    fn diff_is_read_only_and_reports_missing_plugins_and_version_changes() {
        let d = tempfile::tempdir().unwrap();
        let mut env = Environment {
            root: d.path().join("desktop"),
            simulated: true,
            version: "fictional".into(),
            supported: true,
        };
        let request = Request {
            settings: vec![choice()],
        };
        let p = build(request.clone(), &env, 1000).unwrap();
        assert!(p.can_apply);
        assert!(!env.root.exists());
        assert_eq!(p.effects[0].before, json!({"section":"center","index":1}));
        crate::library::private_directory(&env.root).unwrap();
        fs::write(
            env.root.join("shell.json"),
            json!({"version":1,"bar":{"layout":{"left":[],"center":[],"right":[]}}}).to_string(),
        )
        .unwrap();
        let missing = build(request.clone(), &env, 1000).unwrap();
        assert_eq!(missing.effects[0].reason, "existing_single_clock_required");
        assert!(!missing.can_apply);
        env.simulated = false;
        env.version = "changed".into();
        assert!(!build(request, &env, 1000).unwrap().can_apply);
    }
    #[test]
    fn symlink_and_malformed_config_never_enter_an_applicable_diff() {
        use std::os::unix::fs::symlink;
        let d = tempfile::tempdir().unwrap();
        let env = Environment {
            root: d.path().join("desktop"),
            simulated: true,
            version: "fictional".into(),
            supported: true,
        };
        crate::library::private_directory(&env.root).unwrap();
        let secret = d.path().join("private");
        fs::write(&secret, "secret").unwrap();
        symlink(&secret, env.root.join("shell.json")).unwrap();
        let p = build(
            Request {
                settings: vec![choice()],
            },
            &env,
            1000,
        )
        .unwrap();
        assert!(!p.can_apply);
        assert_eq!(p.effects[0].reason, "unsafe_settings_path");
        assert!(!serde_json::to_string(&p).unwrap().contains("secret"));
    }
}
