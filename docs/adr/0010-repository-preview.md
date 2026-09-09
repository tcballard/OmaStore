# ADR 0010: bootstrap discovery from published Omarchy packages

Status: accepted for the development preview, 9 September 2026.

Tom asked to use existing applications in `omacom/omarchy-pkgs` as OmaStore's starting catalogue. This supersedes the empty-public-catalogue bootstrap assumption. It does not replace independent review for author submissions or establish author participation.

The ordinary build bundles six community-indexed desktop applications from an observed Omarchy stable x86_64 repository index. Drivers, system components and duplicate LocalSend variants are excluded. The package database, rather than the current PKGBUILD version on GitHub, supplies the observed package version, architecture and licence. The source observation is archived with digests in `data/repository`; `bin/build-repository-catalogue --check` checks the selected fields against that archive and reproduces the catalogue. No PKGBUILD is sourced or executed. The archived index is evidence for discovery only; its signatures were not independently verified here.

Maker identities remain unclaimed. Screenshots, runtime test records, editorial endorsements and setup recipes are absent until available. The maturity vocabulary gains `unknown`: a stable repository channel does not establish the upstream application's maturity. Existing consumers built against the older strict enum must update before accepting these entries; no deployed consumer is assumed.

This bootstrap selection is reviewed in a code change, not submitted as a fictional author's approved publication. It does not create private approval receipts or update the protected `catalogue-live` branch. Normal author publication continues through its existing authority. The repository preview lives in `data/repository/catalogue.json`, separately from the author publication registry at `data/registry.json`. It is the ordinary client’s bundled fallback and does not become an approved publication base. A successfully fetched approved catalogue takes precedence. An upstream project may subsequently claim its entry through the existing identity/review process.

Package identities use repository `omarchy` and exact observed versions. The existing planner checks the current local repository, architecture, package version, signature policy and distribution status. An archived package observation never enables execution. Real package lifecycle and current distribution-service gates remain closed. Source and support links remain available without that service. The first desktop exercise is ordinary-build browse/search/save and read-only planning for OmaCalc.

No new root helper, package mirror, payment dependency or auto-updater is introduced. Stripe and author recruitment are not prerequisites for this discovery preview. Arch packaging is prepared separately and must be built and exercised on a compatible host before claiming an installable release.
