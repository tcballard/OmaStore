# Build handoff

## Full-scope continuation

Tom authorised implementing all scoped bundles on 8 September 2026, making engineering decisions autonomously and committing regularly. The restored starting revision is `1bc14e0edd512c21af364a42e5c180193f40ad9d`; B04–B09 are on `build/author-workflows`; the next stack continues on `build/native-marketplace`. The earlier preview receipt below remains historical evidence, not a description of the full implementation's completion.

B04a adds the private SQLite identity/role/session/claim authority, GitHub authorization-code/PKCE adapter, desktop proof binding, single-use callbacks, expiring scoped control challenges, DNS-pinned bounded public reads and a production/development database boundary. Four focused tests pass, including populated-database restart, cross-account/replay rejection and session/role revocation. Formatting and Clippy with all features and warnings denied pass. Native/service wiring follows in B04b. A registered provider and real callback remain live integration evidence; they do not block subsequent implementation.

B04a implementation commit: local `c33c6f4e025d5d336d278ab4b730863aa8fd05cc`, published as `c5d8dab4dde416bd63ffee087544eead616d0ea3`; both have tree `eafd7fa4d2c755f9063461df4092f886c35ef601`.

B04b starts from B04a and adds the Axum HTTP boundary, bounded request handling, authenticated claim actions, operator role CLI and the native author workspace. The Qt bridge never receives tokens; Secret Service storage has an explicit session-only fallback. The separate sample build supports fictional local role sessions in a development-marked SQLite database. QA isolates that database as well as preferences. A reusable Cargo cache path and a single Ring TLS provider keep native/core/service builds consistent.

B04b validation: all 19 Rust tests pass (plus the previously separate performance sample), full-workspace Clippy with all features and warnings denied passes, and all 8 demo CTest checks pass. These include actual HTTP/core parity, the native sample-author sign-in, worksheet keyboard flow, three logical sizes and 200% scaling. A dialog sizing loop introduced by the nested author tabs was found and fixed. The local SDK is Qt 6.8.3; CMake/Cargo build resources were bounded after recovering a damaged generated dependency artifact. Real GitHub callback/keyring/Omarchy evidence is still outstanding. Next implementation: B05.

Rollback: revert the B04a implementation and retain private database files for recovery. No external provider registration, author account, machine package or desktop setting has been changed.

## Marketplace continuation receipts

CI follow-up: B10b's ordinary Qt 6.4 build caught an ambiguous QStringView/ASCII comparison in the QA RSS parser (run 34275044638). The comparison now uses QString/QStringLiteral, preserving compatibility with the declared minimum SDK. Local B10 evidence above remains Qt 6.8.3; the updated CI run must verify the older SDK. B11a is published as `da92184fc35006a595f1998fbb1df2dc15c83012`, matching local `a8b4244bf6e4e375b91336fc5e8127bf41d50a2a` and tree `a2ab6ce0c72e37ef43a89b68b11ae1f56bef30c7`.

B11a adds bounded setup metadata/dependency/conflict validation, selective dependency closure, separated/unknown price estimates, stable identity export/import and exact recipe media/review/publication. A local end-to-end test normalises a setup icon, rejects premature reviewer access, freezes app context, independently approves and observes publication without creating component release-feed entries. Proprietary app submissions now require the publisher-domain challenge; control of a source fork cannot substitute. All 45 Rust tests and workspace Clippy with warnings denied pass. Native setup selection follows in B11b.

B10b is published as `5d41ae738c7a5e2de9e284ea045ca60bb8ea91d7`, matching local `ece33add6c8af1d021c508a6db43089535b56d28` and tree `103119a0ccc4b278c2daf5934bfe4d483fe1c991`. The standalone DHH conversation starter is in DHH_NOTE.md; it is a draft and has not been sent. Its wording distinguishes the running development workflows from the remaining build.

B10b connects native maker discovery, attribution, support links, editorial reading and author scheduling, plus RSS export from observed deliveries. `prepare_draft` saves exact published context before confirmation; review/publication preserve referenced app evidence/media and reject changed facts. Current scoped control is projected by the authority, and existing entity ownership is a reviewer conflict. The development seed cannot impersonate new author submissions; those use separate IDs/slugs. Native rehearsal found and fixed canonical catalogue ordering and a delegate lifetime issue. All 40 Rust tests and Clippy with warnings denied pass. All 8 native CTest checks pass in 20.02 seconds, including maker navigation by keyboard, populated RSS parsing after actual local delivery, conditional HTTP feed reads, the author/reviewer/operator rehearsal, three logical sizes and 200% scaling. These are offscreen and fictional-provider checks; real Omarchy and provider evidence remain separate.

