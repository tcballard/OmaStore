//! Each paid invoice is an immutable cycle. Cancellation changes future billing only.
use crate::{
    commerce_checkout::{access, event, intent, provider_guard},
    commerce_finance_provider::{Finance, Invoice, Subscription},
    commerce_model::provider_id,
    commerce_provider::CheckoutObservation,
    digest, nonce, Actor, Error, Result, Store,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
pub(crate) fn root(c: &Connection, id: &str) -> Result<String> {
    Ok(c.query_row(
        "SELECT root_order FROM commerce_cycles WHERE cycle_order=?1",
        [id],
        |r| r.get(0),
    )
    .optional()?
    .unwrap_or_else(|| id.to_owned()))
}
fn original(
    c: &Connection,
    id: &str,
) -> Result<(
    crate::commerce_provider::CheckoutIntent,
    String,
    CheckoutObservation,
)> {
    let (i, mode) = intent(c, id)?;
    provider_guard(c, &mode)?;
    let raw: Option<String> = c
        .query_row(
            "SELECT provider_observation FROM commerce_orders WHERE id=?1 AND payment_state='paid'",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    let o: CheckoutObservation =
        serde_json::from_str(&raw.ok_or(Error::new(409, "paid_subscription_required"))?)?;
    if i.price.delivery_kind != "subscription"
        || o.subscription_id.is_none()
        || o.customer_id.is_none()
    {
        return Err(Error::new(409, "paid_subscription_required"));
    }
    Ok((i, mode, o))
}
fn matches_subscription(
    i: &crate::commerce_provider::CheckoutIntent,
    o: &CheckoutObservation,
    s: &Subscription,
) -> Result<()> {
    if s.account != i.price.connected_account
        || Some(&s.id) != o.subscription_id.as_ref()
        || Some(&s.customer) != o.customer_id.as_ref()
        || s.root_order != i.order_id
        || ![
            "incomplete",
            "incomplete_expired",
            "trialing",
            "active",
            "past_due",
            "canceled",
            "unpaid",
            "paused",
        ]
        .contains(&s.state.as_str())
    {
        return Err(Error::new(409, "subscription_identity_mismatch"));
    }
    Ok(())
}
pub(crate) fn summary(c: &Connection, id: &str, now: i64) -> Result<Value> {
    let root = root(c, id)?;
    let (i, _, o) = original(c, &root)?;
    let latest:Option<(String,i64)>=c.query_row("SELECT cycle_order,period_end FROM commerce_cycles WHERE root_order=?1 ORDER BY period_end DESC LIMIT 1",[&root],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let paid_until = latest
        .as_ref()
        .map(|(_, e)| *e)
        .unwrap_or(o.paid_until.unwrap_or(0))
        .max(o.paid_until.unwrap_or(0));
    let mut q=c.prepare("SELECT cycle_order,period_start,period_end FROM commerce_cycles WHERE root_order=?1 ORDER BY period_end DESC LIMIT 100")?;
    let cycles:Vec<Value>=q.query_map([&root],|r|Ok(json!({"orderId":r.get::<_,String>(0)?,"periodStart":r.get::<_,i64>(1)?,"periodEnd":r.get::<_,i64>(2)?})))?.collect::<std::result::Result<_,_>>()?;
    let cancellation:Option<Value>=c.query_row("SELECT state,error,observation FROM commerce_cancellations WHERE order_id=?1",[&root],|r|Ok(json!({"state":r.get::<_,String>(0)?,"error":r.get::<_,Option<String>>(1)?,"observation":r.get::<_,Option<String>>(2)?.and_then(|s|serde_json::from_str::<Value>(&s).ok())}))).optional()?;
    let error: Option<String> = c
        .query_row(
            "SELECT error FROM commerce_subscription_checks WHERE order_id=?1",
            [&root],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    let observed: Option<(String, i64)> = c
        .query_row(
            "SELECT body,observed_at FROM commerce_subscription_observations WHERE order_id=?1",
            [&root],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let current = observed
        .as_ref()
        .map(|(s, _)| serde_json::from_str::<Value>(s))
        .transpose()?;
    Ok(
        json!({"providerStatus":current,"providerObservedAt":observed.map(|(_,at)|at),"rootOrderId":root,"latestOrderId":latest.map(|(id,_)|id).unwrap_or(root),"paidUntil":paid_until,"paidPeriodElapsed":paid_until<=now,"cycles":cycles,"cancellation":cancellation,"reconciliationError":error,"termsUrl":i.price.terms_url,"notice":"Paid periods are provider-confirmed records, not a guarantee of current hosted access. Cancellation stops future billing; receipts, licences and local documents remain."}),
    )
}
impl Store {
    pub fn commerce_subscription(&self, a: &Actor, id: &str, now: i64) -> Result<Value> {
        let c = self.connection()?;
        access(&c, a, id)?;
        summary(&c, id, now)
    }
    pub async fn commerce_cancel_subscription(
        &self,
        a: &Actor,
        id: &str,
        accepted: bool,
        p: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        if !accepted {
            return Err(Error::new(422, "cancellation_consent_required"));
        }
        let root=self.transaction(|t|{access(t,a,id)?;let root=root(t,id)?;original(t,&root)?;provider_guard(t,p.mode())?;
            t.execute("UPDATE commerce_cancellations SET state='requested' WHERE order_id=?1 AND state='confirmed'",[&root])?;
            let added=t.execute("INSERT OR IGNORE INTO commerce_cancellations(order_id,actor,state,at) VALUES(?1,?2,'requested',?3)",params![root,a.id,now])?;
            if added>0{event(t,&root,"cancellation_requested",json!({"actor":a.id,"futureBillingOnly":true}),now)?;}Ok(root)})?;
        self.commerce_finish_cancellation(&root, p, now).await?;
        self.commerce_subscription(a, &root, now)
    }
    pub async fn commerce_finish_cancellation(
        &self,
        id: &str,
        p: &dyn Finance,
        now: i64,
    ) -> Result<()> {
        let claim=self.transaction(|t|{let(i,m,o)=original(t,id)?;if m!=p.mode(){return Err(Error::new(503,"payment_provider_mode_mismatch"));}
            let state:Option<(String,i64)>=t.query_row("SELECT state,lease_until FROM commerce_cancellations WHERE order_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let Some((state,lease))=state else{return Ok(None)};if state=="confirmed"{return Ok(None);}
if lease>now{return Err(Error::new(409,"cancellation_in_progress"));}
            t.execute("UPDATE commerce_cancellations SET lease_until=?2,state='pending' WHERE order_id=?1",params![id,now+60])?;Ok(Some((i,o)))})?;
        let Some((i, o)) = claim else { return Ok(()) };
        let result = async {
            let sid = o.subscription_id.as_deref().unwrap();
            let current = p.subscription(&i.price.connected_account, sid).await?;
            matches_subscription(&i, &o, &current)?;
            let current = if current.cancel_at_period_end || current.state == "canceled" {
                current
            } else {
                p.cancel_subscription(&i.price.connected_account, sid, &format!("cancel-{id}"))
                    .await?;
                p.subscription(&i.price.connected_account, sid).await?
            };
            matches_subscription(&i, &o, &current)?;
            if !current.cancel_at_period_end && current.state != "canceled" {
                return Err(Error::new(409, "subscription_cancel_unconfirmed"));
            }
            Ok(current)
        }
        .await;
        self.transaction(|t|match &result{
            Ok(s)=>{t.execute("UPDATE commerce_cancellations SET state='confirmed',lease_until=0,error=NULL,observation=?2 WHERE order_id=?1",params![id,serde_json::to_string(s)?])?;event(t,id,"future_billing_cancellation_confirmed",json!({"providerStatus":s.state}),now)},
            Err(e)=>{t.execute("UPDATE commerce_cancellations SET lease_until=0,error=?2 WHERE order_id=?1",params![id,e.code])?;event(t,id,"cancellation_needs_reconciliation",json!({"code":e.code}),now)}
        })?;
        result.map(|_| ())
    }
    pub fn commerce_observe_invoice(&self, mode: &str, v: &Invoice, now: i64) -> Result<String> {
        self.transaction(|t|{provider_guard(t,mode)?;let(i,stored,o)=original(t,&v.root_order)?;
            let amounts=i.price.amounts()?;
            if stored!=mode || !v.paid || v.account!=i.price.connected_account || Some(&v.subscription)!=o.subscription_id.as_ref() || Some(&v.customer)!=o.customer_id.as_ref() || v.currency!=i.price.currency || json!(v.total)!=amounts["total"] || v.tax!=i.price.tax || json!(v.application_fee)!=amounts["serviceFee"] || !provider_id(&v.id,"in_") || !provider_id(&v.charge,"ch_") || !provider_id(&v.fee,"fee_") || v.period_start<=0 || v.period_end<=v.period_start || v.period_end-v.period_start>400*86400 {return Err(Error::new(409,"renewal_invoice_mismatch"));}
            let sha=digest(serde_json::to_vec(v)?);
            if let Some((cycle,old))=t.query_row("SELECT cycle_order,observation_digest FROM commerce_cycles WHERE invoice_id=?1",[&v.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?{if old!=sha{return Err(Error::new(409,"renewal_invoice_changed"));}return Ok(cycle);}
            // The initial paid invoice describes the checkout charge, not another sale.
            if Some(&v.charge)==o.payment_id.as_ref(){if Some(v.period_end)!=o.paid_until{return Err(Error::new(409,"initial_invoice_period_mismatch"));}return Ok(i.order_id);}
            if v.period_start<o.paid_until.unwrap_or(i.created_at){return Err(Error::new(409,"renewal_period_overlap"));}
            let overlaps:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM commerce_cycles WHERE root_order=?1 AND period_start<?2 AND period_end>?3)",params![i.order_id,v.period_end,v.period_start],|r|r.get(0))?;
            if overlaps{return Err(Error::new(409,"renewal_period_overlap"));}
            let cycle=nonce()?;
            let mut obs=o.clone();obs.session_id=format!("cs_test_invoice_{}",v.id);obs.order_id=cycle.clone();obs.payment_id=Some(v.charge.clone());obs.fee_id=Some(v.fee.clone());obs.paid_until=Some(v.period_end);obs.url=None;
            t.execute("INSERT INTO commerce_orders(id,buyer,buyer_reference,request_key,request_digest,mode,price,price_digest,created_at,payment_state,creation_state,payment_id,fee_id,subscription_id,customer_id,delivery_state,provider_observation) SELECT ?2,buyer,buyer_reference,?3,?4,mode,price,price_digest,?5,'paid','observed',?6,?7,subscription_id,customer_id,'delivery_pending',?8 FROM commerce_orders WHERE id=?1",params![i.order_id,cycle,format!("invoice-{}",v.id),sha,now,v.charge,v.fee,serde_json::to_string(&obs)?])?;
            t.execute("INSERT INTO commerce_cycles VALUES(?1,?2,?3,?4,?5,?6)",params![v.id,i.order_id,cycle,v.period_start,v.period_end,sha])?;
            crate::commerce_accounting::sale(t,&cycle,now)?;
            event(t,&cycle,"paid_renewal_confirmed",json!({"rootOrderId":i.order_id,"periodStart":v.period_start,"periodEnd":v.period_end}),now)?;
            event(t,&i.order_id,"renewal_receipt_created",json!({"orderId":cycle}),now)?;Ok(cycle)
        })
    }
    pub async fn commerce_reconcile_invoice(
        &self,
        p: &dyn Finance,
        account: &str,
        id: &str,
        now: i64,
    ) -> Result<String> {
        let result = async {
            let v = p.invoice(account, id).await?;
            if v.account != account || v.id != id {
                return Err(Error::new(409, "renewal_invoice_mismatch"));
            }
            self.commerce_observe_invoice(p.mode(), &v, now)
        }
        .await;
        self.transaction(|t|{provider_guard(t,p.mode())?;match &result{
            Ok(_)=>{t.execute("UPDATE commerce_invoice_issues SET resolved_at=?3 WHERE account=?1 AND invoice_id=?2",params![account,id,now])?;},
            Err(e)=>{let root:Option<String>=t.query_row("SELECT root_order FROM commerce_cycles WHERE invoice_id=?1",[id],|r|r.get(0)).optional()?;
                // No raw provider body or customer contact data enters a support report.
                t.execute("INSERT INTO commerce_invoice_issues VALUES(?1,?2,?3,?4,?5,NULL) ON CONFLICT(account,invoice_id) DO UPDATE SET error=excluded.error,observed_at=excluded.observed_at,resolved_at=NULL",params![account,id,root,e.code,now])?;}
        }Ok(())})?;
        result
    }
    pub async fn commerce_poll_subscription(
        &self,
        id: &str,
        p: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        let (i, m, o, after) = {
            let c = self.connection()?;
            let (i, m, o) = original(&c, id)?;
            let after: Option<String> = c
                .query_row(
                    "SELECT cursor FROM commerce_subscription_checks WHERE order_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            (i, m, o, after)
        };
        if m != p.mode() {
            return Err(Error::new(503, "payment_provider_mode_mismatch"));
        }
        let result: Result<Option<String>> = async {
            let observed=p.subscription(&i.price.connected_account,o.subscription_id.as_deref().unwrap()).await?;
            matches_subscription(&i,&o,&observed)?;
            self.connection()?.execute("INSERT INTO commerce_subscription_observations VALUES(?1,?2,?3) ON CONFLICT(order_id) DO UPDATE SET body=excluded.body,observed_at=excluded.observed_at",params![id,serde_json::to_string(&observed)?,now])?;
            let page = p
                .invoices(
                    &i.price.connected_account,
                    o.subscription_id.as_deref().unwrap(),
                    after.as_deref(),
                )
                .await?;
            let ids = page["ids"]
                .as_array()
                .filter(|a| a.len() <= 100)
                .ok_or(Error::new(503, "invalid_invoice_page"))?;
            for id in ids {
                let id = id
                    .as_str()
                    .filter(|s| provider_id(s, "in_"))
                    .ok_or(Error::new(503, "invalid_invoice_page"))?;
                let _ = self
                    .commerce_reconcile_invoice(p, &i.price.connected_account, id, now)
                    .await;
            }
            let more = page["hasMore"]
                .as_bool()
                .ok_or(Error::new(503, "invalid_invoice_page"))?;
            let cursor = if more {
                Some(
                    page["nextCursor"]
                        .as_str()
                        .filter(|s| provider_id(s, "in_"))
                        .ok_or(Error::new(503, "invalid_invoice_page"))?
                        .to_owned(),
                )
            } else {
                None
            };
            Ok(cursor)
        }
        .await;
        self.transaction(|t|{match &result{Ok(cursor)=>{t.execute("INSERT INTO commerce_subscription_checks VALUES(?1,?2,?3,NULL) ON CONFLICT(order_id) DO UPDATE SET checked_at=excluded.checked_at,cursor=excluded.cursor,error=NULL",params![id,now,cursor])?;},Err(e)=>{t.execute("INSERT INTO commerce_subscription_checks VALUES(?1,?2,NULL,?3) ON CONFLICT(order_id) DO UPDATE SET checked_at=excluded.checked_at,error=excluded.error",params![id,now,e.code])?;}}Ok(())})?;
        result?;
        let c = self.connection()?;
        summary(&c, id, now)
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::{
        commerce_checkout::Purchase,
        commerce_license::{verify, Envelope},
        commerce_provider::Provider,
        commerce_sample::Sample,
    };
    async fn setup() -> (tempfile::TempDir, Store, Actor, Actor, String) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("cycles.db")).unwrap();
        let c =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&c).unwrap();
        s.commerce_seed_sample(1000).unwrap();
        let a = s.development_login("author", 1000).unwrap();
        let a = s.actor(a["token"].as_str().unwrap(), 1000).unwrap();
        let op = s.development_login("operator", 1000).unwrap();
        let op = s.actor(op["token"].as_str().unwrap(), 1000).unwrap();
        s.transaction(|t| crate::commerce::pause(t, &op, false, "Fixture", 1000))
            .unwrap();
        let prices = s.commerce_prices(None).unwrap();
        let offer = prices["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["price"]["id"] == "sample-subscription")
            .unwrap();
        let id = s
            .commerce_begin(
                &a,
                &nonce().unwrap(),
                Purchase {
                    price_id: "sample-subscription".into(),
                    version: 1,
                    digest: offer["digest"].as_str().unwrap().into(),
                    accepted: true,
                },
                "sample",
                1000,
            )
            .unwrap();
        let p = Sample(s.clone());
        s.commerce_checkout(&a, &id, &p, 1001).await.unwrap();
        p.capture(&a, &id, 1002).unwrap();
        s.commerce_reconcile(&a, &id, &p, 1003).await.unwrap();
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        s.commerce_deliver(
            &id,
            Some(&issuer),
            &crate::commerce_delivery::SampleFulfilment(s.clone()),
            1004,
        )
        .await
        .unwrap();
        (dir, s, a, op, id)
    }
    #[tokio::test]
    async fn missing_callbacks_create_one_receipt_per_invoice_and_cancellation_preserves_history() {
        let (_dir, s, a, op, id) = setup().await;
        let p = Sample(s.clone());
        let first = s.commerce_order(&a, &id).unwrap()["grant"].clone();
        let scene = p.finance_scenario(&op, &id, "renewal", 1005).unwrap();
        let iid = scene["invoiceId"].as_str().unwrap();
        // No webhook arrives. Scheduled reconciliation finds the provider invoice.
        s.commerce_lifecycle_tick(&p, 2000).await.unwrap();
        let status = s.commerce_subscription(&a, &id, 2001).unwrap();
        let cycle = status["latestOrderId"].as_str().unwrap();
        assert_ne!(cycle, id);
        let inv = p.invoice("acct_samplemaker", iid).await.unwrap();
        assert_eq!(
            s.commerce_observe_invoice("sample", &inv, 2002).unwrap(),
            cycle
        );
        let mut tampered = inv.clone();
        tampered.application_fee += 1;
        assert!(s
            .commerce_observe_invoice("sample", &tampered, 2003)
            .is_err());
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        s.commerce_deliver(
            cycle,
            Some(&issuer),
            &crate::commerce_delivery::SampleFulfilment(s.clone()),
            inv.period_start + 1,
        )
        .await
        .unwrap();
        let receipt = s.commerce_order(&a, cycle).unwrap();
        let envelope: Envelope = serde_json::from_value(receipt["grant"].clone()).unwrap();
        let key = issuer.public_key();
        let anchors = [(issuer.key_id(), key.as_slice())];
        let grant = verify(&envelope, &anchors, inv.period_start + 2).unwrap();
        assert_eq!(grant.expires_at, Some(inv.period_end));
        assert!(verify(&envelope, &anchors, inv.period_end).is_err());
        assert_eq!(s.commerce_order(&a, &id).unwrap()["grant"], first);
        assert_eq!(
            s.commerce_subscription(&a, &id, 2004).unwrap()["cycles"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(!s
            .commerce_cancel_subscription(&a, &id, false, &p, 2005)
            .await
            .is_ok());
        let canceled = s
            .commerce_cancel_subscription(&a, cycle, true, &p, 2006)
            .await
            .unwrap();
        assert_eq!(canceled["cancellation"]["state"], "confirmed");
        s.commerce_cancel_subscription(&a, &id, true, &p, 2007)
            .await
            .unwrap();
        assert!(p.finance_scenario(&op, &id, "renewal", 2008).is_err());
        assert_eq!(s.commerce_order(&a, &id).unwrap()["grant"], first);
        assert_eq!(
            s.connection()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM commerce_sample_provider WHERE kind='cancellation'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        // Reconciliation is an existing obligation even when the seller account is deactivated.
        s.connection()
            .unwrap()
            .execute("UPDATE users SET active=0 WHERE id=?1", [&a.id])
            .unwrap();
        s.commerce_lifecycle_tick(&p, 2600).await.unwrap();
        assert_eq!(
            s.commerce_finances(&op, "sample-maker").unwrap()["providerObservedAt"],
            2600
        );
    }
    #[tokio::test]
    async fn lost_cancellation_reply_and_fully_refunded_undelivered_orders_remain_recoverable_records(
    ) {
        let (_dir, s, a, op, id) = setup().await;
        let p = Sample(s.clone());
        let (_, _, o) = original(&s.connection().unwrap(), &id).unwrap();
        s.connection().unwrap().execute("INSERT INTO commerce_cancellations(order_id,actor,state,at) VALUES(?1,?2,'pending',1005)",params![id,a.id]).unwrap();
        p.cancel_subscription(
            "acct_samplemaker",
            o.subscription_id.as_deref().unwrap(),
            &format!("cancel-{id}"),
        )
        .await
        .unwrap();
        s.commerce_finish_cancellation(&id, &p, 1000 + 24 * 3600)
            .await
            .unwrap();
        assert_eq!(
            s.commerce_subscription(&a, &id, 1006).unwrap()["cancellation"]["state"],
            "confirmed"
        );
        let r = s
            .commerce_request_refund(
                &a,
                &nonce().unwrap(),
                crate::commerce_refunds::Request {
                    order_id: id.clone(),
                    amount: 1000,
                    expected_refunded: 0,
                    reason: "Fixture full service refund".into(),
                    accepted: true,
                },
                1007,
            )
            .unwrap();
        s.commerce_execute_refund(&op, r["id"].as_str().unwrap(), &p, 1008)
            .await
            .unwrap();
        assert!(s.commerce_order(&a, &id).unwrap()["grant"].is_object());
        assert!(s
            .commerce_recover_download(
                &a,
                &id,
                &crate::commerce_delivery::SampleFulfilment(s.clone()),
                1009
            )
            .await
            .is_err());
        assert!(s.commerce_retry_delivery(&a, &id, 1009).is_err());
        let outsider = s.development_login("reviewer", 1010).unwrap();
        let outsider = s.actor(outsider["token"].as_str().unwrap(), 1010).unwrap();
        assert!(s.commerce_subscription(&outsider, &id, 1010).is_err());
    }
    #[tokio::test]
    async fn financial_webhook_signatures_and_replays_only_observe_current_provider_effects() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        fn event_signature(v: Value) -> (Vec<u8>, String) {
            let bytes = serde_json::to_vec(&v).unwrap();
            let mut h =
                Hmac::<Sha256>::new_from_slice(b"sample-financial-webhook-contract-key").unwrap();
            h.update(b"2000.");
            h.update(&bytes);
            let signature = h
                .finalize()
                .into_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            (bytes, format!("t=2000,v1={signature}"))
        }
        let (_dir, s, a, op, id) = setup().await;
        let p = Sample(s.clone());
        let v = p.finance_scenario(&op, &id, "renewal", 1005).unwrap();
        let ev = json!({"id":"evt_invoicefixture","type":"invoice.paid","api_version":crate::commerce_model::STRIPE_VERSION,"account":"acct_samplemaker","livemode":false,"data":{"object":{"id":v["invoiceId"]}}});
        let (body, signature) = event_signature(ev);
        assert!(s
            .commerce_all_webhooks(
                &p,
                "sample-financial-webhook-contract-key",
                &signature,
                b"{}",
                2000
            )
            .await
            .is_err());
        assert_eq!(
            s.commerce_all_webhooks(
                &p,
                "sample-financial-webhook-contract-key",
                &signature,
                &body,
                2000
            )
            .await
            .unwrap()["replayed"],
            false
        );
        assert_eq!(
            s.commerce_all_webhooks(
                &p,
                "sample-financial-webhook-contract-key",
                &signature,
                &body,
                2000
            )
            .await
            .unwrap()["replayed"],
            true
        );
        assert_eq!(
            s.commerce_subscription(&a, &id, 2000).unwrap()["cycles"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let d = p.finance_scenario(&op, &id, "dispute_open", 2000).unwrap();
        let event = json!({"id":"evt_disputefixture","type":"charge.dispute.created","api_version":crate::commerce_model::STRIPE_VERSION,"account":"acct_samplemaker","livemode":false,"data":{"object":{"id":d["disputeId"],"status":"won","amount":1}}});
        let (body, signature) = event_signature(event);
        s.commerce_all_webhooks(
            &p,
            "sample-financial-webhook-contract-key",
            &signature,
            &body,
            2000,
        )
        .await
        .unwrap();
        let report = s.commerce_finances(&op, "sample-maker").unwrap();
        assert_eq!(report["disputes"][0]["state"], "needs_response");
        assert_eq!(report["disputes"][0]["amount"], 1000);
        // Provider state, not the misleading event body, determines the financial entry.
        let movements: Vec<_> = report["ledger"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "provider_dispute_movement")
            .collect();
        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0]["amount"], -1015);
    }
}
