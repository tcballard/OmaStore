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

The native Library and app details open Purchases and licences. Each purchase shows its frozen seller, subtotal, discount, tax, total, fee, licence and billing terms before consent. Local private checkout attempts retain their request key across process restarts. Receipt history belongs to the signed-in buyer, independent of the original provider browser. Licence export is bound to the exact preview digest; offline verification accepts separately pinned keys only. The ordinary build has no approved production issuer key.

Development rehearsal: sign in as the sample operator, open Author workspace → Commerce, enter a pause-change reason and allow test purchases. Library → Purchases and licences offers fictional purchases and a deliberate failed-delivery rehearsal. Retry produces the same recoverable grant. Sample state is durable and bound to the sample provider. It cannot be opened as a Stripe-test commerce database.

The service has a 30-second worker that observes pending provider payments and fulfils paid delivery jobs. It never creates purchases in the background. Hosted delivery uses a fixed operator-configured endpoint per app/seller with a narrowly scoped credential. `deliver` must persist its idempotency key and return the same grant reference after a lost reply; `recover` returns current access/download details for that reference. Replies bind order, opaque buyer, app, seller and paid service period. Download origins are configured independently of author catalogue content. Original receipts and signed grants remain retained. No author script or catalogue hook is executed.

An explicit `--commerce-test` service flag requires a separate development database and a Stripe test key (`OMASTORE_STRIPE_TEST_KEY`) plus the test webhook signing secret (`OMASTORE_STRIPE_TEST_WEBHOOK_SECRET`). It cannot be combined with `--sandbox`. Live credentials are rejected. An optional private 32-byte issuer seed file (`OMASTORE_COMMERCE_ISSUER_FILE`) uses the `operator-test-issuer` identity; the corresponding public key needs an independent desktop trust-pin review. Optional `OMASTORE_FULFILMENT_FILE` is a private JSON array of `{appId,sellerId,endpoint,secret,downloadOrigins}` bindings. Neither secret enters Qt or a public listing. Missing configuration becomes an explicit delivery failure, not a successful receipt.

The fulfilment HTTPS contract posts `/deliver` and `/recover` under the configured endpoint, with an `Idempotency-Key`, bearer credential, order/app/seller/buyer references, licence, price version, recorded paid-until time and recovery reference. A bounded reply contains `{orderId,buyerReference,appId,sellerId,reference,expiresAt,recoveryUrl,download}`; optional download is `{url,sha256,bytes}`. The fixture exercises a provider effect whose response is lost, then recovery after database restart without a second effect. Actual provider operation remains a launch gate.
