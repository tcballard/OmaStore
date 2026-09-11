//! Typed authenticated desktop/service lifecycle commands.
use crate::{
    commerce_finance_provider::Finance, commerce_refunds::Request, Actor, Error, Result, Store,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Sellers,
    Finances {
        seller_id: String,
        #[serde(default)]
        refresh: bool,
        cursor: Option<String>,
    },
    Refunds {
        id: String,
    },
    RequestRefund {
        request: Box<Request>,
    },
    ExecuteRefund {
        id: String,
    },
    RejectRefund {
        id: String,
        reason: String,
    },
    PollRefund {
        id: String,
    },
    Subscription {
        id: String,
        #[serde(default)]
        refresh: bool,
    },
    CancelSubscription {
        id: String,
        accepted: bool,
    },
    Dispute {
        seller_id: String,
        id: String,
    },
    DisputePacket {
        id: String,
    },
    InvoiceIssues {
        seller_id: String,
    },
    RetryInvoice {
        seller_id: String,
        id: String,
    },
    Support {
        id: String,
        note: Option<String>,
    },
    SampleScenario {
        id: String,
        scenario: String,
    },
}
impl Store {
    pub async fn commerce_lifecycle_action(
        &self,
        a: &Actor,
        key: &str,
        action: Action,
        p: &dyn Finance,
        now: i64,
    ) -> Result<Value> {
        match action {
            Action::Sellers => self.commerce_sellers(a),
            Action::Finances {
                seller_id,
                refresh,
                cursor,
            } => {
                if refresh {
                    self.commerce_reconcile_finances(a, &seller_id, p, cursor.as_deref(), now)
                        .await
                } else {
                    self.commerce_finances(a, &seller_id)
                }
            }
            Action::Refunds { id } => self.commerce_refunds(a, &id),
            Action::RequestRefund { request } => {
                self.commerce_request_refund(a, key, *request, now)
            }
            Action::ExecuteRefund { id } => self.commerce_execute_refund(a, &id, p, now).await,
            Action::RejectRefund { id, reason } => {
                self.commerce_reject_refund(a, &id, &reason, now)
            }
            Action::PollRefund { id } => {
                let order = {
                    let c = self.connection()?;
                    let order: String = c.query_row(
                        "SELECT order_id FROM commerce_refunds WHERE id=?1",
                        [&id],
                        |r| r.get(0),
                    )?;
                    crate::commerce_checkout::access(&c, a, &order)?;
                    order
                };
                self.commerce_poll_refund(&id, p, now).await?;
                self.commerce_refunds(a, &order)
            }
            Action::Subscription { id, refresh } => {
                if refresh {
                    self.commerce_refresh_subscription(a, &id, p, now).await
                } else {
                    self.commerce_subscription(a, &id, now)
                }
            }
            Action::CancelSubscription { id, accepted } => {
                self.commerce_cancel_subscription(a, &id, accepted, p, now)
                    .await
            }
            Action::Dispute { seller_id, id } => {
                self.commerce_reconcile_dispute(a, &seller_id, &id, p, now)
                    .await
            }
            Action::DisputePacket { id } => self.commerce_dispute_packet(a, &id),
            Action::InvoiceIssues { seller_id } => self.commerce_invoice_issues(a, &seller_id),
            Action::RetryInvoice { seller_id, id } => {
                self.commerce_retry_invoice(a, &seller_id, &id, p, now)
                    .await
            }
            Action::Support { id, note } => {
                if let Some(note) = note {
                    self.commerce_record_support(a, &id, &note, now)
                } else {
                    self.commerce_support(a, &id)
                }
            }
            Action::SampleScenario { id, scenario } => {
                #[cfg(feature = "development-workflow")]
                if p.mode() == "sample" {
                    return crate::commerce_sample::Sample(self.clone())
                        .finance_scenario(a, &id, &scenario, now);
                }
                let _ = (id, scenario);
                Err(Error::new(403, "sample_mode_required"))
            }
        }
    }
}
