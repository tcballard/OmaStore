# Bundle status

12 September 2026: B13a device-state foundation implemented_unverified. Shared
read-only inventory/app status, pacman update parsing, Installed/Saved/Updates
destinations and updater handoff are implemented. Focused core tests passed;
clean workspace run passed 96 tests; final focused core run passed 31 tests,
including paging and package-remapping regressions. Core protocol and independent
worker lifecycle process checks pass. Native/package CI is pending on PR #10.
See ADR 0015.

11 September 2026: B02 repository synchronisation continuation implemented.
Startup/hourly/manual checks update the reviewed stable x86_64 app selection
from the published index. Live HTTPS and restart-cache checks pass; approved
publication caches and the approved B03 interface are preserved. See ADR 0014
and BUILD_HANDOFF.md. This does not enable managed installation.

11 September 2026: restored `design/storefront-polish` / PR #5 as the active
continuation. Duplicate scaffold-based PRs #6–#8 are closed. Existing B00–B22
implementation and approved appearance are retained. Native and Arch CI at
641ab7a passed; fresh local restoration checks and capture are in BUILD_HANDOFF.md.

10 September 2026: B03 appearance continuation is verified on offscreen Linux:
11 ordinary and 10 sample checks passed, with one local D-Bus skip per suite.
Real Omarchy acceptance remains pending. Follow Omarchy defaults on; the approved
OmaStore editorial style remains selectable. Read-only theme/font integration
and isolated live-reload tests are implemented (ADR 0013). No host settings,
installation adapter or commerce release gate is enabled.

10 September 2026: approved authored shelf completes the B03/B12 presentation
continuation. Native square-edged dark shell, editorial app showcase and
persistent keyboard shelf are implemented. Ordinary native checks: eight
passes, one local D-Bus skip; sample checks: nine passes, one D-Bus skip.
See ADR 0012, `design-qa.md` and the handoff for actual captures and limits.
Arch preview recipe revision 4 adds the SVG runtime dependency. Remote CI and
package status are tracked on PR 5; real Omarchy acceptance is still pending.

Product spec: version 0.2, native-first edition. Earlier web-first bundle descriptions are superseded.

| Bundle | Deliverable | Dependencies | Status |
| --- | --- | --- | --- |
| B00 | Repository checkout, runnable shell and handoff | None | verified |
| B01 | Catalogue v1 schema, validation and fixtures | B00 | implemented; local and CI contract checks passed |
| B02 | Native catalogue client, cache and read API | B01 | local and CI checks passed, including actual HTTP/core parity |
| B03 | Native discovery and truthful app details | B02 | local and CI native offscreen checks passed; real Omarchy pending |
| B03b | Local author preparation, validation and export | B01, B03 | local and CI checks passed; native portal exercise pending |
| B04 | Publisher identity and persistent workspaces | B01 | implemented_unverified; local service/native checks pass; real OAuth callback and keyring pending |
| B05 | Submission drafts, media and preview | B04 | implemented; 22 Rust tests and 8 native checks pass; retention integration closes in B16 |
| B06 | Bounded checks and evidence intake | B05 | implemented; lease/replay tests pass; live disposable-VM report remains an external gate |
| B07 | Review workflow and approval records | B06 | implemented; 27 Rust tests and 8 native checks pass, including sample submission through independent approval |
| B08 | GitHub publication bridge | B02, B07 | implemented; 33 Rust tests and 8 native checks pass, including publication rehearsal; live App/deployment gate pending |
| B09 | Release monitoring and suspensions | B08 | implemented; 36 Rust tests and 8 native checks pass; configured upstream observation remains an external gate |
| B10 | Makers, editorial selection and RSS | B03, B04, B08 | implemented; 40 Rust tests and 8 native keyboard/feed checks pass; real Omarchy/editorial operations remain external gates |
| B11 | Setup catalogue and selective recipes | B03, B08 | implemented; 45 Rust tests and 8 native checks pass, including selective setup export/import; real recipes and Omarchy dialogs remain external gates |
| B12 | Read-only installation planner | B02, B09 | implemented_unverified; 47 Rust and 8 native checks pass; real Omarchy probe remains external |
| B13 | Native library and identity handoff | B12 | implemented_unverified; 49 Rust and 8 native checks pass; local bus test skipped; CI bus coverage passed; real Omarchy pending |
| B14 | Supported package execution | B12, B13 | implemented_unverified; 52 Rust and 9 native checks pass; real Omarchy execution gate remains closed |
| B15 | Journal reconciliation and lifecycle | B11, B14 | implemented_unverified; recovery, partial retries, removal and diagnostic checks pass; real Omarchy lifecycle gate remains external |
| B16 | Pilot operations and release readiness | B09, B10, B11, B15 | software implemented; advisory audit clear and isolated-video integration passed; actual pilot evidence remains external |
| B17 | Supported settings schema and diffs | B11, B15, B16 | implemented_unverified; typed diffs and native keyboard checks pass; live adapters remain disabled pending actual Omarchy evidence |
| B18 | Setting apply, conflict and restore | B17 | implemented_unverified; 67 Rust and nine native checks pass; live customised-desktop evidence remains external |
| B19 | Recipe remix and private-data-safe export | B18 | implemented_unverified; 70 Rust and nine native checks pass; real Omarchy portal/author evidence remains external |
| B20 | Commerce operating model and disabled integration | B16 | implemented_unverified; Stripe confirmed as default provider on 9 September 2026; 72 Rust tests and nine native checks pass; actual commercial facts/provider gate remain external |
| B21 | Checkout, delivery and entitlements | B20 | implemented_unverified; 81 Rust tests and nine native checks pass; real provider sandbox/issuer and commercial evidence remain external |
| B22 | Refunds, disputes, payouts and commerce readiness | B21 | implemented_unverified; 88 Rust tests and nine native checks pass; actual provider sandbox/commercial sign-off remains external |

