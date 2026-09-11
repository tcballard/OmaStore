# Catalogue v1

`crates/catalogue/src/lib.rs` is the runtime schema authority. Validate public JSON:

```sh
cargo run --locked -p omastore-catalogue --bin catalogue-validate -- data/registry.json
```

Exit 0 means structural validity, not app approval or compatibility verification.
Exit 1 emits a bounded JSON array of `{field, code}` errors (or `read_failed`).
Exit 2 means incorrect command arguments. Unknown fields, unsupported schema versions,
duplicate IDs/slugs, unsafe package identifiers and dangling references fail validation.

The complete synthetic submission snapshot is `tests/fixtures/catalogue.json`.
It covers source commits, publisher artifacts and repository packages, including unknown
runtime requirements and pricing. It is not a real listing. To validate it deliberately:

```sh
cargo run --locked -p omastore-catalogue --features dev-fixtures --bin catalogue-validate -- tests/fixtures/catalogue.json
```

Default/public builds reject its development channel. Keep fixture content outside
`data/`; do not relabel fixtures for publication. Media rights belong to each media
record independently of the application's software licence. Missing submission media
blocks ordinary submission readiness; informational nominations carry no approval.

Repository package identity does not prove the package is available on a user's host.
Source, binary and package evidence have distinct inspection limits. An evidence record
must identify the release and executed bytes, environment, actor, tool and reference.
Old-release evidence cannot produce a current-release pass. Dates alone confer no trust.
