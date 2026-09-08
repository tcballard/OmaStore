//! Exact approved content, external-write checkpoints and delivered-snapshot authority.
use crate::{
    bounded, digest,
    drafts::candidate_digest,
    nonce,
    review::{independent, POLICY},
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use omastore_catalogue::{Catalogue, Channel};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CHECK_NAME: &str = "OmaStore / approved content";
#[derive(Clone, Debug)]
pub struct Lease {
    pub job: String,
    pub revision: String,
    pub kind: String,
    pub token: String,
    pub attempt: i64,
}
#[derive(Clone, Debug)]
pub struct Approved {
    pub revision: String,
    pub owner: String,
    pub candidate: Catalogue,
    pub payload: Catalogue,
    pub payload_digest: String,
    pub approvers: Vec<String>,
    pub created_at: i64,
}

pub(crate) fn submitted(t: &Transaction<'_>, id: &str, now: i64) -> Result<()> {
    t.execute(
        "INSERT OR IGNORE INTO publications(revision_id,created_at,updated_at) VALUES(?1,?2,?2)",
        params![id, now],
    )?;
    t.execute("INSERT OR IGNORE INTO jobs(id,revision_id,kind,state,due_at) VALUES(?1,?2,'intake','queued',?3)",params![format!("intake-{id}"),id,now])?;
    Ok(())
}
pub(crate) fn request(
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
        recheck(t, actor, "maintainer")?;
    }
    if actual != version {
        return Err(Error::new(409, "stale_revision"));
    }
    if state != "approved" && state != "publication_pending" {
        return Err(Error::new(409, "approval_required"));
    }
    approved(t, id, now)?;
    submitted(t, id, now)?;
    t.execute("UPDATE publications SET state='requested',updated_at=?2 WHERE revision_id=?1 AND state='waiting_approval'",params![id,now])?;
    t.execute("INSERT OR IGNORE INTO jobs(id,revision_id,kind,state,due_at) VALUES(?1,?2,'publication','queued',?3)",params![format!("publish-{id}"),id,now])?;
    if state == "approved" {
        t.execute(
            "UPDATE revisions SET state='publication_pending',version=version+1 WHERE id=?1",
            [id],
        )?;
    }
    audit(t, &actor.id, "publication_requested", id, now, &json!({}))?;
    Ok(
        json!({"id":id,"state":"publication_pending","version":if state=="approved"{version+1}else{version}}),
    )
}
/// Recovery changes private intent; the worker still verifies every remote resource and byte.
pub(crate) fn recover(
    t: &Transaction<'_>,
    actor: &Actor,
    id: &str,
    version: i64,
    action: &str,
    number: Option<i64>,
    now: i64,
) -> Result<Value> {
    let (owner, actual, state): (String, i64, String) = t
        .query_row(
            "SELECT owner,version,state FROM revisions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "revision_unavailable"))?;
    if owner != actor.id {
        recheck(t, actor, "maintainer")?;
    }
    if actual != version {
        return Err(Error::new(409, "stale_revision"));
    }
    if state == "published" || state == "withdrawn" {
        return Err(Error::new(409, "revision_closed"));
    }
    let busy:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE revision_id=?1 AND kind IN ('intake','publication') AND state='running' AND lease_until>?2)",params![id,now],|r|r.get(0))?;
    if busy {
        return Err(Error::new(409, "publication_busy"));
    }
    let kind = match action {
        "retry_intake" => "intake",
        "attach_issue" => {
            let n = number
                .filter(|n| *n > 0)
                .ok_or(Error::new(422, "issue_number_required"))?;
            patch_publication(t, id, json!({"attached_issue":n}), now)?;
            "intake"
        }
        "retry" | "rebase" | "attach_pr" => {
            approved(t, id, now)?;
            if action == "rebase" {
                patch_publication(t, id, json!({"rebase_requested":1}), now)?;
            }
            if action == "attach_pr" {
                let n = number
                    .filter(|n| *n > 0)
                    .ok_or(Error::new(422, "pr_number_required"))?;
                patch_publication(
                    t,
                    id,
                    json!({"attached_pr":n,"pr_number":Value::Null,"head_sha":Value::Null,"state":"requested"}),
                    now,
                )?;
            }
            "publication"
        }
        _ => return Err(Error::new(422, "unsupported_publication_recovery")),
    };
    t.execute("INSERT INTO jobs(id,revision_id,kind,state,due_at) VALUES(?1,?2,?3,'queued',?4) ON CONFLICT(revision_id,kind) DO UPDATE SET state='queued',attempts=0,due_at=excluded.due_at,lease_token=NULL,lease_until=NULL,last_error=NULL",params![format!("{kind}-{id}"),id,kind,now])?;
    t.execute("UPDATE revisions SET version=version+1,state=CASE WHEN ?2='publication' THEN 'publication_pending' ELSE state END WHERE id=?1",params![id,kind])?;
    audit(
        t,
        &actor.id,
        "publication_recovery_requested",
        id,
        now,
        &json!({"action":action,"number":number}),
    )?;
    Ok(json!({"id":id,"version":version+1,"action":action,"queued":true}))
}
pub(crate) fn approved(c: &rusqlite::Connection, id: &str, now: i64) -> Result<Approved> {
    let row=c.query_row("SELECT r.owner,r.candidate,r.digest,r.state,a.payload,a.payload_digest,a.policy,a.approvers,a.created_at FROM revisions r JOIN approvals a ON a.revision_id=r.id WHERE r.id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?,r.get::<_,i64>(8)?))).optional()?.ok_or(Error::new(409,"approval_required"))?;
    if !["approved", "publication_pending", "published"].contains(&row.3.as_str())
        || row.6 != POLICY
    {
        return Err(Error::new(409, "approval_stale"));
    }
    let candidate: Catalogue = serde_json::from_str(&row.1)?;
    let payload: Catalogue = serde_json::from_str(&row.4)?;
    if candidate_digest(&candidate)? != row.2 || candidate_digest(&payload)? != row.5 {
        return Err(Error::new(409, "approval_digest_mismatch"));
    }
    let recomputed = crate::review::eligible_payload(c, id, &candidate, now)?;
    if candidate_digest(&recomputed)? != row.5 {
        return Err(Error::new(409, "approval_evidence_changed"));
    }
    let approvers: Vec<String> = serde_json::from_str(&row.7)?;
    let mut eligible = std::collections::BTreeSet::new();
    for user in &approvers {
        let current:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM users u JOIN roles r ON r.user_id=u.id WHERE u.id=?1 AND u.active=1 AND r.role='reviewer')",[user],|r|r.get(0))?;
        let actor = Actor {
            id: user.clone(),
            login: String::new(),
            roles: vec!["reviewer".into()],
        };
        let editorial_eligible = if candidate.stories.is_empty() && candidate.editorial.is_empty() {
            true
        } else {
            c.query_row(
                "SELECT EXISTS(SELECT 1 FROM roles WHERE user_id=?1 AND role='editor')",
                [user],
                |r| r.get::<_, bool>(0),
            )?
        };
        if current && editorial_eligible && independent(c, &actor, &row.0, &candidate, now).is_ok()
        {
            eligible.insert(user);
        }
    }
    if eligible.len() < crate::review::required_reviewers(&candidate) {
        return Err(Error::new(409, "approval_reviewers_no_longer_eligible"));
    }
    Ok(Approved {
        revision: id.into(),
        owner: row.0,
        candidate,
        payload,
        payload_digest: row.5,
        approvers,
        created_at: row.8,
    })
}
impl Store {
    pub fn approved(&self, id: &str, now: i64) -> Result<Approved> {
        let connection = self.connection()?;
        approved(&connection, id, now)
    }
    pub fn publication(&self, actor: &Actor, id: &str) -> Result<Value> {
        self.revision(actor, id)?;
        self.publication_internal(id)
    }
    pub fn publication_internal(&self, id: &str) -> Result<Value> {
        let c = self.connection()?;
        c.query_row("SELECT repository,state,intake_state,issue_number,pr_number,branch,base_sha,head_sha,merged_sha,delivered_revision,error,updated_at,attached_pr,attached_issue,rebase_requested,predecessor_sha FROM publications WHERE revision_id=?1",[id],|r|Ok(json!({"id":id,"repository":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?,"intakeState":r.get::<_,String>(2)?,"issueNumber":r.get::<_,Option<i64>>(3)?,"prNumber":r.get::<_,Option<i64>>(4)?,"branch":r.get::<_,Option<String>>(5)?,"baseSha":r.get::<_,Option<String>>(6)?,"headSha":r.get::<_,Option<String>>(7)?,"mergedSha":r.get::<_,Option<String>>(8)?,"deliveredRevision":r.get::<_,Option<String>>(9)?,"error":r.get::<_,Option<String>>(10)?,"updatedAt":r.get::<_,i64>(11)?,"attachedPr":r.get::<_,Option<i64>>(12)?,"attachedIssue":r.get::<_,Option<i64>>(13)?,"rebaseRequested":r.get::<_,bool>(14)?,"predecessorSha":r.get::<_,Option<String>>(15)?}))).optional()?.ok_or(Error::new(404,"publication_unavailable"))
    }
    pub fn public_submission(&self, id: &str) -> Result<Value> {
        let c = self.connection()?;
        c.query_row("SELECT candidate,digest,state FROM revisions WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).optional()?.ok_or(Error::new(404,"submission_unavailable")).and_then(|r|Ok(json!({"id":id,"candidate":serde_json::from_str::<Value>(&r.0)?,"digest":r.1,"state":r.2,"notice":"Submitted content is a proposal. Approval and current runtime evidence are separate facts."})))
    }
    pub fn lease_provider(&self, now: i64) -> Result<Option<Lease>> {
        self.transaction(|t| {
            t.execute("UPDATE jobs SET state='failed',last_error='provider_attempts_exhausted',lease_token=NULL,lease_until=NULL WHERE kind IN ('intake','publication') AND attempts>=8 AND state='running' AND lease_until<=?1",[now])?;
            let row=t.query_row("SELECT id,revision_id,kind,attempts FROM jobs WHERE kind IN ('intake','publication') AND attempts<8 AND ((state='queued' AND due_at<=?1) OR (state='running' AND lease_until<=?1)) ORDER BY due_at,id LIMIT 1",[now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?))).optional()?;
            let Some((job,revision,kind,attempt))=row else{return Ok(None)};let token=nonce()?;
            t.execute("UPDATE jobs SET state='running',attempts=attempts+1,lease_token=?2,lease_until=?3 WHERE id=?1",params![job,token,now+300])?;
            Ok(Some(Lease{job,revision,kind,token,attempt:attempt+1}))
        })
    }
    pub fn publication_checkpoint(&self, lease: &Lease, patch: Value, now: i64) -> Result<()> {
        self.transaction(|t| {
            valid_lease(t, lease, now)?;
            patch_publication(t, &lease.revision, patch, now)
        })
    }
    pub fn finish_provider(&self, lease: &Lease, error: Option<&Error>, now: i64) -> Result<()> {
        self.transaction(|t| {
            valid_lease(t,lease,now)?;
            let state=if error.is_none(){"completed"}else if lease.attempt<8{"queued"}else{"failed"};
            t.execute("UPDATE jobs SET state=?2,due_at=?3,lease_token=NULL,lease_until=NULL,last_error=?4 WHERE id=?1",params![lease.job,state,now+30*(1_i64<<lease.attempt.min(6)),error.map(|e|e.code)])?;
            t.execute("UPDATE publications SET error=?2,updated_at=?3 WHERE revision_id=?1",params![lease.revision,error.map(|e|e.code),now])?;
            audit(t,"publication-worker","provider_job_result",&lease.revision,now,&json!({"kind":lease.kind,"state":state,"error":error.map(|e|e.code)}))
        })
    }
    pub fn expected_files(&self, id: &str) -> Result<BTreeMap<String, String>> {
        let raw: String = self.connection()?.query_row(
            "SELECT expected_files FROM publications WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )?;
        Ok(serde_json::from_str(&raw)?)
    }
    pub fn expected_registry(&self, id: &str) -> Result<Catalogue> {
        let raw: String = self.connection()?.query_row(
            "SELECT expected_registry FROM publications WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )?;
        Catalogue::parse(raw.as_bytes(), self.is_development()?)
            .map_err(|_| Error::new(500, "stored_publication_invalid"))
    }
    pub fn verify_publication_files(
        &self,
        id: &str,
        files: &BTreeMap<String, String>,
        now: i64,
    ) -> Result<()> {
        let approved = self.approved(id, now)?;
        let base: String = self.connection()?.query_row(
            "SELECT base_registry FROM publications WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )?;
        self.validate_publication_scope(&approved, &serde_json::from_str(&base)?, now)?;
        if self.expected_files(id)? != *files {
            return Err(Error::new(409, "unapproved_pr_content"));
        }
        Ok(())
    }
    pub fn mark_merged(&self, id: &str, sha: &str, now: i64) -> Result<()> {
        if !commit_sha(sha) {
            return Err(Error::new(422, "invalid_commit_identity"));
        }
        self.transaction(|t| {
            approved(t, id, now)?;
            patch_publication(t, id, json!({"state":"merged","merged_sha":sha}), now)
        })
    }
    pub fn mark_delivered(&self, id: &str, delivered: &Catalogue, now: i64) -> Result<()> {
        self.transaction(|t| {
            let approval=approved(t,id,now)?;
            let state:String=t.query_row("SELECT state FROM publications WHERE revision_id=?1",[id],|r|r.get(0))?;
            if state=="delivered" {return Ok(());}
            if state!="merged" {return Err(Error::new(409,"merge_not_observed"));}
            if !contains_payload(delivered,&approval.payload) {return Err(Error::new(409,"delivery_not_observed"));}
            patch_publication(t,id,json!({"state":"delivered","delivered_revision":delivered.revision}),now)?;
            t.execute("UPDATE revisions SET state='published',version=version+1 WHERE id=?1",[id])?;
            let base_raw:String=t.query_row("SELECT base_registry FROM publications WHERE revision_id=?1",[id],|r|r.get(0))?;
            let base:Catalogue=serde_json::from_str(&base_raw)?;
            for (kind,items) in [("app",approval.payload.apps.iter().map(|a|(a.id.as_str(),base.apps.iter().find(|v|v.id==a.id)!=Some(a))).collect::<Vec<_>>()),("maker",approval.payload.makers.iter().map(|m|(m.id.as_str(),base.makers.iter().find(|v|v.id==m.id)!=Some(m))).collect()),("setup",approval.payload.recipes.iter().map(|r|(r.id.as_str(),base.recipes.iter().find(|v|v.id==r.id)!=Some(r))).collect()),("story",approval.payload.stories.iter().map(|s|(s.id.as_str(),base.stories.iter().find(|v|v.id==s.id)!=Some(s))).collect())] {
                for (entity,changed) in items {if changed {t.execute("INSERT INTO entity_owners VALUES(?1,?2,?3,?4) ON CONFLICT(kind,entity_id) DO UPDATE SET owner=excluded.owner,revision_id=excluded.revision_id",params![kind,entity,approval.owner,id])?;}}
            }
            t.execute("INSERT INTO metadata(key,value) VALUES('delivered_catalogue',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(delivered)?])?;
            crate::feeds::delivered(t,id,&approval.payload,now)?;
            audit(t,"publication-worker","catalogue_delivery_observed",id,now,&json!({"revision":delivered.revision,"snapshot":delivered.snapshot_id()}))
        })
    }
    pub fn validate_publication_scope(
        &self,
        approval: &Approved,
        base: &Catalogue,
        now: i64,
    ) -> Result<()> {
        let c = self.connection()?;
        let mut known: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM publications WHERE expected_registry=?1) OR EXISTS(SELECT 1 FROM metadata WHERE key='delivered_catalogue' AND value=?1)",
            [serde_json::to_string(base)?],
            |r| r.get(0),
        )?;
        // Provider reads use canonical array ordering. Equivalent observed content is the same authority.
        if !known {
            let delivered: Option<String> = c
                .query_row(
                    "SELECT value FROM metadata WHERE key='delivered_catalogue'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            known = delivered
                .as_ref()
                .and_then(|s| serde_json::from_str::<Catalogue>(s).ok())
                .is_some_and(|c| c.snapshot_id() == base.snapshot_id());
        }
        let prior: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM metadata WHERE key='delivered_catalogue')",
            [],
            |r| r.get(0),
        )?;
        if !known
            && (prior
                || !base.apps.is_empty()
                || !base.recipes.is_empty()
                || !base.makers.is_empty()
                || !base.editorial.is_empty()
                || !base.stories.is_empty())
        {
            return Err(Error::new(409, "untrusted_base_catalogue"));
        }
        for proposed in &approval.payload.apps {
            if let Some(old) = base.apps.iter().find(|a| a.id == proposed.id) {
                if old == proposed {
                    continue;
                }
                if !owns_entity(&c, &approval.owner, "app", &old.id)?
                    && !controls(
                        &c,
                        &approval.owner,
                        old.source.as_deref().unwrap_or(&old.homepage),
                        now,
                    )?
                {
                    return Err(Error::new(403, "existing_listing_control_required"));
                }
            }
        }
        for proposed in &approval.payload.makers {
            if let Some(old) = base.makers.iter().find(|m| m.id == proposed.id) {
                if old != proposed
                    && !owns_entity(&c, &approval.owner, "maker", &old.id)?
                    && !controls(&c, &approval.owner, &old.homepage, now)?
                {
                    return Err(Error::new(403, "existing_maker_control_required"));
                }
            }
        }
        for story in &approval.payload.stories {
            if base.stories.iter().any(|s| s.id == story.id && s != story)
                && !owns_entity(&c, &approval.owner, "story", &story.id)?
            {
                return Err(Error::new(403, "existing_story_control_required"));
            }
            if let Some(author) = base.makers.iter().find(|m| m.id == story.author_maker_id) {
                if !owns_entity(&c, &approval.owner, "maker", &author.id)?
                    && !controls(&c, &approval.owner, &author.homepage, now)?
                {
                    return Err(Error::new(403, "editorial_author_control_required"));
                }
            }
        }
        for proposed in &approval.payload.recipes {
            if let Some(maker) = base.makers.iter().find(|m| m.id == proposed.maker_id) {
                if !owns_entity(&c, &approval.owner, "maker", &maker.id)?
                    && !controls(&c, &approval.owner, &maker.homepage, now)?
                {
                    return Err(Error::new(403, "setup_author_control_required"));
                }
            }
            if let Some(old) = base.recipes.iter().find(|r| r.id == proposed.id) {
                if old != proposed && old.revision == proposed.revision {
                    return Err(Error::new(409, "setup_revision_reused"));
                }
                if old != proposed && !owns_entity(&c, &approval.owner, "setup", &old.id)? {
                    return Err(Error::new(403, "existing_setup_control_required"));
                }
            }
        }
        Ok(())
    }
    pub fn pending_publications(&self) -> Result<Vec<String>> {
        let c = self.connection()?;
        let mut s=c.prepare("SELECT revision_id FROM publications WHERE state IN ('pr_pending','merged') ORDER BY updated_at,revision_id LIMIT 20")?;
        let rows = s
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn publication_for_pr(&self, repository: &str, number: i64) -> Result<Option<String>> {
        Ok(self
            .connection()?
            .query_row(
                "SELECT revision_id FROM publications WHERE repository=?1 AND pr_number=?2",
                params![repository, number],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn webhook(&self, id: &str, event: &str, body: &[u8], now: i64) -> Result<bool> {
        bounded(id, 128)?;
        bounded(event, 128)?;
        if body.len() > 1024 * 1024 {
            return Err(Error::new(413, "webhook_too_large"));
        }
        let payload: Value = serde_json::from_slice(body)?;
        let hash = digest(body);
        self.transaction(|t| {
            let old=t.query_row("SELECT digest FROM github_deliveries WHERE id=?1",[id],|r|r.get::<_,String>(0)).optional()?;
            if let Some(old)=old {if old!=hash {return Err(Error::new(409,"webhook_delivery_reused"));}return Ok(false);}
            // Store only routing fields. Provider-controlled prose never becomes an instruction.
            let routing=json!({"repository":payload["repository"]["full_name"],"number":payload["number"],"action":payload["action"]});
            t.execute("INSERT INTO github_deliveries(id,digest,event,payload,created_at) VALUES(?1,?2,?3,?4,?5)",params![id,hash,event,routing.to_string(),now])?;Ok(true)
        })
    }
}
fn valid_lease(t: &Transaction<'_>, lease: &Lease, now: i64) -> Result<()> {
    let valid:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE id=?1 AND revision_id=?2 AND lease_token=?3 AND lease_until>?4 AND state='running')",params![lease.job,lease.revision,lease.token,now],|r|r.get(0))?;
    if valid {
        Ok(())
    } else {
        Err(Error::new(409, "job_lease_lost"))
    }
}
fn patch_publication(t: &Transaction<'_>, id: &str, patch: Value, now: i64) -> Result<()> {
    let fields = patch
        .as_object()
        .ok_or(Error::new(500, "invalid_publication_checkpoint"))?;
    for (key, value) in fields {
        if ![
            "repository",
            "state",
            "intake_state",
            "attached_pr",
            "attached_issue",
            "rebase_requested",
            "predecessor_sha",
            "issue_number",
            "pr_number",
            "branch",
            "base_sha",
            "head_sha",
            "base_registry",
            "expected_registry",
            "expected_files",
            "merged_sha",
            "delivered_revision",
            "error",
        ]
        .contains(&key.as_str())
        {
            return Err(Error::new(500, "invalid_publication_checkpoint"));
        }
        let scalar = match value {
            Value::String(s) => rusqlite::types::Value::Text(s.clone()),
            Value::Null => rusqlite::types::Value::Null,
            Value::Number(n) => rusqlite::types::Value::Integer(
                n.as_i64()
                    .ok_or(Error::new(500, "invalid_publication_checkpoint"))?,
            ),
            _ => return Err(Error::new(500, "invalid_publication_checkpoint")),
        };
        t.execute(
            &format!("UPDATE publications SET {key}=?2,updated_at=?3 WHERE revision_id=?1"),
            params![id, scalar, now],
        )?;
    }
    let summary: Vec<_> = fields
        .keys()
        .filter(|k| !k.starts_with("expected_"))
        .collect();
    t.execute("INSERT INTO publication_events(revision_id,action,detail,created_at) VALUES(?1,'checkpoint',?2,?3)",params![id,json!({"fields":summary}).to_string(),now])?;
    Ok(())
}
pub fn commit_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn contains_payload(delivered: &Catalogue, payload: &Catalogue) -> bool {
    payload
        .stories
        .iter()
        .all(|s| delivered.stories.contains(s))
        && payload
            .apps
            .iter()
            .all(|item| delivered.apps.iter().any(|a| a == item))
        && payload
            .makers
            .iter()
            .all(|item| delivered.makers.iter().any(|m| m == item))
        && payload
            .recipes
            .iter()
            .all(|item| delivered.recipes.iter().any(|r| r == item))
        && payload
            .editorial
            .iter()
            .all(|item| delivered.editorial.contains(item))
}
pub fn merge_catalogue(
    base: &Catalogue,
    approval: &Approved,
    base_sha: &str,
    development: bool,
) -> Result<Catalogue> {
    if !commit_sha(base_sha) {
        return Err(Error::new(422, "invalid_commit_identity"));
    }
    let mut c = base.clone();
    for app in &approval.payload.apps {
        if let Some(old) = c.apps.iter().find(|a| a.id == app.id) {
            if old.source != app.source
                || url::Url::parse(&old.homepage).ok().map(|u| u.origin())
                    != url::Url::parse(&app.homepage).ok().map(|u| u.origin())
            {
                return Err(Error::new(409, "project_identity_change_requires_review"));
            }
        }
        c.apps.retain(|a| a.id != app.id);
        c.apps.push(app.clone());
    }
    for maker in &approval.payload.makers {
        c.makers.retain(|m| m.id != maker.id);
        c.makers.push(maker.clone());
    }
    for recipe in &approval.payload.recipes {
        c.recipes.retain(|r| r.id != recipe.id);
        c.recipes.push(recipe.clone());
    }
    for story in &approval.payload.stories {
        c.stories.retain(|s| s.id != story.id);
        c.stories.push(story.clone());
    }
    for editorial in &approval.payload.editorial {
        if !c.editorial.contains(editorial) {
            c.editorial.push(editorial.clone());
        }
    }
    c.apps.sort_by(|a, b| a.id.cmp(&b.id));
    c.makers.sort_by(|a, b| a.id.cmp(&b.id));
    c.recipes.sort_by(|a, b| a.id.cmp(&b.id));
    c.channel = if development {
        Channel::Development
    } else {
        Channel::Public
    };
    c.revision = format!(
        "catalogue-{}",
        approval
            .revision
            .get(..20)
            .ok_or(Error::new(422, "invalid_revision_identity"))?
    );
    c.build_revision = base_sha.into();
    c.generated_at = chrono::DateTime::from_timestamp(approval.created_at, 0)
        .ok_or(Error::new(500, "invalid_publication_time"))?
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    if !c.validate(development).is_empty() {
        return Err(Error::new(409, "publication_catalogue_invalid"));
    }
    Ok(c)
}

fn owns_entity(c: &rusqlite::Connection, owner: &str, kind: &str, id: &str) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM entity_owners WHERE kind=?1 AND entity_id=?2 AND owner=?3)",
        params![kind, id, owner],
        |r| r.get(0),
    )?)
}
fn controls(c: &rusqlite::Connection, owner: &str, url: &str, now: i64) -> Result<bool> {
    let parsed = url::Url::parse(url).ok();
    let origin = parsed.as_ref().map(|u| u.origin().ascii_serialization());
    let claim_url = if parsed
        .as_ref()
        .is_some_and(|u| u.host_str() == Some("github.com"))
    {
        url
    } else {
        origin.as_deref().unwrap_or(url)
    };
    let Ok((target, _)) = crate::auth::claim_target(claim_url) else {
        return Ok(false);
    };
    Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE user_id=?1 AND target=?2 AND revoked_at IS NULL AND expires_at>?3)",params![owner,target,now],|r|r.get(0))?)
}

