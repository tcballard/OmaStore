//! Six-hour upstream observations and a redacted, fail-closed distribution overlay.
use crate::{
    bounded, digest, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use omastore_catalogue::{App, Catalogue, ReleaseIdentity};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const POLL_SECONDS: i64 = 6 * 60 * 60;
pub const STATUS_SECONDS: i64 = 5 * 60;
type StatusRow = (String, Option<i64>, Option<i64>, Option<String>, bool, i64);
#[derive(Clone, Debug)]
pub struct Lease {
    pub app: App,
    pub token: String,
    pub material: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    pub declared_identity_available: bool,
    pub owner_identity: Option<String>,
    pub latest_version: Option<String>,
    pub artifact_sha256: Option<String>,
    pub source: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Report {
        app_id: String,
        kind: String,
        message: String,
    },
    Appeal {
        app_id: String,
        message: String,
    },
    Suspend {
        app_id: String,
        version: i64,
        code: String,
        reason: String,
    },
    Resolve {
        app_id: String,
        version: i64,
        reason: String,
        confirm_reviewed: bool,
    },
    CloseReport {
        id: String,
        reason: String,
    },
}
fn material(app: &App) -> String {
    digest(
        serde_json::to_vec(
            &json!({"release":app.current_release(),"source":app.source,"homepage":app.homepage}),
        )
        .unwrap_or_default(),
    )
}
fn event(
    t: &Transaction<'_>,
    key: &str,
    app: &str,
    action: &str,
    actor: &str,
    detail: Value,
    now: i64,
) -> Result<()> {
    t.execute("INSERT OR IGNORE INTO monitor_events(event_key,app_id,action,detail,actor,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![key,app,action,detail.to_string(),actor,now])?;
    Ok(())
}
fn hold(
    t: &Transaction<'_>,
    app: &str,
    code: &str,
    actor: &str,
    key: &str,
    now: i64,
) -> Result<()> {
    let open:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM distribution_holds WHERE app_id=?1 AND code=?2 AND state='open')",params![app,code],|r|r.get(0))?;
    if open {
        return Ok(());
    }
    let id = nonce()?;
    t.execute("INSERT INTO distribution_holds(id,app_id,code,created_by,created_at) VALUES(?1,?2,?3,?4,?5)",params![id,app,code,actor,now])?;
    event(
        t,
        &format!("{key}:{id}"),
        app,
        "distribution_suspended",
        actor,
        json!({"code":code}),
        now,
    )
}
impl Store {
    /// The caller supplies a validated delivered catalogue, never a submitted candidate.
    pub fn sync_monitor_catalogue(&self, c: &Catalogue, now: i64) -> Result<()> {
        if !c.validate(self.is_development()?).is_empty() {
            return Err(Error::new(422, "catalogue_invalid"));
        }
        self.transaction(|t|{
            t.execute("UPDATE monitored_apps SET active=0",[])?;
            for app in &c.apps {
                let fingerprint=material(app);let payload=serde_json::to_string(app)?;
                let old:Option<(String,String)>=t.query_row("SELECT material_digest,payload FROM monitored_apps WHERE app_id=?1",[&app.id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
                if let Some((previous,raw))=&old {
                    if previous!=&fingerprint {
                        let old_app:App=serde_json::from_str(raw)?;
                        if old_app.source!=app.source || url::Url::parse(&old_app.homepage).ok().map(|u|u.origin())!=url::Url::parse(&app.homepage).ok().map(|u|u.origin()) {
                            hold(t,&app.id,"project_changed","catalogue-monitor",&format!("source-change:{}:{fingerprint}",app.id),now)?;
                        }
                        if old_app.current_release_id==app.current_release_id && old_app.current_release().identity!=app.current_release().identity {
                            hold(t,&app.id,"identity_changed","catalogue-monitor",&format!("identity-change:{}:{fingerprint}",app.id),now)?;
                        }
                        event(t,&format!("material-change:{}:{fingerprint}",app.id),&app.id,"material_change_requires_fresh_observation","catalogue-monitor",json!({"materialDigest":fingerprint}),now)?;
                    }
                }
                t.execute("INSERT INTO monitored_apps(app_id,payload,material_digest,due_at) VALUES(?1,?2,?3,?4) ON CONFLICT(app_id) DO UPDATE SET payload=excluded.payload,active=1,version=version+CASE WHEN material_digest!=excluded.material_digest THEN 1 ELSE 0 END,due_at=CASE WHEN material_digest!=excluded.material_digest THEN excluded.due_at ELSE due_at END,last_success=CASE WHEN material_digest!=excluded.material_digest THEN NULL ELSE last_success END,route_available=CASE WHEN material_digest!=excluded.material_digest THEN 0 ELSE route_available END,lease_token=CASE WHEN material_digest!=excluded.material_digest THEN NULL ELSE lease_token END,lease_until=CASE WHEN material_digest!=excluded.material_digest THEN NULL ELSE lease_until END,material_digest=excluded.material_digest",params![app.id,payload,fingerprint,now])?;
                for test in &app.tests {
                    if let Ok(date)=chrono::DateTime::parse_from_rfc3339(&test.tested_at) {
                        if now>=date.timestamp()+90*86400 {
                            event(t,&format!("evidence-due:{}:{}:{}",app.id,test.release_id,test.tested_at),&app.id,"evidence_retest_due","catalogue-monitor",json!({"releaseId":test.release_id,"testedAt":test.tested_at}),now)?;
                        }
                    }
                }
            }Ok(())
        })
    }
    pub fn lease_monitor(&self, now: i64) -> Result<Option<Lease>> {
        self.transaction(|t|{
            let row:Option<(String,String)>=t.query_row("SELECT payload,material_digest FROM monitored_apps WHERE active=1 AND due_at<=?1 AND (lease_until IS NULL OR lease_until<=?1) ORDER BY due_at,app_id LIMIT 1",[now],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let Some((raw,material))=row else{return Ok(None)};let app:App=serde_json::from_str(&raw)?;let token=nonce()?;
            t.execute("UPDATE monitored_apps SET lease_token=?2,lease_until=?3,last_attempt=?4 WHERE app_id=?1",params![app.id,token,now+90,now])?;
            Ok(Some(Lease{app,token,material}))
        })
    }
    pub fn finish_monitor(
        &self,
        lease: &Lease,
        observed: std::result::Result<&Observation, &Error>,
        now: i64,
    ) -> Result<()> {
        self.transaction(|t|{
            let valid:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM monitored_apps WHERE app_id=?1 AND material_digest=?2 AND lease_token=?3 AND lease_until>?4 AND active=1)",params![lease.app.id,lease.material,lease.token,now],|r|r.get(0))?;
            if !valid {return Err(Error::new(409,"monitor_lease_lost"));}
            let id=&lease.app.id;
            match observed {
                Ok(o)=>{
                    bounded(&o.source,128)?;
                    for value in [&o.owner_identity,&o.latest_version,&o.artifact_sha256].into_iter().flatten(){bounded(value,256)?;}
                    if o.artifact_sha256.as_ref().is_some_and(|s|s.len()!=64||!s.bytes().all(|b|b.is_ascii_hexdigit())) {return Err(Error::new(422,"monitor_digest_invalid"));}
                    let baseline:Option<String>=t.query_row("SELECT baseline_owner FROM monitored_apps WHERE app_id=?1",[id],|r|r.get(0))?;
                    if baseline.is_some() && o.owner_identity.is_some() && baseline!=o.owner_identity {
                        hold(t,id,"ownership_changed","upstream-monitor",&format!("owner-change:{id}:{}",o.owner_identity.as_deref().unwrap()),now)?;
                    }
                    if let ReleaseIdentity::BinaryArtifact{sha256,..}=&lease.app.current_release().identity {
                        if let Some(bytes)=&o.artifact_sha256 {if bytes!=sha256 {hold(t,id,"artifact_changed","upstream-monitor",&format!("artifact-change:{id}:{}:{bytes}",lease.material),now)?;}}
                    }
                    if let Some(version)=&o.latest_version {
                        if version!=&lease.app.current_release().version {
                            let candidate=digest(format!("{id}:{version}"));
                            t.execute("INSERT OR IGNORE INTO release_candidates(id,app_id,source_version,observation,detected_at) VALUES(?1,?2,?3,?4,?5)",params![candidate,id,version,serde_json::to_string(o)?,now])?;
                            event(t,&format!("release-candidate:{candidate}"),id,"new_release_candidate","upstream-monitor",json!({"version":version,"compatibility":"unknown"}),now)?;
                        }
                    }
                    t.execute("UPDATE monitored_apps SET observation=?2,baseline_owner=COALESCE(baseline_owner,?3),last_success=?4,last_error=NULL,route_available=?5,version=version+1 WHERE app_id=?1",params![id,serde_json::to_string(o)?,o.owner_identity,now,o.declared_identity_available])?;
                },
                Err(error)=>{
                    t.execute("UPDATE monitored_apps SET last_error=?2,version=version+1 WHERE app_id=?1",params![id,error.code])?;
                    event(t,&format!("monitor-unavailable:{id}:{}",lease.token),id,"upstream_observation_unavailable","upstream-monitor",json!({"code":error.code}),now)?;
                }
            }
            t.execute("UPDATE monitored_apps SET due_at=?2,lease_token=NULL,lease_until=NULL WHERE app_id=?1",params![id,now+POLL_SECONDS])?;Ok(())
        })
    }
    pub fn public_status(&self, c: &Catalogue, ids: &[String], now: i64) -> Result<Value> {
        if ids.len() > 100 || ids.iter().any(|id| !omastore_catalogue::token(id)) {
            return Err(Error::new(422, "invalid_status_ids"));
        }
        let connection = self.connection()?;
        let mut items = Vec::new();
        for id in ids {
            let Some(app) = c.apps.iter().find(|a| a.id == *id) else {
                continue;
            };
            let row:Option<StatusRow>=connection.query_row("SELECT material_digest,last_attempt,last_success,last_error,route_available,version FROM monitored_apps WHERE app_id=?1 AND active=1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
            let mut statement=connection.prepare("SELECT DISTINCT code FROM distribution_holds WHERE app_id=?1 AND state='open' ORDER BY code")?;
            let holds = statement
                .query_map([id], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let (attempt, success, error, available, version) = row
                .as_ref()
                .map(|r| (r.1, r.2, r.3.clone(), r.4, r.5))
                .unwrap_or((None, None, Some("not_observed".into()), false, 0));
            let current = row.as_ref().is_some_and(|r| r.0 == material(app))
                && success.is_some_and(|t| t <= now && now - t <= POLL_SECONDS)
                && error.is_none();
            let timestamp =
                chrono::DateTime::from_timestamp(now, 0).ok_or(Error::new(422, "invalid_time"))?;
            let evidence = app.evidence(timestamp);
            items.push(json!({"appId":id,"releaseId":app.current_release_id,"identityDigest":digest(serde_json::to_vec(&app.current_release().identity)?),"materialDigest":material(app),"version":version,"distribution":if holds.is_empty(){"active"}else{"suspended"},"reasonCodes":holds,"sourceAvailable":available,"sourceCurrent":current,"lastAttemptAt":attempt,"lastSuccessfulObservationAt":success,"upstreamError":error,"evidence":evidence.0,"distributionEligible":current && available && holds.is_empty(),"runtimeEvidencePassing":evidence.0=="passes"}));
        }
        Ok(
            json!({"schemaVersion":1,"catalogueRevision":c.revision,"catalogueSnapshot":c.snapshot_id(),"generatedAt":now,"validUntil":now+STATUS_SECONDS,"sourcePollSeconds":POLL_SECONDS,"items":items,"notice":"A current service response and a current upstream observation are separate. Installed applications are never deleted by a suspension."}),
        )
    }
    pub fn monitoring_queue(&self, actor: &Actor) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "operator")?;
        let mut s=c.prepare("SELECT app_id,payload,version,last_attempt,last_success,last_error,route_available FROM monitored_apps WHERE active=1 ORDER BY app_id LIMIT 50")?;
        let apps = s
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, bool>(6)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let apps=apps.into_iter().map(|r|{let app:App=serde_json::from_str(&r.1)?;Ok(json!({"appId":r.0,"name":app.name,"version":r.2,"lastAttemptAt":r.3,"lastSuccessAt":r.4,"error":r.5,"routeAvailable":r.6}))}).collect::<Result<Vec<_>>>()?;
        let mut s=c.prepare("SELECT id,app_id,code,state,created_at FROM distribution_holds WHERE state='open' ORDER BY created_at LIMIT 50")?;
        let holds=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"appId":r.get::<_,String>(1)?,"code":r.get::<_,String>(2)?,"state":r.get::<_,String>(3)?,"at":r.get::<_,i64>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut s=c.prepare("SELECT id,app_id,kind,message,created_at FROM integrity_reports WHERE state='open' ORDER BY created_at LIMIT 10")?;
        let reports=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"appId":r.get::<_,String>(1)?,"kind":r.get::<_,String>(2)?,"message":r.get::<_,String>(3)?,"at":r.get::<_,i64>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut s=c.prepare("SELECT id,app_id,message,created_at FROM distribution_appeals WHERE state='open' ORDER BY created_at LIMIT 10")?;
        let appeals=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"appId":r.get::<_,String>(1)?,"message":r.get::<_,String>(2)?,"at":r.get::<_,i64>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut s=c.prepare("SELECT id,app_id,source_version,detected_at FROM release_candidates ORDER BY detected_at DESC LIMIT 20")?;
        let candidates=s.query_map([],|r|Ok(json!({"id":r.get::<_,String>(0)?,"appId":r.get::<_,String>(1)?,"version":r.get::<_,String>(2)?,"at":r.get::<_,i64>(3)?,"compatibility":"unknown"})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(
            json!({"apps":apps,"holds":holds,"reports":reports,"appeals":appeals,"candidates":candidates}),
        )
    }
}

pub(crate) fn act(t: &Transaction<'_>, actor: &Actor, action: Action, now: i64) -> Result<Value> {
    let id = nonce()?;
    match action {
        Action::Report {
            app_id,
            kind,
            message,
        } => {
            bounded(&message, 2000)?;
            if ![
                "security",
                "integrity",
                "compatibility",
                "availability",
                "ownership",
            ]
            .contains(&kind.as_str())
            {
                return Err(Error::new(422, "invalid_report_kind"));
            }
            exists(t, &app_id)?;
            t.execute("INSERT INTO integrity_reports(id,app_id,reporter,kind,message,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![id,app_id,actor.id,kind,message,now])?;
            audit(
                t,
                &actor.id,
                "private_distribution_report",
                &app_id,
                now,
                &json!({"reportId":id,"kind":kind}),
            )?;
            Ok(json!({"id":id,"appId":app_id,"state":"open","private":true}))
        }
        Action::Appeal { app_id, message } => {
            bounded(&message, 2000)?;
            exists(t, &app_id)?;
            let owner:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM entity_owners WHERE kind='app' AND entity_id=?1 AND owner=?2)",params![app_id,actor.id],|r|r.get(0))?;
            if !owner {
                return Err(Error::new(403, "listing_steward_required"));
            }
            t.execute("INSERT INTO distribution_appeals(id,app_id,author,message,created_at) VALUES(?1,?2,?3,?4,?5)",params![id,app_id,actor.id,message,now])?;
            audit(
                t,
                &actor.id,
                "distribution_appeal",
                &app_id,
                now,
                &json!({"appealId":id}),
            )?;
            Ok(json!({"id":id,"appId":app_id,"state":"open","private":true}))
        }
        Action::Suspend {
            app_id,
            version,
            code,
            reason,
        } => {
            recheck(t, actor, "operator")?;
            bounded(&reason, 2000)?;
            current(t, &app_id, version)?;
            if ![
                "security_review",
                "integrity_review",
                "ownership_review",
                "operator_hold",
                "withdrawn",
            ]
            .contains(&code.as_str())
            {
                return Err(Error::new(422, "invalid_suspension_code"));
            }
            hold(
                t,
                &app_id,
                &code,
                &actor.id,
                &format!("manual-hold:{id}"),
                now,
            )?;
            t.execute(
                "UPDATE monitored_apps SET version=version+1 WHERE app_id=?1",
                [&app_id],
            )?;
            audit(
                t,
                &actor.id,
                "operator_distribution_suspended",
                &app_id,
                now,
                &json!({"code":code,"reason":reason}),
            )?;
            Ok(json!({"appId":app_id,"version":version+1,"distribution":"suspended"}))
        }
        Action::Resolve {
            app_id,
            version,
            reason,
            confirm_reviewed,
        } => {
            recheck(t, actor, "operator")?;
            bounded(&reason, 2000)?;
            current(t, &app_id, version)?;
            if !confirm_reviewed {
                return Err(Error::new(422, "resolution_review_required"));
            }
            let (raw,last,err,available,payload):(Option<String>,Option<i64>,Option<String>,bool,String)=t.query_row("SELECT observation,last_success,last_error,route_available,payload FROM monitored_apps WHERE app_id=?1",[&app_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
            if !available
                || err.is_some()
                || !last.is_some_and(|at| at <= now && now - at <= POLL_SECONDS)
            {
                return Err(Error::new(409, "fresh_upstream_observation_required"));
            }
            let o: Observation = serde_json::from_str(
                raw.as_deref()
                    .ok_or(Error::new(409, "fresh_upstream_observation_required"))?,
            )?;
            let app: App = serde_json::from_str(&payload)?;
            if let ReleaseIdentity::BinaryArtifact { sha256, .. } = &app.current_release().identity
            {
                if o.artifact_sha256.as_deref() != Some(sha256) {
                    return Err(Error::new(409, "artifact_mismatch_unresolved"));
                }
            }
            t.execute("UPDATE distribution_holds SET state='resolved',resolved_by=?2,resolved_at=?3,resolution=?4 WHERE app_id=?1 AND state='open'",params![app_id,actor.id,now,reason])?;
            t.execute("UPDATE distribution_appeals SET state='resolved',resolved_at=?2 WHERE app_id=?1 AND state='open'",params![app_id,now])?;
            t.execute(
                "UPDATE monitored_apps SET baseline_owner=?2,version=version+1 WHERE app_id=?1",
                params![app_id, o.owner_identity],
            )?;
            event(
                t,
                &format!("operator-resolution:{id}"),
                &app_id,
                "distribution_restored",
                &actor.id,
                json!({"reviewConfirmed":true}),
                now,
            )?;
            audit(
                t,
                &actor.id,
                "operator_distribution_restored",
                &app_id,
                now,
                &json!({"reason":reason,"reviewConfirmed":true}),
            )?;
            Ok(json!({"appId":app_id,"version":version+1,"distribution":"active"}))
        }
        Action::CloseReport { id: report, reason } => {
            recheck(t, actor, "operator")?;
            bounded(&reason, 2000)?;
            if t.execute("UPDATE integrity_reports SET state='closed',resolved_at=?2 WHERE id=?1 AND state='open'",params![report,now])?!=1 {return Err(Error::new(409,"report_unavailable"));}
            audit(
                t,
                &actor.id,
                "distribution_report_closed",
                &report,
                now,
                &json!({"reason":reason}),
            )?;
            Ok(json!({"id":report,"state":"closed"}))
        }
    }
}
fn exists(t: &Transaction<'_>, id: &str) -> Result<()> {
    let exists: bool = t.query_row(
        "SELECT EXISTS(SELECT 1 FROM monitored_apps WHERE app_id=?1 AND active=1)",
        [id],
        |r| r.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(Error::new(404, "app_unavailable"))
    }
}
fn current(t: &Transaction<'_>, id: &str, version: i64) -> Result<()> {
    exists(t, id)?;
    let actual: i64 = t.query_row(
        "SELECT version FROM monitored_apps WHERE app_id=?1",
        [id],
        |r| r.get(0),
    )?;
    if actual == version {
        Ok(())
    } else {
        Err(Error::new(409, "stale_monitor_status"))
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::drafts::Command;
    fn setup() -> (Store, tempfile::TempDir, Catalogue, Actor, Actor, i64) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let mut c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let app = c
            .apps
            .iter()
            .find(|a| {
                matches!(
                    a.current_release().identity,
                    ReleaseIdentity::BinaryArtifact { .. }
                )
            })
            .unwrap()
            .clone();
        c.apps = vec![app];
        c.recipes.clear();
        c.editorial.clear();
        c.stories.clear();
        let now = crate::now();
        let a = s.development_login("author", now).unwrap();
        let a = s.actor(a["token"].as_str().unwrap(), now).unwrap();
        let o = s.development_login("operator", now).unwrap();
        let o = s.actor(o["token"].as_str().unwrap(), now).unwrap();
        (s, dir, c, a, o, now)
    }
    fn observation(c: &Catalogue) -> Observation {
        let sha = match &c.apps[0].current_release().identity {
            ReleaseIdentity::BinaryArtifact { sha256, .. } => sha256.clone(),
            _ => unreachable!(),
        };
        Observation {
            declared_identity_available: true,
            owner_identity: Some("provider:1:owner:1".into()),
            latest_version: Some(c.apps[0].current_release().version.clone()),
            artifact_sha256: Some(sha),
            source: "simulated".into(),
        }
    }
    fn poll(s: &Store, c: &Catalogue, o: std::result::Result<&Observation, &Error>, now: i64) {
        s.sync_monitor_catalogue(c, now).unwrap();
        let lease = s.lease_monitor(now).unwrap().unwrap();
        s.finish_monitor(&lease, o, now).unwrap();
    }
    fn status(s: &Store, c: &Catalogue, now: i64) -> Value {
        s.public_status(c, &[c.apps[0].id.clone()], now).unwrap()
    }
    fn version(s: &Store, c: &Catalogue, now: i64) -> i64 {
        status(s, c, now)["items"][0]["version"].as_i64().unwrap()
    }
    #[test]
    fn upstream_changes_create_unknown_candidates_and_outages_never_refresh_known_findings() {
        let (s, _dir, c, _a, o, now) = setup();
        let mut observed = observation(&c);
        observed.latest_version = Some("9.9-new".into());
        poll(&s, &c, Ok(&observed), now);
        let q = s.monitoring_queue(&o).unwrap();
        assert_eq!(q["candidates"][0]["compatibility"], "unknown");
        assert!(status(&s, &c, now)["items"][0]["sourceCurrent"]
            .as_bool()
            .unwrap());
        let error = Error::new(503, "upstream_timeout");
        poll(&s, &c, Err(&error), now + POLL_SECONDS);
        let public = status(&s, &c, now + POLL_SECONDS);
        assert_eq!(public["items"][0]["lastSuccessfulObservationAt"], now);
        assert_eq!(public["items"][0]["sourceCurrent"], false);
        assert_eq!(public["items"][0]["distributionEligible"], false);
        assert_eq!(
            s.monitoring_queue(&o).unwrap()["candidates"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(public["generatedAt"], now + POLL_SECONDS);
        assert_eq!(public["validUntil"], now + POLL_SECONDS + STATUS_SECONDS);
        observed.owner_identity = Some("provider:1:owner:2".into());
        poll(&s, &c, Ok(&observed), now + 2 * POLL_SECONDS);
        assert!(
            status(&s, &c, now + 2 * POLL_SECONDS)["items"][0]["reasonCodes"]
                .as_array()
                .unwrap()
                .contains(&json!("ownership_changed"))
        );
    }
    #[test]
    fn changed_bytes_suspend_until_fresh_matching_observation_and_authorized_resolution() {
        let (s, _dir, c, a, o, now) = setup();
        let correct = observation(&c);
        poll(&s, &c, Ok(&correct), now);
        let mut bad = correct.clone();
        bad.artifact_sha256 = Some("f".repeat(64));
        poll(&s, &c, Ok(&bad), now + POLL_SECONDS);
        assert_eq!(
            status(&s, &c, now + POLL_SECONDS)["items"][0]["distribution"],
            "suspended"
        );
        let resolve = |version| Command::Distribution {
            operation: Box::new(Action::Resolve {
                app_id: c.apps[0].id.clone(),
                version,
                reason: "Private incident detail".into(),
                confirm_reviewed: true,
            }),
        };
        let v = version(&s, &c, now + POLL_SECONDS);
        assert_eq!(
            s.command(&a, &nonce().unwrap(), resolve(v), now + POLL_SECONDS)
                .unwrap_err()
                .code,
            "role_revoked"
        );
        assert_eq!(
            s.command(&o, &nonce().unwrap(), resolve(v), now + POLL_SECONDS)
                .unwrap_err()
                .code,
            "artifact_mismatch_unresolved"
        );
        poll(&s, &c, Ok(&correct), now + 2 * POLL_SECONDS);
        let v = version(&s, &c, now + 2 * POLL_SECONDS);
        s.command(&o, &nonce().unwrap(), resolve(v), now + 2 * POLL_SECONDS)
            .unwrap();
        assert_eq!(
            status(&s, &c, now + 2 * POLL_SECONDS)["items"][0]["distribution"],
            "active"
        );
        poll(&s, &c, Ok(&bad), now + 3 * POLL_SECONDS);
        assert_eq!(
            status(&s, &c, now + 3 * POLL_SECONDS)["items"][0]["distribution"],
            "suspended"
        );
        assert!(!status(&s, &c, now + 3 * POLL_SECONDS)
            .to_string()
            .contains("Private incident detail"));
    }
    #[test]
    fn expiry_events_are_once_reports_are_private_and_material_changes_reject_old_workers() {
        let (s, _dir, mut c, a, o, now) = setup();
        let app = &c.apps[0];
        let record = omastore_catalogue::TestRecord {
            release_id: app.current_release_id.clone(),
            candidate_digest: app.candidate_digest(),
            executed_identity: app.current_release().identity.clone(),
            executed_sha256: observation(&c).artifact_sha256.unwrap(),
            result: omastore_catalogue::TestResult::NotTested,
            freshness: omastore_catalogue::Freshness::Current,
            tested_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            environment: "simulated-only".into(),
            tool_version: "test-fixture".into(),
            actor: "fictional-tester".into(),
            evidence: "https://example.invalid/simulated-evidence".into(),
            limitations:
                "Fictional record for elapsed-time validation; no actual program was executed."
                    .into(),
        };
        c.apps[0].tests.push(record);
        let date = chrono::DateTime::from_timestamp(now - 90 * 86400, 0)
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        for test in &mut c.apps[0].tests {
            test.tested_at = date.clone();
        }
        s.sync_monitor_catalogue(&c, now - 1).unwrap();
        let before: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM monitor_events WHERE action='evidence_retest_due'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(before, 0);
        s.sync_monitor_catalogue(&c, now).unwrap();
        s.sync_monitor_catalogue(&c, now + 1).unwrap();
        let after: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM monitor_events WHERE action='evidence_retest_due'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(after, c.apps[0].tests.len() as i64);
        let key = nonce().unwrap();
        let report = Command::Distribution {
            operation: Box::new(Action::Report {
                app_id: c.apps[0].id.clone(),
                kind: "security".into(),
                message: "Confidential incident material".into(),
            }),
        };
        let receipt = s.command(&a, &key, report.clone(), now).unwrap();
        assert_eq!(receipt, s.command(&a, &key, report, now).unwrap());
        assert!(!status(&s, &c, now).to_string().contains("Confidential"));
        assert!(s
            .monitoring_queue(&o)
            .unwrap()
            .to_string()
            .contains("Confidential"));
        assert!(s.monitoring_queue(&a).is_err());
        let lease = s.lease_monitor(now).unwrap().unwrap();
        c.apps[0].source = Some("https://github.com/another/project".into());
        c.apps[0].tests.clear();
        s.sync_monitor_catalogue(&c, now + 1).unwrap();
        assert_eq!(
            s.finish_monitor(&lease, Ok(&observation(&c)), now + 1)
                .unwrap_err()
                .code,
            "monitor_lease_lost"
        );
        s.set_role(&o.id, "operator", false, now + 1).unwrap();
        assert!(s.monitoring_queue(&o).is_err());
    }
}
