# ADR 0002: Native discovery and public delivery

Accepted engineering direction, 8 September 2026.

The catalogue crate is shared by the Rust child process and read service. Search/filter evaluation, explicit public projection and snapshot-bound cursors have one implementation. Cursors bind the complete query, content digest, offset and evidence-evaluation time; changed content or a cursor older than 24 hours requests a restart. Prices never influence ranking. Whole public records use strict typed serialization; unknown/private fields fail intake.

The core uses ureq 3 with rustls and a ten-second total deadline to read one release-controlled HTTPS location. The initial location is the repository's `main/data/registry.json` on raw.githubusercontent.com. Redirects are disabled. No catalogue field, link or IPC parameter can replace that origin. The shared read service can later replace it through a reviewed release configuration. This is public Git delivery, not an install authorisation or approval channel. B08–B09 still gate publication/status and every managed operation remains absent.

The catalogue cache lives under the XDG cache directory, is validated before use/replacement, and is written with an atomic temporary-file rename. Cache write failure preserves current browsing and reports that it could not save. Invalid network data preserves the last valid view. The GUI's process boundary keeps network work off its event loop. Initial startup uses bundled/cached records; the user explicitly refreshes, avoiding network requests as a prerequisite to opening the app.

The read-only Rust service uses tiny_http, binds only to loopback, and expects an operator-managed HTTPS reverse proxy with request/concurrency/time limits. Each request reads one bounded, fully validated snapshot, permitting atomic Git delivery replacement. Invalid replacement returns 503. It has no private database, login, mutation or status endpoint. This modest service is appropriate for development/pilot evaluation; it has not been deployed or load-certified.

Development catalogues require the `development-catalogue` Cargo feature and a separate CMake preset. They are never embedded in ordinary builds. Development mode uses no public cache or remote refresh. Synthetic preview records do not represent real publishers, application media or evidence.

Native conventions inspected at fixed upstream revisions:

- [Omawrite system settings integration](https://github.com/omacom/omawrite/blob/8f98892b26768236b2c20f4e637cf4b102d898bf/src/systemtheme.cpp): portal colour scheme and desktop text scaling, reacting to settings changes.
- [Omawrite application entry](https://github.com/omacom/omawrite/blob/8f98892b26768236b2c20f4e637cf4b102d898bf/src/main.cpp): standalone Qt Quick window, desktop identity and shared interface font scaling.
- [Omacalc QML window](https://github.com/omacom/omacalc/blob/dba63819810d0a3b1a0581f3bcafc9651dbfb85d/src/Main.qml): keyboard actions, explicit minimum size and avoiding maximized geometry as restored normal geometry.
- [Omawrite Arch packaging](https://github.com/omacom/omawrite/blob/8f98892b26768236b2c20f4e637cf4b102d898bf/pkgbuild/PKGBUILD): ordinary Qt/portal dependencies and desktop installation. OmaStore's local recipe will follow that mechanism, without claiming repository inclusion.

These are mechanisms to adopt, not permission to copy product branding or claim real Omarchy verification. OmaStore retains Qt Fusion controls and uses its own restrained storefront composition. Standard [XDG settings](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Settings.html) and public Qt APIs provide the desktop boundary; no Quickshell internals are required.
