use crate::{
    digest,
    github::Api,
    media::{LocalObjects, ObjectStorage, VIDEO_LIMIT},
    publication::{Approved, Lease, CHECK_NAME},
    Error, Result, Store,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use omastore_catalogue::{Catalogue, MAX_CATALOGUE_BYTES};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

pub struct Target<'a> {
    pub repository: &'a str,
    pub branch: &'a str,
    pub delivery_branch: &'a str,
    pub origin: &'a str,
    pub development: bool,
}
fn number(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or(Error::new(503, "github_response_invalid"))
}
fn sha(value: &Value) -> Result<String> {
    value
        .as_str()
        .filter(|s| crate::publication::commit_sha(s))
        .map(str::to_owned)
        .ok_or(Error::new(503, "github_response_invalid"))
}
fn marker(id: &str) -> String {
    format!("OmaStore immutable submission: {id}")
}

pub async fn run_job(
    store: &Store,
    api: &impl Api,
    objects: &LocalObjects,
    target: &Target<'_>,
    lease: &Lease,
) -> Result<()> {
    if store.is_development()? != target.development || api.development() != target.development {
        return Err(Error::new(403, "publication_environment_mismatch"));
    }
    if !crate::github::repository_name(target.repository) {
        return Err(Error::new(422, "github_repository_scope"));
    }
    match lease.kind.as_str() {
        "intake" => intake(store, api, target, lease).await,
        "publication" => publish(store, api, objects, target, lease).await,
        _ => Err(Error::new(422, "unsupported_provider_job")),
    }
}
async fn intake(store: &Store, api: &impl Api, target: &Target<'_>, lease: &Lease) -> Result<()> {
    let current = store.publication_internal(&lease.revision)?;
    if current["issueNumber"].as_i64().is_some() {
        return Ok(());
    }
    let root = format!("/repos/{}", target.repository);
    if let Some(number) = current["attachedIssue"].as_i64() {
        let issue = api
            .call("GET", &format!("{root}/issues/{number}"), Value::Null)
            .await?;
        if issue["pull_request"].is_object()
            || !issue["body"]
                .as_str()
                .is_some_and(|body| body.lines().any(|line| line == marker(&lease.revision)))
        {
            return Err(Error::new(409, "intake_identity_mismatch"));
        }
        return store.publication_checkpoint(
            lease,
            json!({"issue_number":number,"intake_state":"created","repository":target.repository}),
            crate::now(),
        );
    }
    if current["intakeState"] == "creating" {
        // A lost create response is reconciled with reads; never blindly create another issue.
        let issues = api
            .call(
                "GET",
                &format!("{root}/issues?state=all&per_page=100&sort=created&direction=desc"),
                Value::Null,
            )
            .await?;
        let matches: Vec<_> = issues
            .as_array()
            .ok_or(Error::new(503, "github_response_invalid"))?
            .iter()
            .filter(|issue| {
                issue["body"]
                    .as_str()
                    .is_some_and(|s| s.lines().any(|line| line == marker(&lease.revision)))
            })
            .collect();
        if matches.len() != 1 {
            return Err(Error::new(503, "intake_reconciliation_required"));
        }
        let id = number(matches[0], "number")?;
        return store.publication_checkpoint(
            lease,
            json!({"issue_number":id,"intake_state":"created","repository":target.repository}),
            crate::now(),
        );
    }
    let submitted = store.public_submission(&lease.revision)?;
    let candidate: Catalogue = serde_json::from_value(submitted["candidate"].clone())?;
    let title = candidate
        .apps
        .first()
        .map(|a| a.name.as_str())
        .or_else(|| candidate.recipes.first().map(|r| r.name.as_str()))
        .unwrap_or("Editorial proposal");
    let name: String = title
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect();
    let body=format!("{}\n\nThis is untrusted proposed catalogue content, awaiting independent review. It is not a compatibility or ownership endorsement.\n\nCandidate digest: {}\n\n[Inspect the immutable submitted candidate]({}/api/v1/submissions/{})\n\nPrivate drafts, account details, reviewer-only findings and credentials are excluded.",marker(&lease.revision),submitted["digest"].as_str().unwrap_or(""),target.origin,lease.revision);
    store.publication_checkpoint(
        lease,
        json!({"intake_state":"creating","repository":target.repository}),
        crate::now(),
    )?;
    let issue = api
        .call(
            "POST",
            &format!("{root}/issues"),
            json!({"title":format!("Submission: {name}"),"body":body}),
        )
        .await?;
    store.publication_checkpoint(
        lease,
        json!({"issue_number":number(&issue,"number")?,"intake_state":"created"}),
        crate::now(),
    )
}
async fn publish(
    store: &Store,
    api: &impl Api,
    objects: &LocalObjects,
    target: &Target<'_>,
    lease: &Lease,
) -> Result<()> {
    let approval = store.approved(&lease.revision, crate::now())?;
    let root = format!("/repos/{}", target.repository);
    let current = store.publication_internal(&lease.revision)?;
    if current["prNumber"].as_i64().is_some()
        && current["rebaseRequested"] != true
        && current["state"] == "pr_pending"
    {
        return Ok(());
    }
    let branch = format!("catalogue/{}", lease.revision);
    let reuse = current["headSha"]
        .as_str()
        .filter(|_| current["rebaseRequested"] != true);
    let (base_sha, head_sha) = if let Some(head) = reuse {
        (
            current["baseSha"]
                .as_str()
                .ok_or(Error::new(500, "publication_checkpoint_invalid"))?
                .to_owned(),
            head.to_owned(),
        )
    } else {
        let reference = api
            .call(
                "GET",
                &format!("{root}/git/ref/heads/{}", target.branch),
                Value::Null,
            )
            .await?;
        let base_sha = sha(&reference["object"]["sha"])?;
        let base = Catalogue::parse(
            &api.read_file(
                target.repository,
                &base_sha,
                "data/registry.json",
                MAX_CATALOGUE_BYTES,
            )
            .await?,
            target.development,
        )
        .map_err(|_| Error::new(503, "base_catalogue_invalid"))?;
        store.validate_publication_scope(&approval, &base, crate::now())?;
        let catalogue =
            crate::publication::merge_catalogue(&base, &approval, &base_sha, target.development)?;
        let files = files(&approval, &catalogue, objects)?;
        let hashes = files
            .iter()
            .map(|(path, bytes)| (path.clone(), digest(bytes)))
            .collect::<BTreeMap<_, _>>();
        // Freeze the exact allowed paths and content before any externally visible write.
        store.publication_checkpoint(lease,json!({"repository":target.repository,"branch":branch,"base_sha":base_sha,"base_registry":serde_json::to_string(&base)?,"expected_registry":serde_json::to_string(&catalogue)?,"expected_files":serde_json::to_string(&hashes)?,"state":"preparing"}),crate::now())?;
        if let Some(number) = current["attachedPr"].as_i64() {
            let pr = api
                .call("GET", &format!("{root}/pulls/{number}"), Value::Null)
                .await?;
            let head = sha(&pr["head"]["sha"])?;
            store.publication_checkpoint(lease,json!({"pr_number":number,"head_sha":head,"state":"pr_pending","rebase_requested":0}),crate::now())?;
            return verify_pr(store, api, target, &lease.revision, number).await;
        }
        let predecessor = if current["rebaseRequested"] == true {
            current["headSha"].as_str()
        } else {
            None
        };
        let head = commit_files(
            api,
            &root,
            &base_sha,
            predecessor,
            &files,
            &format!(
                "Publish independently approved OmaStore revision {}",
                lease.revision
            ),
        )
        .await?;
        store.publication_checkpoint(
            lease,
            json!({"head_sha":head,"predecessor_sha":predecessor,"rebase_requested":0}),
            crate::now(),
        )?;
        (base_sha, head)
    };
    let reference = api
        .call(
            "GET",
            &format!("{root}/git/ref/heads/{branch}"),
            Value::Null,
        )
        .await;
    match reference {
        Ok(reference) if reference["object"]["sha"] == head_sha => {}
        Ok(reference)
            if reference["object"]["sha"].as_str().is_some_and(|sha| {
                Some(sha)
                    == current["headSha"]
                        .as_str()
                        .filter(|_| current["rebaseRequested"] == true)
                        .or(current["predecessorSha"].as_str())
            }) =>
        {
            api.call(
                "PATCH",
                &format!("{root}/git/refs/heads/{branch}"),
                json!({"sha":head_sha,"force":false}),
            )
            .await?;
        }
        Ok(_) => return Err(Error::new(409, "publication_branch_changed")),
        Err(error) if error.status == 404 => {
            api.call(
                "POST",
                &format!("{root}/git/refs"),
                json!({"ref":format!("refs/heads/{branch}"),"sha":head_sha}),
            )
            .await?;
        }
        Err(error) => return Err(error),
    }
    let owner = target
        .repository
        .split('/')
        .next()
        .ok_or(Error::new(422, "github_repository_scope"))?;
    let pull_requests = api
        .call(
            "GET",
            &format!(
                "{root}/pulls?state=all&head={owner}:{branch}&base={}&per_page=100",
                target.branch
            ),
            Value::Null,
        )
        .await?;
    let prs = pull_requests
        .as_array()
        .ok_or(Error::new(503, "github_response_invalid"))?;
    let pr = if prs.len() == 1 {
        prs[0].clone()
    } else if prs.is_empty() {
        if current["state"] == "creating_pr" {
            return Err(Error::new(503, "pr_reconciliation_required"));
        }
        store.publication_checkpoint(lease, json!({"state":"creating_pr"}), crate::now())?;
        api.call("POST",&format!("{root}/pulls"),json!({"title":format!("Catalogue: approved revision {}",&lease.revision[..12]),"head":branch,"base":target.branch,"body":format!("Independent review approved immutable candidate {}.\n\nThe external OmaStore App check verifies the exact registry, media and receipt against private approval authority. Editing a receipt, workflow or label cannot approve this PR.\n\nApproval policy: {}\nPayload digest: {}\nBase commit: {}",lease.revision,crate::review::POLICY,approval.payload_digest,base_sha)})).await?
    } else {
        return Err(Error::new(409, "publication_pr_ambiguous"));
    };
    let pr_number = number(&pr, "number")?;
    store.publication_checkpoint(
        lease,
        json!({"pr_number":pr_number,"state":"pr_pending"}),
        crate::now(),
    )?;
    verify_pr(store, api, target, &lease.revision, pr_number).await
}
fn files(
    approval: &Approved,
    catalogue: &Catalogue,
    objects: &LocalObjects,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    files.insert("data/registry.json".into(), catalogue.canonical_bytes());
    let receipt = receipt(approval)?;
    files.insert(
        format!("data/approvals/{}.json", approval.revision),
        serde_json::to_vec_pretty(&receipt)?,
    );
    for app in &approval.payload.apps {
        for media in &app.media {
            let key = url::Url::parse(&media.url)
                .ok()
                .and_then(|u| {
                    u.path_segments()
                        .and_then(|mut s| s.next_back())
                        .map(str::to_owned)
                })
                .ok_or(Error::new(422, "media_reference_invalid"))?;
            if !key.starts_with(&media.sha256) {
                return Err(Error::new(422, "media_reference_invalid"));
            }
            let bytes = objects.read_private(&key, VIDEO_LIMIT)?;
            if digest(&bytes) != media.sha256 {
                return Err(Error::new(409, "media_digest_mismatch"));
            }
            files.insert(format!("media/{key}"), bytes);
        }
    }
    Ok(files)
}
async fn commit_files(
    api: &impl Api,
    root: &str,
    parent: &str,
    predecessor: Option<&str>,
    files: &BTreeMap<String, Vec<u8>>,
    message: &str,
) -> Result<String> {
    let parent_commit = api
        .call("GET", &format!("{root}/git/commits/{parent}"), Value::Null)
        .await?;
    let base_tree = sha(&parent_commit["tree"]["sha"])?;
    let mut entries = Vec::new();
    for (path, bytes) in files {
        if !crate::github::allowed_file(path) {
            return Err(Error::new(422, "publication_file_out_of_scope"));
        }
        let blob = api
            .call(
                "POST",
                &format!("{root}/git/blobs"),
                json!({"content":STANDARD.encode(bytes),"encoding":"base64"}),
            )
            .await?;
        entries.push(json!({"path":path,"mode":"100644","type":"blob","sha":sha(&blob["sha"])?}));
    }
    let tree = api
        .call(
            "POST",
            &format!("{root}/git/trees"),
            json!({"base_tree":base_tree,"tree":entries}),
        )
        .await?;
    let mut parents = vec![parent];
    if let Some(previous) = predecessor.filter(|p| *p != parent) {
        parents.push(previous);
    }
    let commit = api
        .call(
            "POST",
            &format!("{root}/git/commits"),
            json!({"message":message,"parents":parents,"tree":sha(&tree["sha"])?}),
        )
        .await?;
    sha(&commit["sha"])
}

