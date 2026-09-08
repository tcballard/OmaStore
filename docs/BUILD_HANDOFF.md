# Build handoff

Date: 8 September 2026. Branch: `build/native-discovery`. Scope: B01, B02, B03 and bounded extension B03b. GitHub revision and CI receipts will be recorded after publication.

## Grounding and implemented outcome

Started from the actual `tcballard/OmaStore` native scaffold at `53b8957335e0842289abc2966d762fe09f7bcded`. No unrelated repository was used as an application baseline. The authoritative native product spec is version 0.2. [ADR 0002](adr/0002-native-discovery.md) records pinned Omawrite/Omacalc sources inspected for desktop settings, Qt windows, shortcuts and Arch packaging conventions.

The application now supports native discovery/search/filter/detail, independent price/type/licence/evidence labels, explicit external acquisition/source/support links, local saved items and a local author worksheet. The Rust crate is the single catalogue schema/query authority, reused by the core and actual public read service. The HTTPS client validates before replacing its bounded atomic XDG cache. Public catalogue data remains empty. Only the separately compiled demo embeds synthetic listings; no real publisher participation or test result was invented.

The worksheet saves partial work privately on this device, rejects concurrent overwrite, checks a typed candidate, previews the exact export and uses a native file dialog. It always exports an unclaimed development candidate. B03b does not complete B04 identity or B05 server drafts. No public intake, authentication, package writes, installed-app scanning, managed checkout or desktop configuration mutation is implemented.

## Validation evidence

Local environment: Linux x86_64, Ubuntu 24.04.3, Qt 6.8.3, GCC 13.3, Rust 1.98.1. Isolated toolchain paths are verification-environment details, not project dependencies.

- Rust catalogue, evidence, query, cache and local-preparation tests pass; 14 tests total, plus one deliberately ignored performance sample run separately. Formatting and Clippy with warnings denied pass. Test records require the SHA-256 of the executed bytes, including source/package releases, and app details expose the recorded environment, date, actor, limitations and evidence link.
- Actual Qt keyboard flow passes at 800×600, 1280×800 and 1920×1080 logical sizes, plus 800×600 at 200% DPI. It exercises search focus/typing, card activation, accessible card name, app detail, saving, restored preferences, back navigation, worksheet typing/saving and field-error feedback.
- Native media tests check byte digest, inert image format, decoded dimensions and disallowed origins. Worksheet tests cover restart recovery, optimistic conflict, invalid saved-file preservation, checked-result invalidation and local-only export.
- Actual Rust HTTP service and core process tests compare query outputs and check conditional GET, invalid filters/methods, snapshot changes and invalid catalogue replacement. These ran successfully here; an earlier socket-restriction assumption was not applicable to this run.
- Ordinary-build tests check that the demo option is rejected and fictional catalogue text is absent from the core executable. CMake staging installs only owned desktop artifacts. The Arch PKGBUILD has syntax validation only until exercised on Arch.

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
