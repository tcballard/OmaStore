# Managed commerce

Managed checkout is disabled. External author checkout and support links remain available with no OmaStore commission. Development examples do not represent real purchases, participating sellers or author demand.

ADR 0009 defines the candidate direct-charge provider arrangement and closed release gate. The managed fee is 5% of the discounted subtotal before tax, rounded down in the currency's minor unit. Refund fee reversals use cumulative proportions; provider charges and retained processing costs stay separate and must be disclosed under the actual terms.

| Required operating fact | Current evidence | Effect |
| --- | --- | --- |
| Five authors requesting managed checkout | Missing | Entry gate closed |
| Actual operator and provider agreement | Missing | No live credentials/charges |
| Seller identity and Connect liability arrangement | Candidate design only | No seller onboarding |
| Regions, currencies and approved products | Missing | No public paid offer |
| Buyer/seller terms and tax/consumer-rights review | Missing | Terms are not publishable |
| Provider charges, payout timing and reserve schedule | Missing | No promised settlement date |
| Provider sandbox sale/delivery/refund/payout report | Missing | Commercial lifecycle gate closed |
| Operating cost observations | Model available, actual inputs missing | Sustainability is not established |

Tom Ballard owns obtaining the records and appointing the actual commercial operator. Code and fixtures cannot manufacture these facts. The source gate can change only with the complete reviewed release evidence; configuring an environment variable or marking a fixture passed cannot enable real money.

Receipt/licence recovery is independent of the original checkout browser. Offline signatures verify against trusted issuer keys, never a key supplied inside the purported licence. A signature establishes the issued grant; refunds, disputes and current service access require their own state. Free/open-source acquisition and existing local documents remain independent of managed checkout.

Commerce support cannot approve apps, clear integrity holds, publish author changes or alter discovery ordering. The independent submission and review workflow keeps that authority.

The native Author workspace → Commerce view exposes readiness and operator-only operating records. `GET /api/v1/commerce/status` returns the missing-facts projection; authenticated operators additionally receive the private model. `commerce_model` commands bind an expected record version and reviewed report digest. Records are append-only. `commerce_pause` requires the current operator role and a reason; allowing test purchases does not change the compiled live gate.

B21a freezes one seller/product/price version per order. Buyer request keys remain with the immutable order, independently of routine workflow-request retention. Ambiguous provider creation is reconciled against the recorded order; after 23 hours it requires an observed provider session and never retries creation blindly. Missing browser callbacks can be recovered through the signed-in buyer's receipt history. Paid delivery failures are explicit and retryable. Perpetual grants verify with Ed25519 against separately pinned issuer keys. The development issuer is public test material and is never a production trust anchor.

The Stripe test adapter accepts test keys only and uses the connected-account header for direct charges. Hosted URLs are restricted to Stripe's checkout origin. Provider responses must match the frozen total, tax, pre-tax fee, account and opaque buyer reference. The adapter validates a paid charge rather than trusting a browser return or the word “completed”.

Stripe's subscription percentage fee is calculated from the invoice total. The implemented test subset therefore permits only zero-tax fixed subscriptions whose discounted amount is divisible by 20 minor units, making 5% exact. Other managed subscription combinations remain unavailable with `subscription_fee_contract_unsupported`; external author subscriptions remain supported. Tom owns obtaining an approved invoice-fee arrangement before extending that subset. [Subscription fee contract](https://docs.stripe.com/api/subscriptions/update), [invoice payment identity](https://docs.stripe.com/api/invoice-payment/list). Recurring-cycle accounting/cancellation follows in B22.