impl Store {
    pub fn github_check_current(
        &self,
        repository: &str,
        pr: i64,
        head: &str,
        conclusion: &str,
    ) -> Result<bool> {
        Ok(self.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM github_checks WHERE repository=?1 AND pr_number=?2 AND head_sha=?3 AND conclusion=?4)",params![repository,pr,head,conclusion],|r|r.get(0))?)
    }
    pub fn record_github_check(
        &self,
        repository: &str,
        pr: i64,
        head: &str,
        conclusion: &str,
        id: i64,
        now: i64,
    ) -> Result<()> {
        self.connection()?.execute("INSERT INTO github_checks VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(repository,pr_number) DO UPDATE SET head_sha=excluded.head_sha,conclusion=excluded.conclusion,check_id=excluded.check_id,checked_at=excluded.checked_at",params![repository,pr,head,conclusion,id,now])?;
        Ok(())
    }
    pub fn webhook_jobs(&self) -> Result<Vec<(String, Value)>> {
        let c = self.connection()?;
        let mut s=c.prepare("SELECT id,payload FROM github_deliveries WHERE state='queued' ORDER BY created_at,id LIMIT 20")?;
        let rows = s
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, body)| Ok((id, serde_json::from_str(&body)?)))
            .collect()
    }
    pub fn finish_webhook(&self, id: &str) -> Result<()> {
        self.connection()?.execute(
            "UPDATE github_deliveries SET state='completed' WHERE id=?1",
            [id],
        )?;
        Ok(())
    }
    pub fn publication_error(&self, id: &str, error: Option<&Error>, now: i64) -> Result<()> {
        self.connection()?.execute(
            "UPDATE publications SET error=?2,updated_at=?3 WHERE revision_id=?1",
            params![id, error.map(|e| e.code), now],
        )?;
        Ok(())
    }
}
pub fn verify_webhook(secret: &[u8], signature: &str, body: &[u8]) -> Result<()> {
    use hmac::{Hmac, Mac};
    let hex = signature
        .strip_prefix("sha256=")
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(Error::new(401, "webhook_signature_invalid"))?;
    let expected = (0..64)
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| Error::new(401, "webhook_signature_invalid"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret)
        .map_err(|_| Error::new(500, "webhook_unconfigured"))?;
    mac.update(body);
    mac.verify_slice(&expected)
        .map_err(|_| Error::new(401, "webhook_signature_invalid"))
}