B00–B03 are implemented and checked on offscreen Linux, including actual process/HTTP interactions. [CI passed for implementation revision 887d449](https://github.com/tcballard/OmaStore/actions/runs/34218944477); the handoff records its full revision and tree. R1 still needs real Omarchy desktop validation. B03b is a bounded local preparation extension; it does not complete B04 or B05. The repository preview now bundles six real, unclaimed community entries (ADR 0010). Publisher identity, submission/review and publication software are implemented; live provider, Omarchy and pilot evidence remain release gates. The sample installation lifecycle is implemented; live installation and commerce have separate release gates.

All implementable software bundles B00–B22 are complete. The native playground includes settings, attributed remixes and the full test commerce lifecycle. The remaining work is actual Omarchy/portal/accessibility/lifecycle validation, genuine authors/listings/setups and pilot participation, deployment/provider credentials and independently reviewed commercial responsibilities. No sample receipt opens those live release gates. BUILD_HANDOFF.md records exact verification and publication receipts.

Repository preview continuation (B01/B03/B12): six published stable x86_64 application identities are bundled with archived source observations and reproducible generation. Maturity remains unknown; no test or author participation is inferred. Current-host planning and live execution gates remain distinct. See ADR 0010 and BUILD_HANDOFF.md.

Repository preview packaging: Arch artifact workflow and package-content check implemented; local syntax checks pass. Actual Arch workflow artifact and real Omarchy installation remain unverified.

Arch packaging correction: first real CI build exposed unresolved bundled SQLite symbols under makepkg LTO. The recipe now uses `!lto` and a single non-debug-split desktop archive; replacement CI remains required.

Repository preview native verification: full ordinary/sample native CI and isolated-media checks passed in [run 34397410622](https://github.com/tcballard/OmaStore/actions/runs/34397410622). The local ordinary Qt build also passed five checks (D-Bus skipped), and its actual six-entry window was inspected. This is offscreen Linux evidence; real Omarchy acceptance remains outstanding.

Arch package verification: [run 34398115220](https://github.com/tcballard/OmaStore/actions/runs/34398115220) passed at `94214915628ba532dc0d7262c442534785c1dbb7`, including the unprivileged package build, archive-content inspection and artifact upload. The `omastore-preview-arch-x86_64` artifact contains the installable preview, checksum and source revision. Disabling makepkg LTO resolved the observed SQLite linker failure. No package was installed on an actual Omarchy host.

B03/B12 storefront polish: in_progress. Discover, purpose navigation, detail hierarchy and installation sheet are being verified against the actual native build. The requested App Store polish is an explicit visual target, distinct from earlier functional completion.

B03/B12 storefront polish: implemented and verified on offscreen Linux. Eight ordinary and nine sample checks pass (D-Bus skipped in each). Actual light/dark, detail and blocked-plan captures are in docs/screenshots. The purpose/filter and preserved scroll/focus journey passes at small and 200% sizes. The whole-store visual target and real Omarchy acceptance remain outstanding; this completes the authorised first polish pass.
