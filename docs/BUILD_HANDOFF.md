# Build handoff

Date: 8 September 2026. Scope: B00 native scaffold.

Starting state: empty repository. README initialization: `9db3481c7ea3c5c9363cf06040e74427e17d5c45`. Verified implementation revision: [`05c7e941e54e0c53369da349d474735c258c222b`](https://github.com/tcballard/OmaStore/commit/05c7e941e54e0c53369da349d474735c258c222b). The published source tree matches the locally checked tree `9bcbaa5633ca61b7e9f3deb47e2b3d6296b12876`.

## Product decision

Tom corrected the earlier web-first plan: OmaStore itself must be native to Omarchy. The authoritative specification is now version 0.2 in `PRODUCT_SPEC.md`. B00 creates the Qt/QML application and Rust core. B01–B03 then establish catalogue contracts and native discovery. No website or cloud account is a prerequisite.

## Repository and changes

The user supplied `tcballard/OmaStore`. GitHub reported an empty public repository with default branch name `main` before this work. No prior commits or application files were replaced. The scaffold is one bounded initial implementation change set.

Implemented: Cargo workspace and pinned toolchain, CMake/Ninja build, Qt/QML window with six navigation destinations and honest empty states, narrow QProcess bridge, read-only Rust `core.info` protocol, standard desktop entry, CI configuration, MIT licence, contribution/submission policy stubs and the native-first product/bundle documents.

The app resolves a sibling core executable. Protocol lines are capped at 256 KiB and request IDs at 128 bytes. Invalid JSON, unsupported versions and unknown methods fail explicitly. Oversized requests terminate the read-only stream with a bounded error. There is no arbitrary command method, installer, privileged helper or URI registration in this scaffold.

## Validation

| Evidence | Result |
| --- | --- |
| Rust unit tests | 3 passed: stream recovery, oversized input and unsupported protocol/write methods |
| Clippy | Passed with warnings denied |
| Rust formatting | Passed |
| Actual process contract | 3 process checks passed through CTest |
| Qt compilation and offscreen startup | Qt 6.8.3 build passed; actual Qt window/core handshake passed offscreen |
| Desktop installation staging | Passed; both executables and the desktop entry staged under build/stage |
| GitHub CI | [Native build, protocol tests, Qt startup and packaging passed](https://github.com/tcballard/OmaStore/actions/runs/34204868598) on the verified implementation revision |
| Real Omarchy desktop | Not run; launcher, theme integration, tiling and accessibility remain unverified |

The local environment initially lacked Rust and Qt tools. An isolated user-space toolchain was installed for verification. Local toolchain paths are not project dependencies and must not be copied into the build configuration. The standard Ubuntu package install path is used in CI.

## Next work

B00 scaffold validation is complete on Linux. Start B01. Establish catalogue schema version 1 in shared Rust types, explicit release/evidence identities, unknown states, validation and development-only fixtures. Keep the public catalogue empty until genuine listings are supplied and approved. Extend the native UI; do not generate a web storefront.

Author services, app scans, installation, setup writes and commerce remain unimplemented. B12–B15 must introduce typed plans, confirmation, actual package outcomes and durable recovery before any platform-write capability is enabled. B00's simple child shutdown is safe only for its current read-only process.

## Recovery

Revert the scaffold change set to remove the source changes. Installation can be rehearsed entirely under `build/stage`; remove only that owned staging directory when cleaning up. A real install/removal workflow is not implemented. No user applications or desktop configuration were changed during scaffold verification.

## B01 — 11 September 2026

Starting revision: 53b8957335e0842289abc2966d762fe09f7bcded. Ending revision: the B01 commit containing this entry (resolve with git log). Implements B01-A1–A5: typed public schema, three release identities, routes, offers, media rights, maker/recipe references, validator CLI and development-only fixtures. Public registry remains empty. See CATALOGUE.md and ADR 0002.

Validation: Rust workspace tests, formatting and Clippy; fixture isolation, malformed/private fields, release-bound evidence, money, references and injection tokens. No native GUI change or package execution. Next: B02 read contract/client/cache. Rollback: revert this additive schema change before published content depends on it.

## B02 — 11 September 2026

Starting revision: be44631. Ending revision: the B02 commit containing this entry. Implements read-only snapshot/app/maker/setup routes, shared query filters, content-bound cursors, ETags, bounded HTTPS client, atomic local cache and core catalogue methods. CMake tracks new Rust/schema dependencies.

Evidence: 12 Rust tests passed before the additional cache round-trip test; formatting and Clippy passed. Actual HTTP entrypoint checks passed (snapshot, conditional ETag, query errors, not-found, reload). Four actual core-process cases and Qt 6.4.2 offscreen startup passed via CTest. A 1,000-entry release-build query benchmark on this Linux x86_64 container (9 exposed logical CPUs, 100 samples) measured p95 14.902 ms; this is not a Dell/UI latency result. Debug-build queries are slower.

Remaining: deployed HTTPS service round-trip; no production service was provisioned. No real Omarchy desktop or package operations were exercised. B02 is implemented_unverified for that external integration gate; B03 can use its tested local read contract and bundled data. Rollback: revert reader bundle, retain schema and B00 shell.

## B03 — 11 September 2026

Starting revision: 98c751f. Ending revision: the B03 commit containing this entry. Implements native Discover/Apps, shared search and eight filters, detail pages, unknown/old evidence states, explicit external acquisition, local preferences, refresh/empty/error recovery and QA capture commands. Four genuine informational nominations are sourced to immutable omarchy-pkgs records; see evidence/README.md. Schema adds explicit unknown maturity rather than infer it from version strings. No maker participation or compatibility is invented.

Evidence: 13 Rust tests passed, Clippy passed, actual HTTP contract passed, public catalogue validator passed, all three CTest suites passed (four core process checks, native keyboard/compact/offline checks plus separate-process preference restoration, Qt startup). Desktop entry and both executables staged successfully. Native renders inspected at 1040×760, 800×600 and 200% scaling. No external acquisition link was activated during automated tests.

Remaining R1 gates: actual Omarchy/Wayland desktop, theme and accessibility observations; deployed HTTPS client/service round-trip; approved media with bounded inline loading (current media actions are external/opt-in). No install, inventory scan, update/remove or author service exists. B03 is implemented_unverified at real-desktop level. Next: complete those R1 observations, then dependency-ready B04; package planning requires B09/B12, not an unreviewed Install button.

Recovery: revert B03 to the tested B02 shell; catalogue/cache data stays versioned. Clear only OmaStore's own preferences/cache if intentionally resetting the preview. No installed apps or personal files were changed.