B10a is published as `e8f39bd888eb8cdc6a66f647db37f8756861d0e2`, matching local `82d2afab08393c77a86989327cf3bd996608616b` and tree `0d15fc2ebac46b818f50df2c0ed10e21400faf39`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34272125374). This continuation is [draft PR #3](https://github.com/tcballard/OmaStore/pull/3), stacked on #2. [B09 CI also passed](https://github.com/tcballard/OmaStore/actions/runs/34269507535).

B10a adds bounded maker profiles, scoped claim labels with expiry, scheduled editorial records, independent editor approval checks, and delivered-release RSS. Release title corrections retain their original GUID and first delivery date. No feed entry is created by approval, PR creation or an unavailable public deployment. Existing catalogue and approval bytes remain stable when the new optional fields are absent. All 38 Rust tests and workspace Clippy with warnings denied pass. Native maker/editorial authoring and frozen published context follow in B10b.

B09 is published as `7a55710df30bce03f8e88e19a7681411a8354495`, matching local `9fe06645236976d2928d1b1bda79918640c6bccf` and tree `eefe47cde5e29163785f4b3907abf2dd23255adf`.

## Author continuation receipts

B09 adds six-hour leased observations, immutable update candidates with unknown compatibility, once-only evidence ageing, integrity/ownership holds, private reports and appeals, fresh operator role/version checks and audited restoration. The public status overlay separates its five-minute response validity from upstream freshness, binds the catalogue snapshot and excludes private incident details. Native detail acquisition respects suspension/status availability; the Operations workspace provides the complete fictional suspension/restoration rehearsal. All 36 Rust tests and Clippy with warnings denied pass; all 8 native checks pass in 19.23 seconds. The final metadata-reader change was checked by the Rust gate; live upstream reads remain unverified. Database environment is checked before applying future migrations. No installed package was modified. See MONITORING.md for adapter scope and remaining live evidence.

B08b is published as `41ed1b5fd25d10cee9dddea60aebc1692d7276e1`, matching local `c99510d983d3a2b1cf94b70fc79774e3d6f64c32` and tree `fbce332d923232a60293437c86b2e9d7d817eb0d`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34266967804).

B08b adds the native publication status, exact-file export, issue/PR attachment, retry, base update and local provider rehearsal. The rehearsal uses the real approval/publication/reconciliation functions against an in-memory provider and labels every result as simulated. The native delivery and media origin now use `catalogue-live`; an unavailable branch preserves truthful empty/cached browsing. Authors can withdraw pending submissions before delivery work starts. All 33 Rust tests and workspace Clippy pass. All 8 native checks pass in 19.36 seconds, including author-to-reviewed-to-delivered rehearsal at three sizes and 200% scale. B08a is published as `b83fa5569bc7449a2b375b335901f014428c9a82`, local `b444dcacf748ca4371ef133f7ceb981b7bd77f3d`, matching tree `8f9a35b8625942ad3d46d6d8fb0aaad2539a5f94`.

B08a implements the operator-owned GitHub App adapter, signed webhook intake, exact private approval checks, durable issue/PR checkpoints, source PR attachment and non-forced rebase recovery, isolated live-branch delivery and atomic catalogue replacement. Provider errors are redacted and explicit; no App credentials or live deployment have been created. B08a verification: all 31 Rust tests pass, including forged approval content, role revocation, raw-body HMAC/delivery deduplication, lost-issue reconciliation and atomic replacement preservation; workspace Clippy with warnings denied passes. B08b connects native recovery and a local rehearsal. Configuration and remaining live gates are in PUBLICATION.md.

B07 is published as `9a8dd9c923d8df3edf890a388890ef50d7692669`, matching local `ee0d87270ee43a219e8f22cf8992f5acfaaedb13` and tree `e82109e1204db3ba5e5026eb57348eff5bab8624`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34259543054).

B07 verification: all 27 Rust tests and Clippy with warnings denied pass. All 8 native CTest checks pass in 16.89 seconds, including sample submission, checks, role switch, evidence import and independent approval via keyboard at three sizes and 200% scaling. No production approval or public listing was created.

