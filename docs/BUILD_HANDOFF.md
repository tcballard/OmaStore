# Build handoff

Date: 8 September 2026. Current change series: native discovery.

## B01 catalogue contract

Implemented shared Rust public types, bounded validation CLI, immutable release variants, offer/currency fields, evidence identity/freshness, recipe references/cycle checks, empty public data and separate synthetic examples. Four catalogue contract tests cover positive and negative cases, deterministic snapshots and changed/stale evidence; the existing three core tests are retained. Rust formatting and Clippy pass. Native compilation/CI will be rerun for the completed discovery series. B01 structural validation is not publication approval. Next: B02 shared queries, native cache and read service.

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
