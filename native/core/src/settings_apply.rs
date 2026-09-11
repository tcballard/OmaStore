//! Durable, narrow setting ownership; no arbitrary paths or commands in the protocol.
use crate::{
    library::Store,
    platform::{self, Result},
    settings::{self, db, Environment, Plan, Snapshot},
};
use omastore_catalogue::settings::Choice;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
};

pub fn migrate(c: &rusqlite::Connection) -> Result<()> {
    db(c.execute_batch("CREATE TABLE IF NOT EXISTS setting_steps(plan_id TEXT NOT NULL REFERENCES settings_plans(id),position INTEGER NOT NULL,state TEXT NOT NULL,before_json TEXT NOT NULL,after_json TEXT NOT NULL,code TEXT NOT NULL,updated_at INTEGER NOT NULL,PRIMARY KEY(plan_id,position)); CREATE TABLE IF NOT EXISTS setting_events(sequence INTEGER PRIMARY KEY,plan_id TEXT NOT NULL,position INTEGER NOT NULL,state TEXT NOT NULL,code TEXT NOT NULL,at INTEGER NOT NULL);"))?;
    let v: i64 = db(c.query_row("PRAGMA user_version", [], |r| r.get(0)))?;
    if v < 4 {
        db(c.execute_batch("PRAGMA user_version=4;"))?;
    }
    Ok(())
}
fn lock(store: &Store) -> Result<File> {
    let path = settings::directory(store)?.join("settings.lock");
    settings::safe_path(&path)?;
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "settings_lock_unavailable")?;
    let m = f.metadata().map_err(|_| "settings_lock_unavailable")?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("unsafe_settings_lock");
    }
    f.try_lock().map_err(|_| "settings_operation_running")?;
    Ok(f)
}
fn record(store: &Store, id: &str, i: usize, state: &str, code: &str, now: i64) -> Result<()> {
    let tx = db(store.connection.unchecked_transaction())?;
    db(tx.execute(
        "UPDATE setting_steps SET state=?3,code=?4,updated_at=?5 WHERE plan_id=?1 AND position=?2",
        params![id, i as i64, state, code, now],
    ))?;
    db(tx.execute(
        "INSERT INTO setting_events(plan_id,position,state,code,at) VALUES(?1,?2,?3,?4,?5)",
        params![id, i as i64, state, code, now],
    ))?;
    db(tx.commit())
}
fn scope(value: &Value, choice: &Choice) -> Result<Value> {
    match choice {
        Choice::Theme { .. } => value
            .get("theme")
            .filter(|v| v.is_string())
            .cloned()
            .ok_or("malformed_settings"),
        Choice::ClockPlacement { .. } => settings::clock(value),
    }
}
fn patch(value: &Value, choice: &Choice, target: &Value) -> Result<Value> {
    let mut result = value.clone();
    match choice {
        Choice::Theme { .. } => {
            if !target.is_string() {
                return Err("malformed_settings");
            }
            result["theme"] = target.clone();
        }
        Choice::ClockPlacement { .. } => {
            let current = settings::clock(value)?;
            let section = target["section"]
                .as_str()
                .filter(|s| ["left", "center", "right"].contains(s))
                .ok_or("invalid_settings_target")?;
            let index = target["index"]
                .as_u64()
                .filter(|n| *n <= 128)
                .ok_or("invalid_settings_target")? as usize;
            let from = current["section"].as_str().unwrap();
            let at = current["index"].as_u64().unwrap() as usize;
            let entry = result["bar"]["layout"][from]
                .as_array_mut()
                .unwrap()
                .remove(at);
            let to = result["bar"]["layout"][section].as_array_mut().unwrap();
            if index > to.len() {
                return Err("settings_restore_position_missing");
            }
            to.insert(index, entry);
        }
    }
    Ok(result)
}
// The only destinations come from the adapter registry. Private sample files use
// atomic replacement; live clock integration stays behind measured release proof.
fn replace(env: &Environment, choice: &Choice, old: &Snapshot, value: &Value) -> Result<()> {
    let path = settings::path(env, choice);
    settings::safe_path(&path)?;
    if env.simulated {
        crate::library::private_directory(&env.root)?;
    }
    let parent = path.parent().ok_or("unsafe_settings_path")?;
    let dir = settings::open_parent(&path)?;
    let metadata = dir.metadata().map_err(|_| "settings_write_failed")?;
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
        return Err("unsafe_settings_directory");
    }
    let temporary = std::ffi::CString::new(format!(
        ".omastore-setting-{}",
        omastore_workflow::nonce().map_err(|_| "settings_write_failed")?
    ))
    .unwrap();
    let target = std::ffi::CString::new(
        path.file_name()
            .ok_or("unsafe_settings_path")?
            .as_encoded_bytes(),
    )
    .map_err(|_| "unsafe_settings_path")?;
    use std::os::fd::{AsRawFd, FromRawFd};
    // Both creation and rename are relative to one verified directory handle;
    // replacing a path ancestor cannot redirect either filesystem mutation.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err("settings_write_failed");
    }
    let mut temp = unsafe { File::from_raw_fd(fd) };
    let result = (|| {
        temp.set_permissions(fs::Permissions::from_mode(old.mode))
            .map_err(|_| "settings_write_failed")?;
        serde_json::to_writer_pretty(&mut temp, value).map_err(|_| "settings_write_failed")?;
        temp.write_all(b"\n").map_err(|_| "settings_write_failed")?;
        temp.sync_all().map_err(|_| "settings_write_failed")?;
        settings::safe_path(&path)?;
        let after_dir = fs::metadata(parent).map_err(|_| "settings_conflict")?;
        if metadata.dev() != after_dir.dev()
            || metadata.ino() != after_dir.ino()
            || settings::snapshot(env, choice)?.fingerprint != old.fingerprint
        {
            return Err("settings_conflict");
        }
        if unsafe {
            libc::renameat(
                dir.as_raw_fd(),
                temporary.as_ptr(),
                dir.as_raw_fd(),
                target.as_ptr(),
            )
        } != 0
        {
            return Err("settings_write_failed");
        }
        dir.sync_all().map_err(|_| "settings_sync_failed")?;
        Ok(())
    })();
    // Only the uniquely named temporary file created above is eligible for cleanup.
    unsafe { libc::unlinkat(dir.as_raw_fd(), temporary.as_ptr(), 0) };
    result
}

