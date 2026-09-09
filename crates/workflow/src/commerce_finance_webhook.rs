//! Signed events request fresh provider reads. The callback snapshot never moves money.
use crate::{
    commerce_checkout::provider_guard, commerce_finance_provider::Finance,
    commerce_model::provider_id, digest, Error, Result, Store,
};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
impl Store {
    pub async fn commerce_all_webhooks(
        &self,
        p: &dyn Finance,
        secret: &str,
        header: &str,
        body: &[u8],
        now: i64,
    ) -> Result<Value> {
        let v = crate::commerce_webhook::verify(secret, header, body, now)?;
        let kind = v["type"].as_str().unwrap_or("");
        if kind.starts_with("checkout.session.") {
            return self.commerce_webhook(p, secret, header, body, now).await;
        }
        let category = match kind {
            "invoice.paid" | "invoice.payment_succeeded" => "invoice",
            "charge.dispute.created"
            | "charge.dispute.updated"
            | "charge.dispute.closed"
            | "charge.dispute.funds_reinstated"
            | "charge.dispute.funds_withdrawn" => "dispute",
            "refund.created" | "refund.updated" | "refund.failed" | "charge.refunded" => "refund",
            "payout.created" | "payout.updated" | "payout.paid" | "payout.failed"
            | "payout.canceled" => "payout",
            "customer.subscription.updated" | "customer.subscription.deleted" => "subscription",
            _ => return Ok(json!({"ignored":true})),
        };
        let account = v["account"].as_str().unwrap();
        let event = v["id"].as_str().unwrap();
        let oid = v["data"]["object"]["id"]
            .as_str()
            .filter(|s| s.len() < 128)
            .ok_or(Error::new(422, "unsupported_payment_event"))?;
        let seller: Option<String> = {
            let c = self.connection()?;
            provider_guard(&c, p.mode())?;
            c.query_row(
                "SELECT id FROM commerce_sellers WHERE account=?1",
                [account],
                |r| r.get(0),
            )
            .optional()?
        };
        let Some(seller) = seller else {
            return Ok(json!({"ignored":true}));
        };
        let sha = digest(body);
        let done = self.transaction(|t| {
            let old: Option<(String, String)> = t
                .query_row(
                    "SELECT digest,state FROM commerce_webhooks WHERE id=?1",
                    [event],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            if let Some((d, s)) = old {
                if d != sha {
                    return Err(Error::new(409, "payment_event_digest_conflict"));
                }
                return Ok(s == "completed");
            }
            t.execute(
                "INSERT INTO commerce_webhooks VALUES(?1,?2,?3,?4,'pending',?5)",
                params![event, sha, account, oid, now],
            )?;
            Ok(false)
        })?;
        if done {
            return Ok(json!({"received":true,"replayed":true}));
        }
        match category {
            "invoice" => {
                self.commerce_reconcile_invoice(p, account, oid, now)
                    .await?;
            }
            "dispute" => {
                let d = p.dispute(account, oid).await?;
                if d.id != oid || d.account != account {
                    return Err(Error::new(409, "dispute_order_mismatch"));
                }
                self.commerce_record_dispute(p.mode(), &d, now)?;
            }
            "refund" => {
                let ids = {
                    let c = self.connection()?;
                    let mut q=c.prepare("SELECT r.id FROM commerce_refunds r JOIN commerce_orders o ON o.id=r.order_id WHERE json_extract(o.price,'$.connectedAccount')=?1 AND (r.provider_id=?2 OR r.state='pending') LIMIT 100")?;
                    let rows = q
                        .query_map(params![account, oid], |r| r.get::<_, String>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    rows
                };
                for id in ids {
                    self.commerce_poll_refund(&id, p, now).await?;
                }
                self.reconcile_seller(&seller, p, None, now).await?;
            }
            "payout" => {
                if !provider_id(oid, "po_") {
                    return Err(Error::new(422, "unsupported_payment_event"));
                }
                self.reconcile_seller(&seller, p, None, now).await?;
            }
            "subscription" => {
                if !provider_id(oid, "sub_") {
                    return Err(Error::new(422, "unsupported_payment_event"));
                }
                let root: Option<String> = {
                    let c = self.connection()?;
                    c.query_row("SELECT id FROM commerce_orders WHERE subscription_id=?1 AND json_extract(price,'$.connectedAccount')=?2 AND NOT EXISTS(SELECT 1 FROM commerce_cycles WHERE cycle_order=commerce_orders.id)",params![oid,account],|r|r.get(0)).optional()?
                };
                if let Some(root) = root {
                    self.commerce_poll_subscription(&root, p, now).await?;
                    self.commerce_finish_cancellation(&root, p, now).await?;
                }
            }
            _ => unreachable!(),
        }
        self.connection()?.execute(
            "UPDATE commerce_webhooks SET state='completed' WHERE id=?1 AND digest=?2",
            params![event, sha],
        )?;
        Ok(json!({"received":true,"replayed":false}))
    }
}
