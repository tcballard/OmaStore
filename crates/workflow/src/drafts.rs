use crate::{
    bounded, digest, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use omastore_catalogue::{Catalogue, Channel};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_DRAFT_BYTES: usize = 80 * 1024;
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Distribution {
        operation: Box<crate::monitor::Action>,
    },
    RecoverPublication {
        id: String,
        version: i64,
        action: String,
        number: Option<i64>,
    },
    RequestPublication {
        id: String,
        version: i64,
    },
    ReviewDecision {
        id: String,
        version: i64,
        decision: String,
        reason: String,
        acknowledge_limits: bool,
    },
    RecordRuntime {
        evidence: Box<crate::checks::RuntimeEvidence>,
    },
    RetryChecks {
        id: String,
        version: i64,
    },
    CreateDraft {
        kind: String,
        candidate: Value,
        base_revision: Option<String>,
    },
    SaveDraft {
        id: String,
        version: i64,
        candidate: Value,
    },
    SubmitDraft {
        id: String,
        version: i64,
        confirm_public_preview: bool,
    },
    WithdrawRevision {
        id: String,
        version: i64,
        reason: String,
    },
}
impl Command {
    pub fn role(&self) -> &'static str {
        match self {
            Self::Distribution { operation }
                if matches!(
                    operation.as_ref(),
                    crate::monitor::Action::Suspend { .. }
                        | crate::monitor::Action::Resolve { .. }
                        | crate::monitor::Action::CloseReport { .. }
                ) =>
            {
                "operator"
            }
            Self::RecordRuntime { .. } | Self::ReviewDecision { .. } => "reviewer",
            _ => "author",
        }
    }
}

