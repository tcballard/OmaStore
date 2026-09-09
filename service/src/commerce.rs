use crate::{actor, db, ApiResult, AppState};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use omastore_workflow::{commerce_checkout::Purchase, commerce_provider::Provider, now, Error};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PriceQuery {
    app_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    #[serde(default)]
    before: i64,
}
pub async fn lifecycle(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(p): Json<omastore_workflow::commerce_actions::Action>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    let key = h
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    Ok(Json(
        store
            .commerce_lifecycle_action(&a, key, p, s.commerce.as_ref(), now())
            .await?,
    ))
}
pub async fn prices(
    State(s): State<AppState>,
    Query(p): Query<PriceQuery>,
) -> ApiResult<Json<Value>> {
    db(&s, move |store| store.commerce_prices(p.app_id.as_deref()))
        .await
        .map(Json)
}
pub async fn orders(
    State(s): State<AppState>,
    h: HeaderMap,
    Query(p): Query<Page>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    db(&s, move |store| store.commerce_orders(&a, p.before))
        .await
        .map(Json)
}
pub async fn order(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    db(&s, move |store| store.commerce_order(&a, &id))
        .await
        .map(Json)
}
pub async fn purchase(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(p): Json<Purchase>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    let key = h
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::new(422, "invalid_idempotency_key"))?
        .to_owned();
    let mode = s.commerce.mode();
    let ac = a.clone();
    let id = db(&s, move |store| {
        store.commerce_begin(&ac, &key, p, mode, now())
    })
    .await?;
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    Ok(Json(
        store
            .commerce_checkout(&a, &id, s.commerce.as_ref(), now())
            .await?,
    ))
}
pub async fn reconcile(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    Ok(Json(
        store
            .commerce_reconcile(&a, &id, s.commerce.as_ref(), now())
            .await?,
    ))
}
pub async fn retry(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    db(&s, move |store| {
        store.commerce_retry_delivery(&a, &id, now())
    })
    .await
    .map(Json)
}
pub async fn recover(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    Ok(Json(
        store
            .commerce_recover_download(&a, &id, s.fulfilment.as_ref(), now())
            .await?,
    ))
}
pub async fn support(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    db(&s, move |store| store.commerce_support(&a, &id))
        .await
        .map(Json)
}
pub async fn webhook(
    State(s): State<AppState>,
    h: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let signature = h
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::new(401, "invalid_payment_signature"))?;
    let secret = s
        .commerce_webhook_secret
        .as_deref()
        .ok_or(Error::new(503, "payment_webhook_unconfigured"))?;
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    Ok(Json(
        store
            .commerce_all_webhooks(s.commerce.as_ref(), secret, signature, &body, now())
            .await?,
    ))
}
pub async fn returned() -> &'static str {
    "Return to OmaStore and refresh Purchases. This browser return does not confirm payment. Your receipt remains recoverable from your account."
}
pub fn start_worker(s: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tick.tick().await;
            if let Some(store) = &s.store {
                let _ = store
                    .commerce_lifecycle_tick(s.commerce.as_ref(), now())
                    .await;
                if let Ok(ids) = store.commerce_payment_queue(s.commerce.mode(), now()) {
                    for id in ids {
                        let _ = store
                            .commerce_poll_payment(&id, s.commerce.as_ref(), now())
                            .await;
                    }
                }
                if let Ok(ids) = store.commerce_pending_delivery() {
                    for id in ids {
                        let _ = store
                            .commerce_deliver(
                                &id,
                                s.commerce_issuer.as_deref(),
                                s.fulfilment.as_ref(),
                                now(),
                            )
                            .await;
                    }
                }
            }
        }
    })
}
/// Called only after an isolated development database and an explicit test flag exist.
pub fn configure(s: &mut AppState) -> omastore_workflow::Result<()> {
    if s.sandbox && s.commerce_test {
        return Err(Error::new(422, "choose_one_commerce_test_mode"));
    }
    #[cfg(feature = "development-workflow")]
    if s.sandbox {
        let store = s
            .store
            .as_ref()
            .ok_or(Error::new(503, "workspace_unconfigured"))?;
        store.commerce_seed_sample(now())?;
        s.commerce = Arc::new(omastore_workflow::commerce_sample::Sample(store.clone()));
        s.fulfilment = Arc::new(omastore_workflow::commerce_delivery::SampleFulfilment(
            store.clone(),
        ));
        s.commerce_issuer = Some(Arc::new(
            omastore_workflow::commerce_license::sample_issuer()?,
        ));
        return Ok(());
    }
    if !s.commerce_test {
        return Ok(());
    }
    if !cfg!(feature = "development-workflow") {
        return Err(Error::new(503, "commerce_test_build_required"));
    }
    let store = s
        .store
        .as_ref()
        .ok_or(Error::new(503, "workspace_unconfigured"))?;
    if store.commerce_status(None)?["environment"] != "development" {
        return Err(Error::new(503, "commerce_test_database_required"));
    }
    store.commerce_bind_provider("stripe_test")?;
    let key = std::env::var("OMASTORE_STRIPE_TEST_KEY")
        .map_err(|_| Error::new(503, "stripe_test_key_required"))?;
    let provider = omastore_workflow::commerce_stripe::StripeTest::new(key, s.origin.clone())?;
    if provider.mode() != "stripe_test" {
        return Err(Error::new(503, "payment_provider_mode_mismatch"));
    }
    s.commerce = Arc::new(provider);
    let secret = std::env::var("OMASTORE_STRIPE_TEST_WEBHOOK_SECRET")
        .map_err(|_| Error::new(503, "payment_webhook_unconfigured"))?;
    if secret.len() < 24 || secret.len() > 256 {
        return Err(Error::new(503, "payment_webhook_unconfigured"));
    }
    s.commerce_webhook_secret = Some(secret);
    if let Ok(path) = std::env::var("OMASTORE_COMMERCE_ISSUER_FILE") {
        let seed = private_file(&path, 32)?;
        s.commerce_issuer = Some(Arc::new(
            omastore_workflow::commerce_license::Issuer::from_seed("operator-test-issuer", &seed)?,
        ));
    }
    if let Ok(path) = std::env::var("OMASTORE_FULFILMENT_FILE") {
        let bytes = private_file(&path, 128 * 1024)?;
        s.fulfilment = Arc::new(omastore_workflow::commerce_delivery::HttpFulfilment::new(
            serde_json::from_slice(&bytes)?,
        )?);
    }
    Ok(())
}
fn private_file(path: &str, limit: usize) -> omastore_workflow::Result<Vec<u8>> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let m = file.metadata()?;
    if !m.is_file()
        || m.mode() & 0o077 != 0
        || m.uid() != unsafe { libc::geteuid() }
        || m.len() > limit as u64
    {
        return Err(Error::new(503, "private_commerce_config_required"));
    }
    let mut bytes = Vec::new();
    file.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::new(503, "private_commerce_config_required"));
    }
    Ok(bytes)
}

