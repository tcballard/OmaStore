# Catalogue version 1

The Rust `omastore-catalogue` crate is the schema authority. Struct fields use camelCase; tagged identity/route variants use snake_case. Unknown fields are rejected at every level, including private fields accidentally added to a public export. Validation returns at most 100 `{path, code}` errors, never submitted values or credentials.

Run `cargo run --locked --bin omastore-validate -- data/registry.json`. Exit 0 means structurally valid, 1 means rejected content, 2 means an unreadable file or invalid invocation. Validation is not publisher authentication, runtime testing or publication approval.

`docs/examples/submission.json` is a complete synthetic starting example. Validate it with `cargo run --locked --bin omastore-validate -- --development docs/examples/submission.json`. It deliberately has no media or test evidence; those are readiness requirements for B05–B07, not invented schema defaults. Replace its example facts, obtain media rights and follow review before seeking public inclusion.

The public loader rejects `channel: development`. Public snapshots start empty. Development catalogue loading in the native core will require a build feature, and development state uses a separate local directory. A production build never embeds the fixture catalogue.

Identities distinguish source commits, publisher binary SHA-256s and repository packages. Route fields contain package tokens and HTTPS destinations, never command text. A package route does not authorise execution. Current evidence requires the current release identity, complete environment/evidence fields, a matching candidate digest and a test less than 90 days old. `App::candidate_digest` hashes the typed app with its tests emptied, so changed routes, capabilities, prices or metadata require new evidence. This conservative policy may require more retesting than a later reviewed policy.

Snapshot digests use typed JSON with apps/makers/recipes sorted by ID. Array order within a record is meaningful. Whole snapshots are bounded at 8 MiB; individual app records at 96 KiB; descriptions at 12 KiB. The bulk snapshot limit is distinct from the specification's 1 MiB limit for fetched third-party metadata. No third-party metadata is fetched by the validator.

Money uses unsigned integer minor units and an explicit checked currency exponent. Initial supported currencies: USD/GBP/EUR/CAD/AUD/CHF (2), JPY (0), KWD (3). Other currencies fail validation pending an exponent-table extension. Prices are informational; final external prices belong to the seller.
