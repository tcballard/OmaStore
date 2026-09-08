//! Durable consent and operation outcomes. Package workers are independent of the pipe core.
use crate::{
    library::Store,
    planner::{Plan, Selection},
    platform::{Host, Result},
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

// Add a release only with its recorded real Omarchy lifecycle acceptance report.
// There is deliberately no environment-variable or IPC bypass.
const VERIFIED_OMARCHY_RELEASES: &[&str] = &[];
pub fn live_enabled(plan: &Plan) -> bool {
    !plan.simulated
        && plan.host["omarchyVersion"]
            .as_str()
            .is_some_and(|s| VERIFIED_OMARCHY_RELEASES.contains(&s))
}
fn db<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|_| "local_database_unavailable")
}
pub fn migrate(connection: &rusqlite::Connection) -> Result<()> {
    db(connection.execute_batch("CREATE TABLE IF NOT EXISTS operation_control(operation_id TEXT PRIMARY KEY REFERENCES operations(id),claimed INTEGER NOT NULL DEFAULT 0,cancel_requested INTEGER NOT NULL DEFAULT 0,heartbeat_at INTEGER NOT NULL,result_json TEXT); PRAGMA user_version=2;"))
}
fn change(
    tx: &Transaction<'_>,
    id: &str,
    from: &str,
    to: &str,
    code: &str,
    now: i64,
) -> Result<()> {
    let allowed = matches!(
        (from, to),
        ("planned", "awaiting_user")
            | ("planned", "failed")
            | ("awaiting_user", "running")
            | ("awaiting_user", "failed")
            | ("awaiting_user", "unknown")
            | ("running", "succeeded")
            | ("running", "failed")
            | ("running", "unknown")
            | ("unknown", "succeeded")
            | ("unknown", "failed")
    );
    if !allowed || code.len() > 80 || !code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
        return Err("invalid_operation_transition");
    }
    if db(tx.execute(
        "UPDATE operations SET state=?3,version=version+1,updated_at=?4 WHERE id=?1 AND state=?2",
        params![id, from, to, now],
    ))? != 1
    {
        return Err("operation_changed");
    }
    db(tx.execute(
        "INSERT INTO operation_events(operation_id,state,code,at) VALUES(?1,?2,?3,?4)",
        params![id, to, code, now],
    ))?;
    Ok(())
}
pub fn status(store: &Store, id: &str) -> Result<Value> {
    let plan = store.plan(id)?;
    let (state, version): (String, i64) = db(store.connection.query_row(
        "SELECT state,version FROM operations WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    type Control = (bool, bool, i64, Option<String>);
    let control:Option<Control>=db(store.connection.query_row("SELECT claimed,cancel_requested,heartbeat_at,result_json FROM operation_control WHERE operation_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional())?;
    let (claimed, cancel, heartbeat, result) = control.unwrap_or((false, false, 0, None));
    let items:Value=result.as_deref().map(serde_json::from_str).transpose().map_err(|_|"local_result_invalid")?.unwrap_or_else(||json!(plan.operations.iter().map(|o|json!({"appId":o.app_id,"name":o.name,"state":if o.action=="noop"{"observed_present"}else if o.action=="external"{"manual"}else{o.action.as_str()},"version":o.installed_version})).collect::<Vec<_>>()));
    Ok(
        json!({"id":id,"state":state,"version":version,"claimed":claimed,"cancelRequested":cancel,"heartbeatAt":heartbeat,"items":items,"enabled":plan.simulated||live_enabled(&plan),"simulated":plan.simulated,"enablementNotice":if plan.simulated{"Fictional package rehearsal only"}else{"Live package writes require a recorded Omarchy adapter acceptance report"},"events":store.events(id,0)?}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consent {
    pub id: String,
    pub digest: String,
    pub accepted: bool,
}
pub fn consent(store: &mut Store, request: &Consent, fresh: &Plan, now: i64) -> Result<()> {
    let plan = store.plan(&request.id)?;
    if !request.accepted || request.digest != plan.digest {
        return Err("explicit_consent_required");
    }
    if plan.created_at > now + 30
        || plan.expires_at <= now
        || plan.expires_at > plan.created_at + 600
    {
        return Err("plan_expired");
    }
    if plan.material_digest != fresh.material_digest
        || plan.catalogue_snapshot != fresh.catalogue_snapshot
        || !fresh.blockers.is_empty()
    {
        return Err("plan_changed_review_again");
    }
    if !plan.simulated && !live_enabled(&plan) {
        return Err("omarchy_adapter_evidence_required");
    }
    if !plan
        .operations
        .iter()
        .any(|o| matches!(o.action.as_str(), "install" | "remove"))
    {
        return Err("no_managed_package_changes");
    }
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let state: String = db(tx.query_row(
        "SELECT state FROM operations WHERE id=?1",
        [&plan.digest],
        |r| r.get(0),
    ))?;
    if state == "awaiting_user" || state == "running" {
        return Ok(());
    }
    if state != "planned" {
        return Err("operation_already_finished");
    }
    db(tx.execute(
        "INSERT INTO operation_control(operation_id,heartbeat_at) VALUES(?1,?2)",
        params![plan.digest, now],
    ))?;
    change(
        &tx,
        &plan.digest,
        "planned",
        "awaiting_user",
        "explicit_plan_consent",
        now,
    )?;
    db(tx.commit())
}
pub fn claim(store: &mut Store, id: &str, now: i64) -> Result<bool> {
    let plan = store.plan(id)?;
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    if plan.expires_at <= now {
        return Err("plan_expired");
    }
    let n=db(tx.execute("UPDATE operation_control SET claimed=1,heartbeat_at=?2 WHERE operation_id=?1 AND claimed=0 AND cancel_requested=0 AND EXISTS(SELECT 1 FROM operations WHERE id=?1 AND state='awaiting_user')",params![id,now]))?;
    db(tx.commit())?;
    Ok(n == 1)
}
pub fn begin(store: &mut Store, id: &str, fresh: &Plan, now: i64) -> Result<()> {
    let plan = store.plan(id)?;
    if plan.expires_at <= now
        || plan.material_digest != fresh.material_digest
        || !fresh.blockers.is_empty()
    {
        return Err("plan_changed_review_again");
    }
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let allowed: bool = db(tx.query_row(
        "SELECT claimed=1 AND cancel_requested=0 FROM operation_control WHERE operation_id=?1",
        [id],
        |r| r.get(0),
    ))?;
    if !allowed {
        return Err("operation_cancelled");
    }
    change(
        &tx,
        id,
        "awaiting_user",
        "running",
        "package_transaction_started",
        now,
    )?;
    db(tx.commit())
}
pub fn unknown(store: &mut Store, id: &str, code: &str, now: i64) -> Result<()> {
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let state: String = db(
        tx.query_row("SELECT state FROM operations WHERE id=?1", [id], |r| {
            r.get(0)
        }),
    )?;
    if state == "awaiting_user" || state == "running" {
        change(&tx, id, &state, "unknown", code, now)?;
    }
    db(tx.commit())
}
pub fn heartbeat(store: &Store, id: &str, now: i64) -> Result<()> {
    db(store.connection.execute(
        "UPDATE operation_control SET heartbeat_at=?2 WHERE operation_id=?1",
        params![id, now],
    ))?;
    Ok(())
}
pub fn fail(store: &mut Store, id: &str, code: &str, now: i64) -> Result<()> {
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let state: String = db(
        tx.query_row("SELECT state FROM operations WHERE id=?1", [id], |r| {
            r.get(0)
        }),
    )?;
    if state == "failed" || state == "succeeded" {
        return Ok(());
    }
    change(&tx, id, &state, "failed", code, now)?;
    db(tx.commit())
}
pub fn cancel(store: &mut Store, id: &str, now: i64) -> Result<Value> {
    store.plan(id)?;
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let state: String = db(
        tx.query_row("SELECT state FROM operations WHERE id=?1", [id], |r| {
            r.get(0)
        }),
    )?;
    if state == "planned" || state == "awaiting_user" {
        change(&tx, id, &state, "failed", "cancelled_before_execution", now)?;
    }
    // A running transaction is never terminated. This flag stops only future safe-boundary work.
    db(tx.execute(
        "UPDATE operation_control SET cancel_requested=1 WHERE operation_id=?1",
        [id],
    ))?;
    db(tx.commit())?;
    status(store, id)
}
pub fn finish(
    store: &mut Store,
    plan: &Plan,
    host: &Host,
    exit_ok: bool,
    code: &str,
    now: i64,
) -> Result<Value> {
    let known = host.state == "supported" && !host.locked;
    let mut items = Vec::new();
    let mut complete = known
        && exit_ok
        && plan
            .packages
            .iter()
            .all(|p| host.installed.get(&p.name) == Some(&p.version));
    for op in &plan.operations {
        let observed = op.package.as_ref().and_then(|p| host.installed.get(p));
        let state = if op.action == "external" {
            "manual"
        } else if !known {
            "unknown"
        } else if op.action == "remove" {
            if observed.is_none() {
                "removed"
            } else {
                "still_installed"
            }
        } else if observed == Some(&op.version) {
            "observed_present"
        } else {
            "not_installed"
        };
        if (op.action == "install" && state != "observed_present")
            || (op.action == "remove" && state != "removed")
        {
            complete = false;
        }
        items.push(json!({"appId":op.app_id,"name":op.name,"state":state,"version":observed,"preexisting":op.installed_version.is_some()}));
    }
    if !plan.simulated
        && (host.configuration != plan.host["configurationFingerprint"].as_str().unwrap_or("")
            || host.sync_databases != plan.host["databaseFingerprint"].as_str().unwrap_or(""))
    {
        complete = false;
    }
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let from: String = db(tx.query_row(
        "SELECT state FROM operations WHERE id=?1",
        [&plan.digest],
        |r| r.get(0),
    ))?;
    if from == "succeeded" || from == "failed" {
        drop(tx);
        return status(store, &plan.digest);
    }
    let to = if !known {
        "unknown"
    } else if complete {
        "succeeded"
    } else {
        "failed"
    };
    let observed_code = if !known {
        "package_outcome_unknown"
    } else if exit_ok && !complete {
        "package_state_mismatch"
    } else {
        code
    };
    if to != from {
        change(&tx, &plan.digest, &from, to, observed_code, now)?;
    }
    db(tx.execute(
        "UPDATE operation_control SET result_json=?2,heartbeat_at=?3 WHERE operation_id=?1",
        params![
            plan.digest,
            serde_json::to_string(&items).map_err(|_| "operation_result_invalid")?,
            now
        ],
    ))?;
    if known {
        for op in &plan.operations {
            if op.action == "remove" {
                if let Some(package) = &op.package {
                    db(tx.execute("UPDATE installed_apps SET present=?2,last_seen_at=?3,source_state=?4 WHERE package=?1",params![package,host.installed.contains_key(package),now,if host.installed.contains_key(package){"removal_incomplete"}else{"removed"}]))?;
                }
                continue;
            }
            let Some(package) = &op.package else { continue };
            let Some(version) = host.installed.get(package) else {
                continue;
            };
            if !matches!(op.action.as_str(), "install" | "noop") {
                continue;
            }
            let managed = complete
                && op.action == "install"
                && matches!(
                    code,
                    "package_state_verified" | "sample_package_state_verified"
                );
            db(tx.execute("INSERT INTO installed_apps(id,name,package,repository,observed_version,present,last_seen_at,preexisting,source_state,catalogue_release,catalogue_version) VALUES(?1,?2,?3,?4,?5,1,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET observed_version=excluded.observed_version,present=1,last_seen_at=excluded.last_seen_at,source_state=excluded.source_state",params![op.app_id,op.name,package,op.repository.as_deref().unwrap_or(""),version,now,op.installed_version.is_some(),if managed{"verified_transaction"}else{"observed_after_operation"},op.release_id,op.version]))?;
            if let Selection::Setup { id, revision, .. } = &plan.selection {
                db(tx.execute("INSERT INTO setup_refs(setup_id,revision,app_id,added_at) VALUES(?1,?2,?3,?4) ON CONFLICT(setup_id,app_id) DO UPDATE SET revision=excluded.revision",params![id,revision,op.app_id,now]))?;
            }
        }
    }
    db(tx.commit())?;
    status(store, &plan.digest)
}
pub fn lock(path: &Path) -> Result<File> {
    if fs::symlink_metadata(path).is_ok_and(|m| !m.is_file() || m.mode() & 0o077 != 0) {
        return Err("unsafe_worker_lock");
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| "worker_lock_unavailable")?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err("operation_worker_active"),
        Err(_) => Err("worker_lock_unavailable"),
    }
}
pub fn worker_active(directory: &Path, id: &str) -> Result<bool> {
    if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid_operation_id");
    }
    match lock(&directory.join(format!("{id}.lock"))) {
        Ok(_) => Ok(false),
        Err("operation_worker_active") => Ok(true),
        Err(e) => Err(e),
    }
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Store, Plan, Host) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::at(&dir.path().join("local"), true).unwrap();
        migrate(&store.connection).unwrap();
        let c: omastore_catalogue::Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let s = Selection::App {
            id: "demo-fieldnotes".into(),
        };
        let (h, status, p) =
            crate::planner::sample(&c, &s, Default::default(), 1_800_000_000).unwrap();
        let plan = crate::planner::build(&c, s, &h, &status, Ok(p), 1_800_000_000).unwrap();
        store.save_plan(&plan).unwrap();
        (dir, store, plan, h)
    }
    #[test]
    fn consent_claim_and_outcomes_are_durable_and_idempotent() {
        let (_dir, mut store, plan, mut host) = fixture();
        let now = plan.created_at;
        let mut request = Consent {
            id: plan.digest.clone(),
            digest: plan.digest.clone(),
            accepted: false,
        };
        assert!(consent(&mut store, &request, &plan, now).is_err());
        request.accepted = true;
        let mut changed = plan.clone();
        changed.material_digest = "changed".into();
        assert!(consent(&mut store, &request, &changed, now).is_err());
        consent(&mut store, &request, &plan, now).unwrap();
        consent(&mut store, &request, &plan, now).unwrap();
        assert!(claim(&mut store, &plan.digest, now).unwrap());
        assert!(!claim(&mut store, &plan.digest, now).unwrap());
        begin(&mut store, &plan.digest, &plan, now).unwrap();
        cancel(&mut store, &plan.digest, now).unwrap();
        assert_eq!(status(&store, &plan.digest).unwrap()["state"], "running");
        for p in &plan.packages {
            host.installed.insert(p.name.clone(), p.version.clone());
        }
        assert_eq!(
            finish(
                &mut store,
                &plan,
                &host,
                true,
                "package_state_verified",
                now + 1
            )
            .unwrap()["state"],
            "succeeded"
        );
        assert_eq!(store.view(0).unwrap()["items"][0]["preexisting"], false);
        let events = store.events(&plan.digest, 0).unwrap();
        assert_eq!(events["events"].as_array().unwrap().len(), 4);
        finish(
            &mut store,
            &plan,
            &host,
            true,
            "package_state_verified",
            now + 2,
        )
        .unwrap();
        assert_eq!(store.events(&plan.digest, 0).unwrap(), events);
    }
    #[test]
    fn cancellation_before_execution_and_kernel_worker_locks() {
        let (dir, mut store, plan, _) = fixture();
        cancel(&mut store, &plan.digest, plan.created_at).unwrap();
        assert!(!claim(&mut store, &plan.digest, plan.created_at).unwrap());
        let path = dir.path().join("worker.lock");
        let held = lock(&path).unwrap();
        assert!(lock(&path).is_err());
        drop(held);
        assert!(lock(&path).is_ok());
        let mut live = plan;
        live.simulated = false;
        assert!(!live_enabled(&live));
    }
}
