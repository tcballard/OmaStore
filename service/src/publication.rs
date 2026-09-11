use crate::{actor, db, ApiResult, AppState};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use omastore_workflow::{
    now,
    publication::verify_webhook,
    publisher::{self, Target},
    Error,
};
use serde_json::{json, Value};
use std::time::Duration;

pub async fn status(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    let mut value = db(&s, move |store| store.publication(&actor, &id)).await?;
    value["bridge"] = json!(s.publication_state);
    Ok(Json(value))
}
pub async fn manifest(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.publication_manifest(&actor, &id))
        .await
        .map(Json)
}
pub async fn submitted(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    db(&s, move |store| store.public_submission(&id))
        .await
        .map(Json)
}
pub async fn webhook(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let github = s
        .github
        .as_ref()
        .ok_or(Error::new(503, "publication_unconfigured"))?;
    let header = |key: &str| {
        headers
            .get(key)
            .and_then(|h| h.to_str().ok())
            .map(str::to_owned)
            .ok_or(Error::new(401, "webhook_headers_required"))
    };
    verify_webhook(
        &github.config.webhook_secret,
        &header("X-Hub-Signature-256")?,
        &body,
    )?;
    let delivery = header("X-GitHub-Delivery")?;
    let event = header("X-GitHub-Event")?;
    if event != "pull_request" {
        return Ok(Json(json!({"accepted":true,"ignored":true})));
    }
    let payload: Value = serde_json::from_slice(&body).map_err(Error::from)?;
    if payload["repository"]["full_name"] != github.config.repository {
        return Err(Error::new(403, "github_repository_scope").into());
    }
    db(&s, move |store| {
        Ok(json!({"accepted":true,"new":store.webhook(&delivery,&event,&body,now())?}))
    })
    .await
    .map(Json)
}

pub fn start_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let (Some(store), Some(api), Some(objects)) =
                (&state.store, &state.github, &state.objects)
            else {
                break;
            };
            if state.sandbox {
                break;
            }
            let Ok(Some(lock)) = store.worker_lock("publication-dispatch", now(), 240) else {
                continue;
            };
            let target = Target {
                repository: &api.config.repository,
                branch: &api.config.branch,
                delivery_branch: &api.config.delivery_branch,
                origin: &state.origin,
                development: false,
            };
            let work = async {
                if let Ok(Some(lease)) = store.lease_provider(now()) {
                    let result = tokio::time::timeout(
                        Duration::from_secs(90),
                        publisher::run_job(store, api, objects, &target, &lease),
                    )
                    .await
                    .unwrap_or_else(|_| Err(Error::new(503, "provider_timeout_outcome_unknown")));
                    let _ = store.finish_provider(&lease, result.as_ref().err(), now());
                }
                if let Ok(ids) = store.pending_publications() {
                    if let Some(id) = ids.first() {
                        let result = tokio::time::timeout(
                            Duration::from_secs(90),
                            publisher::reconcile(
                                store,
                                api,
                                objects,
                                &target,
                                id,
                                &state.catalogue,
                            ),
                        )
                        .await
                        .unwrap_or_else(|_| {
                            Err(Error::new(503, "delivery_timeout_reconciliation_required"))
                        });
                        let _ = store.publication_error(id, result.as_ref().err(), now());
                    }
                }
                if let Ok(jobs) = store.webhook_jobs() {
                    if let Some((id, payload)) = jobs.first() {
                        if payload["repository"] == target.repository {
                            if let Some(number) = payload["number"].as_i64().filter(|n| *n > 0) {
                                let result = tokio::time::timeout(
                                    Duration::from_secs(30),
                                    publisher::check_unattached_pr(store, api, &target, number),
                                )
                                .await;
                                // A verified failure is a completed check; network errors remain queued.
                                if matches!(result, Ok(Ok(())))
                                    || matches!(result,Ok(Err(ref e)) if e.status==409)
                                {
                                    let _ = store.finish_webhook(id);
                                }
                            } else {
                                let _ = store.finish_webhook(id);
                            }
                        } else {
                            let _ = store.finish_webhook(id);
                        }
                    }
                }
            };
            let _ = tokio::time::timeout(Duration::from_secs(220), work).await;
            let _ = store.release_worker_lock("publication-dispatch", &lock);
        }
    })
}
