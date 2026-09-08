# Operating OmaStore

The native application is built with the CMake presets and packaged as two sibling executables plus its desktop entry. The supporting Rust service binds to loopback behind an operator-owned HTTPS proxy. It is not a storefront website. No public service, OAuth registration, real Omarchy acceptance report or operator staffing arrangement has been established by this build.

## Queue and first response

The authenticated operator dashboard reports queued/failed jobs, expired leases, open reports/appeals, held routes, media storage and draft expiry notices. It shows the oldest unanswered submission in Monday–Friday working days in Europe/London. Public holidays are not modelled. An actual finding, explicit independent review start or decision establishes first response; the submit receipt does not. The target is three working days; more than five triggers an operator alert.

Maintenance records the previous completed London week once. Two consecutive breached weeks latch `expansion_paused`, preventing new publisher submissions while existing published authors retain their update flow. Draft preparation remains available. An operator should investigate staffing and review the stored weekly evidence before changing that local metadata through a controlled operational migration. The service does not automatically recruit, notify people externally or expand a pilot.

## Retention

Run `omastore-admin maintenance PRIVATE.db OBJECTS` with the service identity (or use the hourly service worker). Unsubmitted drafts idle for 90 days receive an in-workspace notice and seven additional days to be saved. A save resets the notice. Expiry archives only never-submitted drafts, clears their candidate, and removes their private media records. Shared or retained media bytes remain. Orphan removal is limited to validated private digest keys older than one day, under the same database transaction as upload attachment.

Routine job events, routine unavailable-source observations, old completed webhook payloads and request replay cache expire after 30 days. Durable job/operation identities, webhook hashes, public review findings, evidence, approvals, publications and audit history remain. Media quota counts selected current draft assets, with retained historical storage reported separately. It does not silently count old replaced assets as the author's current selection.

## Backup, restore and rollout

1. Build the exact Git tree with the pinned Rust toolchain; run Rust, ordinary/demo native and staging checks. Record the binary digests and matching database schema version. Build an ordinary release without sample features for deployment.
2. Run `omastore-admin backup PRIVATE.db OBJECTS NEW_BUNDLE`. The destination must be new. SQLite's backup API captures the database; the bundle includes every referenced digest object and a digest manifest. It is private data, including identity/session records: keep it restricted and use the operator's encrypted backup destination.
3. Restore to **new** paths: `omastore-admin restore BUNDLE NEW_PRIVATE.db NEW_OBJECTS`. Digests, referenced object completeness and SQLite integrity are checked first. The environment marker remains intact; a sample database cannot be opened in production mode. Published object visibility is reconstructed. An interrupted restore leaves inspectable new paths; it never replaces the running database.
4. Start the matching service version on a different loopback port with the recovered paths and publication/check workers paused. Inspect private records and public read/status responses. Rehearse a failed catalogue replacement: the previous valid catalogue must remain readable. Verify held and stale routes remain unavailable for managed installation.
5. Switch the operator proxy only after those observations. Keep the previous binary and database/object bundle. Additive schema migrations can make an older binary refuse a newer database; restore the matching backup to new paths when rolling back. Never remove desktop journals or users' installed apps to roll back a service.

Publication is already isolated on `catalogue-live` and requires an independently approved exact source change plus observed delivery. Use `OMASTORE_PUBLICATION_PAUSED=1`, `OMASTORE_CHECKS_PAUSED=1` and `OMASTORE_MONITORING_PAUSED=1` for maintenance as appropriate. Pausing observations makes status expire; cached catalogue data is not installation authority. Keep private monitoring, receipt/recovery and support access available when stopping new distribution.

## Readiness and cost evidence

`release_evidence` records an operator's report URL, SHA-256, outcome, actor and date for each fixed pilot gate. Changes are audited. Reports must include exact host/application versions, screenshots, failures and resolved or excluded cases. Sample records cannot complete a production pilot gate, and recording a report never enables package, setting or payment writes.

Use `scripts/operating_costs.py ops/costs.example.json` after filling actual costs and review-time assumptions. One currency is required; the model performs no foreign exchange. The template is incomplete deliberately. No paid hosting quote, operational review-time measurement or named operator has been supplied.

## Media processor boundary

Video normalization now requires `/usr/bin/bwrap`, FFmpeg, ffprobe and util-linux prlimit. The child has an empty environment apart from fixed locale/PATH/thread settings; a fresh mount/PID/network/IPC namespace; no capabilities; read-only system binaries/libraries; and only its private temporary working directory writable. It receives no home directory, service database, provider environment or desktop/service sockets. Resource and wall-clock limits remain in place. A failed sandbox preflight returns `media_sandbox_unavailable`; there is no unsandboxed fallback. Images continue through the bounded Rust image decoder.

The explicit `video_normalization_is_isolated_and_strips_metadata` test generates a tiny 15-second test video, verifies successful normalization and metadata stripping, and checks absence of the home environment and an outside private file. It is a mandatory CI step on a host supporting Bubblewrap namespaces. This local container lacks Bubblewrap, so that particular positive sandbox exercise remains unverified here. The policy is grounded in the [Bubblewrap project documentation](https://github.com/containers/bubblewrap/blob/main/README.md); Bubblewrap's isolation depends on these explicit arguments.

The 8 September dependency audit initially reported [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) through the JWT library's RustCrypto RSA backend. OmaStore now selects the library's supported AWS-LC backend, removing that dependency. An ephemeral-key test verifies the actual GitHub App RS256 signature, issuer and bounded validity window, and rejects a tampered token. The revised 249-dependency lockfile reports zero advisories and warnings against the database commit recorded in `docs/qa/dependency-audit.json`. This is dated evidence, so rerun it for releases.


The mandatory `isolated-media` CI job uses Ubuntu 22.04, where unprivileged namespaces are supported. Native builds remain on Ubuntu 24.04. Run 34290845779 identified the 24.04 worker startup failure as `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`, consistent with Ubuntu's restricted unprivileged-user-namespace policy. We do not relax AppArmor or global sysctls. Deploy the media worker only on a host that permits its namespace confinement, and run the positive sandbox test there. Unsupported hosts reject video processing; there is no unsandboxed fallback. This CI job remains a required release check.

The sandbox exposes only the fixed system dynamic-loader cache (`/etc/ld.so.cache`) read-only in addition to `/usr`. FFmpeg distribution packages may depend on libraries in cache-resolved subdirectories. The package-manager alternatives symlink directory (`/etc/alternatives`), when present, is also read-only so Debian BLAS/LAPACK selections resolve. The rest of `/etc`, service state, host temporary files, home directories and network remain unavailable. The positive test diagnoses fixed FFprobe startup separately from user-media parsing.