pub async fn author(State(s): State<AppState>, h: HeaderMap) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    db(&s, move |store| store.commerce_author(&a))
        .await
        .map(Json)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capture {
    #[serde(default)]
    fail_delivery: bool,
}
pub async fn sample_capture(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(p): Json<Capture>,
) -> ApiResult<Json<Value>> {
    let a = actor(&s, &h).await?;
    #[cfg(feature = "development-workflow")]
    if s.sandbox && s.commerce.mode() == "sample" {
        let store = s
            .store
            .as_ref()
            .ok_or(Error::new(503, "workspace_unconfigured"))?;
        let provider = omastore_workflow::commerce_sample::Sample(store.clone());
        provider.capture(&a, &id, now())?;
        store.commerce_reconcile(&a, &id, &provider, now()).await?;
        store
            .commerce_deliver(
                &id,
                if p.fail_delivery {
                    None
                } else {
                    s.commerce_issuer.as_deref()
                },
                s.fulfilment.as_ref(),
                now(),
            )
            .await?;
        return Ok(Json(store.commerce_order(&a, &id)?));
    }
    let _ = (a, id, p.fail_delivery);
    Err(Error::new(403, "sample_mode_required").into())
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    async fn call(
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
                    .header("Content-Type", "application/json")
                    .header("X-OmaStore-Client", "native-v1")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Idempotency-Key", key)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = r.status().as_u16();
        let bytes = to_bytes(r.into_body(), 256 * 1024).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    #[tokio::test]
    async fn http_purchase_uses_server_price_and_receipts_outlive_the_browser() {
        let dir = tempfile::tempdir().unwrap();
        let store = omastore_workflow::Store::development(&dir.path().join("commerce.db")).unwrap();
        let c = serde_json::from_str(include_str!("../../tests/fixtures/catalogue.json")).unwrap();
        store.seed_sample_context(&c).unwrap();
        let login = store.development_login("author", now()).unwrap();
        let token = login["token"].as_str().unwrap();
        let other = store.development_login("reviewer", now()).unwrap();
        let mut s = AppState::public("../tests/fixtures/catalogue.json".into());
        s.store = Some(store.clone());
        s.sandbox = true;
        configure(&mut s).unwrap();
        let op = store.development_login("operator", now()).unwrap();
        let op = store.actor(op["token"].as_str().unwrap(), now()).unwrap();
        store
            .command(
                &op,
                &omastore_workflow::nonce().unwrap(),
                omastore_workflow::drafts::Command::CommercePause {
                    paused: false,
                    reason: "HTTP fixture".into(),
                },
                now(),
            )
            .unwrap();
        let app = crate::router(s.clone());
        let (status, prices) = call(
            app.clone(),
            token,
            "GET",
            "/api/v1/commerce/prices",
            Value::Null,
            "",
        )
        .await;
        assert_eq!(status, 200);
        let p = prices["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["price"]["id"] == "sample-perpetual")
            .unwrap();
        let buy = serde_json::json!({"priceId":"sample-perpetual","version":1,"digest":p["digest"],"accepted":true});
        let key = omastore_workflow::nonce().unwrap();
        let mut tampered = buy.clone();
        tampered["total"] = serde_json::json!(1);
        assert_eq!(
            call(
                app.clone(),
                token,
                "POST",
                "/api/v1/commerce/orders",
                tampered,
                &key
            )
            .await
            .0,
            422
        );
        let (status, pending) = call(
            app.clone(),
            token,
            "POST",
            "/api/v1/commerce/orders",
            buy.clone(),
            &key,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(pending["paymentState"], "payment_pending");
        let id = pending["id"].as_str().unwrap();
        assert_eq!(
            call(
                app.clone(),
                token,
                "POST",
                "/api/v1/commerce/orders",
                buy,
                &key
            )
            .await
            .1["id"],
            id
        );
        let path = format!("/api/v1/commerce/orders/{id}");
        assert_eq!(
            call(
                app.clone(),
                other["token"].as_str().unwrap(),
                "GET",
                &path,
                Value::Null,
                ""
            )
            .await
            .0,
            404
        );
        let (status, failed) = call(
            app.clone(),
            token,
            "POST",
            &format!("{path}/sample-capture"),
            serde_json::json!({"failDelivery":true}),
            "",
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(failed["deliveryState"], "delivery_failed");
        call(
            app.clone(),
            token,
            "POST",
            &format!("{path}/retry"),
            serde_json::json!({}),
            "",
        )
        .await;
        store
            .commerce_deliver(
                id,
                s.commerce_issuer.as_deref(),
                s.fulfilment.as_ref(),
                now(),
            )
            .await
            .unwrap();
        let fresh = store.development_login("author", now()).unwrap();
        let (status, receipt) = call(
            app.clone(),
            fresh["token"].as_str().unwrap(),
            "GET",
            &path,
            Value::Null,
            "",
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(receipt["deliveryState"], "delivered");
        assert!(receipt["grant"].is_object());
        assert!(receipt.to_string().find("acct_samplemaker").is_none());
        let key = omastore_workflow::nonce().unwrap();
        let action = serde_json::json!({"action":"request_refund","request":{"orderId":id,"amount":200,"expectedRefunded":0,"reason":"HTTP fixture refund","accepted":true}});
        let (status, refund) = call(
            app.clone(),
            token,
            "POST",
            "/api/v1/commerce/lifecycle",
            action.clone(),
            &key,
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(
            call(
                app.clone(),
                token,
                "POST",
                "/api/v1/commerce/lifecycle",
                action,
                &key
            )
            .await
            .1["id"],
            refund["id"]
        );
        let action = serde_json::json!({"action":"execute_refund","id":refund["id"]});
        assert_eq!(
            call(
                app.clone(),
                token,
                "POST",
                "/api/v1/commerce/lifecycle",
                action.clone(),
                ""
            )
            .await
            .0,
            403
        );
        let operator = store.development_login("operator", now()).unwrap();
        let op_token = operator["token"].as_str().unwrap();
        let (status, refund) = call(
            app.clone(),
            op_token,
            "POST",
            "/api/v1/commerce/lifecycle",
            action,
            "",
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(refund["state"], "succeeded");
        assert_eq!(refund["feeAmount"], 8);
        let (_,report)=call(app,token,"POST","/api/v1/commerce/lifecycle",serde_json::json!({"action":"finances","seller_id":"sample-maker","refresh":true,"cursor":null}),"").await;
        assert_eq!(report["currencies"][0]["recordedProceeds"], 828);
    }
}
