//! Independent persistent sample provider facts, with no network or real money.
use crate::{
    commerce_checkout::provider_guard,
    commerce_finance_provider::*,
    commerce_provider::{CheckoutObservation, RefundObservation},
    commerce_sample::Sample,
    nonce,
    store::recheck,
    Actor, Error, Result,
};
use async_trait::async_trait;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
fn guard(p: &Sample) -> Result<()> {
    provider_guard(&*p.0.connection()?, "sample")
}
impl Sample {
    fn checkouts(&self) -> Result<Vec<CheckoutObservation>> {
        guard(self)?;
        let c = self.0.connection()?;
        let mut q = c.prepare("SELECT body FROM commerce_sample_provider WHERE kind='checkout'")?;
        let mut out = Vec::new();
        for row in q.query_map([], |r| r.get::<_, String>(0))? {
            let v: Value = serde_json::from_str(&row?)?;
            out.push(serde_json::from_value(v["observation"].clone())?);
        }
        Ok(out)
    }
    pub(crate) fn refund_effect(
        &self,
        account: &str,
        charge: &str,
        amount: u64,
        key: &str,
    ) -> Result<RefundObservation> {
        let o = self
            .checkouts()?
            .into_iter()
            .find(|o| o.account == account && o.payment_id.as_deref() == Some(charge) && o.paid)
            .ok_or(Error::new(404, "sample_charge_unavailable"))?;
        self.0.transaction(|t| {
            let old: Option<String> = t
                .query_row(
                    "SELECT body FROM commerce_sample_provider WHERE kind='refund' AND key=?1",
                    [key],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(raw) = old {
                let v: Value = serde_json::from_str(&raw)?;
                let r: RefundObservation = serde_json::from_value(v["observation"].clone())?;
                if v["account"] != account || r.payment_id != charge || r.amount != amount {
                    return Err(Error::new(409, "provider_key_conflict"));
                }
                return Ok(r);
            }
            let mut q =
                t.prepare("SELECT body FROM commerce_sample_provider WHERE kind='refund'")?;
            let mut used = 0;
            for row in q.query_map([], |r| r.get::<_, String>(0))? {
                let v: Value = serde_json::from_str(&row?)?;
                let r: RefundObservation = serde_json::from_value(v["observation"].clone())?;
                if r.payment_id == charge
                    && ["pending", "succeeded", "requires_action"].contains(&r.state.as_str())
                {
                    used += r.amount;
                }
            }
            if amount == 0 || amount > o.total - used {
                return Err(Error::new(422, "provider_refund_exceeds_payment"));
            }
            let next: Option<String> = t
                .query_row(
                    "SELECT value FROM metadata WHERE key='sample_next_refund_state'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            t.execute(
                "DELETE FROM metadata WHERE key='sample_next_refund_state'",
                [],
            )?;
            let r = RefundObservation {
                id: format!("re_{}", nonce()?),
                payment_id: charge.into(),
                currency: o.currency,
                amount,
                state: next.unwrap_or("succeeded".into()),
            };
            t.execute(
                "INSERT INTO commerce_sample_provider VALUES('refund',?1,?2)",
                params![key, json!({"account":account,"observation":r}).to_string()],
            )?;
            Ok(r)
        })
    }
    pub(crate) fn read_refund(&self, account: &str, id: &str) -> Result<RefundObservation> {
        guard(self)?;
        let c = self.0.connection()?;
        let mut q = c.prepare("SELECT body FROM commerce_sample_provider WHERE kind='refund'")?;
        for row in q.query_map([], |r| r.get::<_, String>(0))? {
            let v: Value = serde_json::from_str(&row?)?;
            let r: RefundObservation = serde_json::from_value(v["observation"].clone())?;
            if v["account"] == account && r.id == id {
                return Ok(r);
            }
        }
        Err(Error::new(404, "provider_refund_unavailable"))
    }
    pub(crate) fn cancel_effect(&self, account: &str, id: &str, key: &str) -> Result<Value> {
        let o = self
            .checkouts()?
            .into_iter()
            .find(|o| o.account == account && o.subscription_id.as_deref() == Some(id) && o.paid)
            .ok_or(Error::new(404, "sample_subscription_unavailable"))?;
        let v =
            json!({"id":id,"cancelAtPeriodEnd":true,"status":"active","paidUntil":o.paid_until});
        self.0.transaction(|t| {
            t.execute(
                "INSERT OR IGNORE INTO commerce_sample_provider VALUES('cancellation',?1,?2)",
                params![key, v.to_string()],
            )?;
            Ok(v)
        })
    }
    pub fn finance_scenario(
        &self,
        a: &Actor,
        order: &str,
        scenario: &str,
        now: i64,
    ) -> Result<Value> {
        guard(self)?;
        let o = self
            .checkouts()?
            .into_iter()
            .find(|o| o.order_id == order && o.paid)
            .ok_or(Error::new(404, "sample_charge_unavailable"))?;
        self.0.transaction(|t|{recheck(t,a,"operator")?;
   match scenario{
    "payout_pending"|"payout_failed"|"payout_paid"=>{let id=format!("po_{}",o.order_id);let p=Payout{id:id.clone(),account:o.account,amount:500,currency:o.currency,state:match scenario{"payout_failed"=>"failed","payout_paid"=>"paid",_=>"in_transit"}.into(),arrival_at:Some(now+86400),failure_code:if scenario=="payout_failed"{Some("insufficient_funds".into())}else{None},balance_transaction:Some(format!("txn_payout_{}",o.order_id)),failure_transaction:if scenario=="payout_failed"{Some(format!("txn_return_{}",o.order_id))}else{None}};t.execute("INSERT INTO commerce_sample_provider VALUES('payout',?1,?2) ON CONFLICT(kind,key) DO UPDATE SET body=excluded.body",params![id,serde_json::to_string(&p)?])?;Ok(json!({"payoutId":id,"fictional":true}))},
    "dispute_open"|"dispute_won"|"dispute_lost"=>{let id=format!("du_{}",o.order_id);let mut movements=vec![BalanceEffect{id:format!("txn_dispute_{}",o.order_id),currency:o.currency.clone(),net:-(o.total as i64)-15,fee:15}];if scenario=="dispute_won"{movements.push(BalanceEffect{id:format!("txn_dispute_return_{}",o.order_id),currency:o.currency.clone(),net:o.total as i64,fee:0});}let d=Dispute{id:id.clone(),account:o.account,charge:o.payment_id.unwrap(),amount:o.total,currency:o.currency,state:match scenario{"dispute_won"=>"won","dispute_lost"=>"lost",_=>"needs_response"}.into(),evidence_due:Some(now+7*86400),movements};t.execute("INSERT INTO commerce_sample_provider VALUES('dispute',?1,?2) ON CONFLICT(kind,key) DO UPDATE SET body=excluded.body",params![id,serde_json::to_string(&d)?])?;Ok(json!({"disputeId":id,"fictional":true}))},
    "next_refund_pending"=>{t.execute("INSERT INTO metadata VALUES('sample_next_refund_state','pending') ON CONFLICT(key) DO UPDATE SET value='pending'",[])?;Ok(json!({"fictional":true}))},
    "settle_refunds"=>{let mut q=t.prepare("SELECT key,body FROM commerce_sample_provider WHERE kind='refund'")?;let rows:Vec<(String,String)>=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<_,_>>()?;for(key,raw)in rows{let mut v:Value=serde_json::from_str(&raw)?;if v["observation"]["paymentId"].as_str()==o.payment_id.as_deref()&&v["observation"]["state"]=="pending"{v["observation"]["state"]=json!("succeeded");t.execute("UPDATE commerce_sample_provider SET body=?2 WHERE kind='refund' AND key=?1",params![key,v.to_string()])?;}}Ok(json!({"fictional":true}))},
    _=>Err(Error::new(422,"unknown_finance_scenario")),
   }
  })
    }
}
#[async_trait]
impl Finance for Sample {
    async fn find_refund(
        &self,
        account: &str,
        charge: &str,
        key: &str,
    ) -> Result<Option<RefundObservation>> {
        guard(self)?;
        let c = self.0.connection()?;
        let raw: Option<String> = c
            .query_row(
                "SELECT body FROM commerce_sample_provider WHERE kind='refund' AND key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()?;
        raw.map(|s| {
            let v: Value = serde_json::from_str(&s)?;
            let o: RefundObservation = serde_json::from_value(v["observation"].clone())?;
            if v["account"] != account || o.payment_id != charge {
                return Err(Error::new(409, "provider_key_conflict"));
            }
            Ok(o)
        })
        .transpose()
    }
    async fn refund_fee(&self, i: &FeeIntent, key: &str) -> Result<FeeRefund> {
        let o = self
            .checkouts()?
            .into_iter()
            .find(|o| {
                o.account == i.account
                    && o.payment_id.as_ref() == Some(&i.charge)
                    && o.fee_id.as_ref() == Some(&i.fee)
            })
            .ok_or(Error::new(404, "sample_fee_unavailable"))?;
        self.0.transaction(|t| {
            if let Some(s) = t
                .query_row(
                    "SELECT body FROM commerce_sample_provider WHERE kind='fee_refund' AND key=?1",
                    [key],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
            {
                let r: FeeRefund = serde_json::from_str(&s)?;
                if r.fee != i.fee || r.amount != i.amount || r.currency != i.currency {
                    return Err(Error::new(409, "provider_key_conflict"));
                }
                return Ok(r);
            }
            let mut q =
                t.prepare("SELECT body FROM commerce_sample_provider WHERE kind='fee_refund'")?;
            let mut used = 0;
            for s in q.query_map([], |r| r.get::<_, String>(0))? {
                let r: FeeRefund = serde_json::from_str(&s?)?;
                if r.fee == i.fee {
                    used += r.amount;
                }
            }
            if i.amount == 0
                || i.currency != o.currency
                || i.amount > o.fee_amount.unwrap_or(0) - used
            {
                return Err(Error::new(422, "provider_fee_refund_exceeds_fee"));
            }
            let r = FeeRefund {
                id: format!("fr_{}", nonce()?),
                fee: i.fee.clone(),
                currency: i.currency.clone(),
                amount: i.amount,
            };
            t.execute(
                "INSERT INTO commerce_sample_provider VALUES('fee_refund',?1,?2)",
                params![key, serde_json::to_string(&r)?],
            )?;
            Ok(r)
        })
    }
    async fn find_fee_refund(&self, i: &FeeIntent, key: &str) -> Result<Option<FeeRefund>> {
        guard(self)?;
        let c = self.0.connection()?;
        let s: Option<String> = c
            .query_row(
                "SELECT body FROM commerce_sample_provider WHERE kind='fee_refund' AND key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()?;
        s.map(|s| {
            let r: FeeRefund = serde_json::from_str(&s)?;
            if r.fee != i.fee {
                return Err(Error::new(409, "provider_key_conflict"));
            }
            Ok(r)
        })
        .transpose()
    }
    async fn dispute(&self, account: &str, id: &str) -> Result<Dispute> {
        guard(self)?;
        let c = self.0.connection()?;
        let s: String = c.query_row(
            "SELECT body FROM commerce_sample_provider WHERE kind='dispute' AND key=?1",
            [id],
            |r| r.get(0),
        )?;
        let d: Dispute = serde_json::from_str(&s)?;
        if d.account != account {
            return Err(Error::new(404, "sample_dispute_unavailable"));
        }
        Ok(d)
    }
    async fn report(&self, account: &str, after: Option<&str>) -> Result<AccountReport> {
        let checkouts = self.checkouts()?;
        let mut cash = std::collections::BTreeMap::<String, i64>::new();
        for o in &checkouts {
            if o.account == account && o.paid {
                *cash.entry(o.currency.clone()).or_default() +=
                    o.total as i64 - o.fee_amount.unwrap_or(0) as i64 - 30;
            }
        }
        let c = self.0.connection()?;
        let mut q=c.prepare("SELECT kind,body FROM commerce_sample_provider WHERE kind IN ('refund','fee_refund','dispute','payout')")?;
        let mut payouts = Vec::new();
        for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (kind, raw) = row?;
            match kind.as_str() {
                "refund" => {
                    let v: Value = serde_json::from_str(&raw)?;
                    let r: RefundObservation = serde_json::from_value(v["observation"].clone())?;
                    if v["account"] == account && r.state == "succeeded" {
                        *cash.entry(r.currency).or_default() -= r.amount as i64;
                    }
                }
                "fee_refund" => {
                    let r: FeeRefund = serde_json::from_str(&raw)?;
                    if checkouts
                        .iter()
                        .any(|o| o.account == account && o.fee_id.as_ref() == Some(&r.fee))
                    {
                        *cash.entry(r.currency).or_default() += r.amount as i64;
                    }
                }
                "dispute" => {
                    let d: Dispute = serde_json::from_str(&raw)?;
                    if d.account == account {
                        for m in d.movements {
                            *cash.entry(m.currency).or_default() += m.net;
                        }
                    }
                }
                "payout" => {
                    let p: Payout = serde_json::from_str(&raw)?;
                    if p.account == account {
                        if ["in_transit", "paid", "pending"].contains(&p.state.as_str()) {
                            *cash.entry(p.currency.clone()).or_default() -= p.amount as i64;
                        }
                        if after.is_none_or(|a| p.id.as_str() > a) {
                            payouts.push(p);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(AccountReport {
            account: account.into(),
            balances: cash
                .into_iter()
                .map(|(currency, available)| Balance {
                    currency,
                    available,
                    pending: 0,
                })
                .collect(),
            payouts,
            has_more_payouts: false,
            next_payout_cursor: None,
        })
    }
    async fn charge_costs(&self, account: &str, charge: &str) -> Result<ChargeCosts> {
        let o = self
            .checkouts()?
            .into_iter()
            .find(|o| o.account == account && o.payment_id.as_deref() == Some(charge))
            .ok_or(Error::new(404, "sample_charge_unavailable"))?;
        Ok(ChargeCosts {
            charge: charge.into(),
            account: account.into(),
            currency: o.currency,
            transaction: format!("txn_{charge}"),
            provider_fee: 30,
            application_fee: o.fee_amount.unwrap_or(0),
            refunded: self.0.connection()?.query_row("SELECT COALESCE(SUM(json_extract(body,'$.observation.amount')),0) FROM commerce_sample_provider WHERE kind='refund' AND json_extract(body,'$.observation.paymentId')=?1 AND json_extract(body,'$.observation.state')='succeeded'", [charge], |r| r.get::<_,u32>(0))? as u64,
        })
    }
    async fn invoice(&self, account: &str, id: &str) -> Result<Invoice> {
        guard(self)?;
        let c = self.0.connection()?;
        let raw: String = c.query_row(
            "SELECT body FROM commerce_sample_provider WHERE kind='invoice' AND key=?1",
            [id],
            |r| r.get(0),
        )?;
        let i: Invoice = serde_json::from_str(&raw)?;
        if i.account != account {
            return Err(Error::new(404, "sample_invoice_unavailable"));
        }
        Ok(i)
    }
    async fn invoices(
        &self,
        account: &str,
        subscription: &str,
        after: Option<&str>,
    ) -> Result<Value> {
        guard(self)?;
        let c = self.0.connection()?;
        let mut q = c.prepare(
            "SELECT body FROM commerce_sample_provider WHERE kind='invoice' ORDER BY key",
        )?;
        let mut ids = Vec::new();
        for s in q.query_map([], |r| r.get::<_, String>(0))? {
            let i: Invoice = serde_json::from_str(&s?)?;
            if i.account == account
                && i.subscription == subscription
                && after.is_none_or(|a| i.id.as_str() > a)
            {
                ids.push(i.id);
            }
        }
        Ok(json!({"ids":ids,"hasMore":false,"nextCursor":null}))
    }
}