B07 adds the independent reviewer queue and native workspace, field-level revision differences, author-visible decisions, current role checks, exact approval payloads and append-only review history. Sensitive capabilities require two distinct eligible reviewers. Project-control history remains a conflict even after a claim is revoked. Submitted media can be inspected without exposing later private draft files. Core media previews verify bytes and use owned temporary files; demo video playback uses the native system player. Policy tests cover self-approval, missing checks/evidence, two votes, stale concurrent decisions, replay, immutable receipts, private findings and London DST. B06 is published as `edcd37d8287cefbea413349cab8dd38fef3a7e29`, matching local `51fa9bffb7cad4120bb9b9c86c9a1c01575c033e` and tree `5b6d649d0ee52d6c9c7482c6d7668bc04c151305`.



B06 adds a durable SQLite checks queue, expiring leases, bounded source/link/provenance reads, explicit unavailable findings, retry commands and immutable independent VM-report intake. The service dispatcher and pause mechanism are documented in REVIEW_OPERATIONS.md. Sample checks use the same leases and completion transitions in a development-only database. Seven workflow tests pass, including stale-worker rejection and lease recovery; no actual submitted program has been executed as a check. B05b is published as `d6f1aaa9246d58ac532b71ac9ea4c3812518083b`, matching local `0e2ad5a6b86da6b17b2ef93a04490e349bc496c2` and tree `7d4b94867f0e9482313c73894bb2a4fdb210c1bf`.



B05b adds the native full-field draft editor, local recovery scoped to account and service, optimistic server sync, an explicit conflict merge acknowledgement, media selection, exact preview, submission and revision status. HTTP mutation retries use a persisted key; a failed subsequent refresh cannot misreport a committed action. Expired login polling clears correctly. Local edits survive network failure without becoming public. The template contains empty author facts, not demo claims.

B05a commit: `79129ba`. B05b verification: all 22 Rust tests passed, including authenticated HTTP upload/private-access checks; all 8 native CTest checks passed in 15.40 seconds, including creating a draft, keyboard typing and private autosave at three sizes and 200% scale. The recursive native editor uses dynamic QML loading after the runtime rejected static recursion. Media replacement retains historical bytes and counts the current draft selection. The checked UI is offscreen Linux; real Omarchy accessibility/media-dialog evidence remains outstanding.



