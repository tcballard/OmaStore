# Bundle status

Product spec: version 0.2, native-first edition. Earlier web-first bundle descriptions are superseded.

| Bundle | Deliverable | Dependencies | Status |
| --- | --- | --- | --- |
| B00 | Repository checkout, runnable shell and handoff | None | verified |
| B01 | Catalogue v1 schema, validation and fixtures | B00 | implemented; local and CI contract checks passed |
| B02 | Native catalogue client, cache and read API | B01 | local and CI checks passed, including actual HTTP/core parity |
| B03 | Native discovery and truthful app details | B02 | local and CI native offscreen checks passed; real Omarchy pending |
| B03b | Local author preparation, validation and export | B01, B03 | local and CI checks passed; native portal exercise pending |
| B04 | Publisher identity and persistent workspaces | B01 | not_started |
| B05 | Submission drafts, media and preview | B04 | not_started |
| B06 | Bounded checks and evidence intake | B05 | not_started |
| B07 | Review workflow and approval records | B06 | not_started |
| B08 | GitHub publication bridge | B02, B07 | not_started |
| B09 | Release monitoring and suspensions | B08 | not_started |
| B10 | Makers, editorial selection and RSS | B03, B04, B08 | not_started |
| B11 | Setup catalogue and selective recipes | B03, B08 | not_started |
| B12 | Read-only installation planner | B02, B09 | not_started |
| B13 | Native library and identity handoff | B12 | not_started |
| B14 | Supported package execution | B12, B13 | not_started |
| B15 | Journal reconciliation and lifecycle | B11, B14 | not_started |
| B16 | Pilot operations and release readiness | B09, B10, B11, B15 | not_started |
| B17 | Supported settings schema and diffs | B11, B15, B16 | not_started |
| B18 | Setting apply, conflict and restore | B17 | not_started |
| B19 | Recipe remix and private-data-safe export | B18 | not_started |
| B20 | Commerce operating model and disabled integration | B16 | not_started |
| B21 | Checkout, delivery and entitlements | B20 | not_started |
| B22 | Refunds, disputes, payouts and commerce readiness | B21 | not_started |

B00–B03 are implemented and checked on offscreen Linux, including actual process/HTTP interactions. [CI passed for implementation revision 887d449](https://github.com/tcballard/OmaStore/actions/runs/34218944477); the handoff records its full revision and tree. R1 still needs real Omarchy desktop validation. B03b is a bounded local preparation extension; it does not complete B04 or B05. Public listings remain empty and publisher claims, installations and payments are unimplemented.

Next: real-Omarchy R1 exercise and B04 publisher identity/private workspaces. Review BUILD_HANDOFF.md for exact evidence and configuration gates.
