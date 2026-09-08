use crate::{
    library::Store,
    lifecycle,
    planner::{Operation, Plan, Selection},
    platform::{self, Host, Result},
};
use omastore_catalogue::Catalogue;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf};
fn db<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|_| "local_database_unavailable")
}
fn directory(store: &Store) -> Result<PathBuf> {
    store
        .connection
        .path()
        .and_then(|s| std::path::Path::new(s).parent())
        .map(PathBuf::from)
        .ok_or("local_database_unavailable")
}

pub fn inspect(store: &mut Store, id: &str, now: i64) -> Result<Value> {
    let state = lifecycle::status(store, id)?;
    let active = lifecycle::worker_active(&directory(store)?, id)?;
    if !active
        && (state["state"] == "running"
            || (state["state"] == "awaiting_user" && state["claimed"] == true))
    {
        lifecycle::unknown(store, id, "worker_interrupted", now)?;
    } else if !active && state["state"] == "awaiting_user" && store.plan(id)?.expires_at <= now {
        lifecycle::fail(store, id, "plan_expired", now)?;
    }
    let mut value = lifecycle::status(store, id)?;
    value["workerActive"] = json!(active);
    Ok(value)
}
pub fn reconcile(store: &mut Store, id: &str, host: &Host, now: i64) -> Result<Value> {
    let status = inspect(store, id, now)?;
    if status["workerActive"] == true {
        return Err("operation_worker_active");
    }
    if status["state"] != "unknown" {
        return Ok(status);
    }
    if host.state != "supported" || host.locked {
        return Ok(status);
    }
    let plan = store.plan(id)?;
    // This establishes current package presence/absence, not a recovered process exit or test claim.
    lifecycle::finish(store, &plan, host, true, "observed_after_interruption", now)
}
pub fn removal(store: &Store, c: &Catalogue, id: &str, host: &Host, now: i64) -> Result<Plan> {
    if !omastore_catalogue::token(id) {
        return Err("invalid_id");
    }
    type Installed = (String, String, String, String, String);
    let record:Option<Installed>=db(store.connection.query_row("SELECT name,package,repository,catalogue_release,catalogue_version FROM installed_apps WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional())?;
    let (name, package, repository, release, recorded) = record.ok_or("installed_app_not_found")?;
    if !platform::package_token(&package) {
        return Err("invalid_package_target");
    }
    let installed = host.installed.get(&package).cloned();
    let references = references(store, id)?;
    let mut blockers = Vec::new();
    if host.state != "supported" {
        blockers.push("Current Omarchy package state is unavailable".into());
    }
    if host.locked {
        blockers.push("Another package operation holds the system lock".into());
    }
    if !references.is_empty() {
        blockers.push("Detach this app's setup references before removing its package".into());
    }
    let aliases: i64 = db(store.connection.query_row(
        "SELECT count(*) FROM installed_apps WHERE package=?1 AND id!=?2 AND present=1",
        params![package, id],
        |r| r.get(0),
    ))?;
    if aliases > 0 {
        blockers.push(
            "Other library entries share this package; inspect them in the system package tools"
                .into(),
        );
    }
    if !host.simulated && installed.is_some() && blockers.is_empty() {
        let output = platform::read_command(
            "/usr/bin/pacman",
            &[
                "-R".into(),
                "--print".into(),
                "--print-format".into(),
                "%n %v".into(),
                "--".into(),
                package.clone(),
            ],
        )?;
        if output.code != 0 {
            blockers.push("The package manager cannot remove this target without affecting another dependency".into());
        } else {
            let effects = platform::installed(&output.text)?;
            if effects.len() != 1 || effects.get(&package) != installed.as_ref() {
                blockers.push("The removal effects differ from this observed package".into());
            }
        }
    }
    let version = installed.clone().unwrap_or(recorded);
    let op=Operation{app_id:id.into(),release_id:release,name,action:if installed.is_none(){"noop"}else if blockers.is_empty(){"remove"}else{"blocked"}.into(),reason:"Remove only this package. Keep dependency packages, personal documents and normal pacsave configuration handling. The package manager may run the package's own removal hooks.".into(),package:Some(package.clone()),repository:Some(repository),version:version.clone(),installed_version:installed,identity_digest:platform::hash(&json!({"package":package,"version":version})),privileges:vec!["administrator".into()],services:Vec::new(),external_url:None,disclosure:Some(json!({"setupReferences":references,"otherLibraryEntries":aliases,"restoration":"Reinstallation requires a fresh plan; removal is not a system rollback"}))};
    let selection = Selection::Remove { id: id.into() };
    let snapshot = c.snapshot_id();
    let material = platform::hash(
        &json!({"schemaVersion":1,"selection":selection,"host":host.fingerprint(),"snapshot":snapshot,"operation":op,"blockers":blockers}),
    );
    let mut plan=Plan{schema_version:1,selection,created_at:now,expires_at:now+600,catalogue_snapshot:snapshot,status_until:0,host:host.summary(),operations:vec![op],packages:Vec::new(),blockers,material_digest:material,digest:String::new(),simulated:host.simulated,notice:if host.simulated{"Fictional removal rehearsal. No host package is removed."}else{"Review the exact package removal. This does not delete documents or restore an entire desktop."}.into()};
    plan.digest = platform::hash(&plan);
    Ok(plan)
}
fn references(store: &Store, id: &str) -> Result<Vec<Value>> {
    let mut q = db(store.connection.prepare(
        "SELECT setup_id,revision FROM setup_refs WHERE app_id=?1 ORDER BY setup_id LIMIT 101",
    ))?;
    let rows: Vec<Value> = db(db(q.query_map([id], |r| {
        Ok(json!({"id":r.get::<_,String>(0)?,"revision":r.get::<_,String>(1)?}))
    }))?
    .collect())?;
    if rows.len() > 100 {
        return Err("too_many_setup_references");
    }
    Ok(rows)
}
pub fn setups(store: &Store, offset: u32) -> Result<Value> {
    if offset > 20000 {
        return Err("invalid_offset");
    }
    let mut q=db(store.connection.prepare("SELECT setup_id,revision,count(*) FROM setup_refs GROUP BY setup_id,revision ORDER BY setup_id,revision LIMIT 30 OFFSET ?1"))?;
    let rows:Vec<Value>=db(db(q.query_map([offset],|r|Ok(json!({"id":r.get::<_,String>(0)?,"revision":r.get::<_,String>(1)?,"componentCount":r.get::<_,u32>(2)?}))))?.collect())?;
    Ok(json!({"items":rows,"offset":offset,"hasMore":rows.len()==30}))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detach {
    pub id: String,
    pub revision: String,
    pub confirmed: bool,
}
pub fn detach(store: &mut Store, request: Detach) -> Result<Value> {
    if !request.confirmed
        || !omastore_catalogue::token(&request.id)
        || !omastore_catalogue::token(&request.revision)
    {
        return Err("invalid_detach_request");
    }
    db(store.connection.execute(
        "DELETE FROM setup_refs WHERE setup_id=?1 AND revision=?2",
        params![request.id, request.revision],
    ))?;
    Ok(
        json!({"detached":true,"notice":"Setup references removed. Installed apps and settings were preserved."}),
    )
}
pub fn remember(store: &mut Store, plan: &Plan, host: &Host, now: i64) -> Result<Value> {
    let Selection::Setup { id, revision, .. } = &plan.selection else {
        return Err("setup_required");
    };
    if host.state != "supported" || plan.expires_at <= now || host.simulated != store.demo {
        return Err("fresh_local_selection_required");
    }
    let tx = db(store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate))?;
    let mut count = 0;
    for op in &plan.operations {
        if op.package.as_ref().and_then(|p| host.installed.get(p)) != Some(&op.version) {
            continue;
        }
        db(tx.execute("INSERT INTO setup_refs(setup_id,revision,app_id,added_at) VALUES(?1,?2,?3,?4) ON CONFLICT(setup_id,app_id) DO UPDATE SET revision=excluded.revision",params![id,revision,op.app_id,now]))?;
        count += 1;
    }
    db(tx.commit())?;
    Ok(
        json!({"remembered":true,"observedComponents":count,"notice":"Only already observed components were referenced. No software was installed and no settings changed."}),
    )
}
pub fn diagnostics(store: &Store, id: &str) -> Result<Value> {
    let plan = store.plan(id)?;
    let status = lifecycle::status(store, id)?;
    let mut document = json!({"schemaVersion":1,"applicationVersion":env!("CARGO_PKG_VERSION"),"operationId":id,"state":status["state"],"simulated":plan.simulated,"createdAt":plan.created_at,"components":plan.operations.iter().map(|o|json!({"appId":o.app_id,"package":o.package,"expectedVersion":o.version,"action":o.action})).collect::<Vec<_>>(),"outcomes":status["items"].as_array().into_iter().flatten().map(|o|json!({"appId":o["appId"],"state":o["state"],"version":o["version"]})).collect::<Vec<_>>(),"events":status["events"]["events"],"redaction":"Contains selected public IDs, package versions and fixed event codes. No paths, credentials, raw process output, account data or other installed apps."});
    redact(&mut document);
    let bytes = serde_json::to_vec_pretty(&document).map_err(|_| "diagnostics_unavailable")?;
    if bytes.len() > 128 * 1024 {
        return Err("diagnostics_too_large");
    }
    Ok(json!({"document":document,"digest":platform::digest(bytes)}))
}
fn redact(value: &mut Value) {
    match value {
        Value::String(s)
            if s.contains(['/', '\\'])
                || s.chars().any(char::is_control)
                || s.contains("Bearer ")
                || s.contains("token=")
                || [
                    "sk-",
                    "sk_",
                    "ghp_",
                    "github_pat_",
                    "xoxb-",
                    "xoxp-",
                    "AKIA",
                ]
                .iter()
                .any(|prefix| s.starts_with(prefix)) =>
        {
            *s = "[redacted]".into()
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::Object(fields) => fields.values_mut().for_each(redact),
        _ => {}
    }
}

pub fn export_diagnostics(store: &Store, params: Value) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Export {
        id: String,
        digest: String,
        file: String,
    }
    let p: Export = serde_json::from_value(params).map_err(|_| "invalid_request")?;
    let value = diagnostics(store, &p.id)?;
    if value["digest"] != p.digest {
        return Err("diagnostics_changed_review_again");
    }
    let path = url::Url::parse(&p.file)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .filter(|p| p.is_absolute())
        .ok_or("local_file_required")?;
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err("regular_file_required");
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("local_file_required")?)
        .map_err(|_| "local_export_failed")?;
    file.write_all(
        &serde_json::to_vec_pretty(&value["document"]).map_err(|_| "diagnostics_unavailable")?,
    )
    .map_err(|_| "local_export_failed")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "local_export_failed")?;
    file.persist(path).map_err(|_| "local_export_failed")?;
    Ok(json!({"exported":true,"digest":p.digest}))
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    fn fixture(setup: bool) -> (tempfile::TempDir, Store, Catalogue, Plan, Host) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::at(&dir.path().join("local"), true).unwrap();
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let s = if setup {
            Selection::Setup {
                id: "demo-writing-desk".into(),
                revision: "1".into(),
                chosen: Vec::new(),
            }
        } else {
            Selection::App {
                id: "demo-fieldnotes".into(),
            }
        };
        let now = 1_800_000_000;
        let (h, status, p) = crate::planner::sample(&c, &s, Default::default(), now).unwrap();
        let plan = crate::planner::build(&c, s, &h, &status, Ok(p), now).unwrap();
        store.save_plan(&plan).unwrap();
        (dir, store, c, plan, h)
    }
    fn consent(store: &mut Store, plan: &Plan) {
        lifecycle::consent(
            store,
            &lifecycle::Consent {
                id: plan.digest.clone(),
                digest: plan.digest.clone(),
                accepted: true,
            },
            plan,
            plan.created_at,
        )
        .unwrap();
    }
    #[test]
    fn every_interrupted_transition_requires_observation_and_never_reexecutes() {
        for stage in 0..5 {
            let (_dir, mut store, _c, plan, mut host) = fixture(false);
            let now = plan.created_at;
            if stage >= 1 {
                consent(&mut store, &plan);
            }
            if stage >= 2 {
                assert!(lifecycle::claim(&mut store, &plan.digest, now).unwrap());
            }
            if stage >= 3 {
                lifecycle::begin(&mut store, &plan.digest, &plan, now).unwrap();
            }
            if stage >= 4 {
                for p in &plan.packages {
                    host.installed.insert(p.name.clone(), p.version.clone());
                }
            }
            let result = reconcile(&mut store, &plan.digest, &host, now + 1).unwrap();
            assert_eq!(
                result["state"],
                match stage {
                    0 => "planned",
                    1 => "awaiting_user",
                    2 | 3 => "failed",
                    _ => "succeeded",
                }
            );
            assert!(
                store.sample_packages().unwrap().is_empty(),
                "reconciliation must never install"
            );
            let sequence = store.view(0).unwrap()["lastSequence"].clone();
            reconcile(&mut store, &plan.digest, &host, now + 2).unwrap();
            assert_eq!(store.view(0).unwrap()["lastSequence"], sequence);
            if stage == 4 {
                assert_eq!(
                    store.view(0).unwrap()["items"][0]["sourceState"],
                    "observed_after_operation"
                );
            }
        }
    }
    #[test]
    fn setup_detach_preserves_apps_and_removal_needs_its_own_exact_consent() {
        let (dir, mut store, c, plan, mut host) = fixture(true);
        let now = plan.created_at;
        let document = dir.path().join("notes.txt");
        std::fs::write(&document, "keep this document").unwrap();
        consent(&mut store, &plan);
        lifecycle::claim(&mut store, &plan.digest, now).unwrap();
        lifecycle::begin(&mut store, &plan.digest, &plan, now).unwrap();
        for p in &plan.packages {
            host.installed.insert(p.name.clone(), p.version.clone());
        }
        lifecycle::finish(
            &mut store,
            &plan,
            &host,
            true,
            "sample_package_state_verified",
            now + 1,
        )
        .unwrap();
        assert_eq!(setups(&store, 0).unwrap()["items"][0]["componentCount"], 1);
        assert!(!removal(&store, &c, "demo-fieldnotes", &host, now + 2)
            .unwrap()
            .blockers
            .is_empty());
        detach(
            &mut store,
            Detach {
                id: "demo-writing-desk".into(),
                revision: "1".into(),
                confirmed: true,
            },
        )
        .unwrap();
        assert_eq!(store.view(0).unwrap()["items"][0]["present"], true);
        let removal = removal(&store, &c, "demo-fieldnotes", &host, now + 2).unwrap();
        store.save_plan(&removal).unwrap();
        let args = crate::execution::arguments(&removal).unwrap();
        assert!(args.contains(&"-R".into()));
        assert!(!args.contains(&"-Rns".into()));
        consent(&mut store, &removal);
        lifecycle::claim(&mut store, &removal.digest, now + 2).unwrap();
        lifecycle::begin(&mut store, &removal.digest, &removal, now + 2).unwrap();
        host.installed.clear();
        lifecycle::finish(
            &mut store,
            &removal,
            &host,
            true,
            "sample_package_state_verified",
            now + 3,
        )
        .unwrap();
        assert_eq!(store.view(0).unwrap()["items"][0]["present"], false);
        assert_eq!(
            std::fs::read_to_string(document).unwrap(),
            "keep this document"
        );
    }
    #[test]
    fn partial_setup_replans_only_unfinished_packages_and_old_plan_bytes_survive() {
        let (_dir, mut store, mut c, mut old, _) = fixture(true);
        old.operations
            .iter_mut()
            .for_each(|op| op.disclosure = None);
        old.digest.clear();
        old.digest = platform::hash(&old);
        store.save_plan(&old).unwrap();
        assert_eq!(store.plan(&old.digest).unwrap().digest, old.digest);
        let mut second = c.apps[0].clone();
        second.id = "demo-second".into();
        second.name = "Second tool".into();
        if let omastore_catalogue::InstallRoute::ArchPackage { package, .. } =
            &mut second.releases[0].route
        {
            *package = "demo-second".into();
        }
        if let omastore_catalogue::ReleaseIdentity::RepositoryPackage { package, .. } =
            &mut second.releases[0].identity
        {
            *package = "demo-second".into();
        }
        let mut component = c.recipes[0].components[0].clone();
        component.app_id = second.id.clone();
        c.recipes[0].components.push(component);
        c.apps.push(second);
        let selection = old.selection.clone();
        let now = old.created_at;
        let (mut h, status, packages) =
            crate::planner::sample(&c, &selection, Default::default(), now).unwrap();
        let plan =
            crate::planner::build(&c, selection.clone(), &h, &status, Ok(packages), now).unwrap();
        assert_eq!(
            plan.operations
                .iter()
                .filter(|o| o.action == "install")
                .count(),
            2
        );
        store.save_plan(&plan).unwrap();
        consent(&mut store, &plan);
        lifecycle::claim(&mut store, &plan.digest, now).unwrap();
        lifecycle::begin(&mut store, &plan.digest, &plan, now).unwrap();
        let first = &plan.packages[0];
        h.installed
            .insert(first.name.clone(), first.version.clone());
        let partial = lifecycle::finish(
            &mut store,
            &plan,
            &h,
            false,
            "package_transaction_failed",
            now + 1,
        )
        .unwrap();
        assert_eq!(partial["state"], "failed");
        let (h, status, packages) =
            crate::planner::sample(&c, &selection, h.installed, now + 2).unwrap();
        let retry =
            crate::planner::build(&c, selection, &h, &status, Ok(packages), now + 2).unwrap();
        assert_eq!(
            retry
                .operations
                .iter()
                .filter(|o| o.action == "noop")
                .count(),
            1
        );
        assert_eq!(
            retry
                .operations
                .iter()
                .filter(|o| o.action == "install")
                .count(),
            1
        );
        assert_ne!(retry.digest, plan.digest);
    }
    #[test]
    fn expired_consent_and_diagnostic_private_fields_are_rejected() {
        let (dir, mut store, _c, mut plan, _host) = fixture(false);
        let request = lifecycle::Consent {
            id: plan.digest.clone(),
            digest: plan.digest.clone(),
            accepted: true,
        };
        assert!(matches!(
            lifecycle::consent(&mut store, &request, &plan, plan.expires_at),
            Err("plan_expired")
        ));
        plan.host["accountSecret"] = json!("sk_live_private");
        plan.host["path"] = json!("/home/private/documents");
        plan.operations[0].version = "/home/private/version".into();
        plan.digest.clear();
        plan.digest = platform::hash(&plan);
        store.save_plan(&plan).unwrap();
        let preview = diagnostics(&store, &plan.digest).unwrap();
        let serialized = preview.to_string();
        assert!(!serialized.contains("sk_live_private"));
        assert!(!serialized.contains("/home/private"));
        let file = url::Url::from_file_path(dir.path().join("diagnostics.json"))
            .unwrap()
            .to_string();
        assert!(export_diagnostics(
            &store,
            json!({"id":plan.digest,"digest":"changed","file":file})
        )
        .is_err());
        export_diagnostics(
            &store,
            json!({"id":plan.digest,"digest":preview["digest"],"file":file}),
        )
        .unwrap();
        let bytes = std::fs::read(dir.path().join("diagnostics.json")).unwrap();
        assert_eq!(platform::digest(bytes), preview["digest"]);
    }
}
