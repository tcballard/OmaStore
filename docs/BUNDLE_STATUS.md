# Bundle status

Product spec: version 0.2, native-first edition. Earlier web-first bundle descriptions are superseded.

| Bundle | Deliverable | Dependencies | Status |
| --- | --- | --- | --- |
| B00 | Repository checkout, runnable shell and handoff | None | verified |
| B01 | Catalogue v1 schema, validation and fixtures | B00 | implemented; local contract checks passed |
| B02 | Native catalogue client, cache and read API | B01 | local contract and actual HTTP/core checks passed; CI pending |
| B03 | Native discovery and truthful app details | B02 | implemented; native offscreen interaction passed; real Omarchy pending |
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

B00 is verified for the scoped native foundation on Linux; this is not a real-Omarchy release approval. Catalogue, installation, author tools and payments are future bundles.

The next planned bundle is B01. Real Omarchy desktop observations are tracked separately from offscreen Linux checks in BUILD_HANDOFF.md.
