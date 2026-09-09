//! Persistent fictional provider, compiled only for isolated development workspaces.
use crate::{
    commerce_checkout::{event, provider_guard},
    commerce_model::Price,
    commerce_provider::{CheckoutIntent, CheckoutObservation, Provider, RefundObservation},
    nonce, Actor, Error, Result, Store,
};
use async_trait::async_trait;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
#[derive(Clone)]
pub struct Sample(pub Store);
impl Sample {
    fn guard(&self) -> Result<()> {
        provider_guard(&*self.0.connection()?, "sample")
    }
    pub fn capture(&self, a: &Actor, order: &str, now: i64) -> Result<()> {
        self.guard()?;
        self.0.transaction(|t| {
            crate::commerce_checkout::access(t, a, order)?;
            let key = format!("checkout-{order}");
            let raw: String = t.query_row(
                "SELECT body FROM commerce_sample_provider WHERE kind='checkout' AND key=?1",
                [&key],
                |r| r.get(0),
            )?;
            let mut v: Value = serde_json::from_str(&raw)?;
            let mut o: CheckoutObservation = serde_json::from_value(v["observation"].clone())?;
            if o.paid {
                return Ok(());
            }
            let i: CheckoutIntent = serde_json::from_value(v["intent"].clone())?;
            o.paid = true;
            o.payment_id = Some(format!("ch_{order}"));
            o.fee_amount = i.price.amounts()?["serviceFee"].as_u64();
            o.fee_id = Some(format!("fee_{order}"));
            o.customer_id = Some(format!("cus_{order}"));
            if i.price.delivery_kind == "subscription" {
                o.subscription_id = Some(format!("sub_{order}"));
                o.paid_until = Some(now + 30 * 86400);
            }
            v["observation"] = json!(o);
            t.execute(
                "UPDATE commerce_sample_provider SET body=?2 WHERE kind='checkout' AND key=?1",
                params![key, v.to_string()],
            )?;
            event(
                t,
                order,
                "sample_payment_captured",
                json!({"fictional":true}),
                now,
            )
        })
    }
}
#[async_trait]
impl Provider for Sample {
    fn mode(&self) -> &'static str {
        "sample"
    }
    async fn checkout(&self, i: &CheckoutIntent, key: &str) -> Result<CheckoutObservation> {
        self.guard()?;
        self.0.transaction(|t| {
            if let Some(raw) = t
                .query_row(
                    "SELECT body FROM commerce_sample_provider WHERE kind='checkout' AND key=?1",
                    [key],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                let v: Value = serde_json::from_str(&raw)?;
                if v["intent"] != json!(i) {
                    return Err(Error::new(409, "provider_key_conflict"));
                }
                return Ok(serde_json::from_value(v["observation"].clone())?);
            }
            let o = CheckoutObservation {
                session_id: format!("cs_test_{}", nonce()?),
                account: i.price.connected_account.clone(),
                order_id: i.order_id.clone(),
                buyer_reference: i.buyer_reference.clone(),
                currency: i.price.currency.clone(),
                total: i.price.amounts()?["total"].as_u64().unwrap(),
                tax: i.price.tax,
                paid: false,
                payment_id: None,
                fee_amount: None,
                fee_id: None,
                subscription_id: None,
                customer_id: None,
                paid_until: None,
                url: None,
                livemode: false,
            };
            t.execute(
                "INSERT INTO commerce_sample_provider VALUES('checkout',?1,?2)",
                params![key, json!({"intent":i,"observation":o}).to_string()],
            )?;
            Ok(o)
        })
    }
    async fn lookup(&self, account: &str, session: &str) -> Result<CheckoutObservation> {
        self.guard()?;
        let c = self.0.connection()?;
        let mut q = c.prepare("SELECT body FROM commerce_sample_provider WHERE kind='checkout'")?;
        for row in q.query_map([], |r| r.get::<_, String>(0))? {
            let v: Value = serde_json::from_str(&row?)?;
            let o: CheckoutObservation = serde_json::from_value(v["observation"].clone())?;
            if o.account == account && o.session_id == session {
                return Ok(o);
            }
        }
        Err(Error::new(404, "provider_checkout_unavailable"))
    }
    async fn reconcile_creation(&self, i: &CheckoutIntent) -> Result<Option<CheckoutObservation>> {
        self.guard()?;
        let c = self.0.connection()?;
        let raw: Option<String> = c
            .query_row(
                "SELECT body FROM commerce_sample_provider WHERE kind='checkout' AND key=?1",
                [format!("checkout-{}", i.order_id)],
                |r| r.get(0),
            )
            .optional()?;
        raw.map(|r| {
            let v: Value = serde_json::from_str(&r)?;
            serde_json::from_value(v["observation"].clone()).map_err(Into::into)
        })
        .transpose()
    }
    async fn refund(&self, _: &str, _: &str, _: u64, _: &str) -> Result<RefundObservation> {
        Err(Error::new(503, "refund_adapter_unconfigured"))
    }
    async fn refund_status(&self, _: &str, _: &str) -> Result<RefundObservation> {
        Err(Error::new(503, "refund_adapter_unconfigured"))
    }
    async fn cancel_subscription(&self, _: &str, _: &str, _: &str) -> Result<Value> {
        Err(Error::new(503, "cancellation_adapter_unconfigured"))
    }
}
impl Store {
    pub fn commerce_seed_sample(&self, now: i64) -> Result<()> {
        if !self.is_development()? {
            return Err(Error::new(403, "sample_mode_required"));
        }
        self.commerce_bind_provider("sample")?;
        self.development_login("author", now)?;
        self.transaction(|t|{
   provider_guard(t,"sample")?;
   t.execute("INSERT OR IGNORE INTO commerce_sellers VALUES('sample-maker','development:author','acct_samplemaker','Fictional maker',1,'https://example.com/sample-commercial-evidence',?1)",["a".repeat(64)])?;
   for (name,kind,subtotal,tax)in [("perpetual","perpetual",999,100),("subscription","subscription",1000,0),("service","service",1500,0)]{
    let p=Price{id:format!("sample-{name}"),version:1,app_id:"demo-fieldnotes".into(),seller_id:"sample-maker".into(),seller_name:"Fictional maker".into(),connected_account:"acct_samplemaker".into(),title:format!("Fictional Fieldnotes {name}"),currency:"gbp".into(),subtotal,discount:0,tax,tax_rate_id:if tax>0{Some("txr_sample".into())}else{None},delivery_kind:kind.into(),billing_interval:if kind=="subscription"{Some("month".into())}else{None},licence:if kind=="perpetual"{"Fictional personal perpetual licence; open-source rights are independent".into()}else{"Fictional hosted service access".into()},online_requirement:if kind=="perpetual"{"Offline verification supported".into()}else{"Internet and the fictional provider are required".into()},support_url:"https://example.com/sample-support".into(),terms_url:"https://example.com/sample-terms".into()};let sha=crate::digest(serde_json::to_vec(&p.public()?)?);
    t.execute("INSERT OR IGNORE INTO commerce_prices VALUES(?1,1,?2,?3,'development:author',?4)",params![p.id,serde_json::to_string(&p)?,sha,now])?;t.execute("INSERT OR IGNORE INTO commerce_offers VALUES(?1,1,1)",[&p.id])?;
   }Ok(())
  })
    }
}
