# Native discovery verification — 11 September 2026

Linux x86_64 container, Qt 6.4.2, software rendering with the offscreen platform.
These are screenshots of OmaStore itself, not demonstrations of the listed apps.

- `discover.png`: actual native catalogue at 1040×760.
- `detail-800.png`: actual LocalSend detail at 800×600.
- `discover-200pct.png`: 800×600 logical window at 200% scaling.

CTest exercises real Qt keyboard input: type a search, Enter to open a result,
Alt+Left to return, empty results, compact filters, failed refresh and a second
process restoring the saved query. The Rust tests separately cover superseded
and expired evidence, malicious fields, cursor changes and corrupt cache recovery.

Not established: Wayland/Omarchy theme integration, real-desktop accessibility,
installed-package availability, installation or update outcomes. Media slots are
external opt-in links with explicit absent-media text; inline validated image loading
is deferred until approved media and its bounded delivery adapter are available.
The preview uses Qt's existing Fusion/system palette, not a final store art direction.

## Informational listing provenance

All four entries were checked against `omacom/omarchy-pkgs` revision
`c31ef469c5f0dde2654770f5ce578e8b2e559193`. Each catalogue entry links to its
immutable PKGBUILD. No PKGBUILD was executed. Descriptions are short factual paraphrases.
No third-party artwork or screenshots were copied.

| App | Package definition | Recorded package version | Declared architectures |
| --- | --- | --- | --- |
| LocalSend | pkgbuilds/localsend/PKGBUILD | 1.18.2-1 | x86_64, aarch64 |
| OmaCalc | pkgbuilds/omacalc/PKGBUILD | 0.2.2-1 | x86_64, aarch64 |
| OmaWrite | pkgbuilds/omawrite/PKGBUILD | 0.5.0-1 | x86_64, aarch64 |
| Heroic Games Launcher | pkgbuilds/heroic-games-launcher-bin/PKGBUILD | 2.22.1-1 | x86_64 |

These are informational nominations, as allowed by R1, not approved app submissions.
Maker claims, compatibility evidence, signatures, prices and runtime requirements
have not been supplied. Missing information remains unknown. Independent review and
ordinary media requirements remain prerequisites for approved/featured listings.
