# ADR 0009: frozen orders and a closed commercial release gate

Status: accepted software design; actual commercial operation is unapproved.

Outstanding decisions belong to Tom Ballard, who must appoint the actual legal/operator owner. Being named in the project brief does not make a person the merchant or payment-platform contracting entity.

Stripe Connect direct charges is the integration candidate. The connected account is the seller/merchant of record under that arrangement; the platform can collect an application fee. Provider approval must cover the intended multi-author business, regions, product types and liability allocation. An API key alone does not establish approval. We do not silently substitute another charge type. See [merchant-of-record responsibilities](https://docs.stripe.com/connect/merchant-of-record) and [direct hosted checkout](https://docs.stripe.com/connect/direct-charges.md?platform=web&ui=stripe-hosted).

Listing, baseline review and ordinary updates are free. OmaStore takes 0% of external sales/support. Managed checkout proposes 5% of the discounted subtotal before tax, rounded down to the currency minor unit. Partial refunds reverse the cumulative proportional share of the original fee, with the last refund reversing the remainder. Provider charges remain separate. Amounts in different currencies are never netted or converted implicitly. Payment data does not enter review or ranking.

The missing commercial facts are five actual author requests, operator, seller/region/product scope, provider-supported arrangement, reviewed buyer/seller terms, tax/consumer-rights/refund/cancellation responsibilities, provider charges, payout schedule, reserves and a provider sandbox lifecycle report. Initial currency code coverage (GBP/USD/EUR/JPY) is not an approved sales territory or public offer. The existing operating-cost model still needs actual inputs.

A compiled gate prevents real checkout; no environment variable can bypass it. Pausing new purchases must preserve receipt recovery, paid delivery retries, refunds and reconciliation. Development databases, provider test mode and sample signing identities remain distinct. Fixture success does not satisfy legal or live release evidence.

The service freezes seller, price version, currency, discount, tax, fee and delivery/licence/online terms. A browser return is not payment evidence. Verify raw provider events and match account, payment identity and amount. Ambiguous network outcomes are reconciled using the same recorded idempotency key; expiration of provider idempotency retention never authorises a blind retry with a new key. [Checkout session contract](https://docs.stripe.com/api/checkout/sessions/create), [webhook verification](https://docs.stripe.com/webhooks/signature), [idempotency](https://docs.stripe.com/api/idempotent_requests).

The provider adapter pins API version `2026-08-26.dahlia`, verified against the official versioning page on 9 September 2026. Webhook endpoint versions must match. [Stripe versioning](https://docs.stripe.com/api/versioning).

Payment, delivery, refund and entitlement states are separate. Perpetual signed grants have a pinned-key offline verification path. A signature proves what was issued; later refund/dispute status belongs to the auditable service record and actual agreed terms. Open-source rights and free installations never depend on store entitlement checks. Delisting/cancellation do not delete receipts, recovery records or local documents. [Refund lifecycle](https://docs.stripe.com/refunds).

No live charge, seller onboarding, legal commitment or commercial launch is authorised by this design. External facts remain launch gates while implementation continues.
