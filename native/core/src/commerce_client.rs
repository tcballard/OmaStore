//! Private local checkout attempts preserve request keys across core/browser restarts.
use super::{private_directory, record_id, save_private, Client};
use omastore_workflow::{
    commerce_checkout::Purchase, commerce_license::Envelope, digest, nonce, Error, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attempt {
    id: String,
    owner: String,
    purchase: Purchase,
    price: Value,
    created_at: i64,
    order_id: Option<String>,
}
impl Client {
    fn purchase_owner(&self) -> Result<String> {
        let actor = self.cached["actor"]["id"]
            .as_str()
            .ok_or(Error::new(401, "sign_in_required"))?;
        Ok(digest(format!(
            "{}:{actor}",
            self.origin.as_deref().unwrap_or("sample")
        )))
    }
    fn purchases_directory(&self) -> Result<PathBuf> {
        let path = private_directory(self.demo)?.join("purchases");
        use std::os::unix::fs::DirBuilderExt;
        if !path.exists() {
            std::fs::DirBuilder::new().mode(0o700).create(&path)?;
        }
        let m = std::fs::symlink_metadata(&path)?;
        if !m.is_dir() || m.mode() & 0o077 != 0 || m.uid() != unsafe { libc::geteuid() } {
            return Err(Error::new(500, "private_checkout_storage_required"));
        }
        Ok(path)
    }
    fn attempts(&self) -> Result<Vec<Attempt>> {
        let dir = self.purchases_directory()?;
        let owner = self.purchase_owner()?;
        let mut out = Vec::new();
        let files: Vec<_> = std::fs::read_dir(dir)?
            .take(1001)
            .collect::<std::io::Result<_>>()?;
        if files.len() > 1000 {
            return Err(Error::new(409, "checkout_storage_full"));
        }
        for e in files {
            if e.path().extension().is_some_and(|x| x == "json") {
                let a: Attempt = serde_json::from_slice(&read_private(&e.path(), 24 * 1024)?)?;
                if a.owner == owner {
                    out.push(a);
                }
            }
        }
        out.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(out)
    }
    fn attempt(&self, id: &str) -> Result<Attempt> {
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new(422, "invalid_fields"));
        }
        let a: Attempt = serde_json::from_slice(&read_private(
            &self.purchases_directory()?.join(format!("{id}.json")),
            24 * 1024,
        )?)?;
        if a.owner != self.purchase_owner()? || a.id != id {
            return Err(Error::new(404, "checkout_attempt_unavailable"));
        }
        Ok(a)
    }
    fn save_attempt(&self, a: &Attempt) -> Result<()> {
        save_private(
            &self.purchases_directory()?.join(format!("{}.json", a.id)),
            &serde_json::to_vec(a)?,
        )
    }
    pub(super) fn commerce_action(&mut self, method: &str, p: &Value) -> Result<Value> {
        if method == "commerce.packet.export" {
            let id = p["id"]
                .as_str()
                .filter(|s| omastore_workflow::commerce_model::provider_id(s, "du_"))
                .ok_or(Error::new(422, "invalid_fields"))?;
            let v =
                self.commerce_remote("lifecycle", &json!({"action":"dispute_packet","id":id}), "")?;
            if v["digest"] != p["digest"] {
                return Err(Error::new(409, "evidence_preview_changed"));
            }
            export(p, &serde_json::to_vec_pretty(&v["packet"])?)?;
            return Ok(json!({"exported":true}));
        }
        if method == "commerce.lifecycle" {
            let action: omastore_workflow::commerce_actions::Action =
                serde_json::from_value(p.clone())?;
            let canonical = serde_json::to_value(action)?;
            // An identical refund intent reuses its retained key across restarts/lost replies.
            let key = if canonical["action"] == "request_refund" {
                format!(
                    "refund-{}",
                    digest(serde_json::to_vec(
                        &json!({"owner":self.purchase_owner()?,"intent":canonical})
                    )?)
                )
            } else {
                String::new()
            };
            let mut result = self.commerce_remote("lifecycle", &canonical, &key)?;
            result["operation"] = canonical["action"].clone();
            return Ok(result);
        }
        if method == "commerce.pending" {
            return Ok(
                json!({"items":self.attempts()?.iter().map(|a|json!({"id":a.id,"price":a.price,"createdAt":a.created_at,"orderId":a.order_id})).collect::<Vec<_>>()}),
            );
        }
        if method == "commerce.prepare" {
            let owner = self.purchase_owner()?;
            let pid = p["priceId"]
                .as_str()
                .ok_or(Error::new(422, "invalid_fields"))?;
            let prices = self.commerce_remote("prices", &json!({}), "")?;
            let offer = prices["items"]
                .as_array()
                .and_then(|a| a.iter().find(|v| v["price"]["id"] == pid))
                .ok_or(Error::new(404, "commerce_offer_unavailable"))?;
            let sha = offer["digest"]
                .as_str()
                .ok_or(Error::new(422, "invalid_price_preview"))?;
            let attempts = self.attempts()?;
            if let Some(a) = attempts
                .iter()
                .find(|a| a.purchase.digest == sha && a.order_id.is_none())
            {
                return Ok(json!({"id":a.id,"price":a.price,"digest":sha,"resuming":true}));
            }
            if attempts.len() >= 100 {
                return Err(Error::new(409, "checkout_storage_full"));
            }
            let a = Attempt {
                id: nonce()?,
                owner,
                purchase: Purchase {
                    price_id: pid.into(),
                    version: offer["price"]["version"]
                        .as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or(Error::new(422, "invalid_price_preview"))?,
                    digest: sha.into(),
                    accepted: true,
                },
                price: offer["price"].clone(),
                created_at: omastore_workflow::now(),
                order_id: None,
            };
            self.save_attempt(&a)?;
            return Ok(
                json!({"id":a.id,"price":a.price,"digest":a.purchase.digest,"resuming":false}),
            );
        }
        if method == "commerce.resume" {
            let a = self.attempt(record_id(p)?)?;
            return Ok(
                json!({"id":a.id,"price":a.price,"digest":a.purchase.digest,"resuming":true,"orderId":a.order_id}),
            );
        }
        if method == "commerce.purchase" {
            if p["accepted"] != true {
                return Err(Error::new(422, "purchase_consent_required"));
            }
            let mut a = self.attempt(record_id(p)?)?;
            let v = self.commerce_remote("purchase", &json!(a.purchase), &a.id)?;
            a.order_id = v["id"].as_str().map(str::to_owned);
            self.save_attempt(&a)?;
            return decorate(v);
        }
        if method == "commerce.licence.export" {
            let receipt = self.commerce_remote("order", p, "")?;
            let e: Envelope = serde_json::from_value(receipt["grant"].clone())?;
            let bytes = serde_json::to_vec_pretty(&e)?;
            if p["digest"] != digest(serde_json::to_vec(&e)?) {
                return Err(Error::new(409, "licence_preview_changed"));
            }
            export(p, &bytes)?;
            return Ok(json!({"exported":true}));
        }
        if method == "commerce.licence.verify" {
            let path = file_path(p)?;
            let e: Envelope = serde_json::from_slice(&read_private(&path, 32 * 1024)?)?;
            #[cfg(feature = "development-catalogue")]
            let sample = if self.demo {
                Some(omastore_workflow::commerce_license::sample_issuer()?)
            } else {
                None
            };
            #[cfg(not(feature = "development-catalogue"))]
            let sample: Option<omastore_workflow::commerce_license::Issuer> = None;
            let key = sample.as_ref().map(|i| i.public_key());
            let anchors = sample
                .as_ref()
                .zip(key.as_ref())
                .map(|(i, k)| vec![(i.key_id(), k.as_slice())])
                .unwrap_or_default();
            let grant = omastore_workflow::commerce_license::verify(
                &e,
                &anchors,
                omastore_workflow::now(),
            )?;
            return Ok(
                json!({"verifiedOffline":true,"sample":self.demo,"grant":grant,"notice":"The signature proves the issued grant. Current refunds, disputes and hosted access are separate."}),
            );
        }
        let action = method
            .strip_prefix("commerce.")
            .ok_or(Error::new(422, "unknown_workspace_method"))?;
        decorate(self.commerce_remote(action, p, "")?)
    }
    fn commerce_remote(&self, action: &str, p: &Value, key: &str) -> Result<Value> {
        #[cfg(feature = "development-catalogue")]
        if let Some(store) = &self.sandbox {
            store.commerce_seed_sample(omastore_workflow::now())?;
            if action == "prices" {
                return store.commerce_prices(p["appId"].as_str());
            }
            let a = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            let now = omastore_workflow::now();
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| Error::new(500, "local_worker_unavailable"))?;
            let provider = omastore_workflow::commerce_sample::Sample(store.clone());
            let issuer = omastore_workflow::commerce_license::sample_issuer()?;
            let fulfilment = omastore_workflow::commerce_delivery::SampleFulfilment(store.clone());
            return match action {
                "lifecycle" => runtime.block_on(store.commerce_lifecycle_action(
                    &a,
                    key,
                    serde_json::from_value(p.clone())?,
                    &provider,
                    now,
                )),
                "author" => store.commerce_author(&a),
                "orders" => store.commerce_orders(&a, p["before"].as_i64().unwrap_or(0)),
                "order" => store.commerce_order(&a, record_id(p)?),
                "purchase" => {
                    let buy: Purchase = serde_json::from_value(p.clone())?;
                    let id = store.commerce_begin(&a, key, buy, "sample", now)?;
                    runtime.block_on(store.commerce_checkout(&a, &id, &provider, now))
                }
                "reconcile" => {
                    runtime.block_on(store.commerce_reconcile(&a, record_id(p)?, &provider, now))
                }
                "retry" => {
                    let id = record_id(p)?;
                    store.commerce_retry_delivery(&a, id, now)?;
                    runtime.block_on(store.commerce_deliver(
                        id,
                        Some(&issuer),
                        &fulfilment,
                        now,
                    ))?;
                    store.commerce_order(&a, id)
                }
                "sample.capture" => {
                    let id = record_id(p)?;
                    provider.capture(&a, id, now)?;
                    runtime.block_on(store.commerce_reconcile(&a, id, &provider, now))?;
                    let signed = if p["failDelivery"] == true {
                        None
                    } else {
                        Some(&issuer)
                    };
                    runtime.block_on(store.commerce_deliver(id, signed, &fulfilment, now))?;
                    store.commerce_order(&a, id)
                }
                "recover" => runtime.block_on(store.commerce_recover_download(
                    &a,
                    record_id(p)?,
                    &fulfilment,
                    now,
                )),
                "support" => store.commerce_support(&a, record_id(p)?),
                _ => Err(Error::new(422, "unknown_commerce_action")),
            };
        }
        let (method, path) = match action {
            "lifecycle" => ("POST", "/api/v1/commerce/lifecycle".into()),
            "prices" => {
                let mut path = "/api/v1/commerce/prices".to_string();
                if let Some(id) = p["appId"].as_str() {
                    if !omastore_catalogue::token(id) {
                        return Err(Error::new(422, "invalid_fields"));
                    }
                    path.push_str(&format!("?appId={id}"));
                }
                ("GET", path)
            }
            "author" => ("GET", "/api/v1/commerce/author".into()),
            "sample.capture" => (
                "POST",
                format!("/api/v1/commerce/orders/{}/sample-capture", record_id(p)?),
            ),
            "orders" => (
                "GET",
                format!(
                    "/api/v1/commerce/orders?before={}",
                    p["before"].as_i64().unwrap_or(0)
                ),
            ),
            "order" => ("GET", format!("/api/v1/commerce/orders/{}", record_id(p)?)),
            "purchase" => ("POST", "/api/v1/commerce/orders".into()),
            "reconcile" | "retry" | "recover" | "support" => (
                if action == "support" { "GET" } else { "POST" },
                format!("/api/v1/commerce/orders/{}/{action}", record_id(p)?),
            ),
            _ => return Err(Error::new(422, "unknown_commerce_action")),
        };
        self.http_key(method, &path, p, key)
    }
}
fn decorate(mut v: Value) -> Result<Value> {
    if v["grant"].is_object() {
        let e: Envelope = serde_json::from_value(v["grant"].clone())?;
        v["licenceDigest"] = json!(digest(serde_json::to_vec(&e)?));
        v["licenceDocument"] = json!(serde_json::to_string_pretty(&e)?);
    }
    Ok(v)
}
fn file_path(p: &Value) -> Result<PathBuf> {
    p["file"]
        .as_str()
        .and_then(|s| url::Url::parse(s).ok())
        .and_then(|u| u.to_file_path().ok())
        .filter(|p| p.is_absolute())
        .ok_or(Error::new(422, "local_file_required"))
}
fn read_private(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.len() > limit as u64 || m.uid() != unsafe { libc::geteuid() } {
        return Err(Error::new(422, "invalid_local_commerce_file"));
    }
    let mut b = Vec::new();
    f.take((limit + 1) as u64).read_to_end(&mut b)?;
    if b.len() > limit {
        return Err(Error::new(422, "invalid_local_commerce_file"));
    }
    Ok(b)
}
fn export(p: &Value, bytes: &[u8]) -> Result<()> {
    let path = file_path(p)?;
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err(Error::new(422, "regular_file_required"));
    }
    let parent = path
        .parent()
        .ok_or(Error::new(422, "local_file_required"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|_| Error::new(500, "local_export_failed"))?;
    Ok(())
}