impl Store {
    pub fn worker_lock(&self, name: &str, now: i64, seconds: i64) -> Result<Option<String>> {
        self.transaction(|t| {
            let token=nonce()?;
            let count=t.execute("INSERT INTO worker_locks VALUES(?1,?2,?3) ON CONFLICT(name) DO UPDATE SET token=excluded.token,expires_at=excluded.expires_at WHERE worker_locks.expires_at<=?4",params![name,token,now+seconds,now])?;
            Ok((count==1).then_some(token))
        })
    }
    pub fn release_worker_lock(&self, name: &str, token: &str) -> Result<()> {
        self.connection()?.execute(
            "DELETE FROM worker_locks WHERE name=?1 AND token=?2",
            params![name, token],
        )?;
        Ok(())
    }
    pub fn publication_manifest(&self, actor: &Actor, id: &str) -> Result<Value> {
        self.revision(actor, id)?;
        let approval = self.approved(id, crate::now())?;
        let expected = self.expected_registry(id)?;
        Ok(
            json!({"registry":expected,"receipt":crate::publisher::receipt(&approval)?,"files":self.expected_files(id)?,"notice":"Exact prepared bytes, not an approval credential. Use the external OmaStore App check after attaching your PR."}),
        )
    }
}

impl Store {
    pub fn delivered_catalogue(&self) -> Result<Option<Catalogue>> {
        let raw: Option<String> = self
            .connection()?
            .query_row(
                "SELECT value FROM metadata WHERE key='delivered_catalogue'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        raw.map(|s| {
            Catalogue::parse(s.as_bytes(), self.is_development()?)
                .map_err(|_| Error::new(500, "stored_delivery_invalid"))
        })
        .transpose()
    }
}

