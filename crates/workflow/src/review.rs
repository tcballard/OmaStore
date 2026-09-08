//! Review policy belongs to the private authority, not to contributor-controlled Git content.
use crate::{
    bounded,
    checks::RuntimeEvidence,
    drafts::candidate_digest,
    nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use chrono::{Datelike, TimeZone, Weekday};
use omastore_catalogue::{AppType, Catalogue, InstallRoute};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

pub const POLICY: &str = "omastore-review-v1";
pub fn working_days(start: i64, end: i64) -> u32 {
    let zone = chrono_tz::Europe::London;
    let (Some(start), Some(end)) = (
        zone.timestamp_opt(start, 0).single(),
        zone.timestamp_opt(end, 0).single(),
    ) else {
        return 0;
    };
    if end <= start {
        return 0;
    }
    let mut date = start.date_naive();
    let end = end.date_naive();
    let mut count = 0;
    while date < end && count < 10000 {
        let Some(next) = date.succ_opt() else { break };
        date = next;
        if !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            count += 1;
        }
    }
    count
}
pub fn required_reviewers(c: &Catalogue) -> usize {
    if c.apps.iter().any(|app| {
        matches!(app.app_type, AppType::ShellPlugin)
            || app.capabilities.iter().any(|c| {
                [
                    "remote-control",
                    "remote_control",
                    "privileged-helper",
                    "system-configuration",
                ]
                .contains(&c.as_str())
            })
            || app.releases.iter().any(|r| {
                !r.privileges.is_empty()
                    || !r.services.is_empty()
                    || matches!(r.route, InstallRoute::PluginExternal { .. })
            })
    }) {
        2
    } else {
        1
    }
}
pub(crate) fn independent(
    c: &Connection,
    actor: &Actor,
    owner: &str,
    candidate: &Catalogue,
    _now: i64,
) -> Result<()> {
    if actor.id == owner {
        return Err(Error::new(403, "independent_reviewer_required"));
    }
    let mut targets = Vec::new();
    for app in &candidate.apps {
        if let Some(source) = &app.source {
            if let Ok((target, _)) = crate::auth::claim_target(source) {
                targets.push(target);
            }
        }
        if let Ok(url) = url::Url::parse(&app.homepage) {
            targets.push(url.origin().ascii_serialization());
        }
    }
    for maker in &candidate.makers {
        if let Ok(url) = url::Url::parse(&maker.homepage) {
            targets.push(url.origin().ascii_serialization());
        }
    }
    for target in targets {
        let control: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM claims WHERE user_id=?1 AND target=?2 )",
            params![actor.id, target],
            |r| r.get(0),
        )?;
        if control {
            return Err(Error::new(403, "independent_reviewer_required"));
        }
    }
    Ok(())
}
impl Store {
    pub fn review_queue(&self, actor: &Actor, now: i64) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "reviewer")?;
        let mut s=c.prepare("SELECT r.id,r.owner,r.candidate,r.digest,r.state,r.version,r.submitted_at,r.first_response_at FROM revisions r WHERE state IN ('submitted','checking','in_review','needs_changes','approved','publication_pending') ORDER BY submitted_at,id LIMIT 100")?;
        let rows = s
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut items = Vec::new();
        for (id, owner, raw, digest, state, version, submitted, response) in rows {
            let candidate: Catalogue = serde_json::from_str(&raw)?;
            let days = working_days(submitted, response.unwrap_or(now));
            items.push(json!({"id":id,"owner":owner,"name":candidate.apps.first().map(|a|a.name.as_str()).or_else(||candidate.recipes.first().map(|r|r.name.as_str())).unwrap_or("Editorial submission"),"digest":digest,"state":state,"version":version,"submittedAt":submitted,"firstResponseAt":response,"workingDays":days,"responseOverdue":response.is_none()&&days>=3,"operatorAlert":response.is_none()&&days>5,"requiredReviewers":required_reviewers(&candidate),"independent":independent(&c,actor,&owner,&candidate,now).is_ok()}));
        }
        Ok(json!({"items":items,"policy":POLICY,"calendar":"Mon–Fri, Europe/London"}))
    }
    pub fn review_detail(&self, actor: &Actor, id: &str, now: i64) -> Result<Value> {
        let mut detail = self.revision(actor, id)?;
        let c = self.connection()?;
        recheck(&c, actor, "reviewer")?;
        let candidate: Catalogue = serde_json::from_value(detail["candidate"].clone())?;
        let owner = detail["owner"]
            .as_str()
            .ok_or(Error::new(500, "stored_candidate_invalid"))?;
        detail["independent"] = json!(independent(&c, actor, owner, &candidate, now).is_ok());
        detail["requiredReviewers"] = json!(required_reviewers(&candidate));
        let own = detail["owner"] == actor.id;
        let mut s=c.prepare("SELECT id,actor,decision,reason,created_at FROM review_decisions WHERE revision_id=?1 ORDER BY created_at,id LIMIT 10")?;
        detail["decisions"]=json!(s.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"actor":r.get::<_,String>(1)?,"decision":r.get::<_,String>(2)?,"reason":r.get::<_,String>(3)?,"at":r.get::<_,i64>(4)?})))?.collect::<std::result::Result<Vec<_>,_>>()?);
        let mut s = c.prepare(
            "SELECT body FROM runtime_evidence WHERE revision_id=?1 ORDER BY created_at,id LIMIT 3",
        )?;
        let evidence = s
            .query_map([id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        detail["runtimeEvidence"] = json!(evidence
            .iter()
            .map(|s| serde_json::from_str::<Value>(s))
            .collect::<std::result::Result<Vec<_>, _>>()?);
        if let Some(evidence) = detail["runtimeEvidence"].as_array_mut() {
            for report in evidence {
                for field in ["install", "launch", "updateOrHandoff", "removal"] {
                    if let Some(text) = report[field].as_str() {
                        report[field] = json!(text.chars().take(512).collect::<String>());
                    }
                }
            }
        }
        if own {
            detail["runtimeEvidence"] = json!([]);
        }
        let mut media = Vec::new();
        for app in &candidate.apps {
            for asset in &app.media {
                if let Some(media_id) = c
                    .query_row(
                        "SELECT id FROM media WHERE owner=?1 AND digest=?2 LIMIT 1",
                        params![detail["owner"].as_str().unwrap_or(""), asset.sha256],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?
                {
                    media.push(json!({"id":media_id,"kind":asset.kind,"alt":asset.alt,"rights":asset.rights,"sha256":asset.sha256}));
                }
            }
        }
        detail["media"] = json!(media);
        // Compare against the immediate prior immutable revision of this author's draft.
        let prior:Option<String>=c.query_row("SELECT p.candidate FROM revisions p JOIN revisions r ON p.draft_id=r.draft_id WHERE r.id=?1 AND p.number<r.number ORDER BY p.number DESC LIMIT 1",[id],|r|r.get(0)).optional()?;
        detail["diff"] = json!(diff(
            prior
                .as_deref()
                .and_then(|p| serde_json::from_str::<Value>(p).ok())
                .as_ref(),
            &serde_json::to_value(candidate)?
        ));
        Ok(detail)
    }
}
fn diff(old: Option<&Value>, new: &Value) -> Vec<Value> {
    fn preview(value: Option<&Value>) -> String {
        value
            .map(|v| v.to_string().chars().take(120).collect())
            .unwrap_or_default()
    }
    fn walk(path: &str, old: Option<&Value>, new: Option<&Value>, changes: &mut Vec<Value>) {
        if old == new || changes.len() >= 50 {
            return;
        }
        match (old,new) {
            (Some(Value::Object(a)),Some(Value::Object(b)))=>{
                let keys:std::collections::BTreeSet<_>=a.keys().chain(b.keys()).collect();
                for key in keys {walk(&format!("{path}/{key}"),a.get(key),b.get(key),changes);}
            },
            (Some(Value::Array(a)),Some(Value::Array(b)))=>{
                for index in 0..a.len().max(b.len()) {walk(&format!("{path}/{index}"),a.get(index),b.get(index),changes);}
            },
            _=>changes.push(json!({"path":path,"change":if old.is_none(){"added"}else if new.is_none(){"removed"}else{"changed"},"beforePreview":preview(old),"afterPreview":preview(new)})),
        }
    }
    let mut changes = Vec::new();
    walk("", old, Some(new), &mut changes);
    changes
}
pub(crate) struct Decision<'a> {
    pub id: &'a str,
    pub version: i64,
    pub decision: &'a str,
    pub reason: &'a str,
    pub acknowledge: bool,
}
pub(crate) fn decide(
    t: &Transaction<'_>,
    actor: &Actor,
    input: Decision<'_>,
    now: i64,
) -> Result<Value> {
    let Decision {
        id,
        version,
        decision,
        reason,
        acknowledge,
    } = input;
    recheck(t, actor, "reviewer")?;
    bounded(reason, 2000)?;
    if !["approve", "needs_changes", "reject"].contains(&decision) {
        return Err(Error::new(422, "invalid_review_decision"));
    }
    let (raw, owner, hash, state, actual): (String, String, String, String, i64) = t
        .query_row(
            "SELECT candidate,owner,digest,state,version FROM revisions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "revision_unavailable"))?;
    if actual != version {
        return Err(Error::new(409, "stale_revision"));
    }
    if !["in_review", "needs_changes"].contains(&state.as_str()) {
        return Err(Error::new(409, "transition_unavailable"));
    }
    let candidate: Catalogue = serde_json::from_str(&raw)?;
    independent(t, actor, &owner, &candidate, now)?;
    let payload = if decision == "approve" {
        if !acknowledge {
            return Err(Error::new(422, "review_acknowledgement_required"));
        }
        Some(eligible_payload(t, id, &candidate, now)?)
    } else {
        None
    };
    t.execute(
        "INSERT INTO review_decisions VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![nonce()?, id, actor.id, decision, reason, hash, POLICY, now],
    )?;
    let mut next = match decision {
        "reject" => "rejected",
        "needs_changes" => "needs_changes",
        _ => "in_review",
    };
    let mut receipt = Value::Null;
    if let Some(payload) = payload {
        let mut s=t.prepare("SELECT DISTINCT d.actor,u.login FROM review_decisions d JOIN users u ON u.id=d.actor JOIN roles r ON r.user_id=d.actor AND r.role='reviewer' WHERE d.revision_id=?1 AND d.decision='approve' AND d.candidate_digest=?2 AND d.policy=?3 AND u.active=1")?;
        let votes = s
            .query_map(params![id, hash, POLICY], |r| {
                Ok(Actor {
                    id: r.get(0)?,
                    login: r.get(1)?,
                    roles: vec!["reviewer".into()],
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let eligible: Vec<_> = votes
            .iter()
            .filter(|a| independent(t, a, &owner, &candidate, now).is_ok())
            .map(|a| a.id.clone())
            .collect();
        if eligible.len() >= required_reviewers(&candidate) {
            let approved = nonce()?;
            let payload_digest = candidate_digest(&payload)?;
            t.execute(
                "INSERT INTO approvals VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    approved,
                    id,
                    hash,
                    serde_json::to_string(&payload)?,
                    payload_digest,
                    POLICY,
                    serde_json::to_string(&eligible)?,
                    now
                ],
            )?;
            next = "approved";
            receipt = json!({"id":approved,"candidateDigest":hash,"payloadDigest":payload_digest,"policy":POLICY,"approvers":eligible});
        }
    }
    t.execute("UPDATE revisions SET state=?2,version=version+1,first_response_at=COALESCE(first_response_at,?3) WHERE id=?1",params![id,next,now])?;
    audit(
        t,
        &actor.id,
        "review_decision",
        id,
        now,
        &json!({"decision":decision,"reason":reason,"digest":hash,"policy":POLICY,"state":next}),
    )?;
    Ok(json!({"id":id,"state":next,"version":version+1,"approval":receipt}))
}
pub(crate) fn eligible_payload(
    t: &Connection,
    id: &str,
    candidate: &Catalogue,
    now: i64,
) -> Result<Catalogue> {
    let failed: bool = t.query_row(
        "SELECT EXISTS(SELECT 1 FROM findings WHERE revision_id=?1 AND result='fail')",
        [id],
        |r| r.get(0),
    )?;
    if failed {
        return Err(Error::new(409, "required_checks_failed"));
    }
    for check in ["schema", "links", "media"] {
        let pass:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM findings WHERE revision_id=?1 AND check_name=?2 AND actor='checks-worker' AND tool_version=?3 AND result='pass')",params![id,check,crate::checks::TOOL_VERSION],|r|r.get(0))?;
        if !pass {
            return Err(Error::new(409, "required_checks_missing"));
        }
    }
    let mut result = candidate.clone();
    for app in &mut result.apps {
        let mut s=t.prepare("SELECT body FROM runtime_evidence WHERE revision_id=?1 AND app_id=?2 AND release_id=?3 ORDER BY created_at DESC,id")?;
        let rows = s
            .query_map(params![id, app.id, app.current_release_id], |r| {
                r.get::<_, String>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut records = Vec::new();
        for row in rows {
            let e: RuntimeEvidence = serde_json::from_str(&row)?;
            let time = chrono::DateTime::parse_from_rfc3339(&e.record.tested_at)
                .map_err(|_| Error::new(500, "stored_evidence_invalid"))?
                .timestamp();
            if time <= now + 60 && now - time <= 90 * 86400 {
                records.push(e.record);
            }
        }
        if records.is_empty() {
            return Err(Error::new(409, "runtime_evidence_missing"));
        }
        app.tests = records;
    }
    if !result.validate(true).is_empty() {
        return Err(Error::new(409, "approval_payload_invalid"));
    }
    Ok(result)
}

#[cfg(feature = "development-workflow")]
impl Store {
    pub fn sample_runtime(&self, actor: &Actor, id: &str, now: i64) -> Result<Value> {
        if !self.is_development()? {
            return Err(Error::new(403, "sample_mode_required"));
        }
        let detail = self.review_detail(actor, id, now)?;
        let candidate: Catalogue = serde_json::from_value(detail["candidate"].clone())?;
        self.transaction(|t| {
            recheck(t,actor,"reviewer")?;
            independent(t,actor,detail["owner"].as_str().unwrap_or(""),&candidate,now)?;
            for app in &candidate.apps {
                let existing:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM runtime_evidence WHERE revision_id=?1 AND app_id=?2 AND actor=?3)",params![id,app.id,actor.id],|r|r.get(0))?;
                if existing {continue;}
                let release=app.current_release();
                let executed_sha256=match &release.identity {omastore_catalogue::ReleaseIdentity::BinaryArtifact{sha256,..}=>sha256.clone(),_=>crate::digest(b"fictional executed bytes")};
                let note="Simulated observation in the sample workspace. No program was installed or executed.".to_owned();
                let e=RuntimeEvidence{revision_id:id.into(),revision_digest:detail["digest"].as_str().unwrap_or("").into(),app_id:app.id.clone(),record:omastore_catalogue::TestRecord{release_id:release.id.clone(),candidate_digest:app.candidate_digest(),executed_identity:release.identity.clone(),executed_sha256,result:omastore_catalogue::TestResult::NotTested,freshness:omastore_catalogue::Freshness::Current,tested_at:chrono::DateTime::from_timestamp(now,0).ok_or(Error::new(422,"invalid_evidence_time"))?.to_rfc3339_opts(chrono::SecondsFormat::Secs,true),environment:"Fictional sample workspace · not an Omarchy VM".into(),tool_version:"omastore-sample/1".into(),actor:actor.id.clone(),evidence:"https://example.com/omastore-sample-evidence".into(),limitations:note.clone()},install:note.clone(),launch:note.clone(),update_or_handoff:note.clone(),removal:note};
                crate::checks::record_runtime(t,actor,&e,now)?;
            }
            Ok(json!({"id":id,"sample":true}))
        })
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::drafts::Command;
    fn actor(store: &Store, name: &str, now: i64) -> Actor {
        let login = store.development_login(name, now).unwrap();
        store.actor(login["token"].as_str().unwrap(), now).unwrap()
    }
    fn submitted(store: &Store, author: &Actor, high: bool, now: i64) -> String {
        let mut candidate: Value =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        if high {
            candidate["apps"][0]["capabilities"] = json!(["remote-control"]);
        }
        let draft = store
            .command(
                author,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "app".into(),
                    candidate,
                    base_revision: None,
                },
                now,
            )
            .unwrap();
        let revision = store
            .command(
                author,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: draft["id"].as_str().unwrap().into(),
                    version: 1,
                    confirm_public_preview: true,
                },
                now,
            )
            .unwrap();
        revision["id"].as_str().unwrap().into()
    }
    fn decision(id: &str, version: i64) -> Command {
        Command::ReviewDecision {
            id: id.into(),
            version,
            decision: "approve".into(),
            reason: "Reviewed the explicitly fictional candidate and limitations".into(),
            acknowledge_limits: true,
        }
    }
    #[test]
    fn approval_requires_checks_evidence_independence_two_votes_and_fresh_versions() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let now = crate::now();
        let a = actor(&s, "author", now);
        let r = actor(&s, "reviewer", now);
        let second = actor(&s, "second-reviewer", now);
        s.set_role(&a.id, "reviewer", true, now).unwrap();
        s.set_role(&a.id, "operator", true, now).unwrap();
        let id = submitted(&s, &a, true, now);
        s.sample_checks(now).unwrap();
        let version = s.revision(&a, &id).unwrap()["version"].as_i64().unwrap();
        assert_eq!(
            s.command(&a, &nonce().unwrap(), decision(&id, version), now)
                .unwrap_err()
                .code,
            "independent_reviewer_required"
        );
        assert_eq!(
            s.command(&r, &nonce().unwrap(), decision(&id, version), now)
                .unwrap_err()
                .code,
            "runtime_evidence_missing"
        );
        s.sample_runtime(&r, &id, now).unwrap();
        let first = s
            .command(&r, &nonce().unwrap(), decision(&id, version), now)
            .unwrap();
        assert_eq!(first["state"], "in_review");
        assert_eq!(
            s.command(&second, &nonce().unwrap(), decision(&id, version), now)
                .unwrap_err()
                .code,
            "stale_revision"
        );
        let key = nonce().unwrap();
        let approve = decision(&id, version + 1);
        let final_vote = s.command(&second, &key, approve.clone(), now).unwrap();
        assert_eq!(final_vote["state"], "approved");
        assert_eq!(
            final_vote["approval"]["approvers"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(final_vote, s.command(&second, &key, approve, now).unwrap());
        assert!(s
            .connection()
            .unwrap()
            .execute("UPDATE approvals SET payload='{}'", [])
            .is_err());
        assert!(s
            .connection()
            .unwrap()
            .execute("UPDATE review_decisions SET reason='changed'", [])
            .is_err());
        assert!(s
            .connection()
            .unwrap()
            .execute("UPDATE audit SET action='changed'", [])
            .is_err());
    }
    #[test]
    fn revoked_roles_and_private_findings_do_not_leak_through_cached_actor_claims() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let now = crate::now();
        let a = actor(&s, "author", now);
        let r = actor(&s, "reviewer", now);
        let id = submitted(&s, &a, false, now);
        s.sample_checks(now).unwrap();
        s.transaction(|t| {
            crate::checks::write_finding(
                t,
                &id,
                &r.id,
                "human/1",
                &crate::checks::Finding {
                    check: "private-report".into(),
                    result: crate::checks::Outcome::Unavailable,
                    code: "private_detail".into(),
                    detail: "Private reviewer observation".into(),
                    private: true,
                },
                now,
            )
        })
        .unwrap();
        assert!(!s
            .revision(&a, &id)
            .unwrap()
            .to_string()
            .contains("Private reviewer observation"));
        assert!(s
            .revision(&r, &id)
            .unwrap()
            .to_string()
            .contains("Private reviewer observation"));
        s.set_role(&r.id, "reviewer", false, now).unwrap();
        assert!(s.review_detail(&r, &id, now).is_err());
        assert!(s.revision(&r, &id).is_err());
        s.set_role(&a.id, "reviewer", true, now).unwrap();
        assert!(!s
            .revision(&a, &id)
            .unwrap()
            .to_string()
            .contains("Private reviewer observation"));
    }
    #[test]
    fn review_calendar_handles_london_dst_and_diff_names_changed_fields() {
        let friday = chrono::DateTime::parse_from_rfc3339("2026-03-27T16:00:00Z")
            .unwrap()
            .timestamp();
        let monday = chrono::DateTime::parse_from_rfc3339("2026-03-30T15:00:00Z")
            .unwrap()
            .timestamp();
        assert_eq!(working_days(friday, monday), 1);
        let a = json!({"apps":[{"name":"Before","summary":"unchanged"}]});
        let b = json!({"apps":[{"name":"After","summary":"unchanged"}]});
        let changes = diff(Some(&a), &b);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["path"], "/apps/0/name");
        assert_eq!(changes[0]["afterPreview"], "\"After\"");
    }
}
