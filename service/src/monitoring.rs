use crate::{actor, db, ApiResult, AppState};
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use omastore_workflow::{now, Error};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ids {
    ids: Option<String>,
}
pub async fn status(State(s): State<AppState>, Query(query): Query<Ids>) -> ApiResult<Json<Value>> {
    let ids = query
        .ids
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let catalogue = s.read_catalogue()?;
    db(&s, move |store| {
        store.public_status(&catalogue, &ids, now())
    })
    .await
    .map(Json)
}
pub async fn queue(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.monitoring_queue(&actor))
        .await
        .map(Json)
}
pub fn start_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let Some(store) = &state.store else { break };
            if state.sandbox {
                break;
            }
            let Ok(catalogue) = state.read_catalogue() else {
                continue;
            };
            if store.sync_monitor_catalogue(&catalogue, now()).is_err() {
                continue;
            }
            let Ok(Some(lease)) = store.lease_monitor(now()) else {
                continue;
            };
            let result = tokio::time::timeout(
                Duration::from_secs(45),
                omastore_workflow::monitor_probe::observe(&lease.app),
            )
            .await
            .unwrap_or_else(|_| Err(Error::new(503, "upstream_timeout")));
            let _ = store.finish_monitor(&lease, result.as_ref(), now());
        }
    })
}