fn enabled(env: &Environment, choice: &Choice) -> Result<()> {
    if !env.supported {
        return Err("unsupported_settings_environment");
    }
    if !env.simulated
        && (!settings::VERIFIED_SETTING_RELEASES.contains(&env.version.as_str())
            || matches!(choice, Choice::Theme { .. }))
    {
        return Err("settings_adapter_unverified");
    }
    Ok(())
}
fn reload(env: &Environment) -> Result<()> {
    if env.simulated {
        return Ok(());
    }
    // Fixed documented shell method; pass only the desktop IPC environment.
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let program = "/usr/bin/omarchy-shell";
    platform::trusted_program(program)?;
    let mut command = Command::new(program);
    command
        .args(["shell", "reloadConfig"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C");
    for name in [
        "HOME",
        "XDG_RUNTIME_DIR",
        "WAYLAND_DISPLAY",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "settings_reload_failed")?;
    let out = child.stdout.take().ok_or("settings_reload_failed")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        out.take(4097).read_to_end(&mut bytes).map(|_| bytes)
    });
    let until = Instant::now() + Duration::from_secs(10);
    let success = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break exit.success(),
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "settings_reload_failed")?
        .map_err(|_| "settings_reload_failed")?;
    if !success
        || bytes.len() > 4096
        || std::str::from_utf8(&bytes).is_err()
        || String::from_utf8_lossy(&bytes).trim() != "ok"
    {
        return Err("settings_reload_failed");
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consent {
    pub id: String,
    pub digest: String,
    pub accepted: bool,
}
fn validate(p: &Plan, c: &Consent) -> Result<()> {
    if !c.accepted || c.id != p.digest || c.digest != p.digest {
        return Err("settings_consent_required");
    }
    Ok(())
}
pub fn apply(store: &Store, c: Consent, env: &Environment, now: i64) -> Result<Value> {
    apply_inner(store, c, env, now, "")
}
fn apply_inner(
    store: &Store,
    c: Consent,
    env: &Environment,
    now: i64,
    fault: &str,
) -> Result<Value> {
    let _lock = lock(store)?;
    let p = settings::stored(store, &c.id)?;
    validate(&p, &c)?;
    if p.simulated != env.simulated {
        return Err("local_mode_mismatch");
    }
    let exists: bool = db(store.connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM setting_steps WHERE plan_id=?1)",
        [&c.id],
        |r| r.get(0),
    ))?;
    if exists {
        return history(store);
    }
    if now > p.expires_at || now < p.created_at {
        return Err("settings_plan_expired");
    }
    let fresh = settings::build(p.request.clone(), env, now)?;
    if fresh.material_digest != p.material_digest {
        return Err("settings_conflict");
    }
    if !p.can_apply {
        return Err("settings_plan_blocked");
    }
    for e in &p.effects {
        enabled(env, &e.choice)?;
    }
    if fault == "before_journal" {
        return Err("interrupted");
    }
    let tx = db(store.connection.unchecked_transaction())?;
    for (i, e) in p.effects.iter().enumerate() {
        db(tx.execute(
            "INSERT INTO setting_steps VALUES(?1,?2,?3,?4,?5,'consented',?6)",
            params![
                p.digest,
                i as i64,
                if e.state == "noop" { "noop" } else { "planned" },
                e.before.to_string(),
                e.desired.to_string(),
                now
            ],
        ))?;
    }
    db(tx.commit())?;
    if fault == "after_journal" {
        return Err("interrupted");
    }
    for (i, e) in p
        .effects
        .iter()
        .enumerate()
        .filter(|(_, e)| e.state != "noop")
    {
        record(store, &p.digest, i, "writing", "write_started", now)?;
        let result = (|| {
            let current = settings::snapshot(env, &e.choice)?;
            if current.fingerprint != e.fingerprint {
                return Err("settings_conflict");
            }
            let desired = patch(&current.value, &e.choice, &e.desired)?;
            replace(env, &e.choice, &current, &desired)?;
            if ["after_write", "before_finish"].contains(&fault) {
                return Err("interrupted");
            }
            if fault == "reload" {
                return Err("settings_reload_failed");
            }
            reload(env)?;
            if scope(&settings::snapshot(env, &e.choice)?.value, &e.choice)? != e.desired {
                return Err("settings_postwrite_changed");
            }
            Ok(())
        })();
        match result {
            Ok(()) => record(
                store,
                &p.digest,
                i,
                "applied",
                "value_and_reload_observed",
                now,
            )?,
            Err("interrupted") => return Err("interrupted"),
            Err(code) => record(store, &p.digest, i, "unknown", code, now)?,
        }
    }
    history(store)
}
fn recover(store: &Store, now: i64) -> Result<()> {
    let mut stmt=db(store.connection.prepare("SELECT plan_id,position FROM setting_steps WHERE state IN ('planned','writing','restoring')"))?;
    let pending = db(stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))))?
        .collect::<rusqlite::Result<Vec<_>>>();
    for (id, i) in db(pending)? {
        record(
            store,
            &id,
            i as usize,
            "unknown",
            "interrupted_observation_required",
            now,
        )?;
    }
    Ok(())
}
pub fn history(store: &Store) -> Result<Value> {
    let mut stmt=db(store.connection.prepare("SELECT p.body FROM settings_plans p WHERE EXISTS(SELECT 1 FROM setting_steps s WHERE s.plan_id=p.id) ORDER BY p.created_at DESC,p.id LIMIT 50"))?;
    let rows =
        db(stmt.query_map([], |r| r.get::<_, String>(0)))?.collect::<rusqlite::Result<Vec<_>>>();
    let mut items = Vec::new();
    for raw in db(rows)? {
        let p: Plan = serde_json::from_str(&raw).map_err(|_| "settings_plan_invalid")?;
        let mut steps = Vec::new();
        for (i, e) in p.effects.iter().enumerate() {
            let (state, code, at) = db(store.connection.query_row(
                "SELECT state,code,updated_at FROM setting_steps WHERE plan_id=?1 AND position=?2",
                params![p.digest, i as i64],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            ))?;
            steps.push(json!({"reference":e.reference,"before":e.before,"after":e.desired,"state":state,"code":code,"updatedAt":at}));
        }
        items.push(
            json!({"id":p.digest,"simulated":p.simulated,"createdAt":p.created_at,"steps":steps}),
        );
    }
    Ok(
        json!({"items":items,"notice":"History is private to this device. Recovery changes only the selected setting; unrelated current fields are preserved."}),
    )
}
fn restore_plan(store: &Store, id: &str, env: &Environment) -> Result<Value> {
    let p = settings::stored(store, id)?;
    let mut effects = Vec::new();
    for (i, e) in p.effects.iter().enumerate() {
        let state: Option<String> = db(store
            .connection
            .query_row(
                "SELECT state FROM setting_steps WHERE plan_id=?1 AND position=?2",
                params![id, i as i64],
                |r| r.get(0),
            )
            .optional())?;
        if !state
            .as_deref()
            .is_some_and(|s| ["applied", "unknown"].contains(&s))
        {
            continue;
        }
        let observed =
            settings::snapshot(env, &e.choice).and_then(|s| Ok((scope(&s.value, &e.choice)?, s)));
        let (current, fingerprint, can_restore, code) = match observed {
            Ok((v, s)) => {
                let can = v == e.desired
                    && enabled(env, &e.choice).is_ok()
                    && patch(&s.value, &e.choice, &e.before).is_ok();
                (
                    v,
                    s.fingerprint,
                    can,
                    if can { "ready" } else { "settings_conflict" },
                )
            }
            Err(code) => (Value::Null, String::new(), false, code),
        };
        effects.push(json!({"position":i,"reference":e.reference,"current":current,"restore":e.before,"fingerprint":fingerprint,"canRestore":can_restore,"code":code}));
    }
    let mut value = json!({"id":id,"effects":effects,"simulated":env.simulated,"version":env.version,"canRestore":!effects.is_empty()&&effects.iter().all(|e|e["canRestore"]==true)});
    value["digest"] = json!(platform::hash(&value));
    Ok(value)
}
fn restore(store: &Store, c: Consent, env: &Environment, now: i64) -> Result<Value> {
    let _lock = lock(store)?;
    recover(store, now)?;
    let preview = restore_plan(store, &c.id, env)?;
    if !c.accepted || preview["digest"] != c.digest {
        return Err("settings_restore_changed");
    }
    if preview["canRestore"] != true {
        return Err("settings_restore_conflict");
    }
    let p = settings::stored(store, &c.id)?;
    for effect in preview["effects"].as_array().unwrap() {
        let i = effect["position"].as_u64().unwrap() as usize;
        let e = &p.effects[i];
        record(store, &p.digest, i, "restoring", "restore_started", now)?;
        let result = (|| {
            let current = settings::snapshot(env, &e.choice)?;
            if effect["fingerprint"] != current.fingerprint
                || scope(&current.value, &e.choice)? != e.desired
            {
                return Err("settings_restore_conflict");
            }
            replace(
                env,
                &e.choice,
                &current,
                &patch(&current.value, &e.choice, &e.before)?,
            )?;
            reload(env)?;
            if scope(&settings::snapshot(env, &e.choice)?.value, &e.choice)? != e.before {
                return Err("settings_postwrite_changed");
            }
            Ok(())
        })();
        match result {
            Ok(()) => record(store, &p.digest, i, "restored", "prior_value_observed", now)?,
            Err(code) => record(store, &p.digest, i, "unknown", code, now)?,
        }
    }
    history(store)
}

