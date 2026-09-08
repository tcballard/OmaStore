//! Explicit development-only in-memory provider rehearsal. No network or real repository writes.
use crate::{
    digest,
    github::Api,
    media::LocalObjects,
    publisher::{self, Target},
    Actor, Error, Result, Store,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use omastore_catalogue::Catalogue;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, sync::Mutex};

#[derive(Default)]
struct Repository {
    refs: BTreeMap<String, String>,
    blobs: BTreeMap<String, Vec<u8>>,
    trees: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    commits: BTreeMap<String, String>,
    issues: Vec<Value>,
    prs: Vec<Value>,
    sequence: u64,
    public_unavailable: bool,
    issue_reply_lost: bool,
}
pub struct SampleGithub {
    state: Mutex<Repository>,
}
impl SampleGithub {
    pub fn new(base: &Catalogue) -> Self {
        let root = "a".repeat(40);
        let mut state = Repository::default();
        state.refs.insert("main".into(), root.clone());
        state.commits.insert(root.clone(), root.clone());
        state.trees.insert(
            root,
            BTreeMap::from([("data/registry.json".into(), base.canonical_bytes())]),
        );
        Self {
            state: Mutex::new(state),
        }
    }
    pub fn merge(&self, number: i64) -> Result<()> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| Error::new(500, "sample_provider_unavailable"))?;
        let pr = s
            .prs
            .iter_mut()
            .find(|p| p["number"] == number)
            .ok_or(Error::new(404, "sample_pr_missing"))?;
        pr["merged"] = json!(true);
        pr["state"] = json!("closed");
        pr["merge_commit_sha"] = pr["head"]["sha"].clone();
        let head = pr["head"]["sha"].as_str().unwrap().to_owned();
        s.refs.insert("main".into(), head);
        Ok(())
    }
}
fn identity(s: &mut Repository) -> String {
    s.sequence += 1;
    digest(format!("sample-object-{}", s.sequence))[..40].into()
}
fn read(s: &Repository, reference: &str, path: &str, limit: usize) -> Result<Vec<u8>> {
    let sha = s
        .refs
        .get(reference)
        .map(String::as_str)
        .unwrap_or(reference);
    let bytes = s
        .commits
        .get(sha)
        .and_then(|tree| s.trees.get(tree))
        .and_then(|files| files.get(path))
        .ok_or(Error::new(404, "github_resource_missing"))?;
    if bytes.len() > limit {
        return Err(Error::new(413, "sample_response_too_large"));
    }
    Ok(bytes.clone())
}
#[async_trait]
impl Api for SampleGithub {
    fn development(&self) -> bool {
        true
    }
    async fn call(&self, method: &str, path: &str, body: Value) -> Result<Value> {
        let mut s = self
            .state
            .lock()
            .map_err(|_| Error::new(500, "sample_provider_unavailable"))?;
        let path = path
            .strip_prefix("/repos/sample/omastore/")
            .ok_or(Error::new(403, "sample_repository_scope"))?;
        if method == "GET" {
            if let Some(name) = path.strip_prefix("git/ref/heads/") {
                return s
                    .refs
                    .get(name)
                    .map(|sha| json!({"object":{"sha":sha}}))
                    .ok_or(Error::new(404, "github_resource_missing"));
            }
            if let Some(sha) = path.strip_prefix("git/commits/") {
                return s
                    .commits
                    .get(sha)
                    .map(|tree| json!({"tree":{"sha":tree}}))
                    .ok_or(Error::new(404, "github_resource_missing"));
            }
            if path.starts_with("issues?") {
                return Ok(json!(s.issues));
            }
            if path.starts_with("pulls?") {
                let query = url::form_urlencoded::parse(path.split_once('?').unwrap().1.as_bytes())
                    .collect::<BTreeMap<_, _>>();
                let branch = query
                    .get("head")
                    .and_then(|h| h.split_once(':'))
                    .map(|(_, b)| b)
                    .unwrap_or("");
                return Ok(json!(s
                    .prs
                    .iter()
                    .filter(|p| p["head"]["ref"] == branch)
                    .collect::<Vec<_>>()));
            }
            if let Some(rest) = path.strip_prefix("pulls/") {
                let number = rest
                    .split('/')
                    .next()
                    .and_then(|v| v.parse::<i64>().ok())
                    .unwrap_or(0);
                let pr = s
                    .prs
                    .iter()
                    .find(|p| p["number"] == number)
                    .ok_or(Error::new(404, "github_resource_missing"))?;
                if rest.contains("/files?") {
                    let head = pr["head"]["sha"].as_str().unwrap();
                    let tree = &s.commits[head];
                    return Ok(json!(s.trees[tree]
                        .keys()
                        .filter(|p| crate::github::allowed_file(p)
                            && s.trees
                                .get(&s.commits[pr["base"]["sha"].as_str().unwrap()])
                                .and_then(|f| f.get(*p))
                                != s.trees[tree].get(*p))
                        .map(|p| json!({"filename":p,"status":"modified"}))
                        .collect::<Vec<_>>()));
                }
                return Ok(pr.clone());
            }
        }
        if method == "POST" {
            match path {
                "issues" => {
                    let number = s.issues.len() as i64 + 1;
                    let mut issue = body;
                    issue["number"] = json!(number);
                    s.issues.push(issue.clone());
                    if s.issue_reply_lost {
                        s.issue_reply_lost = false;
                        return Err(Error::new(503, "github_write_outcome_unknown"));
                    }
                    return Ok(issue);
                }
                "git/blobs" => {
                    let bytes = STANDARD
                        .decode(body["content"].as_str().unwrap_or(""))
                        .map_err(|_| Error::new(422, "invalid_fields"))?;
                    let sha = identity(&mut s);
                    s.blobs.insert(sha.clone(), bytes);
                    return Ok(json!({"sha":sha}));
                }
                "git/trees" => {
                    let mut files = s
                        .trees
                        .get(body["base_tree"].as_str().unwrap_or(""))
                        .cloned()
                        .ok_or(Error::new(404, "sample_tree_missing"))?;
                    for item in body["tree"]
                        .as_array()
                        .ok_or(Error::new(422, "invalid_fields"))?
                    {
                        files.insert(
                            item["path"].as_str().unwrap().into(),
                            s.blobs[item["sha"].as_str().unwrap()].clone(),
                        );
                    }
                    let sha = identity(&mut s);
                    s.trees.insert(sha.clone(), files);
                    return Ok(json!({"sha":sha}));
                }
                "git/commits" => {
                    let sha = identity(&mut s);
                    s.commits
                        .insert(sha.clone(), body["tree"].as_str().unwrap().into());
                    return Ok(json!({"sha":sha}));
                }
                "git/refs" => {
                    let name = body["ref"]
                        .as_str()
                        .and_then(|s| s.strip_prefix("refs/heads/"))
                        .ok_or(Error::new(422, "invalid_fields"))?;
                    if s.refs.contains_key(name) {
                        return Err(Error::new(409, "github_conflict"));
                    }
                    s.refs
                        .insert(name.into(), body["sha"].as_str().unwrap().into());
                    return Ok(body);
                }
                "pulls" => {
                    let branch = body["head"].as_str().unwrap();
                    let head = s
                        .refs
                        .get(branch)
                        .ok_or(Error::new(404, "github_resource_missing"))?
                        .clone();
                    let number = s.prs.len() as i64 + 1;
                    let pr = json!({"number":number,"head":{"sha":head,"ref":branch},"base":{"ref":"main","sha":s.refs["main"],"repo":{"full_name":"sample/omastore"}},"merged":false,"state":"open","mergeable":true});
                    s.prs.push(pr.clone());
                    return Ok(pr);
                }
                "check-runs" => return Ok(json!({"id":1})),
                _ => {}
            }
        }
        if method == "PATCH" {
            if let Some(name) = path.strip_prefix("git/refs/heads/") {
                if body["force"] != false {
                    return Err(Error::new(403, "sample_force_rejected"));
                }
                s.refs
                    .insert(name.into(), body["sha"].as_str().unwrap().into());
                return Ok(body);
            }
        }
        Err(Error::new(500, "unsupported_sample_provider_request"))
    }
    async fn read_file(
        &self,
        repository: &str,
        reference: &str,
        path: &str,
        limit: usize,
    ) -> Result<Vec<u8>> {
        if repository != "sample/omastore" {
            return Err(Error::new(403, "sample_repository_scope"));
        }
        let state = self
            .state
            .lock()
            .map_err(|_| Error::new(500, "sample_provider_unavailable"))?;
        read(&state, reference, path, limit)
    }
    async fn read_public(&self, _: &str, limit: usize) -> Result<Vec<u8>> {
        let s = self
            .state
            .lock()
            .map_err(|_| Error::new(500, "sample_provider_unavailable"))?;
        if s.public_unavailable {
            return Err(Error::new(503, "sample_delivery_unavailable"));
        }
        read(&s, "catalogue-live", "data/registry.json", limit)
    }
}

