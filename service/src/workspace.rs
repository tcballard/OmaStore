use crate::{actor, db, ApiError, ApiResult, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use omastore_workflow::{
    drafts::Command,
    media::{normalize, ObjectStorage, Upload, VIDEO_LIMIT},
    now, Error,
};
use serde::Deserialize;
use serde_json::Value;

fn key(headers: &HeaderMap) -> ApiResult<String> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .filter(|v| {
            v.len() >= 16
                && v.len() <= 128
                && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .map(str::to_owned)
        .ok_or_else(|| Error::new(422, "invalid_idempotency_key").into())
}
pub async fn command(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Command>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    let key = key(&headers)?;
    db(&s, move |store| {
        store.check_rate(&format!("commands:{}", actor.id), now(), 120)?;
        store.command(&actor, &key, body, now())
    })
    .await
    .map(Json)
}
pub async fn draft(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.draft(&actor, &id))
        .await
        .map(Json)
}
pub async fn revision(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.revision(&actor, &id))
        .await
        .map(Json)
}
pub async fn upload_guard(
    State(s): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let Ok(_permit) = s.upload_concurrency.clone().try_acquire_owned() else {
        return ApiError(Error::new(429, "uploads_busy")).into_response();
    };
    let headers = request.headers().clone();
    let result = async {
        let actor = actor(&s, &headers).await?;
        db(&s, move |store| {
            store.check_rate(&format!("upload:{}", actor.id), now(), 10)
        })
        .await
    }
    .await;
    match result {
        Ok(()) => next.run(request).await,
        Err(error) => error.into_response(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UploadBody {
    draft_id: String,
    version: i64,
    kind: String,
    alt: String,
    rights: String,
    bytes: String,
}
pub async fn upload(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UploadBody>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    let key = key(&headers)?;
    let objects = s
        .objects
        .clone()
        .ok_or(Error::new(503, "media_storage_unconfigured"))?;
    db(&s, move |store| {
        let draft = store.draft(&actor, &body.draft_id)?;
        // Ownership is checked before decoding; the transaction checks the version again.
        // A repeated upload may already have advanced that version, so replay remains valid.
        let _ = draft;
        let bytes = STANDARD
            .decode(&body.bytes)
            .map_err(|_| Error::new(422, "invalid_media_encoding"))?;
        let asset = normalize(&body.kind, &bytes, &objects)?;
        store.attach_media(
            &actor,
            Upload {
                draft_id: &body.draft_id,
                version: body.version,
                kind: &body.kind,
                alt: &body.alt,
                rights: &body.rights,
                key: &key,
            },
            &asset,
            &objects,
            now(),
        )
    })
    .await
    .map(Json)
}
pub async fn media(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let actor = actor(&s, &headers).await?;
    let objects = s
        .objects
        .clone()
        .ok_or(Error::new(503, "media_storage_unconfigured"))?;
    let (mime, bytes) = db(&s, move |store| {
        let (key, mime) = store.private_media_key(&actor, &id)?;
        Ok((mime, objects.read_private(&key, VIDEO_LIMIT)?))
    })
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, no-store".into()),
            (header::CONTENT_DISPOSITION, "inline".into()),
        ],
        Body::from(bytes),
    )
        .into_response())
}

pub async fn review_queue(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.review_queue(&actor, now()))
        .await
        .map(Json)
}
pub async fn review_detail(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.review_detail(&actor, &id, now()))
        .await
        .map(Json)
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;
    async fn request(
        app: axum::Router,
        token: &str,
        method: &str,
        path: &str,
        body: Value,
        key: &str,
    ) -> (u16, Value) {
        let r = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("X-OmaStore-Client", "native-v1")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .header("Idempotency-Key", key)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = r.status().as_u16();
        let bytes = axum::body::to_bytes(r.into_body(), 1024 * 1024)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    #[tokio::test]
    async fn draft_http_replays_are_scoped_and_uploaded_bytes_stay_private() {
        let dir = tempfile::tempdir().unwrap();
        let store = omastore_workflow::Store::development(&dir.path().join("private.db")).unwrap();
        let a = store.development_login("author", now()).unwrap();
        let b = store.development_login("reviewer", now()).unwrap();
        let mut state = AppState::public(std::path::PathBuf::from("../data/registry.json"));
        state.store = Some(store);
        state.objects =
            Some(omastore_workflow::media::LocalObjects::new(&dir.path().join("objects")).unwrap());
        let app = crate::router(state);
        let token = a["token"].as_str().unwrap();
        let key = omastore_workflow::nonce().unwrap();
        let candidate: Value =
            serde_json::from_str(include_str!("../../docs/examples/submission.json")).unwrap();
        let body = serde_json::json!({"command":"create_draft","kind":"app","candidate":candidate,"base_revision":null});
        let (status, created) = request(
            app.clone(),
            token,
            "POST",
            "/api/v1/commands",
            body.clone(),
            &key,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(
            request(app.clone(), token, "POST", "/api/v1/commands", body, &key)
                .await
                .1,
            created
        );
        let path = format!("/api/v1/drafts/{}", created["id"].as_str().unwrap());
        assert_eq!(
            request(
                app.clone(),
                b["token"].as_str().unwrap(),
                "GET",
                &path,
                Value::Null,
                ""
            )
            .await
            .0,
            404
        );
        // A valid 1x1 PNG is decoded and re-encoded by the actual HTTP path.
        let bytes="iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR4nGP4DwQACfsD/fteaysAAAAASUVORK5CYII=";
        let upload = serde_json::json!({"draftId":created["id"],"version":1,"kind":"icon","alt":"Test pixel","rights":"Synthetic test asset","bytes":bytes});
        let (status, _) = request(
            app.clone(),
            b["token"].as_str().unwrap(),
            "POST",
            "/api/v1/media",
            upload.clone(),
            &omastore_workflow::nonce().unwrap(),
        )
        .await;
        assert_eq!(status, 404);
        let (status, asset) = request(
            app.clone(),
            token,
            "POST",
            "/api/v1/media",
            upload,
            &omastore_workflow::nonce().unwrap(),
        )
        .await;
        assert_eq!(status, 200, "{asset}");
        let media_path = format!("/api/v1/media/{}", asset["id"].as_str().unwrap());
        assert_eq!(
            request(app.clone(), token, "GET", &media_path, Value::Null, "")
                .await
                .0,
            200
        );
        assert_eq!(
            request(
                app.clone(),
                b["token"].as_str().unwrap(),
                "GET",
                &media_path,
                Value::Null,
                ""
            )
            .await
            .0,
            404
        );
        assert_eq!(
            std::fs::read_dir(dir.path().join("objects/public"))
                .unwrap()
                .count(),
            0
        );
        let (status,_) =request(app,token,"POST","/api/v1/commands",serde_json::json!({"command":"save_draft","id":created["id"],"version":999,"candidate":candidate}),&omastore_workflow::nonce().unwrap()).await;
        assert_eq!(status, 409);
    }
}

pub async fn operations(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let actor = actor(&s, &headers).await?;
    db(&s, move |store| store.operations_dashboard(&actor, now()))
        .await
        .map(Json)
}

pub async fn commerce_status(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let a = if headers
        .get("Authorization")
        .is_some_and(|v| v.as_bytes() != b"Bearer ")
    {
        Some(actor(&s, &headers).await?)
    } else {
        None
    };
    db(&s, move |store| store.commerce_status(a.as_ref()))
        .await
        .map(Json)
}