B04b is committed locally as `d2e341a4ee3d8363f5397a6d2a0173c6d92ae308` and published as `d17855a5ab5968882f8cce41bb486f229cecf8c9`, with identical tree `3750cc37a612fe44047fdc3f5b5844cc027af955`. The author continuation is [draft PR #2](https://github.com/tcballard/OmaStore/pull/2), stacked on PR #1.

B05a adds private versioned drafts, atomic idempotent commands, immutable submitted candidates, revision-scoped findings, and authenticated media upload/read endpoints. Images are decoded and normalised; videos are bounded and metadata-stripped by resource-limited FFmpeg subprocesses. Files live behind an explicit private/public object-storage interface. The default persistent-volume implementation works without an external object provider. Public catalogue writes remain unavailable until independent review and publication. Drafts are bounded to 80 KiB to leave room for recovery data inside the native 256 KiB transport. Six workflow unit tests pass; the expanded HTTP/native acceptance checks are running in B05b. Retention and operational media cleanup remain tracked for B16.

## Earlier native preview receipt

Date: 8 September 2026. Branch: `build/native-discovery`. Scope: B01, B02, B03 and bounded extension B03b, followed by the executed-byte evidence correction. Review: [draft PR #1](https://github.com/tcballard/OmaStore/pull/1).

Verified implementation: [`887d44947beee29ca45c89b7ff5f6888542d3ae6`](https://github.com/tcballard/OmaStore/commit/887d44947beee29ca45c89b7ff5f6888542d3ae6), with Git tree `3ddb5cd7b037d8838d3dd72fab187700109e633f`. The published tree exactly matches the locally checked tree. The following documentation receipt commit does not change implementation files.

## Grounding and implemented outcome

Started from the actual `tcballard/OmaStore` native scaffold at `53b8957335e0842289abc2966d762fe09f7bcded`. No unrelated repository was used as an application baseline. The authoritative native product spec is version 0.2. [ADR 0002](adr/0002-native-discovery.md) records pinned Omawrite/Omacalc sources inspected for desktop settings, Qt windows, shortcuts and Arch packaging conventions.

The application now supports native discovery/search/filter/detail, independent price/type/licence/evidence labels, explicit external acquisition/source/support links, local saved items and a local author worksheet. The Rust crate is the single catalogue schema/query authority, reused by the core and actual public read service. The HTTPS client validates before replacing its bounded atomic XDG cache. Public catalogue data remains empty. Only the separately compiled demo embeds synthetic listings; no real publisher participation or test result was invented.

The worksheet saves partial work privately on this device, rejects concurrent overwrite, checks a typed candidate, previews the exact export and uses a native file dialog. It always exports an unclaimed development candidate. B03b does not complete B04 identity or B05 server drafts. No public intake, authentication, package writes, installed-app scanning, managed checkout or desktop configuration mutation is implemented.

## Validation evidence

Local environment: Linux x86_64, Ubuntu 24.04.3, Qt 6.8.3, GCC 13.3, Rust 1.98.1. Isolated toolchain paths are verification-environment details, not project dependencies.

- [GitHub CI run 34218944477](https://github.com/tcballard/OmaStore/actions/runs/34218944477) passed all native gates for the implementation revision above: Rust checks, public catalogue validation, ordinary and separate demo builds, both CTest suites, desktop staging and an actual preview screenshot. The suites contain 5 ordinary-build checks and 8 preview checks. CI uses Ubuntu 24.04; it is not a real Omarchy desktop exercise.
- Rust catalogue, evidence, query, cache and local-preparation tests pass; 14 tests total, plus one deliberately ignored performance sample run separately. Formatting and Clippy with warnings denied pass. Test records require the SHA-256 of the executed bytes, including source/package releases, and app details expose the recorded environment, date, actor, limitations and evidence link.
- Actual Qt keyboard flow passes at 800×600, 1280×800 and 1920×1080 logical sizes, plus 800×600 at 200% DPI. It exercises search focus/typing, card activation, accessible card name, app detail, saving, restored preferences, back navigation, worksheet typing/saving and field-error feedback.
- Native media tests check byte digest, inert image format, decoded dimensions and disallowed origins. Worksheet tests cover restart recovery, optimistic conflict, invalid saved-file preservation, checked-result invalidation and local-only export.
- Actual Rust HTTP service and core process tests compare query outputs and check conditional GET, invalid filters/methods, snapshot changes and invalid catalogue replacement. These ran successfully here; an earlier socket-restriction assumption was not applicable to this run.
- Ordinary-build tests check that the demo option is rejected and fictional catalogue text is absent from the core executable. CMake staging installs only owned desktop artifacts. The Arch PKGBUILD has syntax validation only until exercised on Arch.
- A separate local Release build with `BUILD_TESTING=OFF` and development data disabled built successfully. Its help omits QA/demo switches and its dynamic dependencies omit Qt Test.

Performance sample: Intel Xeon Platinum 8370C at 2.80 GHz, release Rust build, 1,000 synthetic entries, 100 samples. Shared query p95 10.312 ms; pure HTTP handler p95 10.527 ms. These exclude disk/socket/proxy time and do not establish GUI typing latency with 1,000 entries. The numeric target remains a target until that broader exercise is measured.

## Boundaries and remaining gates

R1 still needs a real Omarchy desktop exercise: launcher/app identity, Wayland tiling, live portal theme/text updates, native file dialog, actual keyboard/screen-reader behaviour and visual review. Offscreen success is not that evidence. The current visual composition and theme icon are provisional.

B04 is the next server bundle. Implement the real private workflow database, provider-supported browser sign-in, scoped project claims and desktop secret-service storage. An operator-owned provider registration/callback and deployment configuration have not been supplied or created. Do not put connector credentials into the product. The following author, reviewer, publication/status and installation bundles remain unimplemented and must retain their gates.

The initial native catalogue origin points to this repository's `main/data/registry.json`; until the PR is merged, refresh may return an unavailable state while bundled data remains usable. The read service has not been remotely deployed. It binds to loopback and requires proxy request/time/concurrency limits before public exposure. The cache is browsing data, never permission to install.

Native inline images accept only digest-addressed, inert assets under this repository's controlled `main/media/` prefix. Other publisher media remains an explicit external link pending an approved hosting adapter. No real application assets are supplied in the preview. Videos are never preloaded.

Known deliberate author scope: one worksheet, one external route/offer, manual local save, no media upload or claims. The complete publication candidate will need capabilities, services/privileges, release notes, real media and evidence added/reviewed in B05–B07. No field validation or exported digest grants approval.

## Continue and recover

Read the spec, bundle status and ADRs. Preserve the native window, typed process boundary, public/private separation and current data. Use separate dependency-ready commits. Complete real-desktop R1 validation and B04 before representing the preview as a marketplace pilot.

Revert the relevant bundle commits to undo code. Preserve users' saved items and local worksheet files when changing versions; do not remove broad XDG directories. Stage packaging under `build/stage` for reversible rehearsal. No user applications or Omarchy settings were changed by this build session.
