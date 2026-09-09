//! Provider contract. No live credential or launch switch is accepted by this build.
use crate::{commerce_model::Price, Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckoutIntent {
    pub order_id: String,
    pub buyer_reference: String,
    pub price: Price,
    pub created_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckoutObservation {
    pub session_id: String,
    pub account: String,
    pub order_id: String,
    pub currency: String,
    pub total: u64,
    pub paid: bool,
    pub buyer_reference: String,
    pub tax: u64,
    pub fee_amount: Option<u64>,
    pub fee_id: Option<String>,
    pub paid_until: Option<i64>,
    pub payment_id: Option<String>,
    pub subscription_id: Option<String>,
    pub customer_id: Option<String>,
    pub url: Option<String>,
    pub livemode: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefundObservation {
    pub id: String,
    pub payment_id: String,
    pub currency: String,
    pub amount: u64,
    pub state: String,
}
#[async_trait]
pub trait Provider: Send + Sync {
    fn mode(&self) -> &'static str;
    async fn checkout(&self, intent: &CheckoutIntent, key: &str) -> Result<CheckoutObservation>;
    async fn lookup(&self, account: &str, session: &str) -> Result<CheckoutObservation>;
    async fn reconcile_creation(
        &self,
        intent: &CheckoutIntent,
    ) -> Result<Option<CheckoutObservation>>;
    async fn refund(
        &self,
        account: &str,
        payment: &str,
        amount: u64,
        key: &str,
    ) -> Result<RefundObservation>;
    async fn refund_status(&self, account: &str, id: &str) -> Result<RefundObservation>;
    async fn cancel_subscription(&self, account: &str, id: &str, key: &str) -> Result<Value>;
}
pub struct Disabled;
#[async_trait]
impl Provider for Disabled {
    fn mode(&self) -> &'static str {
        "disabled"
    }
    async fn checkout(&self, _: &CheckoutIntent, _: &str) -> Result<CheckoutObservation> {
        Err(Error::new(503, "managed_checkout_disabled"))
    }
    async fn lookup(&self, _: &str, _: &str) -> Result<CheckoutObservation> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn reconcile_creation(&self, _: &CheckoutIntent) -> Result<Option<CheckoutObservation>> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn refund(&self, _: &str, _: &str, _: u64, _: &str) -> Result<RefundObservation> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn refund_status(&self, _: &str, _: &str) -> Result<RefundObservation> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
    async fn cancel_subscription(&self, _: &str, _: &str, _: &str) -> Result<Value> {
        Err(Error::new(503, "payment_provider_unconfigured"))
    }
}
