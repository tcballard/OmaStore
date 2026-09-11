//! Operator facts and retention never grant installation or payment authority.
use crate::{
    digest,
    media::{LocalObjects, ObjectStorage},
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use chrono::{Datelike, Duration, TimeZone};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};

pub const GATES: &[&str] = &[
    "native_omarchy",
    "five_authors_three_external",
    "author_update",
    "real_catalogue_20_to_30",
    "three_tested_setups",
    "ten_managed_lifecycles",
    "newcomers_18_of_20",
    "backup_restore",
    "failed_delivery",
    "suspension_stale_status",
    "named_operators_support",
    "dependency_secret_audit",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub gate: String,
    pub outcome: String,
    pub report_url: String,
    pub report_digest: String,
}
pub(crate) fn evidence(t: &Transaction<'_>, actor: &Actor, p: Evidence, now: i64) -> Result<Value> {
    recheck(t, actor, "operator")?;
    if !GATES.contains(&p.gate.as_str())
        || !["passed", "failed", "pending"].contains(&p.outcome.as_str())
        || p.report_digest.len() != 64
        || !p.report_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::new(422, "invalid_release_evidence"));
    }
    crate::net::public_url(&p.report_url)?;
    t.execute("INSERT INTO release_evidence VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(gate) DO UPDATE SET outcome=excluded.outcome,report_url=excluded.report_url,report_digest=excluded.report_digest,actor=excluded.actor,recorded_at=excluded.recorded_at",params![p.gate,p.outcome,p.report_url,p.report_digest,actor.id,now])?;
    audit(
        t,
        &actor.id,
        "release_evidence_recorded",
        &p.gate,
        now,
        &json!(p),
    )?;
    Ok(
        json!({"recorded":true,"notice":"Operator evidence recorded; runtime enablement still requires its separate reviewed adapter/provider gate."}),
    )
}
pub fn expansion_paused(c: &Connection) -> Result<bool> {
    Ok(c.query_row(
        "SELECT value='1' FROM metadata WHERE key='expansion_paused'",
        [],
        |r| r.get(0),
    )
    .optional()?
    .unwrap_or(false))
}
fn week_start(now: i64) -> Result<i64> {
    let date = chrono_tz::Europe::London
        .timestamp_opt(now, 0)
        .single()
        .ok_or(Error::new(422, "invalid_time"))?
        .date_naive();
    let monday = date - Duration::days(date.weekday().num_days_from_monday().into());
    Ok(chrono_tz::Europe::London
        .from_local_datetime(&monday.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .ok_or(Error::new(422, "invalid_time"))?
        .timestamp())
}
pub(crate) fn active_media_bytes(c: &Connection, owner: &str, incoming: &str) -> Result<i64> {
    let mut active = BTreeSet::new();
    let mut q = c.prepare("SELECT candidate FROM drafts WHERE owner=?1 AND archived_at IS NULL")?;
    for row in q.query_map([owner], |r| r.get::<_, String>(0))? {
        let v: Value = serde_json::from_str(&row?)?;
        for collection in ["apps", "recipes"] {
            for a in v[collection].as_array().into_iter().flatten() {
                for m in a["media"].as_array().into_iter().flatten() {
                    if let Some(sha) = m["sha256"].as_str() {
                        active.insert(sha.to_owned());
                    }
                }
            }
        }
    }
    let mut q = c.prepare("SELECT digest,MAX(bytes) FROM media WHERE owner=?1 GROUP BY digest")?;
    let mut total = 0;
    for row in q.query_map([owner], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (sha, bytes) = row?;
        if sha != incoming && active.contains(&sha) {
            total += bytes;
        }
    }
    Ok(total)
}
fn extension(content_type: &str) -> Result<&'static str> {
    match content_type {
        "image/png" => Ok("png"),
        "video/mp4" => Ok("mp4"),
        "video/webm" => Ok("webm"),
        _ => Err(Error::new(500, "invalid_media_record")),
    }
}
impl Store {
    pub fn operations_dashboard(&self, actor: &Actor, now: i64) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "operator")?;
        let mut counts = serde_json::Map::new();
        for (label,sql) in [("queuedJobs","SELECT count(*) FROM jobs WHERE state='queued'"),("failedJobs","SELECT count(*) FROM jobs WHERE state='failed'"),("expiredLeases","SELECT count(*) FROM jobs WHERE state='running' AND lease_until<?1"),("openReports","SELECT count(*) FROM integrity_reports WHERE state='open'"),("openAppeals","SELECT count(*) FROM distribution_appeals WHERE state='open'"),("heldRoutes","SELECT count(*) FROM distribution_holds WHERE state='open'"),("draftNotices","SELECT count(*) FROM drafts WHERE expiry_notice_at IS NOT NULL AND archived_at IS NULL"),("storedMediaBytes","SELECT COALESCE(SUM(bytes),0) FROM media")]{let n:i64=if sql.contains("?1"){c.query_row(sql,[now],|r|r.get(0))?}else{c.query_row(sql,[],|r|r.get(0))?};counts.insert(label.into(),json!(n));}
        let oldest:Option<i64>=c.query_row("SELECT MIN(submitted_at) FROM revisions WHERE first_response_at IS NULL AND state NOT IN ('withdrawn','rejected')",[],|r|r.get(0))?;
        let age = oldest
            .map(|v| crate::review::working_days(v, now))
            .unwrap_or(0);
        let mut q=c.prepare("SELECT week_start,week_end,overdue FROM operation_weeks ORDER BY week_start DESC LIMIT 8")?;
        let weeks:Vec<Value>=q.query_map([],|r|Ok(json!({"start":r.get::<_,i64>(0)?,"end":r.get::<_,i64>(1)?,"overdue":r.get::<_,i64>(2)?})))?.collect::<std::result::Result<_,_>>()?;
        let gates:Vec<Value>=GATES.iter().map(|gate|->Result<Value>{let row=c.query_row("SELECT outcome,report_url,report_digest,actor,recorded_at FROM release_evidence WHERE gate=?1",[gate],|r|Ok(json!({"gate":gate,"outcome":r.get::<_,String>(0)?,"reportUrl":r.get::<_,String>(1)?,"reportDigest":r.get::<_,String>(2)?,"actor":r.get::<_,String>(3)?,"recordedAt":r.get::<_,i64>(4)?}))).optional()?;Ok(row.unwrap_or(json!({"gate":gate,"outcome":"pending"})))}).collect::<Result<_>>()?;
        let environment: String = c.query_row(
            "SELECT value FROM metadata WHERE key='environment'",
            [],
            |r| r.get(0),
        )?;
        Ok(
            json!({"environment":environment,"counts":counts,"oldestWorkingDays":age,"operatorAlert":age>5,"expansionPaused":expansion_paused(&c)?,"weeks":weeks,"gates":gates,"pilotEvidenceComplete":environment=="production"&&gates.iter().all(|g|g["outcome"]=="passed"),"calendar":"Mon–Fri, Europe/London; public holidays are not modelled","notice":"Evidence is an operator assertion linked to a report. Sample data cannot satisfy public release gates."}),
        )
    }
    pub fn maintenance(&self, objects: &LocalObjects, now: i64) -> Result<Value> {
        self.transaction(|t|{
   let end=week_start(now)?;let previous=chrono_tz::Europe::London.timestamp_opt(end,0).single().unwrap().date_naive()-Duration::days(7);
   let start=chrono_tz::Europe::London.from_local_datetime(&previous.and_hms_opt(0,0,0).unwrap()).single().unwrap().timestamp();
   let mut q=t.prepare("SELECT submitted_at,first_response_at FROM revisions WHERE submitted_at<?1 AND state!='withdrawn' AND (first_response_at IS NULL OR first_response_at>=?2)")?;
   let rows:Vec<(i64,Option<i64>)>=q.query_map(params![end,start],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<_,_>>()?;
   let overdue=rows.iter().filter(|(submitted,response)|crate::review::working_days(*submitted,response.unwrap_or(end).min(end))>=3).count();
   t.execute("INSERT OR IGNORE INTO operation_weeks VALUES(?1,?2,?3,?4)",params![start,end,overdue as i64,now])?;
   let previous_bad:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM operation_weeks WHERE week_end=?1 AND overdue>0)",[start],|r|r.get(0))?;
   if overdue>0 && previous_bad {t.execute("INSERT INTO metadata VALUES('expansion_paused','1') ON CONFLICT(key) DO UPDATE SET value='1'",[])?;}
   let notice=t.execute("UPDATE drafts SET expiry_notice_at=?1 WHERE archived_at IS NULL AND expiry_notice_at IS NULL AND updated_at<?2 AND NOT EXISTS(SELECT 1 FROM revisions WHERE draft_id=drafts.id)",params![now,now-90*86400])?;
   let mut q=t.prepare("SELECT id FROM drafts WHERE archived_at IS NULL AND expiry_notice_at<?1 AND updated_at<?2 AND NOT EXISTS(SELECT 1 FROM revisions WHERE draft_id=drafts.id) LIMIT 100")?;
   let expired:Vec<String>=q.query_map(params![now-7*86400,now-90*86400],|r|r.get(0))?.collect::<std::result::Result<_,_>>()?;
   for id in &expired {t.execute("DELETE FROM media WHERE draft_id=?1 AND public_at IS NULL",[id])?;t.execute("UPDATE drafts SET candidate='{}',archived_at=?2,version=version+1 WHERE id=?1",params![id,now])?;audit(t,"system:retention","unsubmitted_draft_expired",id,now,&json!({"noticeDays":7}))?;}
   let mut removed=0;
   for (key,mtime) in objects.keys()?{if mtime>now-86400 {continue;}let sha=key.split('.').next().unwrap_or("");let retained:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM media WHERE digest=?1)",[sha],|r|r.get(0))?;if !retained {objects.remove_private(&key)?;removed+=1;}}
   let routine=t.execute("DELETE FROM job_events WHERE created_at<?1",[now-30*86400])?;
   t.execute("DELETE FROM monitor_events WHERE action='upstream_observation_unavailable' AND created_at<?1",[now-30*86400])?;
   t.execute("DELETE FROM request_keys WHERE created_at<?1",[now-30*86400])?;
   t.execute("DELETE FROM sessions WHERE expires_at<?1 OR revoked_at<?1",[now-30*86400])?;
   t.execute("DELETE FROM logins WHERE expires_at<?1",[now-86400])?;
   t.execute("DELETE FROM rate_limits WHERE window<?1",[(now-86400)/60])?;
   // Keep delivery IDs/digests for deduplication; discard only completed raw webhook bodies.
   t.execute("UPDATE github_deliveries SET payload='{}' WHERE state='completed' AND created_at<?1",[now-30*86400])?;
   Ok(json!({"notified":notice,"expiredDrafts":expired.len(),"orphanObjectsRemoved":removed,"routineEventsRemoved":routine,"expansionPaused":expansion_paused(t)?}))
  })
    }
    pub fn backup_bundle(&self, objects: &LocalObjects, destination: &Path) -> Result<Value> {
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        fs::DirBuilder::new().mode(0o700).create(destination)?;
        let dbpath = destination.join("private.db");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&dbpath)?;
        self.connection()?.backup("main", &dbpath, None)?;
        let snapshot = Connection::open(&dbpath)?;
        let mut q = snapshot.prepare("SELECT digest,content_type,MAX(public_at IS NOT NULL) FROM media GROUP BY digest,content_type")?;
        let entries: Vec<(String, String, bool)> = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<_, _>>()?;
        let dest = LocalObjects::new(&destination.join("objects"))?;
        let mut manifest = Vec::new();
        for (sha, content_type, public) in entries {
            let key = format!("{sha}.{}", extension(&content_type)?);
            let bytes = objects.read_private(&key, crate::media::VIDEO_LIMIT)?;
            dest.put_private(&key, &bytes)?;
            manifest.push(json!({"key":key,"digest":digest(bytes),"public":public}));
        }
        let result = json!({"schemaVersion":1,"databaseDigest":digest(fs::read(&dbpath)?),"objects":manifest});
        fs::write(
            destination.join("manifest.json"),
            serde_json::to_vec_pretty(&result)?,
        )?;
        Ok(json!({"backedUp":true,"objects":manifest.len()}))
    }
}
pub fn restore_bundle(bundle: &Path, database: &Path, objects: &Path) -> Result<Value> {
    use std::os::unix::fs::OpenOptionsExt;
    if database.exists() || objects.exists() {
        return Err(Error::new(409, "restore_destination_must_be_new"));
    }
    for p in [
        bundle.to_owned(),
        bundle.join("private.db"),
        bundle.join("manifest.json"),
    ] {
        if fs::symlink_metadata(p)?.file_type().is_symlink() {
            return Err(Error::new(422, "unsafe_backup"));
        }
    }
    let raw = fs::read(bundle.join("manifest.json"))?;
    if raw.len() > 4 * 1024 * 1024 {
        return Err(Error::new(422, "backup_manifest_too_large"));
    }
    let m: Value = serde_json::from_slice(&raw)?;
    let bytes = fs::read(bundle.join("private.db"))?;
    if m["schemaVersion"] != 1 || m["databaseDigest"] != digest(&bytes) {
        return Err(Error::new(422, "backup_digest_mismatch"));
    }
    let c = Connection::open_with_flags(
        bundle.join("private.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    if c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))? != "ok" {
        return Err(Error::new(422, "backup_integrity_failed"));
    }
    let source = LocalObjects {
        root: bundle.join("objects"),
    };
    let rows = m["objects"]
        .as_array()
        .ok_or(Error::new(422, "invalid_backup_manifest"))?;
    // Validate every referenced blob before creating recovery output.
    let mut q = c.prepare("SELECT digest,content_type,MAX(public_at IS NOT NULL) FROM media GROUP BY digest,content_type")?;
    for row in q.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, bool>(2)?,
        ))
    })? {
        let (sha, ct, public) = row?;
        let key = format!("{sha}.{}", extension(&ct)?);
        if !rows
            .iter()
            .any(|r| r["key"] == key && r["digest"] == sha && r["public"] == public)
        {
            return Err(Error::new(422, "backup_manifest_incomplete"));
        }
        let b = source.read_private(&key, crate::media::VIDEO_LIMIT)?;
        if digest(&b) != sha {
            return Err(Error::new(422, "backup_digest_mismatch"));
        }
    }
    let dest = LocalObjects::new(objects)?;
    for row in rows {
        let key = row["key"]
            .as_str()
            .ok_or(Error::new(422, "invalid_backup_manifest"))?;
        let b = source.read_private(key, crate::media::VIDEO_LIMIT)?;
        if digest(&b) != row["digest"] {
            return Err(Error::new(422, "backup_digest_mismatch"));
        }
        dest.put_private(key, &b)?;
        if row["public"] == true {
            dest.promote_public(key)?;
        }
    }
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(database)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    Ok(
        json!({"restored":true,"notice":"New recovery paths created. Verify the matching service version and environment before switching traffic."}),
    )
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Store, LocalObjects, Actor) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("private.db")).unwrap();
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let login = s.development_login("operator", 1_800_000_000).unwrap();
        let a = s
            .actor(login["token"].as_str().unwrap(), 1_800_000_000)
            .unwrap();
        (dir, s, objects, a)
    }
    fn seed(s: &Store, a: &Actor, id: &str, at: i64, submitted: bool) {
        s.transaction(|t|{t.execute("INSERT INTO drafts(id,owner,kind,candidate,created_at,updated_at) VALUES(?1,?2,'app','{}',?3,?3)",params![id,a.id,at])?;if submitted{t.execute("INSERT INTO revisions(id,draft_id,owner,number,candidate,digest,state,submitted_at) VALUES(?1,?1,?2,1,'{}',?1,'submitted',?3)",params![id,a.id,at])?;}Ok(())}).unwrap();
    }
    #[test]
    fn retention_notices_allow_recovery_and_preserve_submitted_history() {
        let (_d, s, o, a) = fixture();
        let now = 1_800_000_000;
        seed(&s, &a, "expired", now - 100 * 86400, false);
        seed(&s, &a, "submitted", now - 100 * 86400, true);
        assert_eq!(s.maintenance(&o, now).unwrap()["notified"], 1);
        assert_eq!(
            s.maintenance(&o, now + 6 * 86400).unwrap()["expiredDrafts"],
            0
        );
        assert_eq!(
            s.maintenance(&o, now + 8 * 86400).unwrap()["expiredDrafts"],
            1
        );
        assert!(s.draft(&a, "expired").is_err());
        assert!(s.draft(&a, "submitted").is_ok());
        assert_eq!(
            s.connection()
                .unwrap()
                .query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn consecutive_completed_weeks_pause_new_intake_and_roles_are_current() {
        let (_d, s, o, a) = fixture();
        let first = week_start(1_800_000_000).unwrap();
        seed(&s, &a, "late", first - 14 * 86400, true);
        s.maintenance(&o, first).unwrap();
        assert!(!expansion_paused(&s.connection().unwrap()).unwrap());
        s.maintenance(&o, first + 7 * 86400).unwrap();
        assert!(expansion_paused(&s.connection().unwrap()).unwrap());
        let report = s.operations_dashboard(&a, first + 7 * 86400).unwrap();
        assert_eq!(report["operatorAlert"], true);
        assert_eq!(report["pilotEvidenceComplete"], false);
        s.set_role(&a.id, "operator", false, first).unwrap();
        assert_eq!(
            s.operations_dashboard(&a, first).unwrap_err().code,
            "role_revoked"
        );
    }
    #[test]
    fn backup_restores_populated_database_and_refuses_tampered_or_existing_targets() {
        let (d, s, o, a) = fixture();
        seed(&s, &a, "draft", 1_800_000_000, false);
        let bytes = b"digest checked backup fixture";
        let sha = digest(bytes);
        let key = format!("{sha}.png");
        o.put_private(&key, bytes).unwrap();
        o.promote_public(&key).unwrap();
        s.transaction(|t|{t.execute("INSERT INTO media VALUES('asset','draft',?1,'icon',?2,'image/png',?3,1,1,NULL,'fixture','fixture',0,1)",params![a.id,sha,bytes.len() as i64])?;Ok(())}).unwrap();
        let bundle = d.path().join("backup");
        s.backup_bundle(&o, &bundle).unwrap();
        let db = d.path().join("recovered.db");
        let objects = d.path().join("recovered-objects");
        restore_bundle(&bundle, &db, &objects).unwrap();
        assert_eq!(
            LocalObjects::new(&objects)
                .unwrap()
                .read_public(&key)
                .unwrap(),
            bytes
        );
        let recovered = Store::development(&db).unwrap();
        assert!(recovered.draft(&a, "draft").is_ok());
        assert!(Store::open(&db).is_err());
        assert!(restore_bundle(&bundle, &db, &objects).is_err());
        fs::write(bundle.join("private.db"), "damaged").unwrap();
        assert_eq!(
            restore_bundle(
                &bundle,
                &d.path().join("new.db"),
                &d.path().join("new-objects")
            )
            .unwrap_err()
            .code,
            "backup_digest_mismatch"
        );
        assert!(!d.path().join("new.db").exists());
    }
    #[test]
    fn current_draft_media_quota_excludes_retained_replaced_bytes() {
        let (_d, s, _o, a) = fixture();
        seed(&s, &a, "draft", 1_800_000_000, false);
        s.transaction(|t|{t.execute("INSERT INTO media VALUES('old','draft',?1,'icon','abc','image/png',209715201,10,10,NULL,'alt','rights',0,NULL)",[&a.id])?;assert_eq!(active_media_bytes(t,&a.id,"new")?,0);t.execute("UPDATE drafts SET candidate=?1 WHERE id='draft'",[json!({"apps":[{"media":[{"sha256":"abc"}]}]}).to_string()])?;assert_eq!(active_media_bytes(t,&a.id,"new")?,209715201);assert_eq!(active_media_bytes(t,&a.id,"abc")?,0);Ok(())}).unwrap();
    }
}
