//! Provider observations contain no card, bank-account or customer contact details.
use crate::{
    commerce_provider::{Provider, RefundObservation},
    Error, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeeIntent {
    pub account: String,
    pub charge: String,
    pub fee: String,
    pub currency: String,
    pub amount: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeeRefund {
    pub id: String,
    pub fee: String,
    pub currency: String,
    pub amount: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BalanceEffect {
    pub id: String,
    pub currency: String,
    pub net: i64,
    pub fee: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dispute {
    pub id: String,
    pub account: String,
    pub charge: String,
    pub amount: u64,
    pub currency: String,
    pub state: String,
    pub evidence_due: Option<i64>,
    pub movements: Vec<BalanceEffect>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payout {
    pub id: String,
    pub account: String,
    pub amount: u64,
    pub currency: String,
    pub state: String,
    pub arrival_at: Option<i64>,
    pub failure_code: Option<String>,
    pub balance_transaction: Option<String>,
    pub failure_transaction: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Balance {
    pub currency: String,
    pub available: i64,
    pub pending: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountReport {
    pub account: String,
    pub balances: Vec<Balance>,
    pub payouts: Vec<Payout>,
    pub has_more_payouts: bool,
    pub next_payout_cursor: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChargeCosts {
    pub charge: String,
    pub account: String,
    pub currency: String,
    pub transaction: String,
    pub provider_fee: u64,
    pub application_fee: u64,
    pub refunded: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Invoice {
    pub id: String,
    pub account: String,
    pub root_order: String,
    pub subscription: String,
    pub customer: String,
    pub charge: String,
    pub fee: String,
    pub currency: String,
    pub total: u64,
    pub tax: u64,
    pub application_fee: u64,
    pub period_start: i64,
    pub period_end: i64,
    pub paid: bool,
}
#[async_trait]
pub trait Finance: Provider {
    async fn find_refund(
        &self,
        account: &str,
        charge: &str,
        key: &str,
    ) -> Result<Option<RefundObservation>>;
    async fn refund_fee(&self, intent: &FeeIntent, key: &str) -> Result<FeeRefund>;
    async fn find_fee_refund(&self, intent: &FeeIntent, key: &str) -> Result<Option<FeeRefund>>;
    async fn dispute(&self, account: &str, id: &str) -> Result<Dispute>;
    async fn report(&self, account: &str, after: Option<&str>) -> Result<AccountReport>;
    async fn charge_costs(&self, account: &str, charge: &str) -> Result<ChargeCosts>;
    async fn invoice(&self, account: &str, id: &str) -> Result<Invoice>;
    async fn invoices(
        &self,
        account: &str,
        subscription: &str,
        after: Option<&str>,
    ) -> Result<Value>;
}
#[async_trait]
impl Finance for crate::commerce_provider::Disabled {
    async fn find_refund(&self, _: &str, _: &str, _: &str) -> Result<Option<RefundObservation>> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn refund_fee(&self, _: &FeeIntent, _: &str) -> Result<FeeRefund> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn find_fee_refund(&self, _: &FeeIntent, _: &str) -> Result<Option<FeeRefund>> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn dispute(&self, _: &str, _: &str) -> Result<Dispute> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn report(&self, _: &str, _: Option<&str>) -> Result<AccountReport> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn charge_costs(&self, _: &str, _: &str) -> Result<ChargeCosts> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn invoice(&self, _: &str, _: &str) -> Result<Invoice> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn invoices(&self, _: &str, _: &str, _: Option<&str>) -> Result<Value> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
}
