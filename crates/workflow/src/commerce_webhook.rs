//! Verify untouched provider bytes before parsing or changing an order.
use crate::{
    commerce_model::{provider_id, STRIPE_VERSION},
    commerce_provider::Provider,
    digest, Error, Result, Store,
};
use hmac::{Hmac, Mac};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::Sha256;
pub fn verify(secret: &str, header: &str, body: &[u8], now: i64) -> Result<Value> {
    if secret.len() < 24 || header.len() > 2048 || body.len() > 64 * 1024 {
        return Err(Error::new(401, "invalid_payment_signature"));
    }
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        let Some((k, v)) = part.trim().split_once('=') else {
            continue;
        };
        if k == "t" {
            if timestamp.is_some() {
                return Err(Error::new(401, "invalid_payment_signature"));
            }
            timestamp = v.parse::<i64>().ok();
        }
        if k == "v1" && v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()) {
            let decoded = (0..64)
                .step_by(2)
                .map(|n| u8::from_str_radix(&v[n..n + 2], 16).unwrap())
                .collect::<Vec<_>>();
            signatures.push(decoded);
        }
    }
    let at = timestamp
        .filter(|n| now.abs_diff(*n) <= 300)
        .ok_or(Error::new(401, "payment_signature_expired"))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| Error::new(401, "invalid_payment_signature"))?;
    mac.update(at.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    if !signatures
        .iter()
        .any(|s| mac.clone().verify_slice(s).is_ok())
    {
        return Err(Error::new(401, "invalid_payment_signature"));
    }
    let v: Value = serde_json::from_slice(body)?;
    if v["livemode"] != false
        || v["api_version"] != STRIPE_VERSION
        || !v["id"].as_str().is_some_and(|s| provider_id(s, "evt_"))
        || !v["account"]
            .as_str()
            .is_some_and(|s| provider_id(s, "acct_"))
    {
        return Err(Error::new(422, "unsupported_payment_event"));
    }
    Ok(v)
}
impl Store {
    pub async fn commerce_webhook(
        &self,
        provider: &dyn Provider,
        secret: &str,
        header: &str,
        body: &[u8],
        now: i64,
    ) -> Result<Value> {
        let v = verify(secret, header, body, now)?;
        let kind = v["type"].as_str().unwrap_or("");
        if ![
            "checkout.session.completed",
            "checkout.session.async_payment_succeeded",
            "checkout.session.async_payment_failed",
            "checkout.session.expired",
        ]
        .contains(&kind)
        {
            return Ok(json!({"ignored":true}));
        }
        let id = v["id"].as_str().unwrap();
        let account = v["account"].as_str().unwrap();
        let session = v["data"]["object"]["id"]
            .as_str()
            .filter(|s| provider_id(s, "cs_test_"))
            .ok_or(Error::new(422, "unsupported_payment_event"))?;
        let sha = digest(body);
        let done = self.transaction(|t| {
            crate::commerce_checkout::provider_guard(t, provider.mode())?;
            if let Some((old, state)) = t
                .query_row(
                    "SELECT digest,state FROM commerce_webhooks WHERE id=?1",
                    [id],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?
            {
                if old != sha {
                    return Err(Error::new(409, "payment_event_digest_conflict"));
                }
                return Ok(state == "completed");
            }
            t.execute(
                "INSERT INTO commerce_webhooks VALUES(?1,?2,?3,?4,'pending',?5)",
                params![id, sha, account, session, now],
            )?;
            Ok(false)
        })?;
        if done {
            return Ok(json!({"received":true,"replayed":true}));
        }
        let o = provider.lookup(account, session).await?;
        if o.account != account || o.session_id != session {
            return Err(Error::new(409, "payment_event_identity_mismatch"));
        }
        self.commerce_observe(provider.mode(), &o, now)?;
        self.connection()?.execute(
            "UPDATE commerce_webhooks SET state='completed' WHERE id=?1 AND digest=?2",
            params![id, sha],
        )?;
        Ok(json!({"received":true,"replayed":false}))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_signatures_reject_forgery_old_times_live_mode_and_reencoding() {
        let secret = "sample-signature-contract-value-123";
        let body=serde_json::to_vec(&json!({"id":"evt_test1","livemode":false,"account":"acct_test1","api_version":STRIPE_VERSION})).unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(b"1000.");
        mac.update(&body);
        let sig = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let header = format!("t=1000,v1={sig}");
        assert!(verify(secret, &header, &body, 1001).is_ok());
        assert!(verify(secret, &header, &body, 1301).is_err());
        assert!(verify(secret, &header, b"{}", 1001).is_err());
        assert!(verify(secret, &format!("t=1001,{header}"), &body, 1001).is_err());
    }
}
