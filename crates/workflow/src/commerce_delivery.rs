//! Fixed operator-configured fulfilment endpoints; catalogue entries cannot supply hooks.
use crate::{
    commerce_checkout::{access, event, intent, provider_guard},
    commerce_license::{Grant, Issuer},
    commerce_provider::CheckoutIntent,
    digest, nonce, Actor, Error, Result, Store,
};
use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Download {
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub order_id: String,
    pub buyer_reference: String,
    pub app_id: String,
    pub seller_id: String,
    pub reference: String,
    pub expires_at: Option<i64>,
    pub recovery_url: Option<String>,
    pub download: Option<Download>,
}
impl Receipt {
    fn validate(&self, i: &CheckoutIntent) -> Result<()> {
        if self.order_id != i.order_id
            || self.buyer_reference != i.buyer_reference
            || self.app_id != i.price.app_id
            || self.seller_id != i.price.seller_id
            || self.expires_at.is_some_and(|n| n <= i.created_at)
            || (i.price.delivery_kind == "subscription" && self.expires_at.is_none())
        {
            return Err(Error::new(409, "fulfilment_identity_mismatch"));
        }
        crate::bounded(&self.reference, 256)?;
        if let Some(u) = &self.recovery_url {
            crate::net::public_url(u)?;
        }
        if let Some(d) = &self.download {
            crate::net::public_url(&d.url)?;
            if d.bytes == 0
                || d.bytes > 20 * 1024 * 1024 * 1024
                || d.sha256.len() != 64
                || !d.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(Error::new(422, "invalid_download_receipt"));
            }
        }
        Ok(())
    }
}
#[async_trait]
pub trait Fulfilment: Send + Sync {
    async fn deliver(
        &self,
        i: &CheckoutIntent,
        key: &str,
        paid_until: Option<i64>,
    ) -> Result<Receipt>;
    async fn recover(&self, i: &CheckoutIntent, reference: &str) -> Result<Receipt>;
}
pub struct Unconfigured;
#[async_trait]
impl Fulfilment for Unconfigured {
    async fn deliver(&self, _: &CheckoutIntent, _: &str, _: Option<i64>) -> Result<Receipt> {
        Err(Error::new(503, "service_fulfilment_unconfigured"))
    }
    async fn recover(&self, _: &CheckoutIntent, _: &str) -> Result<Receipt> {
        Err(Error::new(503, "service_fulfilment_unconfigured"))
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub app_id: String,
    pub seller_id: String,
    pub endpoint: String,
    pub secret: String,
    pub download_origins: Vec<String>,
}
pub struct HttpFulfilment {
    bindings: Vec<Binding>,
}
impl HttpFulfilment {
    pub fn new(bindings: Vec<Binding>) -> Result<Self> {
        let mut seen = std::collections::BTreeSet::new();
        if bindings.len() > 100 {
            return Err(Error::new(422, "too_many_service_bindings"));
        }
        for b in &bindings {
            let u = crate::net::public_url(&b.endpoint)?;
            if u.query().is_some()
                || b.secret.len() < 24
                || b.secret.len() > 256
                || !b
                    .secret
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                || !omastore_catalogue::token(&b.app_id)
                || !omastore_catalogue::token(&b.seller_id)
                || !seen.insert((&b.app_id, &b.seller_id))
                || b.download_origins.len() > 8
            {
                return Err(Error::new(422, "invalid_service_binding"));
            }
            for origin in &b.download_origins {
                let u = crate::net::public_url(origin)?;
                if u.path() != "/" || u.query().is_some() {
                    return Err(Error::new(422, "invalid_download_origin"));
                }
            }
        }
        Ok(Self { bindings })
    }
    async fn call(
        &self,
        i: &CheckoutIntent,
        action: &str,
        key: &str,
        reference: Option<&str>,
        paid_until: Option<i64>,
    ) -> Result<Receipt> {
        let b = self
            .bindings
            .iter()
            .find(|b| b.app_id == i.price.app_id && b.seller_id == i.price.seller_id)
            .ok_or(Error::new(503, "service_fulfilment_unconfigured"))?;
        let u = crate::net::public_url(&format!("{}/{action}", b.endpoint.trim_end_matches('/')))?;
        let c = crate::net::client_for(&u).await?;
        let response=c.post(u.clone()).bearer_auth(&b.secret).header("Idempotency-Key",key).json(&json!({"orderId":i.order_id,"buyerReference":i.buyer_reference,"appId":i.price.app_id,"sellerId":i.price.seller_id,"priceVersion":i.price.version,"licence":i.price.licence,"reference":reference,"paidUntil":paid_until})).send().await.map_err(|_|Error::new(503,"fulfilment_needs_reconciliation"))?;
        let bytes = crate::net::read_response(response, 16 * 1024)
            .await
            .map_err(|_| Error::new(503, "fulfilment_needs_reconciliation"))?
            .bytes;
        let receipt: Receipt = serde_json::from_slice(&bytes)?;
        receipt.validate(i)?;
        for raw in receipt
            .recovery_url
            .iter()
            .chain(receipt.download.iter().map(|d| &d.url))
        {
            let origin = crate::net::public_url(raw)?.origin().ascii_serialization();
            if origin != u.origin().ascii_serialization()
                && !b
                    .download_origins
                    .iter()
                    .any(|v| v.trim_end_matches('/') == origin)
            {
                return Err(Error::new(409, "untrusted_delivery_origin"));
            }
        }
        Ok(receipt)
    }
}
#[async_trait]
impl Fulfilment for HttpFulfilment {
    async fn deliver(
        &self,
        i: &CheckoutIntent,
        key: &str,
        paid_until: Option<i64>,
    ) -> Result<Receipt> {
        self.call(i, "deliver", key, None, paid_until).await
    }
    async fn recover(&self, i: &CheckoutIntent, reference: &str) -> Result<Receipt> {
        self.call(
            i,
            "recover",
            &format!("recover-{}", nonce()?),
            Some(reference),
            None,
        )
        .await
    }
}
impl Store {
    pub async fn commerce_deliver(
        &self,
        id: &str,
        issuer: Option<&Issuer>,
        provider: &dyn Fulfilment,
        now: i64,
    ) -> Result<bool> {
        let local = {
            let c = self.connection()?;
            let (i, _) = intent(&c, id)?;
            !["service", "subscription"].contains(&i.price.delivery_kind.as_str())
        };
        if local {
            return self.commerce_deliver_local(id, issuer, now);
        }
        let Some(issuer) = issuer else {
            self.delivery_failure(id, "licence_issuer_unconfigured", now)?;
            return Ok(false);
        };
        let claim=self.transaction(|t|{let(i,mode)=intent(t,id)?;provider_guard(t,&mode)?;let(paid,state,until):(String,String,i64)=t.query_row("SELECT payment_state,delivery_state,delivery_until FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;if state=="delivered"{return Ok(None);}
if paid!="paid"{return Err(Error::new(409,"confirmed_payment_required"));}
if until>now{return Err(Error::new(409,"delivery_in_progress"));}let lease=nonce()?;t.execute("UPDATE commerce_orders SET delivery_state='delivery_pending',delivery_lease=?2,delivery_until=?3,delivery_attempts=delivery_attempts+1 WHERE id=?1",params![id,lease,now+60])?;event(t,id,"service_delivery_attempt",json!({}),now)?;Ok(Some((i,lease)))})?;
        let Some((i, lease)) = claim else {
            return Ok(false);
        };
        let paid_until = {
            let c = self.connection()?;
            let raw: String = c.query_row(
                "SELECT provider_observation FROM commerce_orders WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            serde_json::from_str::<crate::commerce_provider::CheckoutObservation>(&raw)?.paid_until
        };
        let receipt = match provider
            .deliver(&i, &format!("delivery-{id}"), paid_until)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                self.delivery_failure_lease(id, &lease, e.code, now)?;
                return Ok(false);
            }
        };
        if let Err(e) = receipt.validate(&i) {
            self.delivery_failure_lease(id, &lease, e.code, now)?;
            return Ok(false);
        }
        let finished=self.transaction(|t|{let(current,stored):(String,Option<String>)=t.query_row("SELECT delivery_state,delivery_lease FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;if current=="delivered"{return Ok(false);}
if stored.as_deref()!=Some(&lease){return Err(Error::new(409,"delivery_lease_lost"));}
   let obs:String=t.query_row("SELECT provider_observation FROM commerce_orders WHERE id=?1",[id],|r|r.get(0))?;let obs:crate::commerce_provider::CheckoutObservation=serde_json::from_str(&obs)?;
   if i.price.delivery_kind=="subscription"&&receipt.expires_at!=obs.paid_until{return Err(Error::new(409,"service_period_mismatch"));}
   let grant=Grant{version:1,order_id:id.into(),app_id:i.price.app_id.clone(),buyer_reference:i.buyer_reference.clone(),seller_id:i.price.seller_id.clone(),kind:i.price.delivery_kind.clone(),licence:i.price.licence.clone(),issued_at:now,expires_at:receipt.expires_at,recovery_url:receipt.recovery_url.clone()};let envelope=issuer.sign(&grant)?;
   t.execute("UPDATE commerce_orders SET delivery_state='delivered',delivery_error=NULL,delivery_lease=NULL,delivery_until=0,grant=?2,delivery_reference=?3,delivery_receipt=?4 WHERE id=?1",params![id,serde_json::to_string(&envelope)?,receipt.reference,serde_json::to_string(&receipt)?])?;event(t,id,"delivered",json!({"grantDigest":digest(serde_json::to_vec(&envelope)?),"fulfilmentReference":receipt.reference}),now)?;Ok(true)
  });
        if let Err(e) = &finished {
            self.delivery_failure_lease(id, &lease, e.code, now)?;
        }
        finished
    }
    fn delivery_failure(&self, id: &str, code: &str, now: i64) -> Result<()> {
        self.transaction(|t|{let(paid,state):(String,String)=t.query_row("SELECT payment_state,delivery_state FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;if paid!="paid"{return Err(Error::new(409,"confirmed_payment_required"));}
if state=="delivered"{return Ok(());}t.execute("UPDATE commerce_orders SET delivery_state='delivery_failed',delivery_error=?2,delivery_until=0,delivery_lease=NULL WHERE id=?1",params![id,code])?;event(t,id,"delivery_failed",json!({"code":code}),now)})
    }
    fn delivery_failure_lease(&self, id: &str, lease: &str, code: &str, now: i64) -> Result<()> {
        self.transaction(|t|{let n=t.execute("UPDATE commerce_orders SET delivery_state='delivery_failed',delivery_error=?3,delivery_until=0,delivery_lease=NULL WHERE id=?1 AND delivery_lease=?2 AND delivery_state!='delivered'",params![id,lease,code])?;if n>0{event(t,id,"delivery_failed",json!({"code":code}),now)?;}Ok(())})
    }
    pub async fn commerce_recover_download(
        &self,
        a: &Actor,
        id: &str,
        provider: &dyn Fulfilment,
        now: i64,
    ) -> Result<Value> {
        let (i, reference) = {
            let c = self.connection()?;
            access(&c, a, id)?;
            let (i, _) = intent(&c, id)?;
            let (state, reference): (String, Option<String>) = c.query_row(
                "SELECT delivery_state,delivery_reference FROM commerce_orders WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if state != "delivered" {
                return Err(Error::new(409, "delivery_not_completed"));
            }
            (i, reference.ok_or(Error::new(404, "no_hosted_delivery"))?)
        };
        let r = provider.recover(&i, &reference).await?;
        r.validate(&i)?;
        if r.reference != reference {
            return Err(Error::new(409, "fulfilment_identity_mismatch"));
        }
        self.transaction(|t| {
            access(t, a, id)?;
            event(t, id, "recovery_requested", json!({"actor":a.id}), now)
        })?;
        Ok(json!({"id":id,"download":r.download,"recoveryUrl":r.recovery_url}))
    }
}
#[cfg(feature = "development-workflow")]
pub struct SampleFulfilment(pub Store);
#[cfg(feature = "development-workflow")]
#[async_trait]
impl Fulfilment for SampleFulfilment {
    async fn deliver(
        &self,
        i: &CheckoutIntent,
        key: &str,
        _paid_until: Option<i64>,
    ) -> Result<Receipt> {
        self.0.transaction(|t| {
            provider_guard(t, "sample")?;
            use rusqlite::OptionalExtension;
            if let Some(raw) = t
                .query_row(
                    "SELECT body FROM commerce_sample_provider WHERE kind='delivery' AND key=?1",
                    [key],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                return Ok(serde_json::from_str(&raw)?);
            }
            let raw: String = t.query_row(
                "SELECT provider_observation FROM commerce_orders WHERE id=?1",
                [&i.order_id],
                |r| r.get(0),
            )?;
            let obs: crate::commerce_provider::CheckoutObservation = serde_json::from_str(&raw)?;
            let r = Receipt {
                order_id: i.order_id.clone(),
                buyer_reference: i.buyer_reference.clone(),
                app_id: i.price.app_id.clone(),
                seller_id: i.price.seller_id.clone(),
                reference: format!("sample-delivery-{}", i.order_id),
                expires_at: obs.paid_until,
                recovery_url: Some("https://example.com/fictional-service-recovery".into()),
                download: None,
            };
            t.execute(
                "INSERT INTO commerce_sample_provider VALUES('delivery',?1,?2)",
                params![key, serde_json::to_string(&r)?],
            )?;
            Ok(r)
        })
    }
    async fn recover(&self, i: &CheckoutIntent, _: &str) -> Result<Receipt> {
        self.deliver(i, &format!("delivery-{}", i.order_id), None)
            .await
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::{
        commerce_checkout::Purchase, commerce_provider::Provider, commerce_sample::Sample,
    };
    struct LostReply(SampleFulfilment);
    #[async_trait]
    impl Fulfilment for LostReply {
        async fn deliver(
            &self,
            i: &CheckoutIntent,
            key: &str,
            until: Option<i64>,
        ) -> Result<Receipt> {
            self.0.deliver(i, key, until).await?;
            Err(Error::new(503, "fulfilment_needs_reconciliation"))
        }
        async fn recover(&self, i: &CheckoutIntent, r: &str) -> Result<Receipt> {
            self.0.recover(i, r).await
        }
    }
    #[tokio::test]
    async fn hosted_delivery_retries_the_same_provider_effect_and_keeps_recovery_after_delisting() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("d.db")).unwrap();
        let c =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&c).unwrap();
        s.commerce_seed_sample(1000).unwrap();
        let v = s.development_login("author", 1000).unwrap();
        let a = s.actor(v["token"].as_str().unwrap(), 1000).unwrap();
        s.connection()
            .unwrap()
            .execute(
                "UPDATE metadata SET value='0' WHERE key='commerce_paused'",
                [],
            )
            .unwrap();
        let v = s.commerce_prices(None).unwrap();
        let p = v["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["price"]["id"] == "sample-subscription")
            .unwrap();
        let buy = Purchase {
            price_id: "sample-subscription".into(),
            version: 1,
            digest: p["digest"].as_str().unwrap().into(),
            accepted: true,
        };
        let id = s
            .commerce_begin(&a, &nonce().unwrap(), buy, "sample", 1001)
            .unwrap();
        let provider = Sample(s.clone());
        s.commerce_checkout(&a, &id, &provider, 1002).await.unwrap();
        provider.capture(&a, &id, 1003).unwrap();
        s.commerce_reconcile(&a, &id, &provider, 1004)
            .await
            .unwrap();
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        assert!(!s
            .commerce_deliver(
                &id,
                Some(&issuer),
                &LostReply(SampleFulfilment(s.clone())),
                1005
            )
            .await
            .unwrap());
        assert_eq!(
            s.commerce_order(&a, &id).unwrap()["deliveryState"],
            "delivery_failed"
        );
        let again = Store::development(&dir.path().join("d.db")).unwrap();
        let delivery = SampleFulfilment(again.clone());
        again.commerce_retry_delivery(&a, &id, 1006).unwrap();
        assert!(again
            .commerce_deliver(&id, Some(&issuer), &delivery, 1007)
            .await
            .unwrap());
        assert!(!again
            .commerce_deliver(&id, Some(&issuer), &delivery, 1008)
            .await
            .unwrap());
        let count: i64 = again
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM commerce_sample_provider WHERE kind='delivery'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        again
            .connection()
            .unwrap()
            .execute("UPDATE commerce_offers SET active=0", [])
            .unwrap();
        assert!(again
            .commerce_recover_download(&a, &id, &delivery, 1009)
            .await
            .unwrap()["recoveryUrl"]
            .is_string());
        assert!(again.commerce_bind_provider("stripe_test").is_err());
        assert_eq!(provider.mode(), "sample");
    }
}
