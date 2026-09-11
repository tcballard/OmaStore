# ADR 0014: synchronise published package observations

Accepted 11 September 2026. Tom requested synchronisation while preserving
PR #5's approved native appearance.

The ordinary native app reads the fixed HTTPS Omarchy stable x86_64 package
database at https://pkgs.omarchy.org/stable/x86_64/omarchy.db. This reports
published package versions, rather than unpublished PKGBUILD changes. No
PKGBUILD, downloaded program, shell script or package install is executed.

Refresh checks at startup and hourly while the app is open; a busy core defers
the automatic check to the next minute. Manual Refresh remains available.
Each origin request has a ten-second total timeout, bounded headers and no
redirects. Conditional ETags avoid downloading an unchanged database. The
compressed index is limited to 4 MiB, its decoded archive to 32 MiB, zstd window
to 8 MiB, entries to 20,000 and each descriptor to 64 KiB. Tar entries are read
in memory, never extracted. Invalid, empty or duplicate-name databases fail
without replacing the last valid observation. A valid index containing none
of the selected applications removes those community listings.

The reviewed six-app selection and editorial wording remain release-controlled.
Published versions, architecture eligibility and licence identifiers refresh;
new package names do not automatically become store listings. Unknown changed
licence classifications are not promoted to open source. Repository updates
create no runtime tests or signature-verification claims. The current scope is
stable x86_64, including architecture-independent packages, not other rings or
ARM availability.

Repository observations persist atomically in a distinct XDG cache file.
Approved author publications keep their existing origin, cache and ETag.
The displayed catalogue combines them with approved IDs/slugs/package identities
taking precedence. Community observations never enter the approved cache or
publication registry. Installation still requires the independent current
host/package/distribution checks; browsing never grants execution authority.
The footer shows the last successful package check, including conditional 304s.
Failures keep prior data and show an explicit stale warning.

QA/demo launches disable the automatic network timer. Parser/cache tests use
in-memory archives and the archived genuine database; the explicit ignored
live integration test exercises the fixed HTTPS origin separately. Real Omarchy
launcher, Wayland and package lifecycle acceptance remain separate.
