# Bundle status

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
