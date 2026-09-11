//! Refund requests, provider effects and fee reversals have independent durable states.
use crate::{
    commerce_checkout::{access, event, intent, provider_guard},
    commerce_finance_provider::{FeeIntent, FeeRefund, Finance},
    commerce_provider::RefundObservation,
    digest, nonce,
    store::recheck,
    Actor, Error, Result, Store,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub order_id: String,
    pub amount: u64,
    pub expected_refunded: u64,
    pub reason: String,
    pub accepted: bool,
}
pub fn proportional_fee(original_fee: u64, total: u64, cumulative: u64) -> Result<u64> {
    if total == 0 || cumulative > total || original_fee > total {
        return Err(Error::new(422, "invalid_refund_basis"));
    }
    Ok((u128::from(original_fee) * u128::from(cumulative) / u128::from(total)) as u64)
}
fn refunded(c: &Connection, order: &str) -> Result<u64> {
    Ok(c.query_row("SELECT COALESCE(SUM(amount),0) FROM commerce_refunds WHERE order_id=?1 AND state='succeeded'",[order],|r|r.get::<_,u32>(0))? as u64)
}
fn projection(c: &Connection, id: &str) -> Result<Value> {
    c.query_row("SELECT order_id,amount,state,fee_amount,fee_state,error,reason,created_at FROM commerce_refunds WHERE id=?1",[id],|r|Ok(json!({"id":id,"orderId":r.get::<_,String>(0)?,"amount":r.get::<_,u32>(1)?,"state":r.get::<_,String>(2)?,"feeAmount":r.get::<_,u32>(3)?,"feeState":r.get::<_,String>(4)?,"error":r.get::<_,Option<String>>(5)?,"reason":r.get::<_,String>(6)?,"createdAt":r.get::<_,i64>(7)?}))).optional()?.ok_or(Error::new(404,"refund_unavailable"))
}
impl Store {
    pub fn commerce_refunds(&self, a: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        access(&c, a, id)?;
        let (i, _) = intent(&c, id)?;
        let mut q=c.prepare("SELECT id FROM commerce_refunds WHERE order_id=?1 ORDER BY created_at DESC,id LIMIT 100")?;
        let ids: Vec<String> = q
            .query_map([id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let done = refunded(&c, id)?;
        Ok(
            json!({"orderId":id,"refunded":done,"remaining":i.price.amounts()?["total"].as_u64().unwrap()-done,"items":ids.iter().map(|id|projection(&c,id)).collect::<Result<Vec<_>>>()?,"notice":"A request is not a completed refund. Provider processing fees and platform fee reversals are reported separately."}),
        )
    }
    pub fn commerce_request_refund(
        &self,
        a: &Actor,
        key: &str,
        p: Request,
        now: i64,
    ) -> Result<Value> {
        crate::bounded(&p.reason, 1000)?;
        if !p.accepted
            || p.amount == 0
            || p.amount > 99_999_999
            || key.len() < 16
            || key.len() > 128
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::new(422, "invalid_refund_request"));
        }
        let sha = digest(serde_json::to_vec(&p)?);
        self.transaction(|t|{access(t,a,&p.order_id)?;
   if let Some((id,old))=t.query_row("SELECT id,request_digest FROM commerce_refunds WHERE actor=?1 AND request_key=?2",params![a.id,key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?{if old!=sha{return Err(Error::new(409,"idempotency_key_conflict"));}return projection(t,&id);}
   let(i,mode)=intent(t,&p.order_id)?;provider_guard(t,&mode)?;let total=i.price.amounts()?["total"].as_u64().unwrap();let paid:bool=t.query_row("SELECT payment_state='paid' FROM commerce_orders WHERE id=?1",[&p.order_id],|r|r.get(0))?;if !paid{return Err(Error::new(409,"confirmed_payment_required"));}
   let done=refunded(t,&p.order_id)?;if done!=p.expected_refunded||p.amount>total-done{return Err(Error::new(409,"refund_preview_changed"));}
   let pending:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM commerce_refunds WHERE order_id=?1 AND state IN ('requested','pending'))",[&p.order_id],|r|r.get(0))?;if pending{return Err(Error::new(409,"refund_already_in_progress"));}
   let id=nonce()?;t.execute("INSERT INTO commerce_refunds(id,order_id,actor,request_key,request_digest,amount,expected_refunded,reason,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,p.order_id,a.id,key,sha,p.amount as i64,done as i64,p.reason,now])?;event(t,&p.order_id,"refund_requested",json!({"id":id,"amount":p.amount,"actor":a.id}),now)?;projection(t,&id)
  })
    }
    pub fn commerce_reject_refund(
        &self,
        a: &Actor,
        id: &str,
        reason: &str,
        now: i64,
    ) -> Result<Value> {
        crate::bounded(reason, 1000)?;
        self.transaction(|t|{recheck(t,a,"operator")?;let v=projection(t,id)?;if v["state"]!="requested"{return Err(Error::new(409,"refund_decision_unavailable"));}t.execute("UPDATE commerce_refunds SET state='rejected',error='operator_rejected' WHERE id=?1",[id])?;event(t,v["orderId"].as_str().unwrap(),"refund_rejected",json!({"id":id,"actor":a.id,"reason":reason}),now)?;projection(t,id)})
    }
    pub async fn commerce_execute_refund(
        &self,
        a: &Actor,
        id: &str,
        provider: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        let claim=self.transaction(|t|{recheck(t,a,"operator")?;let v=projection(t,id)?;let order=v["orderId"].as_str().unwrap();let(i,mode)=intent(t,order)?;provider_guard(t,&mode)?;if mode!=provider.mode(){return Err(Error::new(503,"payment_provider_mode_mismatch"));}
   if ["succeeded","failed","rejected","canceled"].contains(&v["state"].as_str().unwrap_or("")){return Ok(None);}
   let(at,lease,provider_id):(Option<i64>,i64,Option<String>)=t.query_row("SELECT attempt_at,lease_until,provider_id FROM commerce_refunds WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;if lease>now{return Err(Error::new(409,"refund_in_progress"));}
   let amount=v["amount"].as_u64().unwrap();let done=refunded(t,order)?;let total=i.price.amounts()?["total"].as_u64().unwrap();if amount>total-done{return Err(Error::new(409,"refund_preview_changed"));}
   let charge:String=t.query_row("SELECT payment_id FROM commerce_orders WHERE id=?1",[order],|r|r.get(0))?;
   t.execute("UPDATE commerce_refunds SET state='pending',approved_by=COALESCE(approved_by,?2),approved_at=COALESCE(approved_at,?3),attempt_at=COALESCE(attempt_at,?3),lease_until=?4 WHERE id=?1",params![id,a.id,now,now+60])?;event(t,order,"refund_approved_or_reconciled",json!({"id":id,"actor":a.id}),now)?;Ok(Some((i,charge,amount,at,provider_id)))
  })?;
        if let Some((i, charge, amount, at, pid)) = claim {
            let key = format!("refund-{id}");
            let result = async {
                if let Some(pid) = pid {
                    return provider
                        .refund_status(&i.price.connected_account, &pid)
                        .await;
                }
                if at.is_some() {
                    if let Some(r) = provider
                        .find_refund(&i.price.connected_account, &charge, &key)
                        .await?
                    {
                        return Ok(r);
                    }
                }
                if at.is_some_and(|v| now - v >= 23 * 3600) {
                    return Err(Error::new(409, "refund_requires_provider_reconciliation"));
                }
                provider
                    .refund(&i.price.connected_account, &charge, amount, &key)
                    .await
            }
            .await;
            match result {
                Ok(o) => {
                    if let Err(e) = self.commerce_observe_refund(id, provider.mode(), &o, now) {
                        self.refund_error(id, e.code, now)?;
                        return Err(e);
                    }
                }
                Err(e) => {
                    self.refund_error(id, e.code, now)?;
                    return Err(e);
                }
            }
        }
        self.commerce_refund_fee(id, provider, now).await?;
        let c = self.connection()?;
        projection(&c, id)
    }
    fn refund_error(&self, id: &str, code: &str, now: i64) -> Result<()> {
        self.transaction(|t| {
            let v = projection(t, id)?;
            t.execute(
                "UPDATE commerce_refunds SET lease_until=0,error=?2 WHERE id=?1",
                params![id, code],
            )?;
            event(
                t,
                v["orderId"].as_str().unwrap(),
                "refund_needs_reconciliation",
                json!({"id":id,"code":code}),
                now,
            )
        })
    }
    pub fn commerce_observe_refund(
        &self,
        id: &str,
        mode: &str,
        o: &RefundObservation,
        now: i64,
    ) -> Result<()> {
        self.transaction(|t|{provider_guard(t,mode)?;let v=projection(t,id)?;let order=v["orderId"].as_str().unwrap();let(i,stored)=intent(t,order)?;let(charge,pid):(String,Option<String>)=t.query_row("SELECT o.payment_id,r.provider_id FROM commerce_orders o JOIN commerce_refunds r ON r.order_id=o.id WHERE r.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
   if stored!=mode||o.payment_id!=charge||o.currency!=i.price.currency||json!(o.amount)!=v["amount"]||!crate::commerce_model::provider_id(&o.id,"re_")||pid.as_ref().is_some_and(|s|s!=&o.id){return Err(Error::new(409,"refund_provider_mismatch"));}
   if !["pending","requires_action","succeeded","failed","canceled"].contains(&o.state.as_str()){return Err(Error::new(422,"invalid_refund_state"));}
   if v["state"]=="succeeded"{return Ok(());}
if ["failed","rejected","canceled"].contains(&v["state"].as_str().unwrap_or("")){return Err(Error::new(409,"terminal_refund_conflict"));}
   let mut fee=0;let mut fee_state="not_started";
   if o.state=="succeeded"{let total=i.price.amounts()?["total"].as_u64().unwrap();let done=refunded(t,order)?;if o.amount>total-done{return Err(Error::new(409,"refund_exceeds_order"));}
    let allocated:u32=t.query_row("SELECT COALESCE(SUM(fee_amount),0) FROM commerce_refunds WHERE order_id=?1 AND state='succeeded'",[order],|r|r.get(0))?;
    fee=proportional_fee(i.price.amounts()?["serviceFee"].as_u64().unwrap(),total,done+o.amount)?.checked_sub(u64::from(allocated)).ok_or(Error::new(409,"refund_fee_allocation_conflict"))?;fee_state=if fee==0{"succeeded"}else{"pending"};
    crate::commerce_accounting::sale(t,order,now)?;crate::commerce_accounting::post(t,crate::commerce_accounting::Entry{source:&format!("refund:{}",o.id),seller:&i.price.seller_id,currency:&i.price.currency,order:Some(order),kind:"buyer_refund",amount:-(o.amount as i64),reserve:0,detail:json!({"refundId":id})},now)?;
   }
   let state=if o.state=="requires_action"{"pending"}else{&o.state};
   t.execute("UPDATE commerce_refunds SET provider_id=?2,state=?3,lease_until=0,error=?4,fee_amount=?5,fee_state=?6 WHERE id=?1",params![id,o.id,state,if o.state=="requires_action"{Some("provider_action_required")}else{None},fee as i64,fee_state])?;
   event(t,order,"refund_observed",json!({"id":id,"state":state,"amount":o.amount,"feeAmount":fee}),now)?;Ok(())
  })
    }
    pub async fn commerce_refund_fee(
        &self,
        id: &str,
        provider: &dyn Finance,
        now: i64,
    ) -> Result<()> {
        let claim=self.transaction(|t|{let v=projection(t,id)?;if v["state"]!="succeeded"||v["feeState"]=="succeeded"{return Ok(None);}let order=v["orderId"].as_str().unwrap();let(i,mode)=intent(t,order)?;provider_guard(t,&mode)?;if provider.mode()!=mode{return Err(Error::new(503,"payment_provider_mode_mismatch"));}
   let(lease,at,pid):(i64,Option<i64>,Option<String>)=t.query_row("SELECT lease_until,fee_attempt_at,fee_provider_id FROM commerce_refunds WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;if lease>now{return Err(Error::new(409,"fee_refund_in_progress"));}
   let(charge,fee):(String,String)=t.query_row("SELECT payment_id,fee_id FROM commerce_orders WHERE id=?1",[order],|r|Ok((r.get(0)?,r.get(1)?)))?;let fi=FeeIntent{account:i.price.connected_account,charge,fee,currency:i.price.currency,amount:v["feeAmount"].as_u64().unwrap()};t.execute("UPDATE commerce_refunds SET fee_attempt_at=COALESCE(fee_attempt_at,?2),lease_until=?3,fee_state='pending' WHERE id=?1",params![id,now,now+60])?;Ok(Some((fi,at,pid)))
  })?;
        let Some((fi, at, _pid)) = claim else {
            return Ok(());
        };
        let key = format!("fee-refund-{id}");
        let result = async {
            if at.is_some() {
                if let Some(r) = provider.find_fee_refund(&fi, &key).await? {
                    return Ok(r);
                }
            }
            if at.is_some_and(|n| now - n >= 23 * 3600) {
                return Err(Error::new(409, "fee_refund_requires_reconciliation"));
            }
            provider.refund_fee(&fi, &key).await
        }
        .await;
        match result {
            Ok(o) => self.commerce_observe_fee(id, &fi, &o, now),
            Err(e) => {
                self.refund_error(id, e.code, now)?;
                Err(e)
            }
        }
    }
    fn commerce_observe_fee(
        &self,
        id: &str,
        fi: &FeeIntent,
        o: &FeeRefund,
        now: i64,
    ) -> Result<()> {
        if o.fee != fi.fee
            || o.amount != fi.amount
            || o.currency != fi.currency
            || !crate::commerce_model::provider_id(&o.id, "fr_")
        {
            self.refund_error(id, "fee_refund_provider_mismatch", now)?;
            return Err(Error::new(409, "fee_refund_provider_mismatch"));
        }
        self.transaction(|t|{let v=projection(t,id)?;if v["feeState"]=="succeeded"{return Ok(());}let order=v["orderId"].as_str().unwrap();let(i,_)=intent(t,order)?;crate::commerce_accounting::post(t,crate::commerce_accounting::Entry{source:&format!("fee-refund:{}",o.id),seller:&i.price.seller_id,currency:&i.price.currency,order:Some(order),kind:"platform_fee_refund",amount:o.amount as i64,reserve:0,detail:json!({"refundId":id})},now)?;t.execute("UPDATE commerce_refunds SET fee_provider_id=?2,fee_state='succeeded',lease_until=0,error=NULL WHERE id=?1",params![id,o.id])?;event(t,order,"platform_fee_refunded",json!({"refundId":id,"amount":o.amount}),now)})
    }
    pub async fn commerce_poll_refund(
        &self,
        id: &str,
        provider: &dyn Finance,
        now: i64,
    ) -> Result<()> {
        let (v, i, pid) = {
            let c = self.connection()?;
            let v = projection(&c, id)?;
            let (i, mode) = intent(&c, v["orderId"].as_str().unwrap())?;
            provider_guard(&c, &mode)?;
            if mode != provider.mode() {
                return Err(Error::new(503, "payment_provider_mode_mismatch"));
            }
            let pid: Option<String> = c.query_row(
                "SELECT provider_id FROM commerce_refunds WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            (v, i, pid)
        };
        if v["state"] == "pending" {
            let charge: String = self.connection()?.query_row(
                "SELECT payment_id FROM commerce_orders WHERE id=?1",
                [&i.order_id],
                |r| r.get(0),
            )?;
            let found = match pid {
                Some(pid) => Some(
                    provider
                        .refund_status(&i.price.connected_account, &pid)
                        .await?,
                ),
                None => {
                    provider
                        .find_refund(&i.price.connected_account, &charge, &format!("refund-{id}"))
                        .await?
                }
            };
            if let Some(o) = found {
                self.commerce_observe_refund(id, provider.mode(), &o, now)?;
            }
        }
        self.commerce_refund_fee(id, provider, now).await
    }
    pub fn commerce_refund_queue(&self) -> Result<Vec<String>> {
        let c = self.connection()?;
        let mut q=c.prepare("SELECT id FROM commerce_refunds WHERE state='pending' OR (state='succeeded' AND fee_state!='succeeded') ORDER BY created_at LIMIT 50")?;
        let rows = q
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cumulative_rounding_is_monotonic_and_last_refund_reverses_the_exact_fee() {
        for total in 1..2000 {
            let fee = total / 20;
            let mut refunded = 0;
            let mut allocated = 0;
            while refunded < total {
                let next = (refunded + 1 + total % 37).min(total);
                let target = proportional_fee(fee, total, next).unwrap();
                let delta = target - allocated;
                assert!(delta <= fee);
                allocated += delta;
                refunded = next;
            }
            assert_eq!(allocated, fee);
        }
        assert!(proportional_fee(50, 1000, 1001).is_err());
        assert!(proportional_fee(50, 0, 0).is_err());
        assert_eq!(
            proportional_fee(u64::MAX, u64::MAX, u64::MAX).unwrap(),
            u64::MAX
        );
    }
}
