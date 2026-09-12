# ADR 0017: Shared actions and retained stale device observations

Accepted 12 September 2026. B13c follows PR #11. Tom explicitly asked to close
the stale-observation and shared-action gaps identified against the seven detailed
Mac App Store reference specifications. This changes ADR 0016's display policy:
a failed check invalidates action authority while retaining historical information.

Reference requirements: detailed spec 01 navigation/error states; spec 02 shared
card behaviour; spec 03 device/operation authority; spec 04 primary-action
precedence; spec 06 failed checks and maintenance; spec 07 journeys B/C/E/F.

The session retains the last complete successful inventory, its ordering, versions,
counts and observed timestamp across failed probes, locks and connection loss.
The aggregate is explicitly stale; individual observed rows are marked stale and
old primary-action intents are removed. First-use unknown remains unknown. A
fresh complete snapshot replaces retained data atomically. Nothing is persisted
as a new successful observation merely because a refresh was attempted.

The bridge now derives and dispatches a single action policy for cards, detail,
Installed and Saved. Active/failed/unknown operations offer their existing review.
Disconnected sessions offer reconnect. Stale observations offer a fresh check.
Unknown eligibility opens a read-only availability plan; absent apps offer review
of installation, preserving existing execution gates and consent. External routes
open their details/options. Open requires a resolved desktop launcher. Zero
launchers is an explicit unavailable Open action; failed lookup offers retry.
When installed and launchable with an update, Open remains primary and the
explicitly confirmed whole-system updater is secondary.

Launcher resolution is background work once per device snapshot. Launch still
revalidates the current package and trusted desktop file in the Rust adapter;
resolved UI state is never execution authority. Cards consume button clicks
separately from identity navigation. Shared chooser/updater confirmations live
at window scope. No pause, partial package upgrade or automatic retry of a
mutation is introduced.

Verification includes the controlled Qt process peer for stale observations,
counts/timestamps, reconnection, failed-operation precedence, update/Open
coexistence and missing launchers. Native keyboard tests verify a card action
opens a plan without also opening its app page. Real Omarchy lifecycle acceptance
remains outstanding.