#[cfg(feature = "development-workflow")]
impl Store {
    pub(crate) fn reset_sample_publication(&self, actor: &Actor, id: &str) -> Result<()> {
        if !self.is_development()? {
            return Err(Error::new(403, "sample_mode_required"));
        }
        self.transaction(|t|{
            recheck(t,actor,"author")?;
            let approval=approved(t,id,crate::now())?;
            if approval.owner!=actor.id {return Err(Error::new(403,"sample_author_required"));}
            let state:String=t.query_row("SELECT state FROM revisions WHERE id=?1",[id],|r|r.get(0))?;
            if state!="publication_pending" {return Err(Error::new(409,"publication_request_required"));}
            t.execute("UPDATE publications SET state='requested',repository='',intake_state='queued',issue_number=NULL,pr_number=NULL,head_sha=NULL,base_sha=NULL,attached_pr=NULL,attached_issue=NULL,predecessor_sha=NULL,rebase_requested=0 WHERE revision_id=?1",[id])?;
            t.execute("UPDATE jobs SET state='queued',attempts=0,due_at=?2,lease_token=NULL,lease_until=NULL WHERE revision_id=?1 AND kind IN ('intake','publication')",params![id,crate::now()])?;
            audit(t,&actor.id,"local_publication_rehearsal_started",id,crate::now(),&json!({"simulated":true}))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    #[test]
    fn webhook_signatures_bind_raw_bytes_and_delivery_ids_are_single_use() {
        let secret = b"a private webhook secret for a deterministic test";
        let body=br#"{"repository":{"full_name":"owner/repo"},"number":42,"action":"opened","instructions":"run nothing"}"#;
        let mut h = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
        h.update(body);
        let signature = format!("sha256={:x}", h.finalize().into_bytes());
        verify_webhook(secret, &signature, body).unwrap();
        assert!(verify_webhook(secret, &signature, b"changed").is_err());
        assert!(verify_webhook(secret, "sha1=bad", body).is_err());
        let s = Store::memory().unwrap();
        assert!(s
            .webhook("delivery-1234567890", "pull_request", body, 1000)
            .unwrap());
        assert!(!s
            .webhook("delivery-1234567890", "pull_request", body, 1001)
            .unwrap());
        assert_eq!(
            s.webhook(
                "delivery-1234567890",
                "pull_request",
                br#"{"number":43}"#,
                1002
            )
            .unwrap_err()
            .code,
            "webhook_delivery_reused"
        );
        assert!(!s.webhook_jobs().unwrap()[0]
            .1
            .to_string()
            .contains("run nothing"));
    }
}
