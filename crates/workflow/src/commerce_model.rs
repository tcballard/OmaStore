//! Commercial facts and integer fee contracts. Fixtures never open real checkout.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
pub const LIVE_COMMERCE_VERIFIED: bool = false;
pub const SERVICE_FEE_BASIS_POINTS: u64 = 500;
pub const STRIPE_VERSION: &str = "2026-08-26.dahlia";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperatingModel {
    pub operator: Option<String>,
    pub provider_approval_report: Option<String>,
    pub seller_of_record: Option<String>,
    pub regions: Vec<String>,
    pub currencies: Vec<String>,
    pub product_types: Vec<String>,
    pub requesting_authors: Vec<String>,
    pub tax_responsibility: Option<String>,
    pub refund_responsibility: Option<String>,
    pub cancellation_terms: Option<String>,
    pub provider_fee_schedule: Option<String>,
    pub payout_schedule: Option<String>,
    pub reserve_policy: Option<String>,
    pub buyer_terms_url: Option<String>,
    pub seller_terms_url: Option<String>,
    pub legal_review_report: Option<String>,
    pub sandbox_lifecycle_report: Option<String>,
}
impl OperatingModel {
    pub fn validate(&self) -> Result<()> {
        if serde_json::to_vec(self)?.len() > 24 * 1024 {
            return Err(Error::new(422, "commercial_model_too_large"));
        }
        for v in [
            &self.operator,
            &self.seller_of_record,
            &self.tax_responsibility,
            &self.refund_responsibility,
            &self.cancellation_terms,
            &self.provider_fee_schedule,
            &self.payout_schedule,
            &self.reserve_policy,
        ]
        .into_iter()
        .flatten()
        {
            crate::bounded(v, 2000)?;
        }
        for v in [
            &self.provider_approval_report,
            &self.buyer_terms_url,
            &self.seller_terms_url,
            &self.legal_review_report,
            &self.sandbox_lifecycle_report,
        ]
        .into_iter()
        .flatten()
        {
            crate::net::public_url(v)?;
        }
        for list in [
            &self.regions,
            &self.currencies,
            &self.product_types,
            &self.requesting_authors,
        ] {
            if list.len() > 100 {
                return Err(Error::new(422, "invalid_commercial_scope"));
            }
            let mut seen = std::collections::BTreeSet::new();
            for v in list {
                crate::bounded(v, 200)?;
                if v.trim() != v || !seen.insert(v) {
                    return Err(Error::new(422, "invalid_commercial_scope"));
                }
            }
        }
        for v in &self.currencies {
            exponent(v)?;
        }
        Ok(())
    }
    pub fn readiness(&self) -> Value {
        let mut missing = Vec::new();
        for (key, value) in [
            ("operator", &self.operator),
            ("providerApprovalReport", &self.provider_approval_report),
            ("sellerOfRecord", &self.seller_of_record),
            ("taxResponsibility", &self.tax_responsibility),
            ("refundResponsibility", &self.refund_responsibility),
            ("cancellationTerms", &self.cancellation_terms),
            ("providerFeeSchedule", &self.provider_fee_schedule),
            ("payoutSchedule", &self.payout_schedule),
            ("reservePolicy", &self.reserve_policy),
            ("buyerTermsUrl", &self.buyer_terms_url),
            ("sellerTermsUrl", &self.seller_terms_url),
            ("legalReviewReport", &self.legal_review_report),
            ("sandboxLifecycleReport", &self.sandbox_lifecycle_report),
        ] {
            if value.as_ref().is_none_or(|s| s.trim().is_empty()) {
                missing.push(key);
            }
        }
        if self.regions.is_empty() {
            missing.push("regions");
        }
        if self.currencies.is_empty() {
            missing.push("currencies");
        }
        if self.product_types.is_empty() {
            missing.push("productTypes");
        }
        if self
            .requesting_authors
            .iter()
            .filter(|s| !s.trim().is_empty())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            < 5
        {
            missing.push("fiveRequestingAuthors");
        }
        json!({"providerCandidate":"Stripe Connect direct charges","apiVersion":STRIPE_VERSION,"missing":missing,"factsComplete":missing.is_empty(),"realCheckoutEnabled":LIVE_COMMERCE_VERIFIED&&missing.is_empty(),"serviceFeeBasisPoints":SERVICE_FEE_BASIS_POINTS,"feeBasis":"Discounted subtotal before tax; provider charges separate","externalIncomeFeeBasisPoints":0,"listingFeeMinorUnits":0,"reviewFeeMinorUnits":0,"updateFeeMinorUnits":0,"owner":"Tom Ballard must appoint the actual commercial operator","notice":"Managed checkout is unavailable. Authors can retain external sales and support. Fixture facts never satisfy the release gate."})
    }
}
pub fn exponent(currency: &str) -> Result<u8> {
    match currency {
        "gbp" | "usd" | "eur" => Ok(2),
        "jpy" => Ok(0),
        _ => Err(Error::new(422, "unsupported_commerce_currency")),
    }
}
pub fn service_fee(subtotal: u64) -> Result<u64> {
    if subtotal > 99_999_999 {
        return Err(Error::new(422, "commerce_amount_out_of_range"));
    }
    Ok((u128::from(subtotal) * u128::from(SERVICE_FEE_BASIS_POINTS) / 10_000) as u64)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Price {
    pub id: String,
    pub version: u32,
    pub app_id: String,
    pub seller_id: String,
    pub seller_name: String,
    pub connected_account: String,
    pub title: String,
    pub currency: String,
    pub subtotal: u64,
    pub discount: u64,
    pub tax: u64,
    pub tax_rate_id: Option<String>,
    pub delivery_kind: String,
    pub billing_interval: Option<String>,
    pub licence: String,
    pub online_requirement: String,
    pub support_url: String,
    pub terms_url: String,
}
impl Price {
    pub fn amounts(&self) -> Result<Value> {
        let exp = exponent(&self.currency)?;
        if self.version == 0
            || self.subtotal == 0
            || self.subtotal > 99_999_999
            || self.discount > self.subtotal
            || self.tax > 99_999_999
            || ![
                "perpetual",
                "service",
                "subscription",
                "paid_upgrade",
                "paid_feature",
                "support",
            ]
            .contains(&self.delivery_kind.as_str())
        {
            return Err(Error::new(422, "invalid_commerce_price"));
        }
        if (self.delivery_kind == "subscription") != self.billing_interval.is_some()
            || self
                .billing_interval
                .as_deref()
                .is_some_and(|s| !["month", "year"].contains(&s))
        {
            return Err(Error::new(422, "invalid_billing_interval"));
        }
        if self
            .tax_rate_id
            .as_ref()
            .is_some_and(|s| !provider_id(s, "txr_"))
        {
            return Err(Error::new(422, "configured_tax_rate_required"));
        }
        if self.tax > 0
            && !self
                .tax_rate_id
                .as_ref()
                .is_some_and(|s| provider_id(s, "txr_"))
        {
            return Err(Error::new(422, "configured_tax_rate_required"));
        }
        for id in [&self.id, &self.app_id, &self.seller_id] {
            if !omastore_catalogue::token(id) {
                return Err(Error::new(422, "invalid_commerce_price"));
            }
        }
        for text in [
            &self.title,
            &self.seller_name,
            &self.licence,
            &self.online_requirement,
        ] {
            crate::bounded(text, 1000)?;
        }
        for url in [&self.support_url, &self.terms_url] {
            crate::net::public_url(url)?;
        }
        if !provider_id(&self.connected_account, "acct_") {
            return Err(Error::new(422, "invalid_seller_account"));
        }
        let net = self.subtotal - self.discount;
        let total = net
            .checked_add(self.tax)
            .filter(|n| *n <= 99_999_999)
            .ok_or(Error::new(422, "commerce_amount_out_of_range"))?;
        if net == 0 {
            return Err(Error::new(422, "use_free_acquisition"));
        }
        Ok(
            json!({"currency":self.currency,"exponent":exp,"subtotal":self.subtotal,"discount":self.discount,"tax":self.tax,"feeBasis":net,"serviceFee":service_fee(net)?,"total":total}),
        )
    }
    pub fn public(&self) -> Result<Value> {
        Ok(
            json!({"id":self.id,"version":self.version,"appId":self.app_id,"sellerId":self.seller_id,"sellerName":self.seller_name,"title":self.title,"amounts":self.amounts()?,"deliveryKind":self.delivery_kind,"billingInterval":self.billing_interval,"licence":self.licence,"onlineRequirement":self.online_requirement,"supportUrl":self.support_url,"termsUrl":self.terms_url}),
        )
    }
}
pub fn provider_id(id: &str, prefix: &str) -> bool {
    id.starts_with(prefix)
        && id.len() > prefix.len()
        && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fee_contract_and_missing_facts_never_enable_live_charges() {
        let m = OperatingModel::default();
        assert_eq!(m.readiness()["realCheckoutEnabled"], false);
        assert_eq!(m.readiness()["externalIncomeFeeBasisPoints"], 0);
        assert_eq!(service_fee(1000).unwrap(), 50);
        assert_eq!(service_fee(999).unwrap(), 49);
        assert!(service_fee(u64::MAX).is_err());
        assert_eq!(exponent("jpy").unwrap(), 0);
        assert!(exponent("xyz").is_err());
    }
}
