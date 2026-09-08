use crate::{db, ApiResult, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
};

pub async fn all(State(s): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    render(s, headers, None).await
}
pub async fn maker(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    render(s, headers, Some(id)).await
}
async fn render(s: AppState, headers: HeaderMap, maker: Option<String>) -> ApiResult<Response> {
    let xml = db(&s, move |store| store.release_feed(maker.as_deref())).await?;
    let etag = format!("\"{}\"", omastore_workflow::digest(&xml));
    let unchanged = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|s| s.to_str().ok())
        .is_some_and(|s| {
            s.split(',')
                .any(|tag| tag.trim().trim_start_matches("W/") == etag || tag.trim() == "*")
        });
    Ok(Response::builder()
        .status(if unchanged {
            StatusCode::NOT_MODIFIED
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")
        .header(header::ETAG, etag)
        .body(if unchanged {
            Body::empty()
        } else {
            Body::from(xml)
        })
        .unwrap())
}
