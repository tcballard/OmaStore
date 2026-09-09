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
