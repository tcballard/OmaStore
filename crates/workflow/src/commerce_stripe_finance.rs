//! Test-only financial observations and exact fee refunds on the fixed provider origin.
use crate::{
    commerce_finance_provider::*,
    commerce_model::provider_id,
    commerce_provider::RefundObservation,
    commerce_stripe::{field, number, refund_observation, StripeTest},
    Error, Result,
};
use async_trait::async_trait;
use serde_json::{json, Value};
fn id(s: &str, p: &str) -> Result<()> {
    if provider_id(s, p) {
        Ok(())
    } else {
        Err(Error::new(422, "invalid_provider_identity"))
    }
}
fn data(v: &Value) -> Result<&Vec<Value>> {
    v["data"]
        .as_array()
        .filter(|a| a.len() <= 100)
        .ok_or(Error::new(503, "invalid_provider_response"))
}
fn signed(v: &Value, key: &str) -> Result<i64> {
    v[key]
        .as_i64()
        .filter(|n| n.unsigned_abs() <= 9_000_000_000_000)
        .ok_or(Error::new(503, "invalid_provider_response"))
}
fn fee_refund(v: Value) -> Result<FeeRefund> {
    let fid = field(&v, "id")?;
    id(&fid, "fr_")?;
    Ok(FeeRefund {
        id: fid,
        fee: field(&v, "fee")?,
        currency: field(&v, "currency")?,
        amount: number(&v, "amount")?,
    })
}
impl StripeTest {
    async fn matching(
        &self,
        path: &str,
        account: Option<&str>,
        params: Vec<(String, String)>,
        key: &str,
    ) -> Result<Option<Value>> {
        let mut after = None;
        let mut found = None;
        for _ in 0..10 {
            let mut f = params.clone();
            f.push(("limit".into(), "100".into()));
            if let Some(a) = after {
                f.push(("starting_after".into(), a));
            }
            let v = self.request("GET", path, account, None, f).await?;
            for r in data(&v)? {
                if r["metadata"]["omastore_key"] == key {
                    if found.is_some() {
                        return Err(Error::new(409, "multiple_provider_effects_require_support"));
                    }
                    found = Some(r.clone());
                }
            }
            if v["has_more"] == false {
                return Ok(found);
            }
            after = data(&v)?
                .last()
                .and_then(|r| r["id"].as_str())
                .map(str::to_owned);
            if after.is_none() {
                break;
            }
        }
        Err(Error::new(503, "financial_reconciliation_incomplete"))
    }
    async fn fee_authority(&self, i: &FeeIntent) -> Result<()> {
        id(&i.account, "acct_")?;
        id(&i.charge, "ch_")?;
        id(&i.fee, "fee_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/application_fees/{}", i.fee),
                None,
                None,
                vec![],
            )
            .await?;
        if v["account"] != i.account
            || v["charge"] != i.charge
            || v["currency"] != i.currency
            || v["livemode"] != false
        {
            return Err(Error::new(409, "application_fee_authority_mismatch"));
        }
        Ok(())
    }
    async fn charge(&self, account: &str, charge: &str) -> Result<Value> {
        id(charge, "ch_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/charges/{charge}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        if v["livemode"] != false || v["id"] != charge {
            return Err(Error::new(409, "charge_identity_mismatch"));
        }
        Ok(v)
    }
}
#[async_trait]
impl Finance for StripeTest {
    async fn subscription(&self, account: &str, sid: &str) -> Result<Subscription> {
        id(sid, "sub_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/subscriptions/{sid}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        let items = data(&v["items"])?;
        if v["id"] != sid
            || v["livemode"] != false
            || v["items"]["has_more"] != false
            || items.len() != 1
        {
            return Err(Error::new(409, "subscription_identity_mismatch"));
        }
        Ok(Subscription {
            id: sid.into(),
            account: account.into(),
            root_order: field(&v["metadata"], "omastore_order")?,
            customer: field(&v, "customer")?,
            state: field(&v, "status")?,
            cancel_at_period_end: v["cancel_at_period_end"]
                .as_bool()
                .ok_or(Error::new(503, "invalid_provider_response"))?,
            current_period_end: signed(&items[0], "current_period_end")?,
        })
    }

    async fn find_refund(
        &self,
        account: &str,
        charge: &str,
        key: &str,
    ) -> Result<Option<RefundObservation>> {
        id(charge, "ch_")?;
        self.matching(
            "/v1/refunds",
            Some(account),
            vec![("charge".into(), charge.into())],
            key,
        )
        .await?
        .map(refund_observation)
        .transpose()
    }
    async fn refund_fee(&self, i: &FeeIntent, key: &str) -> Result<FeeRefund> {
        self.fee_authority(i).await?;
        if i.amount == 0 || i.amount > 99_999_999 {
            return Err(Error::new(422, "invalid_fee_refund_amount"));
        }
        fee_refund(
            self.request(
                "POST",
                &format!("/v1/application_fees/{}/refunds", i.fee),
                None,
                Some(key),
                vec![
                    ("amount".into(), i.amount.to_string()),
                    ("metadata[omastore_key]".into(), key.into()),
                ],
            )
            .await?,
        )
    }
    async fn find_fee_refund(&self, i: &FeeIntent, key: &str) -> Result<Option<FeeRefund>> {
        self.fee_authority(i).await?;
        self.matching(
            &format!("/v1/application_fees/{}/refunds", i.fee),
            None,
            vec![],
            key,
        )
        .await?
        .map(fee_refund)
        .transpose()
    }
    async fn dispute(&self, account: &str, did: &str) -> Result<Dispute> {
        id(did, "du_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/disputes/{did}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        if v["livemode"] != false || v["id"] != did {
            return Err(Error::new(409, "dispute_identity_mismatch"));
        }
        let rows = v["balance_transactions"]
            .as_array()
            .filter(|a| a.len() <= 10)
            .ok_or(Error::new(503, "invalid_provider_response"))?;
        let mut movements = Vec::new();
        for m in rows {
            movements.push(BalanceEffect {
                id: field(m, "id")?,
                currency: field(m, "currency")?,
                net: signed(m, "net")?,
                fee: number(m, "fee")?,
            });
        }
        Ok(Dispute {
            id: did.into(),
            account: account.into(),
            charge: field(&v, "charge")?,
            amount: number(&v, "amount")?,
            currency: field(&v, "currency")?,
            state: field(&v, "status")?,
            evidence_due: v["evidence_details"]["due_by"].as_i64(),
            movements,
        })
    }
    async fn report(&self, account: &str, after: Option<&str>) -> Result<AccountReport> {
        let v = self
            .request("GET", "/v1/balance", Some(account), None, vec![])
            .await?;
        if v["livemode"] != false {
            return Err(Error::new(409, "live_payment_rejected"));
        }
        let mut sums = std::collections::BTreeMap::<String, (i64, i64)>::new();
        for (kind, index) in [("available", 0), ("pending", 1)] {
            let rows = v[kind]
                .as_array()
                .filter(|a| a.len() <= 20)
                .ok_or(Error::new(503, "invalid_provider_response"))?;
            for r in rows {
                let currency = field(r, "currency")?;
                crate::commerce_model::exponent(&currency)?;
                let n = signed(r, "amount")?;
                let b = sums.entry(currency).or_default();
                if index == 0 {
                    b.0 = n;
                } else {
                    b.1 = n;
                }
            }
        }
        let mut f = vec![("limit".into(), "100".into())];
        if let Some(a) = after {
            id(a, "po_")?;
            f.push(("starting_after".into(), a.into()));
        }
        let p = self
            .request("GET", "/v1/payouts", Some(account), None, f)
            .await?;
        let mut payouts = Vec::new();
        for r in data(&p)? {
            if r["livemode"] != false {
                return Err(Error::new(409, "live_payment_rejected"));
            }
            payouts.push(Payout {
                id: field(r, "id")?,
                account: account.into(),
                amount: r["amount"]
                    .as_u64()
                    .filter(|n| *n <= 9_000_000_000_000)
                    .ok_or(Error::new(503, "invalid_provider_response"))?,
                currency: field(r, "currency")?,
                state: field(r, "status")?,
                arrival_at: r["arrival_date"].as_i64(),
                failure_code: r["failure_code"].as_str().map(str::to_owned),
                balance_transaction: r["balance_transaction"].as_str().map(str::to_owned),
                failure_transaction: r["failure_balance_transaction"].as_str().map(str::to_owned),
            });
        }
        Ok(AccountReport {
            account: account.into(),
            balances: sums
                .into_iter()
                .map(|(currency, (available, pending))| Balance {
                    currency,
                    available,
                    pending,
                })
                .collect(),
            next_payout_cursor: payouts.last().map(|p| p.id.clone()),
            payouts,
            has_more_payouts: p["has_more"]
                .as_bool()
                .ok_or(Error::new(503, "invalid_provider_response"))?,
        })
    }
    async fn charge_costs(&self, account: &str, charge: &str) -> Result<ChargeCosts> {
        let c = self.charge(account, charge).await?;
        let transaction = field(&c, "balance_transaction")?;
        id(&transaction, "txn_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/balance_transactions/{transaction}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        let currency = field(&c, "currency")?;
        if v["currency"] != currency {
            return Err(Error::new(409, "settlement_currency_requires_review"));
        }
        let application = c["application_fee_amount"].as_u64().unwrap_or(0);
        let details = v["fee_details"]
            .as_array()
            .ok_or(Error::new(503, "provider_fee_details_unavailable"))?;
        let mut fee = 0u64;
        let mut app = 0u64;
        for d in details {
            if d["currency"] != currency {
                return Err(Error::new(409, "settlement_currency_requires_review"));
            }
            let n = number(d, "amount")?;
            if d["type"] == "application_fee" {
                app = app
                    .checked_add(n)
                    .ok_or(Error::new(503, "invalid_provider_response"))?;
            } else {
                fee = fee
                    .checked_add(n)
                    .ok_or(Error::new(503, "invalid_provider_response"))?;
            }
        }
        if app != application || app.checked_add(fee) != v["fee"].as_u64() {
            return Err(Error::new(503, "provider_fee_details_unavailable"));
        }
        Ok(ChargeCosts {
            charge: charge.into(),
            account: account.into(),
            currency,
            transaction,
            provider_fee: fee,
            application_fee: application,
            refunded: number(&c, "amount_refunded")?,
        })
    }
    async fn invoice(&self, account: &str, iid: &str) -> Result<Invoice> {
        id(iid, "in_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/invoices/{iid}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        if v["livemode"] != false
            || v["status"] != "paid"
            || v["amount_remaining"] != 0
            || v["amount_paid"] != v["total"]
        {
            return Err(Error::new(409, "invoice_payment_unconfirmed"));
        }
        let sid = field(&v["parent"]["subscription_details"], "subscription")?;
        id(&sid, "sub_")?;
        let s = self
            .request(
                "GET",
                &format!("/v1/subscriptions/{sid}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        let root = field(&s["metadata"], "omastore_order")?;
        let payments = self
            .request(
                "GET",
                "/v1/invoice_payments",
                Some(account),
                None,
                vec![
                    ("invoice".into(), iid.into()),
                    ("status".into(), "paid".into()),
                    ("limit".into(), "2".into()),
                ],
            )
            .await?;
        let rows = data(&payments)?;
        if payments["has_more"] != false
            || rows.len() != 1
            || rows[0]["amount_paid"] != v["total"]
            || rows[0]["livemode"] != false
        {
            return Err(Error::new(409, "invoice_payment_mismatch"));
        }
        let pi = field(&rows[0]["payment"], "payment_intent")?;
        id(&pi, "pi_")?;
        let payment = self
            .request(
                "GET",
                &format!("/v1/payment_intents/{pi}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        let ch = field(&payment, "latest_charge")?;
        let charge = self.charge(account, &ch).await?;
        if charge["paid"] != true
            || charge["captured"] != true
            || charge["amount_captured"] != v["total"]
            || charge["currency"] != v["currency"]
        {
            return Err(Error::new(409, "invoice_charge_mismatch"));
        }
        let lines = data(&v["lines"])?;
        if v["lines"]["has_more"] != false || lines.len() != 1 {
            return Err(Error::new(409, "subscription_invoice_shape_unsupported"));
        }
        let start = lines[0]["period"]["start"]
            .as_i64()
            .ok_or(Error::new(503, "invalid_provider_response"))?;
        let end = lines[0]["period"]["end"]
            .as_i64()
            .ok_or(Error::new(503, "invalid_provider_response"))?;
        let total = number(&v, "total")?;
        let before_tax = number(&v, "total_excluding_tax")?;
        let tax = total
            .checked_sub(before_tax)
            .ok_or(Error::new(409, "invalid_invoice_tax"))?;
        Ok(Invoice {
            id: iid.into(),
            account: account.into(),
            root_order: root,
            subscription: sid,
            customer: field(&v, "customer")?,
            charge: ch,
            fee: field(&charge, "application_fee")?,
            currency: field(&v, "currency")?,
            total,
            tax,
            application_fee: number(&charge, "application_fee_amount")?,
            period_start: start,
            period_end: end,
            paid: true,
        })
    }
    async fn invoices(
        &self,
        account: &str,
        subscription: &str,
        after: Option<&str>,
    ) -> Result<Value> {
        id(subscription, "sub_")?;
        let mut f = vec![
            ("subscription".into(), subscription.into()),
            ("status".into(), "paid".into()),
            ("limit".into(), "100".into()),
        ];
        if let Some(a) = after {
            id(a, "in_")?;
            f.push(("starting_after".into(), a.into()));
        }
        let v = self
            .request("GET", "/v1/invoices", Some(account), None, f)
            .await?;
        let ids = data(&v)?
            .iter()
            .map(|v| field(v, "id"))
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"ids":ids,"hasMore":v["has_more"],"nextCursor":ids.last()}))
    }
}