pub async fn verify_pr(
    store: &Store,
    api: &impl Api,
    target: &Target<'_>,
    id: &str,
    pr_number: i64,
) -> Result<()> {
    let root = format!("/repos/{}", target.repository);
    let pr = api
        .call("GET", &format!("{root}/pulls/{pr_number}"), Value::Null)
        .await?;
    let head = sha(&pr["head"]["sha"])?;
    let result = async {
        if pr["base"]["ref"] != target.branch
            || pr["base"]["repo"]["full_name"] != target.repository
        {
            return Err(Error::new(409, "publication_pr_target_changed"));
        }
        let expected = store.expected_files(id)?;
        let changes = api
            .call(
                "GET",
                &format!("{root}/pulls/{pr_number}/files?per_page=100"),
                Value::Null,
            )
            .await?;
        let changes = changes
            .as_array()
            .ok_or(Error::new(503, "github_response_invalid"))?;
        if changes.is_empty()
            || changes.len() >= 100
            || changes.iter().any(|f| {
                f["filename"]
                    .as_str()
                    .is_none_or(|name| !expected.contains_key(name))
                    || !["added", "modified"].contains(&f["status"].as_str().unwrap_or(""))
            })
        {
            return Err(Error::new(409, "unapproved_pr_content"));
        }
        let mut observed = BTreeMap::new();
        for path in expected.keys() {
            let limit = if path.starts_with("media/") {
                VIDEO_LIMIT
            } else {
                MAX_CATALOGUE_BYTES
            };
            let bytes = api.read_file(target.repository, &head, path, limit).await?;
            observed.insert(path.clone(), digest(bytes));
        }
        store.verify_publication_files(id, &observed, crate::now())
    }
    .await;
    let conclusion = if result.is_ok() { "success" } else { "failure" };
    let summary = if result.is_ok() {
        "The exact registry, media and public receipt match current independent approval. Contributor workflows and labels are not approval authority."
    } else {
        "This PR does not currently satisfy independent catalogue approval. Review the native publication status; changing this PR's workflow, label or receipt cannot grant approval."
    };
    emit_check(store, api, target, pr_number, &head, conclusion, summary).await?;
    result
}
async fn emit_check(
    store: &Store,
    api: &impl Api,
    target: &Target<'_>,
    pr: i64,
    head: &str,
    conclusion: &str,
    summary: &str,
) -> Result<()> {
    if store.github_check_current(target.repository, pr, head, conclusion)? {
        return Ok(());
    }
    let result=api.call("POST",&format!("/repos/{}/check-runs",target.repository),json!({"name":CHECK_NAME,"head_sha":head,"status":"completed","conclusion":conclusion,"external_id":format!("omastore:{pr}:{head}"),"output":{"title":CHECK_NAME,"summary":summary}})).await?;
    store.record_github_check(
        target.repository,
        pr,
        head,
        conclusion,
        number(&result, "id")?,
        crate::now(),
    )
}
pub async fn check_unattached_pr(
    store: &Store,
    api: &impl Api,
    target: &Target<'_>,
    number: i64,
) -> Result<()> {
    if let Some(id) = store.publication_for_pr(target.repository, number)? {
        return verify_pr(store, api, target, &id, number).await;
    }
    let root = format!("/repos/{}", target.repository);
    let pr = api
        .call("GET", &format!("{root}/pulls/{number}"), Value::Null)
        .await?;
    let head = sha(&pr["head"]["sha"])?;
    let changes = api
        .call(
            "GET",
            &format!("{root}/pulls/{number}/files?per_page=100"),
            Value::Null,
        )
        .await?;
    let changes = changes
        .as_array()
        .ok_or(Error::new(503, "github_response_invalid"))?;
    let catalogue = changes.len() >= 100
        || changes.iter().any(|f| {
            f["filename"].as_str().is_none_or(|s| {
                s == "data/registry.json"
                    || s.starts_with("media/")
                    || s.starts_with("data/approvals/")
            })
        });
    emit_check(store,api,target,number,&head,if catalogue{"failure"}else{"success"},if catalogue{"No independent private approval is attached to this catalogue PR. Submit the candidate through the author workflow and attach this PR for verification."}else{"This PR does not change published catalogue content."}).await
}
pub async fn reconcile(
    store: &Store,
    api: &impl Api,
    objects: &LocalObjects,
    target: &Target<'_>,
    id: &str,
    catalogue_path: &Path,
) -> Result<()> {
    if store.is_development()? != target.development || api.development() != target.development {
        return Err(Error::new(403, "publication_environment_mismatch"));
    }
    let current = store.publication_internal(id)?;
    let Some(number) = current["prNumber"].as_i64() else {
        return Err(Error::new(409, "publication_pr_missing"));
    };
    let pr = api
        .call(
            "GET",
            &format!("/repos/{}/pulls/{number}", target.repository),
            Value::Null,
        )
        .await?;
    verify_pr(store, api, target, id, number).await?;
    if pr["merged"].as_bool() != Some(true) {
        if pr["state"] == "closed" {
            return Err(Error::new(409, "publication_pr_closed"));
        }
        if pr["mergeable"] == false {
            return Err(Error::new(409, "publication_rebase_required"));
        }
        return Ok(());
    }
    let merged = sha(&pr["merge_commit_sha"])?;
    store.mark_merged(id, &merged, crate::now())?;
    let approved = store.approved(id, crate::now())?;
    let mut expected = store.expected_registry(id)?;
    // A merged source PR is not the native delivery surface. The App alone advances this branch.
    let root = format!("/repos/{}", target.repository);
    let live = api
        .call(
            "GET",
            &format!("{root}/git/ref/heads/{}", target.delivery_branch),
            Value::Null,
        )
        .await;
    let (parent, exists) = match live {
        Ok(value) => (sha(&value["object"]["sha"])?, true),
        Err(error) if error.status == 404 => (merged.clone(), false),
        Err(error) => return Err(error),
    };
    let already = if exists {
        api.read_file(
            target.repository,
            &parent,
            "data/registry.json",
            MAX_CATALOGUE_BYTES,
        )
        .await
        .ok()
        .and_then(|b| Catalogue::parse(&b, target.development).ok())
        .is_some_and(|c| c == expected)
    } else {
        false
    };
    if !already {
        if exists {
            let live = Catalogue::parse(
                &api.read_file(
                    target.repository,
                    &parent,
                    "data/registry.json",
                    MAX_CATALOGUE_BYTES,
                )
                .await?,
                target.development,
            )
            .map_err(|_| Error::new(503, "delivered_catalogue_invalid"))?;
            store.validate_publication_scope(&approved, &live, crate::now())?;
            if !preserves_delivered(&live, &expected, &approved.payload) {
                expected = crate::publication::merge_catalogue(
                    &live,
                    &approved,
                    &parent,
                    target.development,
                )?;
            }
        }
        let files = files(&approved, &expected, objects)?;
        let commit = commit_files(
            api,
            &root,
            &parent,
            None,
            &files,
            &format!("Deliver independently verified OmaStore revision {id}"),
        )
        .await?;
        // Recheck role, evidence and approved bytes after network writes, before the live ref update.
        store.verify_publication_files(id, &store.expected_files(id)?, crate::now())?;
        if exists {
            api.call(
                "PATCH",
                &format!("{root}/git/refs/heads/{}", target.delivery_branch),
                json!({"sha":commit,"force":false}),
            )
            .await?;
        } else {
            api.call(
                "POST",
                &format!("{root}/git/refs"),
                json!({"ref":format!("refs/heads/{}",target.delivery_branch),"sha":commit}),
            )
            .await?;
        }
    }
    let live = Catalogue::parse(
        &api.read_file(
            target.repository,
            target.delivery_branch,
            "data/registry.json",
            MAX_CATALOGUE_BYTES,
        )
        .await?,
        target.development,
    )
    .map_err(|_| Error::new(503, "delivered_catalogue_invalid"))?;
    if live != expected {
        return Err(Error::new(409, "delivery_not_observed"));
    }
    atomic_catalogue(catalogue_path, &live)?;
    for path in store
        .expected_files(id)?
        .keys()
        .filter(|p| p.starts_with("media/"))
    {
        objects.promote_public(path.trim_start_matches("media/"))?;
    }
    // Fetch the deployed public HTTP API itself; a redirect or write receipt is not delivery proof.
    let public = api.read_public(target.origin, MAX_CATALOGUE_BYTES).await?;
    let observed = Catalogue::parse(&public, target.development)
        .map_err(|_| Error::new(503, "public_catalogue_unavailable"))?;
    store.mark_delivered(id, &observed, crate::now())
}
fn preserves_delivered(live: &Catalogue, next: &Catalogue, changed: &Catalogue) -> bool {
    live.apps
        .iter()
        .filter(|a| !changed.apps.iter().any(|c| c.id == a.id))
        .all(|a| next.apps.contains(a))
        && live
            .makers
            .iter()
            .filter(|m| !changed.makers.iter().any(|c| c.id == m.id))
            .all(|m| next.makers.contains(m))
        && live
            .recipes
            .iter()
            .filter(|r| !changed.recipes.iter().any(|c| c.id == r.id))
            .all(|r| next.recipes.contains(r))
        && live.editorial.iter().all(|e| next.editorial.contains(e))
}
pub fn atomic_catalogue(path: &Path, catalogue: &Catalogue) -> Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or(Error::new(500, "catalogue_storage_unavailable"))?;
    if std::fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(Error::new(500, "unsafe_catalogue_path"));
    }
    let bytes = catalogue.canonical_bytes();
    if bytes.len() > MAX_CATALOGUE_BYTES {
        return Err(Error::new(413, "catalogue_too_large"));
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|_| Error::new(500, "catalogue_delivery_failed"))?;
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn receipt(approval: &Approved) -> Result<Value> {
    Ok(
        json!({"schemaVersion":1,"revisionId":approval.revision,"candidateDigest":crate::drafts::candidate_digest(&approval.candidate)?,"payloadDigest":approval.payload_digest,"policy":crate::review::POLICY,"notice":"This receipt is a public reference. Only the configured external approval check is authoritative."}),
    )
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::{drafts::Command, publication};
    use async_trait::async_trait;
    use std::sync::Mutex;
    struct LostIssue {
        body: Mutex<Option<String>>,
        creates: std::sync::atomic::AtomicUsize,
    }
    #[async_trait]
    impl Api for LostIssue {
        fn development(&self) -> bool {
            true
        }
        async fn call(&self, method: &str, path: &str, body: Value) -> Result<Value> {
            if method == "POST" && path.ends_with("/issues") {
                self.creates
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                *self.body.lock().unwrap() = Some(body["body"].as_str().unwrap().into());
                return Err(Error::new(503, "github_write_outcome_unknown"));
            }
            if method == "GET" && path.contains("/issues?") {
                return Ok(
                    json!([{"number":42,"body":self.body.lock().unwrap().clone().unwrap()}]),
                );
            }
            Err(Error::new(500, "unexpected_test_request"))
        }
        async fn read_file(&self, _: &str, _: &str, _: &str, _: usize) -> Result<Vec<u8>> {
            Err(Error::new(500, "unexpected_test_request"))
        }
    }
    fn actor(s: &Store, name: &str, now: i64) -> crate::Actor {
        let login = s.development_login(name, now).unwrap();
        s.actor(login["token"].as_str().unwrap(), now).unwrap()
    }
    fn submit(s: &Store, a: &crate::Actor, now: i64) -> String {
        let c: Value =
            serde_json::from_str(include_str!("../../../docs/examples/submission.json")).unwrap();
        let draft = s
            .command(
                a,
                &crate::nonce().unwrap(),
                Command::CreateDraft {
                    kind: "app".into(),
                    candidate: c,
                    base_revision: None,
                },
                now,
            )
            .unwrap();
        let revision = s
            .command(
                a,
                &crate::nonce().unwrap(),
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
    #[tokio::test]
    async fn lost_intake_response_reconciles_without_a_second_create() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::development(&dir.path().join("private.db")).unwrap();
        let now = crate::now();
        let a = actor(&store, "author", now);
        let id = submit(&store, &a, now);
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let api = LostIssue {
            body: Mutex::new(None),
            creates: std::sync::atomic::AtomicUsize::new(0),
        };
        let target = Target {
            repository: "owner/repo",
            branch: "main",
            delivery_branch: "catalogue-live",
            origin: "https://example.com",
            development: true,
        };
        let lease = store.lease_provider(now).unwrap().unwrap();
        assert_eq!(lease.kind, "intake");
        let error = run_job(&store, &api, &objects, &target, &lease)
            .await
            .unwrap_err();
        store.finish_provider(&lease, Some(&error), now).unwrap();
        let retry = store.lease_provider(now + 61).unwrap().unwrap();
        run_job(&store, &api, &objects, &target, &retry)
            .await
            .unwrap();
        store.finish_provider(&retry, None, now + 61).unwrap();
        assert_eq!(api.creates.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(store.publication(&a, &id).unwrap()["issueNumber"], 42);
    }
    #[test]
    fn a_candidate_cannot_authorize_itself_by_editing_its_receipt_or_workflow() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::development(&dir.path().join("private.db")).unwrap();
        let now = crate::now();
        let a = actor(&store, "author", now);
        let r = actor(&store, "reviewer", now);
        let id = submit(&store, &a, now);
        store.sample_checks(now).unwrap();
        store.sample_runtime(&r, &id, now).unwrap();
        let version = store.revision(&r, &id).unwrap()["version"]
            .as_i64()
            .unwrap();
        store
            .command(
                &r,
                &crate::nonce().unwrap(),
                Command::ReviewDecision {
                    id: id.clone(),
                    version,
                    decision: "approve".into(),
                    reason: "Explicit fictional fixture".into(),
                    acknowledge_limits: true,
                },
                now,
            )
            .unwrap();
        let approved = store.approved(&id, now).unwrap();
        let base = Catalogue::parse(include_bytes!("../../../data/registry.json"), false).unwrap();
        let next = publication::merge_catalogue(&base, &approved, &"a".repeat(40), true).unwrap();
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let expected = files(&approved, &next, &objects)
            .unwrap()
            .into_iter()
            .map(|(k, v)| (k, digest(v)))
            .collect::<BTreeMap<_, _>>();
        store.connection().unwrap().execute("UPDATE publications SET base_registry=?2,expected_registry=?3,expected_files=?4 WHERE revision_id=?1",rusqlite::params![id,serde_json::to_string(&base).unwrap(),serde_json::to_string(&next).unwrap(),serde_json::to_string(&expected).unwrap()]).unwrap();
        store.verify_publication_files(&id, &expected, now).unwrap();
        let mut forged = expected.clone();
        forged.insert(
            format!("data/approvals/{id}.json"),
            digest(b"self-approved"),
        );
        assert_eq!(
            store
                .verify_publication_files(&id, &forged, now)
                .unwrap_err()
                .code,
            "unapproved_pr_content"
        );
        let mut workflow = expected.clone();
        workflow.insert(
            ".github/workflows/approve.yml".into(),
            digest(b"always pass"),
        );
        assert!(store.verify_publication_files(&id, &workflow, now).is_err());
        store.set_role(&r.id, "reviewer", false, now).unwrap();
        assert_eq!(
            store
                .verify_publication_files(&id, &expected, now)
                .unwrap_err()
                .code,
            "approval_reviewers_no_longer_eligible"
        );
    }
    #[test]
    fn delivery_is_atomic_and_does_not_drop_unrelated_entries() {
        let live: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let mut smaller = live.clone();
        smaller.apps.pop();
        let mut changed = live.clone();
        changed.apps.clear();
        changed.makers.clear();
        changed.recipes.clear();
        changed.editorial.clear();
        assert!(!preserves_delivered(&live, &smaller, &changed));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalogue.json");
        atomic_catalogue(&path, &live).unwrap();
        let previous = std::fs::read(&path).unwrap();
        let impossible = dir.path().join("missing/catalogue.json");
        assert!(atomic_catalogue(&impossible, &smaller).is_err());
        assert_eq!(std::fs::read(path).unwrap(), previous);
    }
}
