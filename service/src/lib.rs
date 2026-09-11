pub mod monitoring;
pub mod publication;
mod workspace;
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use omastore_catalogue::{http, Catalogue, MAX_CATALOGUE_BYTES};
use omastore_workflow::{auth::GithubOAuth, net, now, Actor, Error, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs::File, io::Read, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct AppState {
    pub catalogue: PathBuf,
    pub store: Option<Store>,
    pub objects: Option<omastore_workflow::media::LocalObjects>,
    pub oauth: Option<Arc<GithubOAuth>>,
    pub github: Option<omastore_workflow::github::Github>,
    pub publication_state: String,
    pub origin: String,
    pub sandbox: bool,
    pub concurrency: Arc<Semaphore>,
    pub upload_concurrency: Arc<Semaphore>,
}
impl AppState {
    pub fn public(catalogue: PathBuf) -> Self {
        Self {
            catalogue,
            store: None,
            objects: None,
            oauth: None,
            github: None,
            publication_state: "unconfigured".into(),
            origin: String::new(),
            sandbox: false,
            concurrency: Arc::new(Semaphore::new(32)),
            upload_concurrency: Arc::new(Semaphore::new(2)),
        }
    }
    pub fn read_catalogue(&self) -> omastore_workflow::Result<Catalogue> {
        let mut bytes = Vec::new();
        File::open(&self.catalogue)?
            .take((MAX_CATALOGUE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Catalogue::parse(&bytes, cfg!(feature = "development-catalogue"))
            .map_err(|_| Error::new(503, "catalogue_unavailable"))
    }
}
pub struct ApiError(pub Error);
impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.0.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({"error":{"code":self.0.code}})),
        )
            .into_response()
    }
}
pub type ApiResult<T> = std::result::Result<T, ApiError>;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/status", get(monitoring::status))
        .route("/api/v1/monitoring", get(monitoring::queue))
        .route("/api/v1/auth/info", get(auth_info))
        .route("/api/v1/auth/start", post(auth_start))
        .route("/api/v1/auth/callback", get(auth_callback))
        .route("/api/v1/auth/poll", post(auth_poll))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/sandbox", post(sandbox_login))
        .route("/api/v1/workspace", get(workspace))
        .route("/api/v1/publication/{id}", get(publication::status))
        .route(
            "/api/v1/publication/{id}/manifest",
            get(publication::manifest),
        )
        .route("/api/v1/submissions/{id}", get(publication::submitted))
        .route("/api/v1/github/webhook", post(publication::webhook))
        .route("/api/v1/review", get(workspace::review_queue))
        .route("/api/v1/review/{id}", get(workspace::review_detail))
        .route("/api/v1/commands", post(workspace::command))
        .route("/api/v1/drafts/{id}", get(workspace::draft))
        .route("/api/v1/revisions/{id}", get(workspace::revision))
        .route("/api/v1/media/{id}", get(workspace::media))
        .route(
            "/api/v1/media",
            post(workspace::upload)
                .layer(DefaultBodyLimit::max(42 * 1024 * 1024))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    workspace::upload_guard,
                )),
        )
        .route("/api/v1/claims/start", post(claim_start))
        .route("/api/v1/claims/verify", post(claim_verify))
        .route("/api/v1/claims/revoke", post(claim_revoke))
        .fallback(public_read)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(
            42 * 1024 * 1024,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
async fn guard(State(s): State<AppState>, request: axum::extract::Request, next: Next) -> Response {
    if request.uri().to_string().len() > 4096
        || request
            .headers()
            .iter()
            .map(|(k, v)| k.as_str().len() + v.len())
            .sum::<usize>()
            > 16384
    {
        return ApiError(Error::new(431, "request_headers_too_large")).into_response();
    }
    let Ok(_permit) = s.concurrency.clone().try_acquire_owned() else {
        return ApiError(Error::new(429, "service_busy")).into_response();
    };
    let mut response = match tokio::time::timeout(Duration::from_secs(30), next.run(request)).await
    {
        Ok(response) => response,
        Err(_) => ApiError(Error::new(503, "request_timeout")).into_response(),
    };
    response
        .headers_mut()
        .insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    response
        .headers_mut()
        .insert("Cache-Control", "no-store".parse().unwrap());
    response
}
pub async fn db<T: Send + 'static>(
    s: &AppState,
    action: impl FnOnce(Store) -> omastore_workflow::Result<T> + Send + 'static,
) -> ApiResult<T> {
    let store = s
        .store
        .clone()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    tokio::task::spawn_blocking(move || action(store))
        .await
        .map_err(|_| ApiError(Error::new(500, "worker_unavailable")))?
        .map_err(Into::into)
}
fn native(headers: &HeaderMap, s: &AppState) -> ApiResult<()> {
    if headers
        .get("X-OmaStore-Client")
        .and_then(|h| h.to_str().ok())
        != Some("native-v1")
    {
        return Err(Error::new(403, "native_client_required").into());
    }
    if headers
        .get("Origin")
        .is_some_and(|h| h.to_str().ok() != Some(s.origin.as_str()))
    {
        return Err(Error::new(403, "cross_origin_request").into());
    }
    Ok(())
}
fn bearer(headers: &HeaderMap) -> ApiResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_owned)
        .ok_or_else(|| Error::new(401, "sign_in_required").into())
}
pub async fn actor(s: &AppState, headers: &HeaderMap) -> ApiResult<Actor> {
    native(headers, s)?;
    let token = bearer(headers)?;
    db(s, move |store| store.actor(&token, now())).await
}
async fn auth_info(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"workspaceEnabled":s.store.is_some(),"signInConfigured":s.oauth.is_some(),"sandbox":s.sandbox,"origin":s.origin,"publicationBridge":s.publication_state}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Start {
    desktop_challenge: String,
}
async fn auth_start(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Start>,
) -> ApiResult<Json<Value>> {
    native(&headers, &s)?;
    let oauth = s
        .oauth
        .clone()
        .ok_or(Error::new(503, "sign_in_unconfigured"))?;
    db(&s, move |store| {
        store.check_rate("sign-in", now(), 60)?;
        Ok(json!(store.start_login(
            &body.desktop_challenge,
            &oauth,
            now()
        )?))
    })
    .await
    .map(Json)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Callback {
    state: String,
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    error_uri: Option<String>,
}
async fn auth_callback(
    State(s): State<AppState>,
    Query(body): Query<Callback>,
) -> ApiResult<&'static str> {
    let _ = (&body.error_description, &body.error_uri);
    if body.error.is_some() {
        return Err(Error::new(401, "provider_authorization_denied").into());
    }
    let oauth = s
        .oauth
        .clone()
        .ok_or(Error::new(503, "sign_in_unconfigured"))?;
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    oauth
        .complete(
            store,
            &body.state,
            body.code
                .as_deref()
                .ok_or(Error::new(401, "authorization_code_missing"))?,
            now(),
        )
        .await?;
    Ok("Sign-in complete. Return to OmaStore. You may close this browser tab.")
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Poll {
    attempt_id: String,
    verifier: String,
}
async fn auth_poll(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Poll>,
) -> ApiResult<Json<Value>> {
    native(&headers, &s)?;
    db(&s, move |store| {
        store.check_rate(&format!("poll:{}", body.attempt_id), now(), 40)?;
        store.poll_login(&body.attempt_id, &body.verifier, now())
    })
    .await
    .map(Json)
}
async fn logout(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    native(&headers, &s)?;
    let token = bearer(&headers)?;
    db(&s, move |store| {
        store.revoke_session(&token, now())?;
        Ok(json!({"signedOut":true}))
    })
    .await
    .map(Json)
}
async fn sandbox_login(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    native(&headers, &s)?;
    #[cfg(feature = "development-workflow")]
    if s.sandbox {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Input {
            name: String,
        }
        let input: Input =
            serde_json::from_slice(&body).map_err(|_| Error::new(422, "invalid_fields"))?;
        return db(&s, move |store| store.development_login(&input.name, now()))
            .await
            .map(Json);
    }
    let _ = body;
    Err(Error::new(404, "not_found").into())
}
async fn workspace(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.workspace(&actor, now()))
        .await
        .map(Json)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    target: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Id {
    id: String,
}
async fn claim_start(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Target>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    let issuer = s.origin.clone();
    db(&s, move |store| {
        store.check_rate(&format!("claim:{}", actor.id), now(), 10)?;
        store.begin_claim(&actor, &body.target, &issuer, now())
    })
    .await
    .map(Json)
}
async fn claim_verify(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Id>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    let id = body.id.clone();
    let a = actor.clone();
    let url = db(&s, move |store| store.claim_proof_url(&a, &id, now())).await?;
    let proof = net::fetch(&url, 4096).await?;
    db(&s, move |store| {
        store.verify_claim(&actor, &body.id, &proof.bytes, now())
    })
    .await
    .map(Json)
}
async fn claim_revoke(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Target>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| {
        store.revoke_claim(&actor, &body.target, now())?;
        Ok(json!({"revoked":true}))
    })
    .await
    .map(Json)
}
async fn public_read(
    State(s): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let result = tokio::task::spawn_blocking(move || match s.read_catalogue() {
        Ok(c) => http::handle(
            &c,
            method.as_str(),
            &uri.to_string(),
            headers.get("If-None-Match").and_then(|h| h.to_str().ok()),
            chrono::Utc::now(),
        ),
        Err(_) => http::Response {
            status: 503,
            etag: String::new(),
            body: br#"{"error":{"code":"catalogue_unavailable"}}"#.to_vec(),
        },
    })
    .await;
    let Ok(result) = result else {
        return ApiError(Error::new(503, "catalogue_unavailable")).into_response();
    };
    let mut builder = Response::builder()
        .status(result.status)
        .header("Content-Type", "application/json; charset=utf-8");
    if !result.etag.is_empty() {
        builder = builder.header("ETag", result.etag);
    }
    if result.status == 405 {
        builder = builder.header("Allow", "GET");
    }
    builder.body(Body::from(result.body)).unwrap()
}

/// One bounded worker per service process; SQLite leases coordinate replicas/restarts.
pub fn start_checks_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let job = match db(&state, |store| store.lease_checks(now())).await {
                Ok(Some(job)) => job,
                _ => continue,
            };
            let Some(store) = state.store.as_ref() else {
                break;
            };
            let findings = omastore_workflow::checks::run(store, &job).await;
            let _ = db(&state, move |store| {
                store.finish_checks(&job, &findings, now())
            })
            .await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    fn state() -> AppState {
        AppState::public(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/registry.json"))
    }
    #[tokio::test]
    async fn unavailable_identity_keeps_public_catalogue_usable() {
        let app = router(state());
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/catalogue")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/workspace")
                    .header("X-OmaStore-Client", "native-v1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/start")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"desktopChallenge":"abc"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
}