impl Store {
    /// One transaction owns the effect and replay receipt. A lost reply can be retried.
    pub fn command(&self, actor: &Actor, key: &str, command: Command, now: i64) -> Result<Value> {
        if key.len() < 16
            || key.len() > 128
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::new(422, "invalid_idempotency_key"));
        }
        let hash = digest(serde_json::to_vec(&command)?);
        self.transaction(|t| {
            recheck(t, actor, command.role())?;
            let prior = t
                .query_row(
                    "SELECT digest,response FROM request_keys WHERE actor=?1 AND key=?2",
                    params![actor.id, key],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;
            if let Some((old, result)) = prior {
                if old != hash {
                    return Err(Error::new(409, "idempotency_key_reused"));
                }
                return Ok(serde_json::from_str(&result)?);
            }
            let result = execute(t, actor, command, now)?;
            t.execute(
                "INSERT INTO request_keys VALUES(?1,?2,?3,?4,?5)",
                params![actor.id, key, hash, result.to_string(), now],
            )?;
            Ok(result)
        })
    }
    pub fn workspace(&self, actor: &Actor, now: i64) -> Result<Value> {
        let claims = self.claims(actor, now)?;
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        let mut s=c.prepare("SELECT id,kind,candidate,version,updated_at,expiry_notice_at FROM drafts WHERE owner=?1 AND archived_at IS NULL ORDER BY updated_at DESC,id LIMIT 40")?;
        let rows=s.query_map([&actor.id],|r|{
            let candidate:Value=serde_json::from_str(&r.get::<_,String>(2)?).unwrap_or(Value::Null);
            Ok(json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"name":candidate["apps"][0]["name"].as_str().or_else(||candidate["recipes"][0]["name"].as_str()).unwrap_or("Untitled listing").chars().take(160).collect::<String>(),"version":r.get::<_,i64>(3)?,"updatedAt":r.get::<_,i64>(4)?,"expiryNoticeAt":r.get::<_,Option<i64>>(5)?}))
        })?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut s=c.prepare("SELECT id,draft_id,number,digest,state,version,submitted_at FROM revisions WHERE owner=?1 ORDER BY submitted_at DESC,id LIMIT 40")?;
        let revisions=s.query_map([&actor.id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"draftId":r.get::<_,String>(1)?,"number":r.get::<_,i64>(2)?,"digest":r.get::<_,String>(3)?,"state":r.get::<_,String>(4)?,"version":r.get::<_,i64>(5)?,"submittedAt":r.get::<_,i64>(6)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(json!({"actor":actor,"claims":claims,"drafts":rows,"revisions":revisions}))
    }
    pub fn draft(&self, actor: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        let row=c.query_row("SELECT candidate,version,kind,updated_at,base_revision FROM drafts WHERE id=?1 AND owner=?2 AND archived_at IS NULL",params![id,actor.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,Option<String>>(4)?))).optional()?.ok_or(Error::new(404,"draft_unavailable"))?;
        Ok(
            json!({"id":id,"candidate":serde_json::from_str::<Value>(&row.0)?,"version":row.1,"kind":row.2,"updatedAt":row.3,"baseRevision":row.4}),
        )
    }
    pub fn revision(&self, actor: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        let reviewer:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM roles WHERE user_id=?1 AND role IN ('reviewer','maintainer','operator'))",[&actor.id],|r|r.get(0))?;
        let row = c
            .query_row(
                "SELECT owner,candidate,digest,state,version,draft_id FROM revisions WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or(Error::new(404, "revision_unavailable"))?;
        if row.0 != actor.id && !reviewer {
            return Err(Error::new(404, "revision_unavailable"));
        }
        let mut s=c.prepare("SELECT id,actor,check_name,result,code,detail,created_at FROM findings WHERE revision_id=?1 AND (private=0 OR ?2=1) ORDER BY created_at,id LIMIT 10")?;
        let findings=s.query_map(params![id,reviewer && row.0!=actor.id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"actor":r.get::<_,String>(1)?,"check":r.get::<_,String>(2)?,"result":r.get::<_,String>(3)?,"code":r.get::<_,String>(4)?,"detail":r.get::<_,String>(5)?,"at":r.get::<_,i64>(6)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let mut decisions=c.prepare("SELECT actor,decision,reason,created_at FROM review_decisions WHERE revision_id=?1 ORDER BY created_at DESC,id LIMIT 10")?;
        let decisions=decisions.query_map([id],|r|Ok(json!({"actor":r.get::<_,String>(0)?,"decision":r.get::<_,String>(1)?,"reason":r.get::<_,String>(2)?,"at":r.get::<_,i64>(3)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        let approval=c.query_row("SELECT id,candidate_digest,payload_digest,policy,created_at FROM approvals WHERE revision_id=?1",[id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"candidateDigest":r.get::<_,String>(1)?,"payloadDigest":r.get::<_,String>(2)?,"policy":r.get::<_,String>(3)?,"at":r.get::<_,i64>(4)?}))).optional()?;
        Ok(
            json!({"id":id,"owner":row.0,"candidate":serde_json::from_str::<Value>(&row.1)?,"digest":row.2,"state":row.3,"version":row.4,"draftId":row.5,"findings":findings,"decisions":decisions,"approval":approval}),
        )
    }
}
fn execute(t: &Transaction<'_>, actor: &Actor, command: Command, now: i64) -> Result<Value> {
    match command {
        Command::Distribution { operation } => crate::monitor::act(t, actor, *operation, now),
        Command::RecoverPublication {
            id,
            version,
            action,
            number,
        } => crate::publication::recover(t, actor, &id, version, &action, number, now),
        Command::RequestPublication { id, version } => {
            crate::publication::request(t, actor, &id, version, now)
        }
        Command::ReviewDecision {
            id,
            version,
            decision,
            reason,
            acknowledge_limits,
        } => crate::review::decide(
            t,
            actor,
            crate::review::Decision {
                id: &id,
                version,
                decision: &decision,
                reason: &reason,
                acknowledge: acknowledge_limits,
            },
            now,
        ),
        Command::RecordRuntime { evidence } => {
            crate::checks::record_runtime(t, actor, &evidence, now)
        }
        Command::RetryChecks { id, version } => crate::checks::retry(t, actor, &id, version, now),
        Command::CreateDraft {
            kind,
            candidate,
            base_revision,
        } => {
            if !["app", "setup", "editorial"].contains(&kind.as_str()) {
                return Err(Error::new(422, "invalid_draft_kind"));
            }
            if let Some(base) = &base_revision {
                bounded(base, 128)?;
            }
            let count: u32 = t.query_row(
                "SELECT COUNT(*) FROM drafts WHERE owner=?1 AND archived_at IS NULL",
                [&actor.id],
                |r| r.get(0),
            )?;
            if count >= 40 {
                return Err(Error::new(422, "draft_limit_reached"));
            }
            let candidate = bounded_candidate(candidate)?;
            let id = nonce()?;
            t.execute("INSERT INTO drafts(id,owner,kind,candidate,version,base_revision,created_at,updated_at) VALUES(?1,?2,?3,?4,1,?5,?6,?6)",params![id,actor.id,kind,candidate,base_revision,now])?;
            audit(
                t,
                &actor.id,
                "draft_created",
                &id,
                now,
                &json!({"kind":kind}),
            )?;
            Ok(json!({"id":id,"version":1}))
        }
        Command::SaveDraft {
            id,
            version,
            candidate,
        } => {
            owned_version(t, actor, &id, version)?;
            let candidate = bounded_candidate(candidate)?;
            t.execute("UPDATE drafts SET candidate=?2,version=version+1,updated_at=?3,expiry_notice_at=NULL WHERE id=?1",params![id,candidate,now])?;
            audit(
                t,
                &actor.id,
                "draft_saved",
                &id,
                now,
                &json!({"version":version+1}),
            )?;
            Ok(json!({"id":id,"version":version+1}))
        }
        Command::SubmitDraft {
            id,
            version,
            confirm_public_preview,
        } => {
            if !confirm_public_preview {
                return Err(Error::new(422, "public_preview_required"));
            }
            owned_version(t, actor, &id, version)?;
            let (body, kind): (String, String) = t.query_row(
                "SELECT candidate,kind FROM drafts WHERE id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let c = validate_candidate(body.as_bytes(), &kind)?;
            let hash = candidate_digest(&c)?;
            let existing=t.query_row("SELECT id,state,version FROM revisions WHERE draft_id=?1 AND digest=?2",params![id,hash],|r|Ok(json!({"id":r.get::<_,String>(0)?,"state":r.get::<_,String>(1)?,"version":r.get::<_,i64>(2)?}))).optional()?;
            if let Some(prior) = existing {
                return Ok(prior);
            }
            let revision = nonce()?;
            let number: i64 = t.query_row(
                "SELECT COALESCE(MAX(number),0)+1 FROM revisions WHERE draft_id=?1",
                [&id],
                |r| r.get(0),
            )?;
            t.execute("INSERT INTO revisions(id,draft_id,owner,number,candidate,digest,state,version,submitted_at) VALUES(?1,?2,?3,?4,?5,?6,'submitted',1,?7)",params![revision,id,actor.id,number,serde_json::to_string(&c)?,hash,now])?;
            t.execute("INSERT INTO jobs(id,revision_id,kind,state,due_at) VALUES(?1,?2,'checks','queued',?3)",params![nonce()?,revision,now])?;
            audit(
                t,
                &actor.id,
                "revision_submitted",
                &revision,
                now,
                &json!({"digest":hash,"number":number}),
            )?;
            crate::publication::submitted(t, &revision, now)?;
            Ok(json!({"id":revision,"digest":hash,"state":"submitted","version":1,"number":number}))
        }
        Command::WithdrawRevision {
            id,
            version,
            reason,
        } => {
            bounded(&reason, 2000)?;
            let row = t
                .query_row(
                    "SELECT state,version FROM revisions WHERE id=?1 AND owner=?2",
                    params![id, actor.id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(Error::new(404, "revision_unavailable"))?;
            if row.1 != version {
                return Err(Error::new(409, "stale_revision"));
            }
            if row.0 == "publication_pending" {
                let busy:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM worker_locks WHERE name='publication-dispatch' AND expires_at>?1) OR EXISTS(SELECT 1 FROM publications WHERE revision_id=?2 AND state IN ('merged','delivered'))",params![now,id],|r|r.get(0))?;
                if busy {
                    return Err(Error::new(409, "delivery_in_progress"));
                }
            }
            if ["published", "withdrawn", "rejected"].contains(&row.0.as_str()) {
                return Err(Error::new(409, "transition_unavailable"));
            }
            t.execute(
                "UPDATE revisions SET state='withdrawn',version=version+1 WHERE id=?1",
                [&id],
            )?;
            t.execute("UPDATE jobs SET state='cancelled',lease_token=NULL,lease_until=NULL WHERE revision_id=?1 AND state IN ('queued','running')",[&id])?;
            audit(
                t,
                &actor.id,
                "revision_withdrawn",
                &id,
                now,
                &json!({"reason":reason}),
            )?;
            Ok(json!({"id":id,"state":"withdrawn","version":version+1}))
        }
    }
}
fn bounded_candidate(candidate: Value) -> Result<String> {
    if !candidate.is_object() {
        return Err(Error::new(422, "invalid_candidate"));
    }
    let bytes = serde_json::to_string(&candidate)?;
    if bytes.len() > MAX_DRAFT_BYTES {
        return Err(Error::new(413, "candidate_too_large"));
    }
    Ok(bytes)
}
pub(crate) fn owned_version(
    t: &Transaction<'_>,
    actor: &Actor,
    id: &str,
    version: i64,
) -> Result<()> {
    let actual: Option<i64> = t
        .query_row(
            "SELECT version FROM drafts WHERE id=?1 AND owner=?2 AND archived_at IS NULL",
            params![id, actor.id],
            |r| r.get(0),
        )
        .optional()?;
    match actual {
        None => Err(Error::new(404, "draft_unavailable")),
        Some(v) if v != version => Err(Error::new(409, "stale_revision")),
        _ => Ok(()),
    }
}
pub fn validate_candidate(bytes: &[u8], kind: &str) -> Result<Catalogue> {
    if bytes.len() > MAX_DRAFT_BYTES {
        return Err(Error::new(413, "candidate_too_large"));
    }
    let mut c = Catalogue::parse(bytes, true).map_err(|_| Error::new(422, "candidate_invalid"))?;
    if (kind == "app"
        && (c.apps.len() != 1
            || !c.recipes.is_empty()
            || !c.editorial.is_empty()
            || !c.stories.is_empty()))
        || (kind == "setup" && c.recipes.len() != 1)
        || c.makers.len() > 10
        || (kind != "app" && kind != "setup" && kind != "editorial")
    {
        return Err(Error::new(422, "candidate_scope_invalid"));
    }
    if c.apps.iter().any(|a| !a.tests.is_empty())
        || c.makers.iter().any(|m| {
            m.claim != omastore_catalogue::Claim::Unclaimed
                || m.claim_evidence.is_some()
                || m.claim_verified_at.is_some()
                || m.claim_expires_at.is_some()
        })
    {
        return Err(Error::new(422, "candidate_cannot_self_verify"));
    }
    c.channel = Channel::Development;
    Ok(c)
}
pub fn candidate_digest(c: &Catalogue) -> Result<String> {
    // All publishable content is bound. Delivery timestamps/revisions are assigned by the publisher.
    let mut value =
        json!({"apps":c.apps,"makers":c.makers,"recipes":c.recipes,"editorial":c.editorial});
    if !c.stories.is_empty() {
        value["stories"] = json!(c.stories);
    }
    Ok(digest(serde_json::to_vec(&value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::test_actor;
    fn candidate() -> Value {
        serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap()
    }
    #[test]
    fn private_draft_conflicts_idempotency_and_immutable_submission() {
        let s = Store::memory().unwrap();
        let (a, _) = test_actor(&s, "author", 1000);
        let (b, _) = test_actor(&s, "other", 1000);
        let key = nonce().unwrap();
        let command = Command::CreateDraft {
            kind: "app".into(),
            candidate: candidate(),
            base_revision: None,
        };
        let created = s.command(&a, &key, command.clone(), 1000).unwrap();
        assert_eq!(created, s.command(&a, &key, command, 1001).unwrap());
        let id = created["id"].as_str().unwrap();
        assert!(s.draft(&b, id).is_err());
        let save = Command::SaveDraft {
            id: id.into(),
            version: 1,
            candidate: candidate(),
        };
        s.command(&a, &nonce().unwrap(), save.clone(), 1002)
            .unwrap();
        assert_eq!(
            s.command(&a, &nonce().unwrap(), save, 1003)
                .unwrap_err()
                .code,
            "stale_revision"
        );
        let rev = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: id.into(),
                    version: 2,
                    confirm_public_preview: true,
                },
                1004,
            )
            .unwrap();
        let rid = rev["id"].as_str().unwrap();
        let mut newer = candidate();
        newer["apps"][0]["summary"] = json!("A changed description");
        s.command(
            &a,
            &nonce().unwrap(),
            Command::SaveDraft {
                id: id.into(),
                version: 2,
                candidate: newer,
            },
            1005,
        )
        .unwrap();
        assert_eq!(
            s.revision(&a, rid).unwrap()["candidate"]["apps"][0]["summary"],
            candidate()["apps"][0]["summary"]
        );
        assert!(s.revision(&b, rid).is_err());
    }
}
