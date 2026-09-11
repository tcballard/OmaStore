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

## Read service and native client

```sh
cargo run --locked -p omastore-catalogue --bin catalogue-serve -- data/registry.json 127.0.0.1:8080
```

The service binds to loopback; a deployment must provide an HTTPS reverse proxy.
GET `/api/v1/catalogue`, `/api/v1/apps`, `/api/v1/apps/:id`, `/api/v1/makers/:id`
and `/api/v1/setups/:id` expose only typed public data. Apps also resolve slugs.
ETags identify the complete snapshot, with conditional 304 responses. Each request
loads one validated snapshot. Publish a replacement with atomic file rename.

Apps queries accept `text`, `category`, `appType`, `license`, `architecture`, `pricing`,
`offline`, `testResult`, `limit` (1–50) and `cursor`. Empty filters mean all. Unknown,
duplicate or malformed parameters return 400. Cursors bind to content digest and query;
a changed snapshot returns 409 `snapshot_changed`, asking the client to restart paging.
No status/approval endpoint is fabricated before B09.

The native core offers `catalogue.info`, `catalogue.query`, `catalogue.detail` and
`catalogue.refresh` in its existing JSON-lines protocol. Query parameters use the same
names as HTTP and return the same `Page` object. It starts immediately from the last
valid cache or bundled data; refresh is deliberate. Cache location is
`$XDG_CACHE_HOME/omastore/catalogue.json`, falling back to `~/.cache/omastore/catalogue.json`.
Writes use a private temporary file and atomic rename. Failure retains the usable
snapshot and reports stale state. Oversized replies fail rather than break framing.

The default catalogue URL is the repository's raw `main/data/registry.json`.
An operator can set `OMASTORE_CATALOGUE_URL` to an HTTPS snapshot URL. Catalogue records
and inbound requests cannot change the origin. Network reads reject internal IPs,
pin validated DNS addresses, disallow redirects/proxies, bound DNS resolution to two
seconds and the overall request to ten seconds, and cap response text at 1 MiB.
Compression is not requested or automatically decoded. No inventory or telemetry is sent.
