//! Durable, bounded reads. Submitted programs, build files and hooks never execute here.
use crate::{
    bounded,
    drafts::candidate_digest,
    net, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use omastore_catalogue::{Catalogue, TestRecord};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, time::Duration};

pub const TOOL_VERSION: &str = concat!("omastore-checks/", env!("CARGO_PKG_VERSION"));
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Finding {
    pub check: String,
    pub result: Outcome,
    pub code: String,
    pub detail: String,
    pub private: bool,
}
impl Finding {
    fn new(check: &str, result: Outcome, code: &str, detail: &str) -> Self {
        Self {
            check: check.into(),
            result,
            code: code.into(),
            detail: detail.into(),
            private: false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub revision: String,
    pub token: String,
    pub candidate: Catalogue,
    pub owner: String,
    pub attempt: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuntimeEvidence {
    pub revision_id: String,
    pub revision_digest: String,
    pub app_id: String,
    pub record: TestRecord,
    pub install: String,
    pub launch: String,
    pub update_or_handoff: String,
    pub removal: String,
}
impl Store {
    pub fn lease_checks(&self, now: i64) -> Result<Option<Job>> {
        self.transaction(|t| {
            t.execute("UPDATE jobs SET state='failed',last_error='attempts_exhausted' WHERE kind='checks' AND attempts>=4 AND ((state='running' AND lease_until<=?1) OR state='queued')",[now])?;
            let row=t.query_row("SELECT j.id,j.revision_id,r.candidate,r.owner,j.attempts FROM jobs j JOIN revisions r ON r.id=j.revision_id WHERE j.kind='checks' AND j.attempts<4 AND r.state IN ('submitted','checking') AND ((j.state='queued' AND j.due_at<=?1) OR (j.state='running' AND j.lease_until<=?1)) ORDER BY j.due_at,j.id LIMIT 1",[now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?))).optional()?;
            let Some((id,revision,body,owner,attempt))=row else{return Ok(None)};
            let token=nonce()?;
            t.execute("UPDATE jobs SET state='running',attempts=attempts+1,lease_token=?2,lease_until=?3 WHERE id=?1",params![id,token,now+120])?;
            t.execute("UPDATE revisions SET state='checking',version=version+1 WHERE id=?1",[&revision])?;
            t.execute("INSERT INTO job_events(job_id,event,code,created_at) VALUES(?1,'leased','checks_started',?2)",params![id,now])?;
            Ok(Some(Job{id,revision,token,candidate:Catalogue::parse(body.as_bytes(),true).map_err(|_|Error::new(500,"stored_candidate_invalid"))?,owner,attempt:attempt+1}))
        })
    }
    pub fn finish_checks(&self, job: &Job, findings: &[Finding], now: i64) -> Result<()> {
        if findings.len() > 20 {
            return Err(Error::new(422, "too_many_findings"));
        }
        self.transaction(|t| {
            let live:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM jobs j JOIN revisions r ON r.id=j.revision_id WHERE j.id=?1 AND j.lease_token=?2 AND j.lease_until>?3 AND j.state='running' AND r.state='checking')",params![job.id,job.token,now],|r|r.get(0))?;
            if !live {return Err(Error::new(409,"job_lease_lost"));}
            for f in findings {write_finding(t,&job.revision,"checks-worker",TOOL_VERSION,f,now)?;}
            t.execute("UPDATE jobs SET state='completed',lease_token=NULL,lease_until=NULL,last_error=NULL WHERE id=?1",[&job.id])?;
            t.execute("UPDATE revisions SET state='in_review',version=version+1 WHERE id=?1",[&job.revision])?;
            t.execute("INSERT INTO job_events(job_id,event,code,created_at) VALUES(?1,'completed','findings_recorded',?2)",params![job.id,now])?;
            audit(t,"checks-worker","checks_completed",&job.revision,now,&json!({"attempt":job.attempt,"tool":TOOL_VERSION,"findings":findings.len()}))
        })
    }
    pub fn check_control(&self, job: &Job, now: i64) -> Result<Finding> {
        let c = self.connection()?;
        let mut targets = BTreeSet::new();
        for app in &job.candidate.apps {
            if let Some(source) = &app.source {
                if let Ok((target, _)) = crate::auth::claim_target(source) {
                    targets.insert(target);
                }
            }
            if let Ok(url) = url::Url::parse(&app.homepage) {
                targets.insert(url.origin().ascii_serialization());
            }
        }
        let mut controlled = 0;
        for target in &targets {
            let yes:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE user_id=?1 AND target=?2 AND expires_at>?3 AND revoked_at IS NULL)",params![job.owner,target,now],|r|r.get(0))?;
            controlled += usize::from(yes);
        }
        Ok(if controlled > 0 {
            Finding::new("control",Outcome::Pass,"scoped_control_current","At least one relevant project-control challenge is current. Review the exact scope before assigning maker control.")
        } else {
            Finding::new("control",Outcome::Unavailable,"unclaimed_nomination","No current project control is proven. An informational nomination can remain explicitly unclaimed; seller control cannot be granted.")
        })
    }
    pub fn media_findings(&self, job: &Job) -> Result<Finding> {
        let c = self.connection()?;
        for app in &job.candidate.apps {
            for media in &app.media {
                let owned: bool = c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM media WHERE owner=?1 AND digest=?2)",
                    params![job.owner, media.sha256],
                    |r| r.get(0),
                )?;
                if !owned {
                    return Ok(Finding::new("media",Outcome::Unavailable,"external_media_needs_review","Some media was not normalised by this workspace. A reviewer must establish byte identity and rights before approval."));
                }
            }
        }
        Ok(Finding::new("media",Outcome::Pass,"normalised_or_absent","All declared app media is absent or refers to this author's normalised upload. Rights assertions still need human review."))
    }
    #[cfg(feature = "development-workflow")]
    pub fn sample_checks(&self, now: i64) -> Result<Value> {
        if !self.is_development()? {
            return Err(Error::new(403, "sample_mode_required"));
        }
        let Some(job) = self.lease_checks(now)? else {
            return Ok(json!({"idle":true}));
        };
        let findings = [
            Finding::new(
                "schema",
                Outcome::Pass,
                "sample_fixture",
                "Fictional sample schema check.",
            ),
            Finding::new(
                "links",
                Outcome::Pass,
                "sample_fixture",
                "Simulated link availability; no public evidence.",
            ),
            Finding::new(
                "media",
                Outcome::Pass,
                "sample_fixture",
                "Simulated media check; no public evidence.",
            ),
            Finding::new(
                "control",
                Outcome::Unavailable,
                "sample_fixture",
                "Fictional identities do not control real projects.",
            ),
        ];
        self.finish_checks(&job, &findings, now)?;
        Ok(json!({"id":job.revision,"sample":true}))
    }
    pub fn is_development(&self) -> Result<bool> {
        Ok(self.connection()?.query_row(
            "SELECT value='development' FROM metadata WHERE key='environment'",
            [],
            |r| r.get(0),
        )?)
    }
}
pub async fn run(store: &Store, job: &Job) -> Vec<Finding> {
    let mut findings=vec![Finding::new("schema",Outcome::Pass,"typed_candidate_valid","The frozen candidate passes the catalogue schema. This is not a runtime or safety endorsement.")];
    let mut links = BTreeSet::new();
    for app in &job.candidate.apps {
        links.insert(app.homepage.clone());
        links.insert(app.support.clone());
        if let Some(source) = &app.source {
            links.insert(source.clone());
        }
        for offer in &app.offers {
            links.insert(offer.url.clone());
            links.insert(offer.terms.clone());
        }
    }
    let link_result = tokio::time::timeout(Duration::from_secs(55), async {
        if links.len() > 12 {
            return Err(Error::new(422, "link_budget_exceeded"));
        }
        let mut tasks = tokio::task::JoinSet::new();
        for link in links {
            tasks.spawn(async move { net::fetch(&link, net::TEXT_LIMIT).await.map(|_| ()) });
        }
        let mut failed = false;
        while let Some(result) = tasks.join_next().await {
            if !matches!(result, Ok(Ok(()))) {
                failed = true;
            }
        }
        if failed {
            Err(Error::new(503, "one_or_more_links_unavailable"))
        } else {
            Ok(())
        }
    })
    .await;
    findings.push(if matches!(link_result,Ok(Ok(()))) {Finding::new("links",Outcome::Pass,"bounded_links_available","Declared links responded within the public HTTPS read budget. Their content is untrusted.")}else{Finding::new("links",Outcome::Unavailable,"links_unavailable","At least one source, support, seller or terms link could not be checked within the read budget. This is not a pass.")});
    findings.push(store.check_control(job, crate::now()).unwrap_or_else(|_| {
        Finding::new(
            "control",
            Outcome::Unavailable,
            "control_unavailable",
            "Project control could not be checked.",
        )
    }));
    findings.push(store.media_findings(job).unwrap_or_else(|_| {
        Finding::new(
            "media",
            Outcome::Unavailable,
            "media_unavailable",
            "Media provenance could not be checked.",
        )
    }));
    findings.push(
        match tokio::time::timeout(Duration::from_secs(30), identity_finding(&job.candidate)).await
        {
            Ok(finding) => finding,
            Err(_) => Finding::new(
                "identity",
                Outcome::Unavailable,
                "identity_budget_exceeded",
                "Source comparison exceeded its bounded read budget.",
            ),
        },
    );
    findings
}
pub(crate) fn write_finding(
    t: &Transaction<'_>,
    revision: &str,
    actor: &str,
    tool: &str,
    f: &Finding,
    now: i64,
) -> Result<()> {
    bounded(&f.check, 64)?;
    bounded(&f.code, 128)?;
    bounded(&f.detail, 2000)?;
    let result = match f.result {
        Outcome::Pass => "pass",
        Outcome::Fail => "fail",
        Outcome::Unavailable => "unavailable",
    };
    t.execute("INSERT INTO findings VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(revision_id,check_name,tool_version,actor) DO UPDATE SET result=excluded.result,code=excluded.code,detail=excluded.detail,private=excluded.private,created_at=excluded.created_at",params![nonce()?,revision,actor,f.check,tool,result,f.code,f.detail,f.private,now])?;
    Ok(())
}
pub(crate) fn record_runtime(
    t: &Transaction<'_>,
    actor: &Actor,
    e: &RuntimeEvidence,
    now: i64,
) -> Result<Value> {
    recheck(t, actor, "reviewer")?;
    let (body, owner, state): (String, String, String) = t
        .query_row(
            "SELECT candidate,owner,state FROM revisions WHERE id=?1",
            [&e.revision_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "revision_unavailable"))?;
    if owner == actor.id {
        return Err(Error::new(403, "independent_tester_required"));
    }
    if !["submitted", "checking", "in_review", "needs_changes"].contains(&state.as_str()) {
        return Err(Error::new(409, "transition_unavailable"));
    }
    let mut candidate = Catalogue::parse(body.as_bytes(), true)
        .map_err(|_| Error::new(500, "stored_candidate_invalid"))?;
    if candidate_digest(&candidate)? != e.revision_digest {
        return Err(Error::new(409, "evidence_candidate_mismatch"));
    }
    let app = candidate
        .apps
        .iter_mut()
        .find(|a| a.id == e.app_id)
        .ok_or(Error::new(422, "evidence_app_missing"))?;
    if app.candidate_digest() != e.record.candidate_digest || e.record.actor != actor.id {
        return Err(Error::new(422, "evidence_identity_mismatch"));
    }
    let tested = chrono::DateTime::parse_from_rfc3339(&e.record.tested_at)
        .map_err(|_| Error::new(422, "invalid_evidence_time"))?
        .timestamp();
    if tested > now + 60 || now - tested > 90 * 86400 {
        return Err(Error::new(422, "evidence_not_current"));
    }
    for value in [&e.install, &e.launch, &e.update_or_handoff, &e.removal] {
        bounded(value, 2000)?;
    }
    let release = app
        .releases
        .iter()
        .find(|r| r.id == e.record.release_id)
        .ok_or(Error::new(422, "evidence_invalid"))?;
    if release.identity != e.record.executed_identity {
        return Err(Error::new(422, "evidence_invalid"));
    }
    app.tests.push(e.record.clone());
    if !candidate.validate(true).is_empty() {
        return Err(Error::new(422, "evidence_invalid"));
    }
    let id = nonce()?;
    t.execute(
        "INSERT INTO runtime_evidence VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id,
            e.revision_id,
            actor.id,
            e.app_id,
            e.record.release_id,
            e.revision_digest,
            serde_json::to_string(e)?,
            now
        ],
    )?;
    audit(
        t,
        &actor.id,
        "runtime_evidence_recorded",
        &e.revision_id,
        now,
        &json!({"evidence":id,"digest":e.revision_digest,"app":e.app_id}),
    )?;
    Ok(json!({"id":id,"revisionId":e.revision_id}))
}
pub(crate) fn retry(
    t: &Transaction<'_>,
    actor: &Actor,
    id: &str,
    version: i64,
    now: i64,
) -> Result<Value> {
    let (owner, state, actual): (String, String, i64) = t
        .query_row(
            "SELECT owner,state,version FROM revisions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "revision_unavailable"))?;
    if owner != actor.id {
        recheck(t, actor, "reviewer")?;
    }
    if actual != version {
        return Err(Error::new(409, "stale_revision"));
    }
    if !["in_review", "needs_changes"].contains(&state.as_str()) {
        return Err(Error::new(409, "transition_unavailable"));
    }
    t.execute("UPDATE jobs SET state='queued',attempts=0,due_at=?2,lease_token=NULL,lease_until=NULL WHERE revision_id=?1 AND kind='checks'",params![id,now])?;
    t.execute(
        "UPDATE revisions SET state='submitted',version=version+1 WHERE id=?1",
        [id],
    )?;
    audit(t, &actor.id, "checks_retried", id, now, &json!({}))?;
    Ok(json!({"id":id,"state":"submitted","version":version+1}))
}
pub async fn identity_finding(candidate: &Catalogue) -> Finding {
    use omastore_catalogue::ReleaseIdentity;
    let mut unavailable = false;
    for app in &candidate.apps {
        let identity = &app.current_release().identity;
        match identity {
            ReleaseIdentity::SourceCommit { repository, commit } => {
                let target = crate::auth::claim_target(repository).ok().map(|v| v.0);
                let Some(target) =
                    target.and_then(|v| v.strip_prefix("https://github.com/").map(str::to_owned))
                else {
                    unavailable = true;
                    continue;
                };
                let response = net::fetch(
                    &format!("https://api.github.com/repos/{target}/commits/{commit}"),
                    net::TEXT_LIMIT,
                )
                .await;
                match response
                    .ok()
                    .and_then(|r| serde_json::from_slice::<Value>(&r.bytes).ok())
                {
                    Some(body) if body["sha"] == *commit => {}
                    Some(body) if body["sha"].is_string() => {
                        return Finding::new(
                            "identity",
                            Outcome::Fail,
                            "source_identity_mismatch",
                            "The source host returned a different commit identity.",
                        )
                    }
                    _ => unavailable = true,
                }
            }
            ReleaseIdentity::RepositoryPackage {
                repository,
                package,
                version,
                ..
            } => {
                if !["core", "extra", "multilib"].contains(&repository.as_str()) {
                    unavailable = true;
                    continue;
                }
                let url =
                    format!("https://archlinux.org/packages/{repository}/x86_64/{package}/json/");
                match net::fetch(&url, net::TEXT_LIMIT)
                    .await
                    .ok()
                    .and_then(|r| serde_json::from_slice::<Value>(&r.bytes).ok())
                {
                    Some(body) => {
                        let epoch = body["epoch"].as_u64().unwrap_or(0);
                        let observed = format!(
                            "{}{}-{}",
                            if epoch > 0 {
                                format!("{epoch}:")
                            } else {
                                String::new()
                            },
                            body["pkgver"].as_str().unwrap_or(""),
                            body["pkgrel"].as_str().unwrap_or("")
                        );
                        if observed != *version {
                            return Finding::new("identity",Outcome::Unavailable,"package_version_changed","The current repository version differs. Preserve this release identity and submit the changed version as a new candidate.");
                        }
                    }
                    None => unavailable = true,
                }
            }
            ReleaseIdentity::BinaryArtifact { .. } => unavailable = true,
        }
    }
    if unavailable {
        Finding::new("identity",Outcome::Unavailable,"manual_byte_evidence_required","Some identities need an independent VM report of the actual tested bytes. A declared digest alone is not verification.")
    } else {
        Finding::new("identity",Outcome::Pass,"source_identity_observed","Current source metadata matches the declared identity. Runtime evidence separately records executed bytes.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::test_actor, drafts::Command};
    #[test]
    fn imported_vm_evidence_is_independent_current_and_bound_to_executed_identity() {
        let now = chrono::Utc::now().timestamp();
        let store = Store::memory().unwrap();
        let (author, _) = test_actor(&store, "author", now);
        let (reviewer, _) = test_actor(&store, "reviewer", now);
        store.set_role(&author.id, "reviewer", true, now).unwrap();
        store.set_role(&reviewer.id, "reviewer", true, now).unwrap();
        let candidate: Catalogue =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        let draft = store
            .command(
                &author,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "app".into(),
                    candidate: json!(candidate),
                    base_revision: None,
                },
                now,
            )
            .unwrap();
        let submitted = store
            .command(
                &author,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: draft["id"].as_str().unwrap().into(),
                    version: 1,
                    confirm_public_preview: true,
                },
                now,
            )
            .unwrap();
        let app = &candidate.apps[0];
        let release = app.current_release();
        let mut e = RuntimeEvidence {
            revision_id: submitted["id"].as_str().unwrap().into(),
            revision_digest: submitted["digest"].as_str().unwrap().into(),
            app_id: app.id.clone(),
            record: TestRecord {
                release_id: release.id.clone(),
                candidate_digest: app.candidate_digest(),
                executed_identity: release.identity.clone(),
                executed_sha256: "a".repeat(64),
                result: omastore_catalogue::TestResult::Limitations,
                freshness: omastore_catalogue::Freshness::Current,
                tested_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                environment: "Synthetic VM report for a unit test".into(),
                tool_version: "fixture/1".into(),
                actor: reviewer.id.clone(),
                evidence: "https://example.com/test-evidence".into(),
                limitations: "Fictional data; no runtime verification is claimed".into(),
            },
            install: "Test fixture".into(),
            launch: "Test fixture".into(),
            update_or_handoff: "Test fixture".into(),
            removal: "Test fixture".into(),
        };
        assert_eq!(
            store
                .command(
                    &author,
                    &nonce().unwrap(),
                    Command::RecordRuntime {
                        evidence: Box::new(e.clone())
                    },
                    now
                )
                .unwrap_err()
                .code,
            "independent_tester_required"
        );
        e.revision_digest = "0".repeat(64);
        assert_eq!(
            store
                .command(
                    &reviewer,
                    &nonce().unwrap(),
                    Command::RecordRuntime {
                        evidence: Box::new(e.clone())
                    },
                    now
                )
                .unwrap_err()
                .code,
            "evidence_candidate_mismatch"
        );
        e.revision_digest = submitted["digest"].as_str().unwrap().into();
        let good = e.record.executed_identity.clone();
        e.record.executed_identity = omastore_catalogue::ReleaseIdentity::BinaryArtifact {
            publisher: "https://example.com".into(),
            version: "wrong".into(),
            sha256: "b".repeat(64),
        };
        assert_eq!(
            store
                .command(
                    &reviewer,
                    &nonce().unwrap(),
                    Command::RecordRuntime {
                        evidence: Box::new(e.clone())
                    },
                    now
                )
                .unwrap_err()
                .code,
            "evidence_invalid"
        );
        e.record.executed_identity = good;
        let mut inspected = candidate.clone();
        inspected.apps[0].tests.push(e.record.clone());
        assert!(
            inspected.validate(true).is_empty(),
            "{:?}",
            inspected.validate(true)
        );
        let key = nonce().unwrap();
        let command = Command::RecordRuntime {
            evidence: Box::new(e),
        };
        let receipt = store
            .command(&reviewer, &key, command.clone(), now)
            .unwrap();
        assert_eq!(
            receipt,
            store.command(&reviewer, &key, command, now).unwrap()
        );
    }
    #[test]
    fn leases_recover_and_stale_workers_cannot_publish_findings() {
        let s = Store::memory().unwrap();
        let (a, _) = test_actor(&s, "author", 1000);
        let c: Value =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        let draft = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "app".into(),
                    candidate: c,
                    base_revision: None,
                },
                1000,
            )
            .unwrap();
        s.command(
            &a,
            &nonce().unwrap(),
            Command::SubmitDraft {
                id: draft["id"].as_str().unwrap().into(),
                version: 1,
                confirm_public_preview: true,
            },
            1001,
        )
        .unwrap();
        let first = s.lease_checks(1002).unwrap().unwrap();
        assert!(s.lease_checks(1003).unwrap().is_none());
        let second = s.lease_checks(1123).unwrap().unwrap();
        assert_eq!(second.attempt, 2);
        assert_ne!(first.token, second.token);
        assert_eq!(
            s.finish_checks(&first, &[], 1124).unwrap_err().code,
            "job_lease_lost"
        );
        s.finish_checks(
            &second,
            &[Finding::new(
                "links",
                Outcome::Unavailable,
                "test_timeout",
                "Unavailable is not a pass",
            )],
            1124,
        )
        .unwrap();
        assert_eq!(
            s.revision(&a, &second.revision).unwrap()["findings"][0]["result"],
            "unavailable"
        );
    }
}
