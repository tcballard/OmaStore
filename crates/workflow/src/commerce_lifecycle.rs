//! Narrow internal reconciliation worker. It never starts sales or bank payouts.
use crate::{
    commerce_checkout::{access, event, intent, provider_guard, signed_in},
    commerce_finance_provider::Finance,
    store::recheck,
    Actor, Error, Result, Store,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
pub(crate) fn delivery_allowed(c: &Connection, id: &str, now: i64) -> Result<()> {
    let (i, _) = intent(c, id)?;
    let refunded:i64=c.query_row("SELECT COALESCE(SUM(amount),0) FROM commerce_refunds WHERE order_id=?1 AND state='succeeded'",[id],|r|r.get(0))?;
    if refunded as u64 >= i.price.amounts()?["total"].as_u64().unwrap() {
        return Err(Error::new(409, "refunded_order_requires_support"));
    }
    if i.price.delivery_kind == "subscription" {
        let start: Option<i64> = c
            .query_row(
                "SELECT period_start FROM commerce_cycles WHERE cycle_order=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        if start.is_some_and(|n| n > now) {
            return Err(Error::new(409, "subscription_period_not_started"));
        }

        let end:Option<i64>=c.query_row("SELECT json_extract(provider_observation,'$.paidUntil') FROM commerce_orders WHERE id=?1",[id],|r|r.get(0))?;
        if end.is_none_or(|n| n <= now) {
            return Err(Error::new(409, "subscription_period_elapsed"));
        }
    }
    Ok(())
}
impl Store {
    pub fn commerce_sellers(&self, a: &Actor) -> Result<Value> {
        let c = self.connection()?;
        signed_in(&c, a)?;
        let op = recheck(&c, a, "operator").is_ok();
        let mut q=c.prepare("SELECT id,name,active FROM commerce_sellers WHERE owner=?1 OR ?2=1 ORDER BY id LIMIT 100")?;
        let rows:Vec<Value>=q.query_map(params![a.id,op],|r|Ok(json!({"id":r.get::<_,String>(0)?,"name":r.get::<_,String>(1)?,"active":r.get::<_,bool>(2)?})))?.collect::<std::result::Result<_,_>>()?;
        Ok(json!({"items":rows}))
    }
    pub async fn commerce_refresh_subscription(
        &self,
        a: &Actor,
        id: &str,
        p: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        let root = {
            let c = self.connection()?;
            access(&c, a, id)?;
            crate::commerce_subscriptions::root(&c, id)?
        };
        self.commerce_poll_subscription(&root, p, now).await
    }
    pub fn commerce_invoice_issues(&self, a: &Actor, seller: &str) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, a, "operator")?;
        let account: String = c
            .query_row(
                "SELECT account FROM commerce_sellers WHERE id=?1",
                [seller],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::new(404, "seller_unavailable"))?;
        let mut q=c.prepare("SELECT invoice_id,root_order,error,observed_at FROM commerce_invoice_issues WHERE account=?1 AND resolved_at IS NULL ORDER BY observed_at DESC LIMIT 100")?;
        let rows:Vec<Value>=q.query_map([account],|r|Ok(json!({"invoiceId":r.get::<_,String>(0)?,"rootOrderId":r.get::<_,Option<String>>(1)?,"error":r.get::<_,String>(2)?,"observedAt":r.get::<_,i64>(3)?})))?.collect::<std::result::Result<_,_>>()?;
        Ok(json!({"items":rows}))
    }
    pub async fn commerce_retry_invoice(
        &self,
        a: &Actor,
        seller: &str,
        id: &str,
        p: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        let account = {
            let c = self.connection()?;
            recheck(&c, a, "operator")?;
            c.query_row(
                "SELECT account FROM commerce_sellers WHERE id=?1",
                [seller],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .ok_or(Error::new(404, "seller_unavailable"))?
        };
        let order = self
            .commerce_reconcile_invoice(p, &account, id, now)
            .await?;
        self.commerce_order(a, &order)
    }
    fn lifecycle_queue(&self, kind: &str, now: i64) -> Result<Vec<String>> {
        let sql=match kind{
            "refund"=>"SELECT id FROM commerce_refunds WHERE state='pending' OR (state='succeeded' AND fee_state!='succeeded')",
            "cancellation"=>"SELECT order_id AS id FROM commerce_cancellations WHERE state!='confirmed'",
            "subscription"=>"SELECT id FROM commerce_orders WHERE payment_state='paid' AND subscription_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM commerce_cycles WHERE cycle_order=commerce_orders.id)",
            "seller"=>"SELECT id FROM commerce_sellers",
            "dispute"=>"SELECT id FROM commerce_disputes WHERE json_extract(body,'$.state') IN ('needs_response','under_review','warning_needs_response','warning_under_review')",
            _=>return Err(Error::new(422,"unknown_lifecycle_job")),
        };
        self.transaction(|t|{
            let sql=format!("SELECT job.id FROM ({sql}) job LEFT JOIN commerce_lifecycle_ticks ticks ON ticks.kind=?1 AND ticks.id=job.id WHERE COALESCE(ticks.at,0)<=?2 ORDER BY COALESCE(ticks.at,0),job.id LIMIT 10");
            let mut q=t.prepare(&sql)?;let ids:Vec<String>=q.query_map(params![kind,now-300],|r|r.get(0))?.collect::<std::result::Result<_,_>>()?;
            for id in &ids{t.execute("INSERT INTO commerce_lifecycle_ticks VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET at=excluded.at",params![kind,id,now])?;}Ok(ids)
        })
    }
    pub async fn commerce_lifecycle_tick(&self, p: &dyn Finance, now: i64) -> Result<()> {
        {
            let c = self.connection()?;
            provider_guard(&c, p.mode())?;
        }
        for id in self.lifecycle_queue("refund", now)? {
            let _ = self.commerce_poll_refund(&id, p, now).await;
        }
        for id in self.lifecycle_queue("cancellation", now)? {
            let _ = self.commerce_finish_cancellation(&id, p, now).await;
        }
        for id in self.lifecycle_queue("subscription", now)? {
            let _ = self.commerce_poll_subscription(&id, p, now).await;
        }
        for id in self.lifecycle_queue("seller", now)? {
            let cursor = {
                let c = self.connection()?;
                c.query_row("SELECT CASE WHEN json_extract(body,'$.hasMorePayouts')=1 THEN json_extract(body,'$.nextPayoutCursor') ELSE NULL END FROM commerce_provider_reports WHERE seller_id=?1",[&id],|r|r.get::<_,Option<String>>(0)).optional()?.flatten()
            };
            let _ = self.reconcile_seller(&id, p, cursor.as_deref(), now).await;
        }
        for id in self.lifecycle_queue("dispute", now)? {
            let account = {
                let c = self.connection()?;
                c.query_row(
                    "SELECT json_extract(body,'$.account') FROM commerce_disputes WHERE id=?1",
                    [&id],
                    |r| r.get::<_, String>(0),
                )?
            };
            if let Ok(d) = p.dispute(&account, &id).await {
                let _ = self.commerce_record_dispute(p.mode(), &d, now);
            }
        }
        Ok(())
    }
    pub fn commerce_record_support(
        &self,
        a: &Actor,
        id: &str,
        note: &str,
        now: i64,
    ) -> Result<Value> {
        crate::bounded(note, 1000)?;
        self.transaction(|t| {
            recheck(t, a, "operator")?;
            intent(t, id)?;
            event(
                t,
                id,
                "support_note",
                json!({"actor":a.id,"note":note}),
                now,
            )
        })?;
        self.commerce_support(a, id)
    }
}
