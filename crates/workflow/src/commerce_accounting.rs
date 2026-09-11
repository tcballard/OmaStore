//! Attributable order proceeds are separate from the provider account's overall cash.
use crate::{
    commerce_checkout::{event, intent, provider_guard},
    commerce_finance_provider::{AccountReport, ChargeCosts, Dispute, Finance},
    digest,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
pub(crate) struct Entry<'a> {
    pub source: &'a str,
    pub seller: &'a str,
    pub currency: &'a str,
    pub order: Option<&'a str>,
    pub kind: &'a str,
    pub amount: i64,
    pub reserve: i64,
    pub detail: Value,
}
pub(crate) fn post(t: &Transaction<'_>, e: Entry<'_>, now: i64) -> Result<bool> {
    crate::commerce_model::exponent(e.currency)?;
    if e.amount.unsigned_abs() > 9_000_000_000_000 || e.reserve.unsigned_abs() > 9_000_000_000_000 {
        return Err(Error::new(422, "ledger_amount_out_of_range"));
    }
    if let Some((seller,currency,order,kind,amount,reserve,detail))=t.query_row("SELECT seller_id,currency,order_id,kind,amount,reserve_delta,detail FROM commerce_ledger WHERE source=?1",[e.source],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?))).optional()?{
  if seller!=e.seller||currency!=e.currency||order.as_deref()!=e.order||kind!=e.kind||amount!=e.amount||reserve!=e.reserve||serde_json::from_str::<Value>(&detail)?!=e.detail{return Err(Error::new(409,"ledger_source_conflict"));}return Ok(false);
 }
    t.execute("INSERT INTO commerce_ledger(source,seller_id,currency,order_id,kind,amount,reserve_delta,at,detail) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![e.source,e.seller,e.currency,e.order,e.kind,e.amount,e.reserve,now,e.detail.to_string()])?;
    Ok(true)
}
pub(crate) fn sale(t: &Transaction<'_>, order: &str, now: i64) -> Result<()> {
    let (i, _) = intent(t, order)?;
    let paid: bool = t.query_row(
        "SELECT payment_state='paid' FROM commerce_orders WHERE id=?1",
        [order],
        |r| r.get(0),
    )?;
    if !paid {
        return Err(Error::new(409, "confirmed_payment_required"));
    }
    let a = i.price.amounts()?;
    let total = a["total"].as_u64().unwrap();
    let fee = a["serviceFee"].as_u64().unwrap();
    post(
        t,
        Entry {
            source: &format!("sale:{order}"),
            seller: &i.price.seller_id,
            currency: &i.price.currency,
            order: Some(order),
            kind: "sale_after_platform_fee",
            amount: (total - fee) as i64,
            reserve: 0,
            detail: json!({"total":total,"platformFee":fee,"tax":i.price.tax}),
        },
        now,
    )?;
    Ok(())
}
pub(crate) fn backfill(t: &Transaction<'_>) -> Result<()> {
    let mut q=t.prepare("SELECT id,COALESCE((SELECT MIN(at) FROM commerce_events WHERE order_id=o.id AND kind='paid'),created_at) FROM commerce_orders o WHERE payment_state='paid'")?;
    let rows: Vec<(String, i64)> = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    for (id, at) in rows {
        sale(t, &id, at)?;
    }
    Ok(())
}
fn seller(c: &Connection, a: &Actor, id: &str) -> Result<String> {
    crate::commerce_checkout::signed_in(c, a)?;
    let (owner, account): (String, String) = c
        .query_row(
            "SELECT owner,account FROM commerce_sellers WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "seller_unavailable"))?;
    if owner != a.id && recheck(c, a, "operator").is_err() {
        return Err(Error::new(404, "seller_unavailable"));
    }
    Ok(account)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reserve {
    pub seller_id: String,
    pub currency: String,
    pub target: u64,
    pub expected: u64,
    pub reason: String,
    pub report_url: String,
    pub report_digest: String,
}
pub(crate) fn reserve(t: &Transaction<'_>, a: &Actor, p: Reserve, now: i64) -> Result<Value> {
    recheck(t, a, "operator")?;
    seller(t, a, &p.seller_id)?;
    crate::commerce_model::exponent(&p.currency)?;
    crate::bounded(&p.reason, 1000)?;
    crate::net::public_url(&p.report_url)?;
    if p.target > 99_999_999
        || p.report_digest.len() != 64
        || !p.report_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::new(422, "invalid_reserve_record"));
    }
    let held:u32=t.query_row("SELECT COALESCE(SUM(reserve_delta),0) FROM commerce_ledger WHERE seller_id=?1 AND currency=?2",params![p.seller_id,p.currency],|r|r.get(0))?;
    if u64::from(held) != p.expected {
        return Err(Error::new(409, "reserve_preview_changed"));
    }
    post(
        t,
        Entry {
            source: &format!("reserve:{}", crate::nonce()?),
            seller: &p.seller_id,
            currency: &p.currency,
            order: None,
            kind: "operator_accounting_reserve",
            amount: 0,
            reserve: p.target as i64 - held as i64,
            detail: json!({"reason":p.reason,"reportUrl":p.report_url,"reportDigest":p.report_digest}),
        },
        now,
    )?;
    audit(
        t,
        &a.id,
        "commerce_reserve_recorded",
        &p.seller_id,
        now,
        &json!({"target":p.target,"currency":p.currency,"reportDigest":p.report_digest}),
    )?;
    Ok(
        json!({"recorded":true,"target":p.target,"notice":"Accounting reserve recorded. Provider enforcement needs its own confirmed arrangement."}),
    )
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub dispute_id: String,
    pub report_url: String,
    pub report_digest: String,
    pub note: String,
}
pub(crate) fn evidence(t: &Transaction<'_>, a: &Actor, p: Evidence, now: i64) -> Result<Value> {
    recheck(t, a, "operator")?;
    crate::net::public_url(&p.report_url)?;
    crate::bounded(&p.note, 1000)?;
    if p.report_digest.len() != 64 || !p.report_digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::new(422, "invalid_dispute_evidence"));
    }
    let order: String = t
        .query_row(
            "SELECT order_id FROM commerce_disputes WHERE id=?1",
            [&p.dispute_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(Error::new(404, "dispute_unavailable"))?;
    t.execute("INSERT INTO commerce_dispute_evidence(dispute_id,actor,report_url,report_digest,note,at) VALUES(?1,?2,?3,?4,?5,?6)",params![p.dispute_id,a.id,p.report_url,p.report_digest,p.note,now])?;
    event(
        t,
        &order,
        "dispute_evidence_recorded",
        json!({"disputeId":p.dispute_id,"reportDigest":p.report_digest,"actor":a.id}),
        now,
    )?;
    Ok(
        json!({"recorded":true,"notice":"Evidence packet recorded. The responsible operator submits it through the provider's dispute process and records that report."}),
    )
}
impl Store {
    pub fn commerce_finances(&self, a: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        let account = seller(&c, a, id)?;
        let mut q=c.prepare("SELECT currency,COALESCE(SUM(amount),0),COALESCE(SUM(reserve_delta),0) FROM commerce_ledger WHERE seller_id=?1 GROUP BY currency ORDER BY currency")?;
        let balances:Vec<Value>=q.query_map([id],|r|{let net:i64=r.get(1)?;let held:i64=r.get(2)?;Ok(json!({"currency":r.get::<_,String>(0)?,"recordedProceeds":net,"accountingReserve":held,"afterReserve":net-held,"negative":net-held<0}))})?.collect::<std::result::Result<_,_>>()?;
        let mut q=c.prepare("SELECT seq,source,currency,kind,amount,reserve_delta,order_id,at FROM commerce_ledger WHERE seller_id=?1 ORDER BY seq DESC LIMIT 100")?;
        let ledger:Vec<Value>=q.query_map([id],|r|Ok(json!({"seq":r.get::<_,i64>(0)?,"source":r.get::<_,String>(1)?,"currency":r.get::<_,String>(2)?,"kind":r.get::<_,String>(3)?,"amount":r.get::<_,i64>(4)?,"reserveDelta":r.get::<_,i64>(5)?,"orderId":r.get::<_,Option<String>>(6)?,"at":r.get::<_,i64>(7)?})))?.collect::<std::result::Result<_,_>>()?;
        let report:Option<(String,i64)>=c.query_row("SELECT body,observed_at FROM commerce_provider_reports WHERE seller_id=?1 AND account=?2",params![id,account],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let mut q=c.prepare("SELECT body,observed_at FROM commerce_payouts WHERE seller_id=?1 ORDER BY observed_at DESC,id LIMIT 100")?;
        let payouts: Vec<Value> = q
            .query_map([id], |r| {
                let raw: String = r.get(0)?;
                let mut v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                v["observedAt"] = json!(r.get::<_, i64>(1)?);
                Ok(v)
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q=c.prepare("SELECT d.body,d.observed_at,d.order_id FROM commerce_disputes d JOIN commerce_orders o ON o.id=d.order_id WHERE json_extract(o.price,'$.sellerId')=?1 ORDER BY d.observed_at DESC LIMIT 100")?;
        let disputes: Vec<Value> = q
            .query_map([id], |r| {
                let raw: String = r.get(0)?;
                let mut v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                v["observedAt"] = json!(r.get::<_, i64>(1)?);
                v["orderId"] = json!(r.get::<_, String>(2)?);
                Ok(v)
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut q=c.prepare("SELECT id,payment_state,delivery_state FROM commerce_orders WHERE json_extract(price,'$.sellerId')=?1 ORDER BY created_at DESC LIMIT 100")?;
        let orders:Vec<Value>=q.query_map([id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"paymentState":r.get::<_,String>(1)?,"deliveryState":r.get::<_,String>(2)?})))?.collect::<std::result::Result<_,_>>()?;
        let provider_fees_pending: i64=c.query_row("SELECT COUNT(*) FROM commerce_orders o WHERE payment_state='paid' AND json_extract(price,'$.sellerId')=?1 AND NOT EXISTS(SELECT 1 FROM commerce_ledger WHERE order_id=o.id AND kind='provider_processing_cost')",[id],|r|r.get(0))?;
        let mut q=c.prepare("SELECT o.id,c.error,json_extract(c.body,'$.refunded'),COALESCE((SELECT SUM(amount) FROM commerce_refunds WHERE order_id=o.id AND state='succeeded'),0),c.checked_at FROM commerce_orders o JOIN commerce_charge_checks c ON c.order_id=o.id WHERE json_extract(o.price,'$.sellerId')=?1 AND (c.error IS NOT NULL OR json_extract(c.body,'$.refunded')!=COALESCE((SELECT SUM(amount) FROM commerce_refunds WHERE order_id=o.id AND state='succeeded'),0)) ORDER BY c.checked_at DESC LIMIT 100")?;
        let issues:Vec<Value>=q.query_map([id],|r|Ok(json!({"orderId":r.get::<_,String>(0)?,"error":r.get::<_,Option<String>>(1)?,"providerRefunded":r.get::<_,Option<i64>>(2)?,"recordedRefunded":r.get::<_,i64>(3)?,"observedAt":r.get::<_,i64>(4)?})))?.collect::<std::result::Result<_,_>>()?;
        Ok(
            json!({"providerFeesPending":provider_fees_pending,"reconciliationIssues":issues,"sellerId":id,"currencies":balances,"ledger":ledger,"orders":orders,"payouts":payouts,"disputes":disputes,"providerReport":report.as_ref().map(|(s,_)|serde_json::from_str::<Value>(s)).transpose()?,"providerObservedAt":report.map(|(_,at)|at),"notice":"Recorded OmaStore proceeds are separate from the connected account balance, which may include outside sales. Provider payouts are account-level observations and are never allocated to individual store orders implicitly. Accounting reserves do not prove provider enforcement."}),
        )
    }
    pub fn commerce_record_costs(&self, mode: &str, o: &ChargeCosts, now: i64) -> Result<()> {
        self.transaction(|t| {
            provider_guard(t, mode)?;
            let id: String = t
                .query_row(
                    "SELECT id FROM commerce_orders WHERE payment_id=?1",
                    [&o.charge],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(Error::new(404, "order_unavailable"))?;
            let (i, stored) = intent(t, &id)?;
            if stored != mode
                || o.account != i.price.connected_account
                || o.currency != i.price.currency
                || json!(o.application_fee) != i.price.amounts()?["serviceFee"]
                || o.provider_fee > 99_999_999
                || o.refunded > i.price.amounts()?["total"].as_u64().unwrap()
                || !crate::commerce_model::provider_id(&o.transaction, "txn_")
            {
                return Err(Error::new(409, "provider_costs_mismatch"));
            }
            t.execute("INSERT INTO commerce_charge_checks VALUES(?1,?2,?3,NULL) ON CONFLICT(order_id) DO UPDATE SET body=excluded.body,checked_at=excluded.checked_at,error=NULL",params![id,serde_json::to_string(o)?,now])?;
            sale(t, &id, now)?;
            post(
                t,
                Entry {
                    source: &format!("processing:{}", o.transaction),
                    seller: &i.price.seller_id,
                    currency: &o.currency,
                    order: Some(&id),
                    kind: "provider_processing_cost",
                    amount: -(o.provider_fee as i64),
                    reserve: 0,
                    detail: json!({"providerTransaction":o.transaction}),
                },
                now,
            )?;
            Ok(())
        })
    }
    pub fn commerce_record_dispute(&self, mode: &str, d: &Dispute, now: i64) -> Result<()> {
        self.transaction(|t|{provider_guard(t,mode)?;let id:String=t.query_row("SELECT id FROM commerce_orders WHERE payment_id=?1",[&d.charge],|r|r.get(0)).optional()?.ok_or(Error::new(404,"order_unavailable"))?;let(i,stored)=intent(t,&id)?;if stored!=mode||d.account!=i.price.connected_account||d.currency!=i.price.currency||d.movements.len()>10||!crate::commerce_model::provider_id(&d.id,"du_")||!["warning_needs_response","warning_under_review","warning_closed","needs_response","under_review","won","lost","prevented"].contains(&d.state.as_str()){return Err(Error::new(409,"dispute_order_mismatch"));}
  sale(t,&id,now)?;for m in &d.movements{if m.currency!=d.currency||!crate::commerce_model::provider_id(&m.id,"txn_"){return Err(Error::new(409,"dispute_movement_mismatch"));}post(t,Entry{source:&format!("dispute:{}",m.id),seller:&i.price.seller_id,currency:&m.currency,order:Some(&id),kind:"provider_dispute_movement",amount:m.net,reserve:0,detail:json!({"disputeId":d.id,"providerFee":m.fee})},now)?;}
  let raw=serde_json::to_string(d)?;let old:Option<String>=t.query_row("SELECT body FROM commerce_disputes WHERE id=?1",[&d.id],|r|r.get(0)).optional()?;
  if let Some(raw)=&old { let prior:Dispute=serde_json::from_str(raw)?; if prior.account!=d.account || prior.charge!=d.charge || prior.amount!=d.amount || prior.currency!=d.currency {return Err(Error::new(409,"dispute_identity_conflict"));}}
  t.execute("INSERT INTO commerce_disputes VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET body=excluded.body,observed_at=excluded.observed_at",params![d.id,id,raw,now])?;if old.as_deref()!=Some(&raw){event(t,&id,"dispute_observed",json!({"id":d.id,"state":d.state,"evidenceDue":d.evidence_due}),now)?;}Ok(())
 })
    }
    pub fn commerce_record_report(
        &self,
        a: &Actor,
        seller_id: &str,
        mode: &str,
        r: &AccountReport,
        now: i64,
    ) -> Result<Value> {
        {
            let c = self.connection()?;
            seller(&c, a, seller_id)?;
        }
        self.record_account_report(seller_id, mode, r, now)?;
        self.commerce_finances(a, seller_id)
    }
    pub(crate) fn record_account_report(
        &self,
        seller_id: &str,
        mode: &str,
        r: &AccountReport,
        now: i64,
    ) -> Result<()> {
        self.transaction(|t|{provider_guard(t,mode)?;let account:String=t.query_row("SELECT account FROM commerce_sellers WHERE id=?1",[seller_id],|r|r.get(0)).optional()?.ok_or(Error::new(404,"seller_unavailable"))?;if r.account!=account||r.balances.len()>20||r.payouts.len()>100{return Err(Error::new(409,"provider_report_mismatch"));}let mut seen=std::collections::BTreeSet::new();for b in &r.balances{crate::commerce_model::exponent(&b.currency)?;if !seen.insert(&b.currency)||b.available.unsigned_abs()>9_000_000_000_000||b.pending.unsigned_abs()>9_000_000_000_000{return Err(Error::new(409,"invalid_provider_balance"));}}
  for p in &r.payouts{if p.account!=account||!crate::commerce_model::provider_id(&p.id,"po_")||!["pending","in_transit","paid","failed","canceled"].contains(&p.state.as_str())||p.amount>9_000_000_000_000{return Err(Error::new(409,"invalid_provider_payout"));}crate::commerce_model::exponent(&p.currency)?;let raw=serde_json::to_string(p)?;let old:Option<String>=t.query_row("SELECT body FROM commerce_payouts WHERE id=?1",[&p.id],|r|r.get(0)).optional()?;
  if let Some(raw)=&old { let prior:crate::commerce_finance_provider::Payout=serde_json::from_str(raw)?; if prior.account!=p.account || prior.amount!=p.amount || prior.currency!=p.currency {return Err(Error::new(409,"payout_identity_conflict"));}}
  t.execute("INSERT INTO commerce_payouts VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET body=excluded.body,observed_at=excluded.observed_at",params![p.id,seller_id,account,raw,now])?;if old.as_deref()!=Some(&raw){t.execute("INSERT INTO commerce_reconciliation_events(seller_id,kind,detail,at) VALUES(?1,'payout_observed',?2,?3)",params![seller_id,raw,now])?;}}
  t.execute("INSERT INTO commerce_provider_reports VALUES(?1,?2,?3,?4) ON CONFLICT(seller_id) DO UPDATE SET account=excluded.account,body=excluded.body,observed_at=excluded.observed_at",params![seller_id,account,serde_json::to_string(r)?,now])?;Ok(())
 })
    }
    pub async fn commerce_reconcile_finances(
        &self,
        a: &Actor,
        seller_id: &str,
        provider: &dyn Finance,
        after: Option<&str>,
        now: i64,
    ) -> Result<Value> {
        {
            let c = self.connection()?;
            seller(&c, a, seller_id)?;
        }
        self.reconcile_seller(seller_id, provider, after, now)
            .await?;
        self.commerce_finances(a, seller_id)
    }
    pub(crate) async fn reconcile_seller(
        &self,
        seller_id: &str,
        provider: &dyn Finance,
        after: Option<&str>,
        now: i64,
    ) -> Result<()> {
        let (account, charges) = {
            let c = self.connection()?;
            let account: String = c
                .query_row(
                    "SELECT account FROM commerce_sellers WHERE id=?1",
                    [seller_id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(Error::new(404, "seller_unavailable"))?;
            provider_guard(&c, provider.mode())?;
            let mut q=c.prepare("SELECT o.id,o.payment_id FROM commerce_orders o LEFT JOIN commerce_charge_checks c ON c.order_id=o.id WHERE o.payment_state='paid' AND json_extract(o.price,'$.sellerId')=?1 ORDER BY COALESCE(c.checked_at,0),o.id LIMIT 20")?;
            let charges: Vec<(String, String)> = q
                .query_map([seller_id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<std::result::Result<_, _>>()?;
            (account, charges)
        };
        for (order, charge) in charges {
            let result = async {
                let costs = provider.charge_costs(&account, &charge).await?;
                self.commerce_record_costs(provider.mode(), &costs, now)
            }
            .await;
            if let Err(e) = result {
                self.connection()?.execute("INSERT INTO commerce_charge_checks VALUES(?1,NULL,?2,?3) ON CONFLICT(order_id) DO UPDATE SET checked_at=excluded.checked_at,error=excluded.error",params![order,now,e.code])?;
            }
        }
        let report = provider.report(&account, after).await?;
        self.record_account_report(seller_id, provider.mode(), &report, now)
    }
    pub async fn commerce_reconcile_dispute(
        &self,
        a: &Actor,
        seller_id: &str,
        id: &str,
        provider: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        let account = {
            let c = self.connection()?;
            seller(&c, a, seller_id)?
        };
        let d = provider.dispute(&account, id).await?;
        if d.account != account || d.id != id {
            return Err(Error::new(409, "dispute_order_mismatch"));
        }
        self.commerce_record_dispute(provider.mode(), &d, now)?;
        self.commerce_finances(a, seller_id)
    }
    pub fn commerce_dispute_packet(&self, a: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, a, "operator")?;
        let (order, raw): (String, String) = c
            .query_row(
                "SELECT order_id,body FROM commerce_disputes WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(Error::new(404, "dispute_unavailable"))?;
        let mut q=c.prepare("SELECT report_url,report_digest,note,at FROM commerce_dispute_evidence WHERE dispute_id=?1 ORDER BY seq DESC LIMIT 20")?;
        let evidence:Vec<Value>=q.query_map([id],|r|Ok(json!({"reportUrl":r.get::<_,String>(0)?,"reportDigest":r.get::<_,String>(1)?,"note":r.get::<_,String>(2)?,"at":r.get::<_,i64>(3)?})))?.collect::<std::result::Result<_,_>>()?;
        let packet = json!({"dispute":serde_json::from_str::<Value>(&raw)?,"order":crate::commerce_checkout::order_projection(&c,&order,false)?,"evidence":evidence});
        Ok(
            json!({"packet":packet,"digest":digest(serde_json::to_vec(&packet)?),"notice":"Private operator evidence packet. Submission to the provider requires the responsible operator's review."}),
        )
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::{commerce_checkout::Purchase, commerce_refunds::Request, commerce_sample::Sample};
    fn setup() -> (tempfile::TempDir, Store, Actor, Actor, Purchase) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("finance.db")).unwrap();
        let c =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&c).unwrap();
        s.commerce_seed_sample(1000).unwrap();
        let av = s.development_login("author", 1000).unwrap();
        let a = s.actor(av["token"].as_str().unwrap(), 1000).unwrap();
        let ov = s.development_login("operator", 1000).unwrap();
        let op = s.actor(ov["token"].as_str().unwrap(), 1000).unwrap();
        s.command(
            &op,
            &crate::nonce().unwrap(),
            crate::drafts::Command::CommercePause {
                paused: false,
                reason: "Fixture finance rehearsal".into(),
            },
            1000,
        )
        .unwrap();
        let prices = s.commerce_prices(None).unwrap();
        let p = prices["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["price"]["id"] == "sample-perpetual")
            .unwrap();
        let buy = Purchase {
            price_id: "sample-perpetual".into(),
            version: 1,
            digest: p["digest"].as_str().unwrap().into(),
            accepted: true,
        };
        (dir, s, a, op, buy)
    }
    async fn paid(s: &Store, a: &Actor, p: Purchase) -> String {
        let id = s
            .commerce_begin(a, &crate::nonce().unwrap(), p, "sample", 1001)
            .unwrap();
        let provider = Sample(s.clone());
        s.commerce_checkout(a, &id, &provider, 1002).await.unwrap();
        provider.capture(a, &id, 1003).unwrap();
        s.commerce_reconcile(a, &id, &provider, 1004).await.unwrap();
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        s.commerce_deliver_local(&id, Some(&issuer), 1005).unwrap();
        id
    }
    fn request(s: &Store, a: &Actor, id: &str, amount: u64, expected: u64) -> String {
        s.commerce_request_refund(
            a,
            &crate::nonce().unwrap(),
            Request {
                order_id: id.into(),
                amount,
                expected_refunded: expected,
                reason: "Fictional buyer refund request".into(),
                accepted: true,
            },
            1010,
        )
        .unwrap()["id"]
            .as_str()
            .unwrap()
            .into()
    }
    #[tokio::test]
    async fn complete_sale_partial_full_refund_and_failed_payout_preserve_exact_fees_and_negative_balances(
    ) {
        let (_dir, s, a, op, p) = setup();
        let id = paid(&s, &a, p).await;
        let provider = Sample(s.clone());
        let f = s
            .commerce_reconcile_finances(&op, "sample-maker", &provider, None, 1006)
            .await
            .unwrap();
        assert_eq!(f["currencies"][0]["recordedProceeds"], 1020);
        assert_eq!(f["providerReport"]["balances"][0]["available"], 1020);
        let refund = request(&s, &a, &id, 200, 0);
        assert!(s
            .commerce_execute_refund(&a, &refund, &provider, 1011)
            .await
            .is_err());
        let r = s
            .commerce_execute_refund(&op, &refund, &provider, 1011)
            .await
            .unwrap();
        assert_eq!(r["state"], "succeeded");
        assert_eq!(r["feeState"], "succeeded");
        assert_eq!(r["feeAmount"], 8);
        s.commerce_execute_refund(&op, &refund, &provider, 1012)
            .await
            .unwrap();
        let rows: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM commerce_sample_provider WHERE kind='refund'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1);
        provider
            .finance_scenario(&op, &id, "payout_pending", 1013)
            .unwrap();
        let pending = s
            .commerce_reconcile_finances(&a, "sample-maker", &provider, None, 1014)
            .await
            .unwrap();
        assert_eq!(pending["providerReport"]["balances"][0]["available"], 328);
        assert_eq!(pending["currencies"][0]["recordedProceeds"], 828);
        provider
            .finance_scenario(&op, &id, "payout_failed", 1015)
            .unwrap();
        let failed = s
            .commerce_reconcile_finances(&a, "sample-maker", &provider, None, 1016)
            .await
            .unwrap();
        assert_eq!(failed["providerReport"]["balances"][0]["available"], 828);
        assert_eq!(failed["payouts"][0]["state"], "failed");
        let again = s
            .commerce_reconcile_finances(&a, "sample-maker", &provider, None, 1017)
            .await
            .unwrap();
        assert_eq!(again["providerReport"]["balances"][0]["available"], 828);
        assert_eq!(again["payouts"].as_array().unwrap().len(), 1);
        s.transaction(|t| {
            reserve(
                t,
                &op,
                Reserve {
                    seller_id: "sample-maker".into(),
                    currency: "gbp".into(),
                    target: 150,
                    expected: 0,
                    reason: "Fixture accounting reserve".into(),
                    report_url: "https://example.com/reserve".into(),
                    report_digest: "a".repeat(64),
                },
                1018,
            )
        })
        .unwrap();
        assert_eq!(
            s.commerce_finances(&a, "sample-maker").unwrap()["currencies"][0]["afterReserve"],
            678
        );
        s.command(
            &op,
            &crate::nonce().unwrap(),
            crate::drafts::Command::CommercePause {
                paused: true,
                reason: "Pause new sales; continue obligations".into(),
            },
            1019,
        )
        .unwrap();
        let last = request(&s, &a, &id, 899, 200);
        let last = s
            .commerce_execute_refund(&op, &last, &provider, 1020)
            .await
            .unwrap();
        assert_eq!(last["feeAmount"], 41);
        assert_eq!(s.commerce_refunds(&a, &id).unwrap()["remaining"], 0);
        let final_report = s
            .commerce_reconcile_finances(&a, "sample-maker", &provider, None, 1021)
            .await
            .unwrap();
        assert_eq!(final_report["currencies"][0]["recordedProceeds"], -30);
        assert_eq!(
            final_report["providerReport"]["balances"][0]["available"],
            -30
        );
        assert_eq!(final_report["currencies"][0]["negative"], true);
        assert!(s.commerce_order(&a, &id).unwrap()["grant"].is_object());
        assert!(s
            .connection()
            .unwrap()
            .execute("DELETE FROM commerce_ledger", [])
            .is_err());
        assert!(s
            .connection()
            .unwrap()
            .execute("UPDATE commerce_refunds SET amount=1", [])
            .is_err());
    }
    #[tokio::test]
    async fn delayed_refunds_dispute_movements_and_evidence_are_auditable_without_double_posting() {
        let (_dir, s, a, op, p) = setup();
        let id = paid(&s, &a, p).await;
        let provider = Sample(s.clone());
        provider
            .finance_scenario(&op, &id, "next_refund_pending", 1010)
            .unwrap();
        let rid = request(&s, &a, &id, 1, 0);
        let pending = s
            .commerce_execute_refund(&op, &rid, &provider, 1011)
            .await
            .unwrap();
        assert_eq!(pending["state"], "pending");
        assert_eq!(s.commerce_refunds(&a, &id).unwrap()["refunded"], 0);
        provider
            .finance_scenario(&op, &id, "settle_refunds", 1012)
            .unwrap();
        s.commerce_poll_refund(&rid, &provider, 1013).await.unwrap();
        assert_eq!(s.commerce_refunds(&a, &id).unwrap()["refunded"], 1);
        let scenario = provider
            .finance_scenario(&op, &id, "dispute_open", 1014)
            .unwrap();
        let did = scenario["disputeId"].as_str().unwrap();
        s.commerce_reconcile_dispute(&op, "sample-maker", did, &provider, 1015)
            .await
            .unwrap();
        s.commerce_reconcile_dispute(&op, "sample-maker", did, &provider, 1016)
            .await
            .unwrap();
        let n: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM commerce_ledger WHERE kind='provider_dispute_movement'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        s.transaction(|t| {
            evidence(
                t,
                &op,
                Evidence {
                    dispute_id: did.into(),
                    report_url: "https://example.com/actual-review".into(),
                    report_digest: "b".repeat(64),
                    note: "Fictional delivery and communication report".into(),
                },
                1017,
            )
        })
        .unwrap();
        assert_eq!(
            s.commerce_dispute_packet(&op, did).unwrap()["packet"]["evidence"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(s.commerce_dispute_packet(&a, did).is_err());
        provider
            .finance_scenario(&op, &id, "dispute_won", 1018)
            .unwrap();
        s.commerce_reconcile_dispute(&op, "sample-maker", did, &provider, 1019)
            .await
            .unwrap();
        s.commerce_reconcile_dispute(&op, "sample-maker", did, &provider, 1020)
            .await
            .unwrap();
        let n: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM commerce_ledger WHERE kind='provider_dispute_movement'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(
            s.commerce_finances(&a, "sample-maker").unwrap()["disputes"][0]["state"],
            "won"
        );
        let outsider = s.development_login("reviewer", 1021).unwrap();
        let outsider = s.actor(outsider["token"].as_str().unwrap(), 1021).unwrap();
        assert!(s.commerce_finances(&outsider, "sample-maker").is_err());
        s.set_role(&op.id, "operator", false, 1021).unwrap();
        assert!(s
            .transaction(|t| evidence(
                t,
                &op,
                Evidence {
                    dispute_id: did.into(),
                    report_url: "https://example.com/revoked".into(),
                    report_digest: "c".repeat(64),
                    note: "Revoked".into()
                },
                1022
            ))
            .is_err());
    }
    #[tokio::test]
    async fn lost_refund_and_fee_replies_reconcile_after_provider_key_expiry_without_new_effects() {
        use crate::commerce_finance_provider::FeeIntent;
        use crate::commerce_provider::Provider;
        let (_dir, s, a, op, p) = setup();
        let id = paid(&s, &a, p).await;
        let provider = Sample(s.clone());
        let rid = request(&s, &a, &id, 200, 0);
        let (charge, fee): (String, String) = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT payment_id,fee_id FROM commerce_orders WHERE id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        // Persist the claim and the external effect, then lose the response before observation.
        s.connection()
            .unwrap()
            .execute(
                "UPDATE commerce_refunds SET state='pending',attempt_at=1000 WHERE id=?1",
                [&rid],
            )
            .unwrap();
        let effect = provider
            .refund("acct_samplemaker", &charge, 200, &format!("refund-{rid}"))
            .await
            .unwrap();
        assert_eq!(s.commerce_refunds(&a, &id).unwrap()["refunded"], 0);
        s.commerce_observe_refund(&rid, "sample", &effect, 1001)
            .unwrap();
        let fi = FeeIntent {
            account: "acct_samplemaker".into(),
            charge: charge.clone(),
            fee,
            currency: "gbp".into(),
            amount: 8,
        };
        s.connection()
            .unwrap()
            .execute(
                "UPDATE commerce_refunds SET fee_attempt_at=1000 WHERE id=?1",
                [&rid],
            )
            .unwrap();
        provider
            .refund_fee(&fi, &format!("fee-refund-{rid}"))
            .await
            .unwrap();
        let r = s
            .commerce_execute_refund(&op, &rid, &provider, 1000 + 24 * 3600)
            .await
            .unwrap();
        assert_eq!(r["feeState"], "succeeded");
        let counts:(i64,i64)=s.connection().unwrap().query_row("SELECT (SELECT COUNT(*) FROM commerce_sample_provider WHERE kind='refund'),(SELECT COUNT(*) FROM commerce_sample_provider WHERE kind='fee_refund')",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(counts, (1, 1));
        // No observed effect after the provider's retention window must never create a new one.
        let pending = request(&s, &a, &id, 10, 200);
        s.connection()
            .unwrap()
            .execute(
                "UPDATE commerce_refunds SET state='pending',attempt_at=1000 WHERE id=?1",
                [&pending],
            )
            .unwrap();
        let error = s
            .commerce_execute_refund(&op, &pending, &provider, 1000 + 24 * 3600)
            .await
            .unwrap_err();
        assert_eq!(error.code, "refund_requires_provider_reconciliation");
        assert_eq!(
            s.connection()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM commerce_sample_provider WHERE kind='refund'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        // A dashboard refund is visible as a mismatch, not silently reported as reconciled proceeds.
        provider
            .refund("acct_samplemaker", &charge, 100, "fixture-dashboard-refund")
            .await
            .unwrap();
        let report = s
            .commerce_reconcile_finances(&op, "sample-maker", &provider, None, 1000 + 24 * 3600)
            .await
            .unwrap();
        assert_eq!(report["reconciliationIssues"][0]["providerRefunded"], 300);
        assert_eq!(report["reconciliationIssues"][0]["recordedRefunded"], 200);
        assert_eq!(report["providerFeesPending"], 0);
        assert!(s
            .connection()
            .unwrap()
            .execute(
                "UPDATE commerce_sellers SET account='acct_different' WHERE id='sample-maker'",
                []
            )
            .is_err());
    }
}
