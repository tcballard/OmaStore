//! Fixed-origin Stripe test-mode adapter. Live keys and arbitrary endpoints are rejected.
use crate::{
    commerce_model::{provider_id, STRIPE_VERSION},
    commerce_provider::{CheckoutIntent, CheckoutObservation, Provider, RefundObservation},
    Error, Result,
};
use async_trait::async_trait;
use serde_json::{json, Value};
pub struct StripeTest {
    secret: String,
    return_origin: String,
}
impl StripeTest {
    pub fn new(secret: String, return_origin: String) -> Result<Self> {
        if !secret.starts_with(&["sk", "test", ""].join("_"))
            || secret.len() < 24
            || secret.len() > 256
            || !secret
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(Error::new(503, "stripe_test_key_required"));
        }
        let u = crate::net::public_url(&return_origin)?;
        if u.path() != "/" || u.query().is_some() {
            return Err(Error::new(503, "commerce_return_origin_required"));
        }
        Ok(Self {
            secret,
            return_origin: return_origin.trim_end_matches('/').into(),
        })
    }
    pub(crate) async fn request(
        &self,
        method: &str,
        path: &str,
        account: Option<&str>,
        key: Option<&str>,
        form: Vec<(String, String)>,
    ) -> Result<Value> {
        if let Some(a) = account {
            if !provider_id(a, "acct_") {
                return Err(Error::new(422, "invalid_seller_account"));
            }
        }
        if !path.starts_with("/v1/") || path.contains("..") || path.contains('?') {
            return Err(Error::new(422, "invalid_provider_path"));
        }
        let mut url = crate::net::public_url(&format!("https://api.stripe.com{path}"))?;
        let c = crate::net::client_for(&url).await?;
        let mut req = if method == "GET" {
            url.query_pairs_mut().extend_pairs(&form);
            c.get(url)
        } else {
            c.post(url).form(&form)
        };
        req = req
            .bearer_auth(&self.secret)
            .header("Stripe-Version", STRIPE_VERSION);
        if let Some(a) = account {
            req = req.header("Stripe-Account", a);
        }
        if let Some(k) = key {
            req = req.header("Idempotency-Key", k);
        }
        let response = req
            .send()
            .await
            .map_err(|_| Error::new(503, "payment_provider_ambiguous"))?;
        // Never project provider bodies, card/billing data or secrets into diagnostics.
        let bytes = crate::net::read_response(response, 256 * 1024)
            .await
            .map_err(|_| Error::new(503, "payment_provider_ambiguous"))?
            .bytes;
        let v: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Error::new(503, "invalid_provider_response"))?;
        if v.get("livemode").is_some_and(|b| b != &json!(false)) {
            return Err(Error::new(503, "live_payment_rejected"));
        }
        Ok(v)
    }
    async fn observe(&self, account: &str, s: Value) -> Result<CheckoutObservation> {
        if s["livemode"] != false || s["object"] != "checkout.session" {
            return Err(Error::new(503, "invalid_provider_response"));
        }
        let sid = field(&s, "id")?;
        id(&sid, "cs_test_")?;
        let paid = s["payment_status"] == "paid";
        let subscription = s["subscription"].as_str().map(str::to_owned);
        let mut paid_until = None;
        let mut pi = s["payment_intent"].as_str().map(str::to_owned);
        if paid && subscription.is_some() {
            let invoice = field(&s, "invoice")?;
            id(&invoice, "in_")?;
            let inv = self
                .request(
                    "GET",
                    &format!("/v1/invoices/{invoice}"),
                    Some(account),
                    None,
                    vec![],
                )
                .await?;
            if inv["livemode"] != false
                || inv["status"] != "paid"
                || inv["amount_paid"] != s["amount_total"]
            {
                return Err(Error::new(503, "invoice_payment_mismatch"));
            }
            paid_until = inv["lines"]["data"]
                .as_array()
                .and_then(|a| a.iter().filter_map(|l| l["period"]["end"].as_i64()).min());
            let payments = self
                .request(
                    "GET",
                    "/v1/invoice_payments",
                    Some(account),
                    None,
                    vec![
                        ("invoice".into(), invoice),
                        ("status".into(), "paid".into()),
                        ("limit".into(), "2".into()),
                    ],
                )
                .await?;
            let rows = payments["data"]
                .as_array()
                .ok_or(Error::new(503, "invalid_provider_response"))?;
            if payments["has_more"] != false
                || rows.len() != 1
                || rows[0]["livemode"] != false
                || rows[0]["amount_paid"] != s["amount_total"]
            {
                return Err(Error::new(503, "invoice_payment_mismatch"));
            }
            pi = rows[0]["payment"]["payment_intent"]
                .as_str()
                .map(str::to_owned);
        }
        let (payment_id, fee_amount, fee_id) = if paid {
            let pi = pi.ok_or(Error::new(503, "payment_identity_missing"))?;
            id(&pi, "pi_")?;
            let p = self
                .request(
                    "GET",
                    &format!("/v1/payment_intents/{pi}"),
                    Some(account),
                    None,
                    vec![],
                )
                .await?;
            let charge = field(&p, "latest_charge")?;
            id(&charge, "ch_")?;
            let ch = self
                .request(
                    "GET",
                    &format!("/v1/charges/{charge}"),
                    Some(account),
                    None,
                    vec![],
                )
                .await?;
            if ch["paid"] != true
                || ch["captured"] != true
                || ch["livemode"] != false
                || ch["currency"] != s["currency"]
                || ch["amount_captured"] != s["amount_total"]
            {
                return Err(Error::new(503, "payment_amount_mismatch"));
            }
            (
                Some(charge),
                Some(ch["application_fee_amount"].as_u64().unwrap_or(0)),
                ch["application_fee"].as_str().map(str::to_owned),
            )
        } else {
            (None, None, None)
        };
        let url = s["url"].as_str().map(str::to_owned);
        if let Some(u) = &url {
            checkout_url(u)?;
        }
        Ok(CheckoutObservation {
            session_id: sid,
            account: account.into(),
            order_id: field(&s["metadata"], "omastore_order")?,
            buyer_reference: field(&s["metadata"], "omastore_buyer")?,
            currency: field(&s, "currency")?,
            total: number(&s, "amount_total")?,
            tax: number(&s["total_details"], "amount_tax")?,
            paid,
            payment_id,
            fee_amount,
            fee_id,
            subscription_id: subscription,
            customer_id: s["customer"].as_str().map(str::to_owned),
            paid_until,
            url,
            livemode: false,
        })
    }
}
pub fn checkout_url(raw: &str) -> Result<()> {
    let u = url::Url::parse(raw).map_err(|_| Error::new(503, "untrusted_checkout_url"))?;
    if raw.len() > 4096
        || u.scheme() != "https"
        || u.host_str() != Some("checkout.stripe.com")
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some_and(|p| p != 443)
        || !u.path().starts_with("/c/pay/")
    {
        return Err(Error::new(503, "untrusted_checkout_url"));
    }
    Ok(())
}
fn id(v: &str, prefix: &str) -> Result<()> {
    if provider_id(v, prefix) {
        Ok(())
    } else {
        Err(Error::new(503, "invalid_provider_identity"))
    }
}
pub(crate) fn field(v: &Value, key: &str) -> Result<String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256)
        .map(str::to_owned)
        .ok_or(Error::new(503, "invalid_provider_response"))
}
pub(crate) fn number(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .filter(|n| *n <= 99_999_999)
        .ok_or(Error::new(503, "invalid_provider_response"))
}
#[async_trait]
impl Provider for StripeTest {
    fn mode(&self) -> &'static str {
        "stripe_test"
    }
    async fn checkout(&self, i: &CheckoutIntent, key: &str) -> Result<CheckoutObservation> {
        let p = &i.price;
        let a = p.amounts()?;
        let subscription = p.delivery_kind == "subscription";
        // Provider percentage includes tax and rounds independently. Only this exact fixed-price
        // subset preserves the agreed pre-tax fee without invoice intervention.
        if subscription && (p.tax != 0 || !(p.subtotal - p.discount).is_multiple_of(20)) {
            return Err(Error::new(422, "subscription_fee_contract_unsupported"));
        }
        let mut f = vec![
            (
                "mode".into(),
                if subscription {
                    "subscription"
                } else {
                    "payment"
                }
                .into(),
            ),
            (
                "success_url".into(),
                format!("{}/api/v1/commerce/return", self.return_origin),
            ),
            (
                "cancel_url".into(),
                format!("{}/api/v1/commerce/return", self.return_origin),
            ),
            ("client_reference_id".into(), i.buyer_reference.clone()),
            ("metadata[omastore_order]".into(), i.order_id.clone()),
            ("metadata[omastore_buyer]".into(), i.buyer_reference.clone()),
            (
                "line_items[0][price_data][currency]".into(),
                p.currency.clone(),
            ),
            (
                "line_items[0][price_data][unit_amount]".into(),
                (p.subtotal - p.discount).to_string(),
            ),
            (
                "line_items[0][price_data][product_data][name]".into(),
                p.title.clone(),
            ),
            (
                "line_items[0][price_data][tax_behavior]".into(),
                "exclusive".into(),
            ),
            ("line_items[0][quantity]".into(), "1".into()),
        ];
        if subscription {
            f.push((
                "line_items[0][price_data][recurring][interval]".into(),
                p.billing_interval.clone().unwrap(),
            ));
            f.push((
                "subscription_data[application_fee_percent]".into(),
                "5".into(),
            ));
            f.push((
                "subscription_data[metadata][omastore_order]".into(),
                i.order_id.clone(),
            ));
        } else {
            f.push(("customer_creation".into(), "always".into()));
            if a["serviceFee"].as_u64().unwrap() > 0 {
                f.push((
                    "payment_intent_data[application_fee_amount]".into(),
                    a["serviceFee"].to_string(),
                ));
            }
            f.push((
                "payment_intent_data[metadata][omastore_order]".into(),
                i.order_id.clone(),
            ));
        }
        if let Some(tax) = &p.tax_rate_id {
            f.push(("line_items[0][tax_rates][0]".into(), tax.clone()));
        }
        let v = self
            .request(
                "POST",
                "/v1/checkout/sessions",
                Some(&p.connected_account),
                Some(key),
                f,
            )
            .await?;
        self.observe(&p.connected_account, v).await
    }
    async fn lookup(&self, account: &str, session: &str) -> Result<CheckoutObservation> {
        id(session, "cs_test_")?;
        let v = self
            .request(
                "GET",
                &format!("/v1/checkout/sessions/{session}"),
                Some(account),
                None,
                vec![],
            )
            .await?;
        self.observe(account, v).await
    }
    async fn reconcile_creation(&self, i: &CheckoutIntent) -> Result<Option<CheckoutObservation>> {
        let mut after = None;
        let mut found = None;
        for _ in 0..10 {
            let mut f = vec![
                ("created[gte]".into(), (i.created_at - 300).to_string()),
                ("created[lte]".into(), (i.created_at + 86400).to_string()),
                ("limit".into(), "100".into()),
            ];
            if let Some(a) = after {
                f.push(("starting_after".into(), a));
            }
            let v = self
                .request(
                    "GET",
                    "/v1/checkout/sessions",
                    Some(&i.price.connected_account),
                    None,
                    f,
                )
                .await?;
            let data = v["data"]
                .as_array()
                .ok_or(Error::new(503, "invalid_provider_response"))?;
            for s in data {
                if s["metadata"]["omastore_order"] == i.order_id {
                    if found.is_some() {
                        return Err(Error::new(
                            409,
                            "multiple_checkout_sessions_require_support",
                        ));
                    }
                    found = Some(self.observe(&i.price.connected_account, s.clone()).await?);
                }
            }
            if v["has_more"] == false {
                return Ok(found);
            }
            after = data
                .last()
                .and_then(|s| s["id"].as_str())
                .map(str::to_owned);
            if after.is_none() {
                break;
            }
        }
        Err(Error::new(503, "checkout_reconciliation_incomplete"))
    }
    async fn refund(
        &self,
        account: &str,
        payment: &str,
        amount: u64,
        key: &str,
    ) -> Result<RefundObservation> {
        id(payment, "ch_")?;
        if amount == 0 || amount > 99_999_999 {
            return Err(Error::new(422, "invalid_refund_amount"));
        }
        let v = self
            .request(
                "POST",
                "/v1/refunds",
                Some(account),
                Some(key),
                vec![
                    ("charge".into(), payment.into()),
                    ("amount".into(), amount.to_string()),
                    ("refund_application_fee".into(), "false".into()),
                    ("metadata[omastore_key]".into(), key.into()),
                ],
            )
            .await?;
        refund_observation(v)
    }
    async fn refund_status(&self, account: &str, rid: &str) -> Result<RefundObservation> {
        id(rid, "re_")?;
        refund_observation(
            self.request(
                "GET",
                &format!("/v1/refunds/{rid}"),
                Some(account),
                None,
                vec![],
            )
            .await?,
        )
    }
    async fn cancel_subscription(&self, account: &str, sid: &str, key: &str) -> Result<Value> {
        id(sid, "sub_")?;
        let v = self
            .request(
                "POST",
                &format!("/v1/subscriptions/{sid}"),
                Some(account),
                Some(key),
                vec![
                    ("cancel_at_period_end".into(), "true".into()),
                    ("proration_behavior".into(), "none".into()),
                ],
            )
            .await?;
        if v["id"] != sid || v["livemode"] != false || v["cancel_at_period_end"] != true {
            return Err(Error::new(503, "subscription_cancel_unconfirmed"));
        }
        Ok(json!({"id":sid,"cancelAtPeriodEnd":true,"status":v["status"]}))
    }
}
pub(crate) fn refund_observation(v: Value) -> Result<RefundObservation> {
    let rid = field(&v, "id")?;
    id(&rid, "re_")?;
    let state = field(&v, "status")?;
    if ![
        "pending",
        "requires_action",
        "succeeded",
        "failed",
        "canceled",
    ]
    .contains(&state.as_str())
    {
        return Err(Error::new(503, "invalid_refund_state"));
    }
    Ok(RefundObservation {
        id: rid,
        payment_id: field(&v, "charge")?,
        currency: field(&v, "currency")?,
        amount: number(&v, "amount")?,
        state,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_and_checkout_targets_are_strict() {
        assert!(StripeTest::new(
            "live-key-is-never-supported".into(),
            "https://example.com".into()
        )
        .is_err());
        assert!(checkout_url("https://checkout.stripe.com.evil.example/c/pay/x").is_err());
        assert!(
            checkout_url("https://checkout.stripe.com/c/pay/cs_test_x#provider-fragment").is_ok()
        );
        assert!(checkout_url("file:///tmp/payment").is_err());
    }
}