/// Rehearse a confirmed publication against a local provider. It cannot contact GitHub.
pub fn rehearse(
    store: &Store,
    actor: &Actor,
    id: &str,
    objects: &LocalObjects,
    path: &Path,
) -> Result<Value> {
    if !store.is_development()? {
        return Err(Error::new(403, "sample_mode_required"));
    }
    let status = store.publication(actor, id)?;
    if status["state"] == "delivered" {
        return Ok(json!({"id":id,"state":"published","simulated":true}));
    }
    store.reset_sample_publication(actor, id)?;
    let base = store.delivered_catalogue()?.unwrap_or(
        Catalogue::parse(include_bytes!("../../../data/registry.json"), false)
            .map_err(|_| Error::new(500, "sample_base_invalid"))?,
    );
    let api = SampleGithub::new(&base);
    let target = Target {
        repository: "sample/omastore",
        branch: "main",
        delivery_branch: "catalogue-live",
        origin: "https://sandbox.omastore.invalid",
        development: true,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::new(500, "sample_runtime_unavailable"))?;
    runtime.block_on(async {
        for _ in 0..40 {
            let Some(lease)=store.lease_provider(crate::now())? else{break};
            let result=publisher::run_job(store,&api,objects,&target,&lease).await;
            store.finish_provider(&lease,result.as_ref().err(),crate::now())?;result?;
        }
        let number=store.publication_internal(id)?["prNumber"].as_i64().ok_or(Error::new(409,"publication_request_required"))?;
        api.merge(number)?;
        publisher::reconcile(store,&api,objects,&target,id,path).await?;
        Ok(json!({"id":id,"state":"published","simulated":true,"notice":"Local provider rehearsal completed. No real issue, PR, review merge or deployment occurred."}))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{drafts::Command, nonce, now};
    fn prepared(s: &Store) -> (Actor, String) {
        let login = s.development_login("author", now()).unwrap();
        let a = s.actor(login["token"].as_str().unwrap(), now()).unwrap();
        let login = s.development_login("reviewer", now()).unwrap();
        let r = s.actor(login["token"].as_str().unwrap(), now()).unwrap();
        let draft = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "app".into(),
                    candidate: serde_json::from_str(include_str!(
                        "../../../docs/examples/submission.json"
                    ))
                    .unwrap(),
                    base_revision: None,
                },
                now(),
            )
            .unwrap();
        let revision = s
            .command(
                &a,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: draft["id"].as_str().unwrap().into(),
                    version: 1,
                    confirm_public_preview: true,
                },
                now(),
            )
            .unwrap();
        let id = revision["id"].as_str().unwrap().to_owned();
        s.sample_checks(now()).unwrap();
        s.sample_runtime(&r, &id, now()).unwrap();
        let version = s.revision(&r, &id).unwrap()["version"].as_i64().unwrap();
        let approval = s
            .command(
                &r,
                &nonce().unwrap(),
                Command::ReviewDecision {
                    id: id.clone(),
                    version,
                    decision: "approve".into(),
                    reason: "Simulated integration fixture".into(),
                    acknowledge_limits: true,
                },
                now(),
            )
            .unwrap();
        s.command(
            &a,
            &nonce().unwrap(),
            Command::RequestPublication {
                id: id.clone(),
                version: approval["version"].as_i64().unwrap(),
            },
            now(),
        )
        .unwrap();
        (a, id)
    }
    #[tokio::test]
    async fn complete_provider_flow_waits_for_observed_delivery_and_retries_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let (a, id) = prepared(&s);
        let base = Catalogue::parse(include_bytes!("../../../data/registry.json"), false).unwrap();
        let api = SampleGithub::new(&base);
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let target = Target {
            repository: "sample/omastore",
            branch: "main",
            delivery_branch: "catalogue-live",
            origin: "https://sandbox.omastore.invalid",
            development: true,
        };
        while let Some(lease) = s.lease_provider(now()).unwrap() {
            publisher::run_job(&s, &api, &objects, &target, &lease)
                .await
                .unwrap();
            s.finish_provider(&lease, None, now()).unwrap();
        }
        assert_eq!(api.state.lock().unwrap().issues.len(), 1);
        assert_eq!(api.state.lock().unwrap().prs.len(), 1);
        let path = dir.path().join("catalogue.json");
        publisher::atomic_catalogue(&path, &base).unwrap();
        publisher::reconcile(&s, &api, &objects, &target, &id, &path)
            .await
            .unwrap();
        assert_eq!(s.revision(&a, &id).unwrap()["state"], "publication_pending");
        assert_eq!(
            Catalogue::parse(&std::fs::read(&path).unwrap(), true).unwrap(),
            base
        );
        api.merge(1).unwrap();
        api.state.lock().unwrap().public_unavailable = true;
        assert_eq!(
            publisher::reconcile(&s, &api, &objects, &target, &id, &path)
                .await
                .unwrap_err()
                .code,
            "sample_delivery_unavailable"
        );
        assert_eq!(s.publication(&a, &id).unwrap()["state"], "merged");
        assert_eq!(s.revision(&a, &id).unwrap()["state"], "publication_pending");
        api.state.lock().unwrap().public_unavailable = false;
        publisher::reconcile(&s, &api, &objects, &target, &id, &path)
            .await
            .unwrap();
        assert_eq!(s.revision(&a, &id).unwrap()["state"], "published");
        let delivered = s.delivered_catalogue().unwrap().unwrap();
        assert!(crate::publication::contains_payload(
            &delivered,
            &s.approved(&id, now()).unwrap().payload
        ));
        assert_eq!(api.state.lock().unwrap().prs.len(), 1);
    }
    #[test]
    fn rehearsal_is_idempotent_local_and_never_accepts_a_production_database() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let (a, id) = prepared(&s);
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let path = dir.path().join("catalogue.json");
        assert_eq!(
            rehearse(&s, &a, &id, &objects, &path).unwrap()["state"],
            "published"
        );
        assert_eq!(
            rehearse(&s, &a, &id, &objects, &path).unwrap()["state"],
            "published"
        );
        assert_eq!(
            rehearse(&Store::memory().unwrap(), &a, &id, &objects, &path)
                .unwrap_err()
                .code,
            "sample_mode_required"
        );
    }
}