pub fn request(store: &Store, method: &str, params: Value, now: i64) -> Result<Value> {
    let env = settings::environment(store)?;
    match method {
        "settings.apply" => apply(
            store,
            serde_json::from_value(params).map_err(|_| "invalid_settings_request")?,
            &env,
            now,
        ),
        "settings.restore" => restore(
            store,
            serde_json::from_value(params).map_err(|_| "invalid_settings_request")?,
            &env,
            now,
        ),
        "settings.history" if params == json!({}) => {
            let _lock = lock(store)?;
            recover(store, now)?;
            history(store)
        }
        "settings.restore_preview" | "settings.reconcile" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Lookup {
                id: String,
            }
            let p: Lookup =
                serde_json::from_value(params).map_err(|_| "invalid_settings_request")?;
            let _lock = lock(store)?;
            recover(store, now)?;
            if method == "settings.restore_preview" {
                return restore_plan(store, &p.id, &env);
            }
            let plan = settings::stored(store, &p.id)?;
            for (i, e) in plan.effects.iter().enumerate() {
                let state: Option<String> = db(store
                    .connection
                    .query_row(
                        "SELECT state FROM setting_steps WHERE plan_id=?1 AND position=?2",
                        params![p.id, i as i64],
                        |r| r.get(0),
                    )
                    .optional())?;
                if state.as_deref() != Some("unknown") {
                    continue;
                }
                enabled(&env, &e.choice)?;
                let current = scope(&settings::snapshot(&env, &e.choice)?.value, &e.choice)?;
                if current == e.desired {
                    reload(&env)?;
                    record(
                        store,
                        &p.id,
                        i,
                        "applied",
                        "reconciled_value_and_reload",
                        now,
                    )?;
                } else if current == e.before {
                    record(store, &p.id, i, "unchanged", "prior_value_observed", now)?;
                } else {
                    record(store, &p.id, i, "unknown", "settings_conflict", now)?;
                }
            }
            history(store)
        }
        _ => Err("unsupported_method"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omastore_catalogue::setups::SettingReference;
    fn fixture() -> (tempfile::TempDir, Store, Environment) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(&dir.path().join("state"), true).unwrap();
        let env = settings::environment(&store).unwrap();
        (dir, store, env)
    }
    fn plan(store: &Store, env: &Environment) -> Plan {
        let p = settings::build(
            settings::Request {
                settings: vec![SettingReference {
                    adapter: "omarchy-clock-placement".into(),
                    value_id: "right-start".into(),
                    revision: "1".into(),
                }],
            },
            env,
            1000,
        )
        .unwrap();
        settings::save(store, &p).unwrap();
        p
    }
    fn consent(p: &Plan) -> Consent {
        Consent {
            id: p.digest.clone(),
            digest: p.digest.clone(),
            accepted: true,
        }
    }
    fn write(env: &Environment, value: &Value) {
        crate::library::private_directory(&env.root).unwrap();
        let path = env.root.join("shell.json");
        fs::write(&path, value.to_string()).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o640)).unwrap();
    }
    #[test]
    fn compare_before_write_and_restore_preserve_unrelated_fields_and_permissions() {
        let (_d, store, env) = fixture();
        let p = plan(&store, &env);
        let choice = &p.effects[0].choice;
        let mut v = settings::snapshot(&env, choice).unwrap().value;
        v["accountSecret"] = json!("never exported");
        write(&env, &v);
        assert_eq!(
            apply(&store, consent(&p), &env, 1001).unwrap_err(),
            "settings_conflict"
        );
        let p = plan(&store, &env);
        let result = apply(&store, consent(&p), &env, 1001).unwrap();
        assert_eq!(result["items"][0]["steps"][0]["state"], "applied");
        assert_eq!(apply(&store, consent(&p), &env, 9999).unwrap(), result);
        assert!(
            !settings::build(p.request.clone(), &env, 1002)
                .unwrap()
                .can_apply
        );
        let mut v = settings::snapshot(&env, choice).unwrap().value;
        v["unrelated"] = json!({"manual":true});
        v["bar"]["layout"]["right"][0]["format"] = json!("HH:mm:ss");
        write(&env, &v);
        let preview = restore_plan(&store, &p.digest, &env).unwrap();
        assert_eq!(preview["canRestore"], true);
        let result = restore(
            &store,
            Consent {
                id: p.digest.clone(),
                digest: preview["digest"].as_str().unwrap().into(),
                accepted: true,
            },
            &env,
            1003,
        )
        .unwrap();
        assert_eq!(result["items"][0]["steps"][0]["state"], "restored");
        let actual = settings::snapshot(&env, choice).unwrap().value;
        assert_eq!(actual["unrelated"], v["unrelated"]);
        assert_eq!(actual["accountSecret"], v["accountSecret"]);
        assert_eq!(actual["bar"]["layout"]["center"][1]["format"], "HH:mm:ss");
        assert_eq!(
            fs::metadata(env.root.join("shell.json")).unwrap().mode() & 0o777,
            0o640
        );
        assert!(!result.to_string().contains("never exported"));
    }
    #[test]
    fn manual_edits_and_symlink_or_permission_changes_block_restoration() {
        let (_d, store, env) = fixture();
        let p = plan(&store, &env);
        apply(&store, consent(&p), &env, 1001).unwrap();
        let choice = &p.effects[0].choice;
        let v = settings::snapshot(&env, choice).unwrap().value;
        let moved = patch(&v, choice, &json!({"section":"left","index":0})).unwrap();
        write(&env, &moved);
        assert_eq!(
            restore_plan(&store, &p.digest, &env).unwrap()["canRestore"],
            false
        );
        fs::set_permissions(
            env.root.join("shell.json"),
            fs::Permissions::from_mode(0o666),
        )
        .unwrap();
        assert!(settings::snapshot(&env, choice).is_err());
        fs::remove_file(env.root.join("shell.json")).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", env.root.join("shell.json")).unwrap();
        assert_eq!(
            restore_plan(&store, &p.digest, &env).unwrap()["canRestore"],
            false
        );
    }
    #[test]
    fn interruption_and_reload_failures_never_record_success_without_observation() {
        for fault in [
            "before_journal",
            "after_journal",
            "after_write",
            "before_finish",
            "reload",
        ] {
            let (_d, store, env) = fixture();
            let p = plan(&store, &env);
            let _ = apply_inner(&store, consent(&p), &env, 1001, fault);
            recover(&store, 1002).unwrap();
            let h = history(&store).unwrap();
            if fault == "before_journal" {
                assert!(h["items"].as_array().unwrap().is_empty());
                continue;
            }
            assert_eq!(h["items"][0]["steps"][0]["state"], "unknown");
            let recovered =
                request(&store, "settings.reconcile", json!({"id":p.digest}), 1003).unwrap();
            assert_eq!(
                recovered["items"][0]["steps"][0]["state"],
                if fault == "after_journal" {
                    "unchanged"
                } else {
                    "applied"
                }
            );
        }
    }
}
