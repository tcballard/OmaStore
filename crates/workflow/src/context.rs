//! Frozen published context and authority-derived project claims for exact previews.
use crate::{
    drafts::{candidate_digest, MAX_DRAFT_BYTES},
    Actor, Error, Result, Store,
};
use omastore_catalogue::{Catalogue, Channel, Claim, LicenceClass};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(crate) struct Prepared {
    pub catalogue: Catalogue,
    pub snapshot: String,
    pub apps: Vec<String>,
    pub makers: Vec<String>,
}
fn published(c: &Connection) -> Result<Option<Catalogue>> {
    let raw: Option<String> = c
        .query_row(
            "SELECT value FROM metadata WHERE key='delivered_catalogue'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
        .transpose()
}
fn project_target(value: &str) -> String {
    crate::auth::claim_target(value)
        .ok()
        .map(|v| v.0)
        .or_else(|| {
            url::Url::parse(value)
                .ok()
                .filter(|u| u.host_str() != Some("github.com"))
                .map(|u| u.origin().ascii_serialization())
        })
        .unwrap_or_default()
}
fn timestamp(n: i64) -> Result<String> {
    Ok(chrono::DateTime::from_timestamp(n, 0)
        .ok_or(Error::new(500, "stored_claim_invalid"))?
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
pub(crate) fn prepare(
    db: &Connection,
    actor: &Actor,
    raw: Value,
    kind: &str,
    now: i64,
) -> Result<Prepared> {
    if serde_json::to_vec(&raw)?.len() > MAX_DRAFT_BYTES {
        return Err(Error::new(413, "candidate_too_large"));
    }
    let mut c: Catalogue =
        serde_json::from_value(raw).map_err(|_| Error::new(422, "candidate_invalid"))?;
    if !c.editorial.is_empty()
        || match kind {
            "app" => c.apps.len() != 1 || !c.recipes.is_empty() || !c.stories.is_empty(),
            "setup" => c.recipes.len() != 1 || !c.stories.is_empty(),
            "editorial" => c.stories.len() != 1 || !c.recipes.is_empty(),
            _ => true,
        }
    {
        return Err(Error::new(422, "candidate_scope_invalid"));
    }
    let base = published(db)?;
    let mut app_ids = BTreeSet::new();
    for recipe in &c.recipes {
        for part in &recipe.components {
            app_ids.insert(part.app_id.clone());
        }
    }
    for story in &c.stories {
        app_ids.extend(story.app_ids.iter().cloned());
    }
    if kind != "app" {
        if c.apps.iter().any(|a| !app_ids.contains(&a.id)) {
            return Err(Error::new(422, "unreferenced_context"));
        }
        for id in &app_ids {
            let app = base
                .as_ref()
                .and_then(|b| b.apps.iter().find(|a| a.id == *id))
                .ok_or(Error::new(409, "published_context_required"))?;
            match c.apps.iter().find(|a| a.id == *id) {
                Some(a) if a != app => return Err(Error::new(409, "published_context_changed")),
                Some(_) => (),
                None => c.apps.push(app.clone()),
            }
        }
    } else if c.apps.iter().any(|a| !a.tests.is_empty()) {
        return Err(Error::new(422, "candidate_cannot_self_verify"));
    }
    let needed: BTreeSet<String> = c
        .apps
        .iter()
        .flat_map(|a| a.maker_ids.iter().cloned())
        .chain(c.recipes.iter().map(|r| r.maker_id.clone()))
        .chain(c.stories.iter().map(|s| s.author_maker_id.clone()))
        .collect();
    if c.makers.iter().any(|m| !needed.contains(&m.id)) {
        return Err(Error::new(422, "unreferenced_context"));
    }
    for id in needed {
        if !c.makers.iter().any(|m| m.id == id) {
            c.makers.push(
                base.as_ref()
                    .and_then(|b| b.makers.iter().find(|m| m.id == id))
                    .ok_or(Error::new(409, "published_context_required"))?
                    .clone(),
            );
        }
    }
    let mut maker_ids = Vec::new();
    for maker in &mut c.makers {
        if base
            .as_ref()
            .is_some_and(|b| b.makers.iter().any(|m| m == maker))
        {
            maker_ids.push(maker.id.clone());
            continue;
        }
        let target = project_target(&maker.homepage);
        let claim:Option<(String,i64,i64)>=db.query_row("SELECT proof_url,verified_at,expires_at FROM claims WHERE user_id=?1 AND target=?2 AND revoked_at IS NULL AND expires_at>?3 ORDER BY verified_at DESC LIMIT 1",params![actor.id,target,now],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let mut expected = maker.clone();
        expected.claim = Claim::Unclaimed;
        expected.claim_evidence = None;
        expected.claim_verified_at = None;
        expected.claim_expires_at = None;
        if let Some((proof, start, end)) = claim {
            expected.claim = Claim::Verified;
            expected.claim_evidence = Some(proof);
            expected.claim_verified_at = Some(timestamp(start)?);
            expected.claim_expires_at = Some(timestamp(end)?);
        }
        let self_claim = maker.claim != Claim::Unclaimed
            || maker.claim_evidence.is_some()
            || maker.claim_verified_at.is_some()
            || maker.claim_expires_at.is_some();
        if self_claim && *maker != expected {
            return Err(Error::new(422, "candidate_cannot_self_verify"));
        }
        *maker = expected;
    }
    if c.makers.len() > 10 {
        return Err(Error::new(422, "candidate_scope_invalid"));
    }
    if kind == "app"
        && c.apps
            .iter()
            .any(|a| a.licence.class == LicenceClass::Proprietary)
    {
        // Require current author control of this project, independently of public maker labels.
        for app in &c.apps {
            let target = project_target(&app.homepage);
            let source = app
                .source
                .as_ref()
                .and_then(|s| crate::auth::claim_target(s).ok())
                .map(|v| v.0)
                .unwrap_or_default();
            let owns:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE user_id=?1 AND target IN (?2,?3) AND revoked_at IS NULL AND expires_at>?4)",params![actor.id,target,source,now],|r|r.get(0))?;
            if !owns {
                return Err(Error::new(403, "verified_project_control_required"));
            }
        }
    }
    c.channel = Channel::Development;
    if !c.validate(true).is_empty() {
        return Err(Error::new(422, "candidate_invalid"));
    }
    if serde_json::to_vec(&c)?.len() > MAX_DRAFT_BYTES {
        return Err(Error::new(413, "candidate_too_large"));
    }
    Ok(Prepared {
        snapshot: base.as_ref().map(|b| b.snapshot_id()).unwrap_or_default(),
        catalogue: c,
        apps: app_ids.into_iter().collect(),
        makers: maker_ids,
    })
}
pub(crate) fn check_frozen(
    db: &Connection,
    id: &str,
    candidate: &Catalogue,
    now: i64,
) -> Result<Vec<String>> {
    let owner: String = db.query_row("SELECT owner FROM revisions WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    let kind: String = db.query_row(
        "SELECT d.kind FROM drafts d JOIN revisions r ON r.draft_id=d.id WHERE r.id=?1",
        [id],
        |r| r.get(0),
    )?;
    // Reuse current authority, but do not require a session or grant a role here.
    let actor = Actor {
        id: owner,
        login: String::new(),
        roles: vec![],
    };
    let prepared = prepare(db, &actor, json!(candidate), &kind, now)?;
    if candidate_digest(&prepared.catalogue)? != candidate_digest(candidate)? {
        return Err(Error::new(409, "public_preview_refresh_required"));
    }
    let maker_context: Option<String> = db
        .query_row(
            "SELECT makers FROM revision_context WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let maker_context: Vec<String> = maker_context
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default();
    for maker in &candidate.makers {
        if maker.claim != Claim::Verified || maker_context.contains(&maker.id) {
            continue;
        }
        let target = project_target(&maker.homepage);
        let current:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE target=?1 AND user_id=?2 AND proof_url=?3 AND revoked_at IS NULL AND expires_at>?4)",params![target,actor.id,maker.claim_evidence,now],|r|r.get(0))?;
        if !current {
            return Err(Error::new(409, "project_claim_no_longer_current"));
        }
    }
    let raw: Option<String> = db
        .query_row(
            "SELECT apps FROM revision_context WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(raw
        .map(|s| serde_json::from_str(&s))
        .transpose()?
        .unwrap_or_default())
}
impl Store {
    pub(crate) fn context_apps(&self, id: &str) -> Result<Vec<String>> {
        let raw: Option<String> = self
            .connection()?
            .query_row(
                "SELECT apps FROM revision_context WHERE revision_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .unwrap_or_default())
    }
}
impl Store {
    #[cfg(feature = "development-workflow")]
    pub fn seed_sample_context(&self, catalogue: &Catalogue) -> Result<()> {
        if !self.is_development()?
            || catalogue.channel != Channel::Development
            || !catalogue.validate(true).is_empty()
        {
            return Err(Error::new(403, "sample_mode_required"));
        }
        self.transaction(|t| {
            t.execute(
                "INSERT OR IGNORE INTO metadata(key,value) VALUES('delivered_catalogue',?1)",
                [String::from_utf8(catalogue.canonical_bytes())
                    .map_err(|_| Error::new(500, "sample_context_invalid"))?],
            )?;
            Ok(())
        })
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::{drafts::Command, nonce};
    fn actor(s: &Store, name: &str, now: i64) -> Actor {
        let v = s.development_login(name, now).unwrap();
        s.actor(v["token"].as_str().unwrap(), now).unwrap()
    }
    fn story(base: &Catalogue, now: i64) -> Value {
        let mut c: Catalogue =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        c.apps.clear();
        c.stories.push(omastore_catalogue::editorial::Story {
            id: "story-one".into(),
            revision: "r1".into(),
            kind: omastore_catalogue::editorial::Kind::Story,
            title: "Fictional workflow story".into(),
            summary: "An editorial test".into(),
            body: "Fictional material for a local review exercise.".into(),
            author_maker_id: c.makers[0].id.clone(),
            app_ids: vec![base.apps[0].id.clone()],
            publish_at: timestamp(now).unwrap(),
            end_at: None,
            rights: "Test fixture rights".into(),
        });
        json!(c)
    }
    #[test]
    fn exact_context_requires_preview_and_editor_review_without_retesting_or_transferring_app_facts(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let now = crate::now();
        let a = actor(&s, "author", now);
        let r = actor(&s, "reviewer", now);
        let base: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&base).unwrap();
        let draft = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "editorial".into(),
                    candidate: story(&base, now),
                    base_revision: None,
                },
                now,
            )
            .unwrap();
        let id = draft["id"].as_str().unwrap();
        assert_eq!(
            s.command(
                &a,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: id.into(),
                    version: 1,
                    confirm_public_preview: true
                },
                now
            )
            .unwrap_err()
            .code,
            "public_preview_refresh_required"
        );
        let p = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::PrepareDraft {
                    id: id.into(),
                    version: 1,
                },
                now,
            )
            .unwrap();
        assert_eq!(p["errors"], json!([]));
        assert_eq!(p["candidate"]["apps"][0], json!(base.apps[0]));
        let rev = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: id.into(),
                    version: 2,
                    confirm_public_preview: true,
                },
                now,
            )
            .unwrap();
        let rid = rev["id"].as_str().unwrap();
        s.sample_checks(now).unwrap();
        let version = s.revision(&a, rid).unwrap()["version"].as_i64().unwrap();
        let vote = Command::ReviewDecision {
            id: rid.into(),
            version,
            decision: "approve".into(),
            reason: "Reviewed attribution and fixture rights".into(),
            acknowledge_limits: true,
        };
        assert_eq!(
            s.command(&r, &nonce().unwrap(), vote.clone(), now)
                .unwrap_err()
                .code,
            "role_revoked"
        );
        s.set_role(&r.id, "editor", true, now).unwrap();
        s.command(&r, &nonce().unwrap(), vote, now).unwrap();
        let approved = s.approved(rid, now).unwrap();
        assert_eq!(approved.payload.apps[0], base.apps[0]);
        let canonical: Catalogue = serde_json::from_slice(&base.canonical_bytes()).unwrap();
        assert_eq!(
            s.validate_publication_scope(&approved, &canonical, now)
                .unwrap_err()
                .code,
            "editorial_author_control_required"
        );
        assert!(!s.release_feed(None).unwrap().contains("<item>"));
        let mut changed = base.clone();
        changed.apps[0].summary = "Changed after the exact preview".into();
        s.connection()
            .unwrap()
            .execute(
                "UPDATE metadata SET value=?1 WHERE key='delivered_catalogue'",
                [serde_json::to_string(&changed).unwrap()],
            )
            .unwrap();
        assert_eq!(
            s.approved(rid, now).unwrap_err().code,
            "published_context_changed"
        );
        let mut forged = p["candidate"].clone();
        forged["apps"][0]["summary"] = json!("Author attempted to rewrite context");
        assert_eq!(
            prepare(&s.connection().unwrap(), &a, forged, "editorial", now)
                .err()
                .unwrap()
                .code,
            "published_context_changed"
        );
    }
    #[test]
    fn project_claim_projection_is_scoped_and_cannot_be_self_assigned() {
        let s = Store::memory().unwrap();
        let now = crate::now();
        let (a, _) = crate::auth::test_actor(&s, "author", now);
        let mut c: Value =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        c["makers"][0]["claim"] = json!("verified");
        c["makers"][0]["claimEvidence"] = json!("https://example.com/proof");
        assert_eq!(
            prepare(&s.connection().unwrap(), &a, c.clone(), "app", now)
                .err()
                .unwrap()
                .code,
            "candidate_cannot_self_verify"
        );
        c["makers"][0]["claim"] = json!("unclaimed");
        c["makers"][0]["claimEvidence"] = Value::Null;
        let origin = url::Url::parse(c["makers"][0]["homepage"].as_str().unwrap())
            .unwrap()
            .origin()
            .ascii_serialization();
        s.connection()
            .unwrap()
            .execute(
                "INSERT INTO claims VALUES(?1,?2,?3,?4,?5,NULL)",
                params![
                    origin,
                    a.id,
                    "https://example.com/proof",
                    now,
                    now + 30 * 86400
                ],
            )
            .unwrap();
        let p = prepare(&s.connection().unwrap(), &a, c, "app", now).unwrap();
        assert_eq!(p.catalogue.makers[0].claim, Claim::Verified);
        s.connection()
            .unwrap()
            .execute("UPDATE claims SET revoked_at=?1", [now + 1])
            .unwrap();
        assert_eq!(
            prepare(
                &s.connection().unwrap(),
                &a,
                json!(p.catalogue),
                "app",
                now + 2
            )
            .err()
            .unwrap()
            .code,
            "candidate_cannot_self_verify"
        );
    }
}
