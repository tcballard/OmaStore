# OmaStore product specification and coding-agent handoff

Version 0.2, native-first edition · 8 September 2026 · Owner: Tom Ballard

This document turns the OmaStore storefront plan into requirements and discrete implementation bundles. It is self-contained. It specifies work to implement; it does not claim that the features, author relationships, compatibility reviews or commercial arrangements already exist.

OmaStore is the working codename for a new project. Tom has supplied [tcballard/OmaStore](https://github.com/tcballard/OmaStore) as the implementation repository. On 8 September 2026, GitHub reported a public repository, default branch name `main`, and an empty repository with no initial contents. Clone that repository or use its existing local checkout, then inspect its current state before editing. No deployed website, domain, catalogue, product credentials or hosting project is assumed to exist. B00 now has a native scaffold; its actual verification state is recorded in the repository handoff. All later bundles remain planned until supported by implementation evidence.

Correction: Tom explicitly confirmed on 8 September that the storefront itself must be native to Omarchy. This edition supersedes the web-first implementation order. The previous edition also incorrectly treated `tcballard/omarchy-apps` as this project's implementation baseline. That relationship was not established by the brief. Its repository-specific constraints, claimed seed records and migration work are superseded by the greenfield requirements below. Do not clone or modify that repository on the strength of this document.

9 September 2026 repository-preview decision: Tom authorised bootstrapping discovery from existing published Omarchy application packages. Six community-indexed entries now replace the empty bundle. These are unclaimed informational entries, not author-submitted approvals or runtime test evidence. See ADR 0010. Author participation and managed commerce are not prerequisites for this discovery preview.

**1. Read this execution contract before beginning a bundle.**

Read this specification and any instructions in the working environment. Inspect the current state of `tcballard/OmaStore` before scaffolding; the empty-repository observation is a dated snapshot. If it is still empty, create the foundation described in B00. Otherwise preserve subsequent work and read its instructions, contribution/submission policies, package scripts and latest handoff where present. Do not claim a file, command or integration exists before creating or inspecting it.

Implement the next dependency-ready bundle within the scope authorised by the user. Each bundle should be a separately reviewable branch/PR or local change set. Split an oversized bundle into suffixed parts such as B07a/B07b while retaining its full acceptance gate. Do not combine unrelated bundles or repeat completed work. Reuse existing capabilities when they already satisfy the requirement and cite the evidence.

After each bundle, record its starting/ending revision, requirement IDs, changes, checks, observed results, limitations and next ready bundle in `docs/BUILD_HANDOFF.md`. Update `docs/BUNDLE_STATUS.md` using `not_started`, `in_progress`, `implemented_unverified`, `verified` or `blocked`. A mocked test can prove protocol logic; it cannot prove actual Omarchy installation or GUI behaviour.

Do not treat this document as authority to publish a site, send messages, charge users, create seller accounts, merge to a protected branch or bypass an existing approval boundary. In an implementation session, use the authority actually supplied there. Continue useful local work while preparing any externally gated action. Feature and commercial gates below concern readiness, rather than a requirement to ask Tom about routine engineering decisions.

Establish a coherent visual direction and reusable components for the new site. Aim for clear editorial presentation, useful product demonstrations and keyboard-friendly interaction. Decide routine engineering details through a short ADR. Bring Tom concrete visual options for matters of taste and a concrete proposal when changing product scope, the public name or commercial terms. Keep infrastructure/configuration details in developer and reviewer interfaces where they help someone make a decision.

**2. Build the storefront as a native application.**

Design target confirmed by Tom: macOS App Store-level polish expressed through native Omarchy conventions. Functional bundle completion is not visual acceptance. Prioritise Discover and the app-detail-to-installation journey before catalogue expansion; ADR 0011 records the first polish pass.

The implementation repository is `tcballard/OmaStore`. It was empty before B00. Inspect the current checkout and handoff before extending it. The storefront opens in its own desktop window. A browser or webview does not supply the primary interface.

| Area | Implementation contract | Owning bundle |
| --- | --- | --- |
| Native interface | Qt 6/QML with public Qt APIs, keyboard navigation, normal Wayland window behaviour and launcher integration | B00 foundation, B03 discovery, B13 library/handoff |
| Local core | Rust; bounded, versioned JSON-lines protocol over a C++ QProcess bridge; no arbitrary command messages | B00 handshake, B02 catalogue client, B12 plans, B14 execution |
| Local state | Cached catalogue and preferences; SQLite journal when lifecycle operations arrive; no default inventory upload | B02, B13, B15 |
| Shared catalogue | Public Git-versioned records and an HTTPS read/status contract; clients can inspect exact revisions | B01–B02, B08–B09 |
| Author/reviewer service | Separate authenticated service behind native views; Rust service and SQLite are initial proposals to verify in B04 | B04–B09 |
| Media | Validated local development storage, then an object-storage adapter with public/private separation | B05 |
| Build | CMake/Ninja for Qt/C++, Cargo for Rust; pinned Rust and crate lockfile; Qt 6.4+ baseline | B00 |
| Distribution | Two sibling executables and a desktop entry; packaging rehearsed locally before an Omarchy release | B00, B13–B16 |
| Optional website | A later distribution/editorial surface, outside the initial build; never a prerequisite for opening the app | Requires a later scoped decision |

No cloud hosting project, domain, web framework or database account is required for B00. Product credentials are distinct from the coding agent's GitHub access. Use the existing remote rather than creating another repository. The architecture is recorded in `docs/adr/0001-native-first.md`.

**3. These product decisions define the target.**

| ID | Requirement |
| --- | --- |
| P01 | Help people discover appropriate applications, understand their limitations, install through supported routes and support their makers. |
| P02 | Provide Discover, Apps, Setups, Library, Makers and Submit as native application views. Shared services support the desktop application; a web storefront is deferred. |
| P03 | Browsing, free installation routes and the local library require no OmaStore account. Author workspaces require sign-in. |
| P04 | Keep app type, maturity, software licence, price model, maintainer identity and compatibility evidence independent. |
| P05 | Make “Use this setup” a selective recipe. Initial recipes select apps and refer to existing plugin flows; configuration writes come later. |
| P06 | Listings, baseline review and normal updates are free. OmaStore takes 0% of external sales and sponsorships; providers may charge their own fees. |
| P07 | Support free software, voluntary support, pay-what-you-want, paid apps, paid upgrades, paid features, subscriptions, professional services and working paid previews. |
| P08 | Published catalogue content remains inspectable and versioned. Authors can distribute independently and use their own checkout. |
| P09 | Publish the project's own new code under MIT, preserving third-party notices. Store media/description publication rights separately. |
| P10 | Source execution, install route, updates and personal data stay under the user's applicable authorisation boundaries. |
| P11 | Compatibility evidence describes a particular release and environment. It is not official Omarchy certification or a security guarantee. |
| P12 | Payments do not change search ranking, review priority, approval or editorial selection. Tom's apps require independent approval and editorial selection. |
| P13 | Initial launch is an independent community project. Reference the current plugin marketplace, preserving upstream IDs and attribution. |
| P14 | Stripe is the default payment provider for optional managed checkout (confirmed by Tom on 9 September 2026). The checkout proposes a 5% service fee before tax, plus disclosed provider charges. It remains disabled until the commerce gate is met. |

Omarchy already documents commercial apps and an existing plugin publishing process. Follow the live runtime contract when implementing adapters. Do not assume plugin listing validation establishes security. [Commercial apps](https://omarchy.org/manual/commercial-apps-services/), [plugin publishing](https://plugins.omarchy.org/publish.html), [plugin runtime](https://omarchy.org/manual/shell-plugins/).

**4. Deliver four releases with explicit boundaries.**

| Release | Included | Deferred |
| --- | --- | --- |
| R1: Native discovery preview | Qt/QML storefront, Rust catalogue client, search, app details, honest evidence/price states, cached and empty views | Author service, managed installation, configuration writes, checkout and a web storefront |
| R2: Native public pilot | Native makers/setups/library, author and review workflows, catalogue publication, supported installation plans/execution, lifecycle reconciliation and external author commerce | Automatic plugin installation without a proven adapter, configuration writes and managed checkout |
| R3: Selective setup customisation | Allowlisted setting diffs, conflict-aware apply/restore and deliberate recipe sharing | Arbitrary scripts, general dotfile synchronisation, automatic desktop changes when following a recipe |
| R4: Optional managed commerce | Provider-approved purchasing, delivery, entitlement recovery, refunds, dispute/payout reconciliation | Multi-author carts, pooled sponsorship, escrow, feature-bounty funds and subscription bundles |

AUR stays an accurately labelled discovery/external route until a separately proven adapter exists. A source-only app remains discoverable with a source/learn action; it does not receive an invented install route. Unreviewed nominations may remain clearly labelled informational candidates. New previews must demonstrate a working core task; future features are labelled as plans.

R1 requires B00–B03 and a real Omarchy desktop exercise of discovery, navigation and cached/empty states. B00 alone is a development scaffold. B16 is the R2 public-pilot gate, covering the complete author/reviewer flow and actual installation evidence. R3 requires B17–B19; R4 requires B20–B22 and the commercial operating gate. Keep later capabilities disabled until their own requirements pass.

**5. Implement these user-facing flows and states.**

| ID / native view | Required interaction | Required exceptional states |
| --- | --- | --- |
| UX01 Discover | Search immediately available, a real editorial story/picks when present, setups and meaningful releases | Empty catalogue, stale cache, no editorial content, unavailable media |
| UX02 Apps | Search and filter name, purpose, category, pricing, licence, architecture, offline behaviour and test result; retain local filter state | No results, invalid/stale filter, missing evidence distinct from incompatible |
| UX03 App detail | Purpose, maker, price/type/maturity, real demonstration, limitations, evidence, supported acquisition, removal and support | Source only, paid external route, older tested release, suspension, missing artifact, unclaimed maker |
| UX04 Maker | Project-control evidence, apps, support destinations and release feed | Unclaimed profile, expired claim, ownership suspension, unavailable feed |
| UX05 Setup | Select components, show dependencies/conflicts, itemise prices and preview changes | Missing component, wrong architecture, changed price, external purchase required |
| UX06 Submit / author workspace | Provider sign-in through the system browser, project claim, native draft editor, preview, submit, findings, revision and publication status | Provider unavailable, expired claim, failed upload, stale edit, interrupted draft save |
| UX07 Reviewer workspace | Assigned queue, candidate diff, typed findings, evidence, independent decisions and audit history | Conflict of interest, stale evidence, role revocation, concurrent decision |
| UX08 Library | Local installed state, saved items, source and supported open/update/remove actions | Offline cache, external upgrade/removal, ongoing package transaction, unsupported host |

Search ordering is deterministic: exact name/ID, name prefix, textual relevance, then known compatibility with an explicitly selected profile, then stable name/ID tie-break. Unknown compatibility is not treated as failure. Editorial picks do not affect this order. Implement one shared search/filter contract for the native client and API.

Use `Open` only when local presence is established, `Install` only when an eligible supported route exists, and `Buy/Get from developer` for external acquisition. The UI must use actual local state rather than infer installation from a catalogue view. Always retain useful source and support information except where a destination itself must be withheld for an integrity incident.

Require an icon and two real screenshots for ordinary app submissions. Require a 15–45 second real task demonstration for editorial feature eligibility; allow accessible longer transcripts separately. Missing media blocks the applicable readiness gate; local fixture assets must be labelled and excluded from public catalogue builds. Never generate evidence screenshots of an application that was not run.

**6. Keep each data object and its authority explicit.**

| Object | Minimum contract | Authority |
| --- | --- | --- |
| Catalogue snapshot | `schemaVersion`, catalogue revision, build revision, generated time, apps, makers, recipes and editorial references | Reviewed Git content delivered by the site |
| App | Stable `id`, slug, name, summary, description, task tags, category, `appType`, maturity, licence class/identifier, maker references, source, media, declared capabilities and support | Published Git record; private candidate in the workflow database |
| Release | App ID, opaque upstream version, immutable source commit where available, artifact digests where applicable, architecture, route, release notes and account/service requirements | Immutable reviewed candidate |
| Install route | `kind` of `arch_package`, `aur_external`, `upstream_external` or `plugin_external`; typed package/source identity; declared privilege and service changes | Reviewed release; executable adapters are application code |
| Offer | Seller ID, model, destination URL, currency and minor-unit amount when supplied, billing interval, tax-inclusion state, checked time, entitlement, licence, support/refund/cancellation links | Author-reviewed content; final price belongs to external checkout in R1/R2 |
| Maker claim | Account, maker/project ID, challenge method, issuer, nonce, verified control evidence, scope, issued/expiry/revocation times | Private the workflow database record with a public verified/claimed projection |
| Draft | Owner, base published revision, editable fields, media IDs, optimistic version and save time | the workflow database, private |
| Submission revision | Immutable candidate ID/digest, submitting actor, source references, GitHub intake issue and state | the workflow database workflow record, public redacted intake |
| Check/test record | Candidate digest, tool/test version, actor, environment, result, timestamp, limitations and evidence reference | Append-only record; public projection excludes private material |
| Approval | Candidate digest, route/release identity, reviewer identities, decision, evidence references and policy version | Trusted review service; published approval receipt in Git |
| Recipe revision | Stable recipe ID, revision, parent attribution, media, components, dependencies, supported settings references and rights | Reviewed Git content |
| Suspension | Target app/release/route, reason, authorised actor, effective time, appeal/resolution history | the workflow database operational override; exposed through a small public status endpoint |
| Native plan | Plan ID/digest, snapshot/release references, local-state fingerprint, selected operations, required privileges, external steps and expiry | Local Rust core |
| Native journal | Plan/operation IDs, intended action, before/after identity, timestamps, state, observed outcome and bounded diagnostics | Local SQLite; no automatic upload |

Use `desktop`, `terminal`, `shell_plugin`, `web` and `service` for app type; `unknown`, `development`, `preview` and `stable` for maturity; `open_source`, `source_available`, `proprietary` and `unknown` for licence class. Keep uncertain values unknown until established. Upstream version strings are opaque: do not require every project to use semantic versioning.

Define release identity as a discriminated union: `source_commit` with canonical repository/full commit; `binary_artifact` with publisher-controlled source/version and SHA-256; or `repository_package` with repository/package/version and available package-signature evidence. Each test record also identifies the bytes executed. Proprietary submissions use a verified publisher and binary/package evidence, with inspection limitations. The new submission policy must explain the different inspection paths.

Create stable IDs and slugs when genuine listings enter the catalogue. A date alone never creates a passing review. There are no inherited records to migrate: use explicitly synthetic fixtures for development and obtain real project facts, media rights and evidence before publishing real listings. Descriptive metadata must not be inferred from a similarly named repository.

Git is the single source for published descriptions, releases, recipes and editorial content. the workflow database stores private drafts, accounts, workflow, audit events and operational suspensions; it cannot silently edit published catalogue content. Object storage holds validated media/evidence with explicit public/private separation. The effective ability to start a managed install is computed from the delivered catalogue plus a fresh suspension/status check. This overlay owns operational eligibility only.

**7. Enforce the workflow on the server.**

The revision state machine is `draft -> submitted -> checking -> in_review -> approved -> publication_pending -> published`. Checking/review can return `needs_changes`; edits create a new immutable candidate and rerun relevant checks. A decision can reject a revision with a reason, and an author can withdraw an unpublished revision. Neither event erases the history. A published release remains intact while a newer candidate is reviewed.

Authors edit their own drafts and request publication. Reviewers inspect and decide. Maintainers own catalogue PR merge eligibility. Operators suspend affected distribution and manage appeals. A GitHub sign-in, hosting-level visitor access and permission to control an upstream repository are three different facts.

Every approval binds to the complete candidate digest, release identity, route and policy version. A label such as `approved-and-verified` is a visible indicator, not sufficient authorisation by itself. Approval receipts are issued through a trusted path. A PR cannot approve itself by editing its receipt, validator, workflow or labels. The trusted publication check validates the proposed data against current authoritative approval records and required reviewer identities.

Own-project approval is prohibited. Privileged helpers, remote-control capabilities and substantial system changes require two eligible independent reviewers. An ordinary application needs one eligible independent reviewer. At bootstrap, missing independent reviewers blocks production approval while fixture-based development can continue.

Publication requests create or update a catalogue PR referencing the exact approved payload. Merging makes a revision eligible for catalogue delivery. The author sees publication pending until the delivered public API reports that catalogue revision. Scheduler retries, webhooks and duplicate button presses must not create duplicate submissions or publication PRs.

Use a three-working-day first-response target. For R2, define a working day as Monday–Friday in Europe/London; disclose that public holidays are not yet modelled. Queue age starts when a complete revision is submitted. A response is an actual finding, review start or decision, not an automated receipt. Alert the operator if the oldest complete item exceeds five working days; expansion pauses after two consecutive weekly breaches.

Compatibility result is `passes`, `limitations`, `fails` or `not_tested`. Freshness is independently `current`, `retest_due` or `superseded`. At 90 days mark a result due for retest. New app identities, material route/capability changes or relevant Omarchy changes invalidate use of the prior result for the new candidate. Unknown and old evidence stay visible without a current-test badge.

Detect new releases and upstream ownership changes on a scheduled read job, initially every six hours. Create candidates and findings; do not promote releases or update the user's computer automatically. An identity/digest change for an allegedly unchanged artifact suspends that managed route pending review. Community reports are labelled community evidence until reviewed.

**8. Expose a versioned boundary between native clients and services.**

| Interface | Required contract |
| --- | --- |
| `GET /api/v1/catalogue` | Published snapshot metadata and records; ETag, schema version and catalogue revision. Public content only. |
| `GET /api/v1/apps` | Validated query/filter parameters, stable pagination and ordering; all pages in one browse sequence refer to one snapshot or request a restart. |
| `GET /api/v1/apps/:id`, `/makers/:id`, `/setups/:id` | Stable ID lookup, current content and version/evidence references; UI routes use stable slugs. |
| `GET /api/v1/status?ids=...` | Bounded target list, current suspensions and freshness timestamp; managed-action decisions fail closed if eligibility cannot be established. |
| `/feeds/releases.xml`, `/feeds/makers/:id.xml` | Valid RSS with stable GUIDs, escaped content and item links. Publish events only after delivery, not on draft creation. |
| Author/reviewer commands | Authenticated draft save/submit/revise/review/publish-request/report actions with ownership checks, anti-CSRF protection, optimistic concurrency and idempotency keys. Exact route names can follow repository conventions. |
| GitHub integration | Signed webhook verification, delivery-ID deduplication, least-privilege app credentials, retries and explicit reconciliation of API outages. |
| Native handoff | `omastore://app/<id>` or `omastore://setup/<id>?revision=<revision>` carries identity and selection intent only. Opening the URI never authorises execution. |
| Native transport | Qt GUI talks to the Rust core through a versioned, bounded JSON-lines protocol over child-process pipes. Include request/operation IDs and monotonic event sequence numbers. |

Reject unsupported major schema versions. Use integer minor units for money, ISO currencies, UTC timestamps, and structured field errors. Mutation responses distinguish authentication, authorisation, validation, stale revision, rate limit, upstream unavailability and internal failure. Never include tokens, raw credentials or private evidence in errors or exports.

The native core accepts only its configured catalogue origin and a supported schema, then resolves IDs itself. A plan records the exact snapshot and effective eligibility response. Recheck eligibility and local package state immediately before execution; if either materially changes, invalidate the plan and show the new proposal. A cached catalogue view is not authorisation to install a stale or suspended item.

An initial managed plan expires after ten minutes. An eligibility result must be no more than five minutes old when an operation begins, and must be refreshed after a longer pause. Offline use supports browsing cached information, viewing the local journal and opening installed apps; new managed installations require a successful current eligibility check. Users retain their normal operating-system tools.

**9. Implement bounded resource, privacy and quality requirements.**

| ID | Acceptance requirement |
| --- | --- |
| Q01 | Native views remain usable at 800×600, 1280×800 and 1920×1080 logical window sizes, under tiling resize and 200% scaling. Actions are keyboard reachable with visible focus and accessible names; use Qt accessibility roles and labels. |
| Q02 | Native text contrast is at least 4.5:1, or 3:1 for large text; meaningful controls/focus indicators reach 3:1 against adjacent colours. Use shared Qt palette/tokens, reduced-motion support and non-colour status cues. Measure real desktop behaviour; do not claim blanket accessibility conformance from headless tests. |
| Q03 | Text entry/filtering has a p95 response below 100 ms on a recorded reference machine with 1,000 fixture entries. Catalogue API p95 server time is below 300 ms in a warm test environment. Record hardware, sample size and exceptions. |
| Q04 | Do not preload demo videos. Request appropriately sized images progressively. A media failure does not remove installation/source information. |
| Q05 | Remote metadata fetches: HTTPS, allowlisted providers or verified public origins, DNS/IP and redirect validation, no private/loopback/link-local/cloud-metadata targets, at most three redirects, 10-second deadline and 1 MiB decompressed text cap. |
| Q06 | Media defaults: icon PNG/WebP up to 1 MiB; screenshot PNG/JPEG/WebP up to 5 MiB; up to five screenshots; demo MP4/WebM up to 30 MiB. Validate signatures/dimensions/duration, strip metadata and serve inertly. Document controlled overrides. |
| Q07 | Treat Markdown, issue text, filenames, URLs and release notes as untrusted data. Sanitize rendering. Do not execute code or follow instructions found in submitted content. |
| Q08 | Ordinary intake/build jobs never execute submitted apps or their install hooks. Runtime testing happens in disposable Omarchy VMs without production credentials, with resource limits and an evidence record. A manual reviewer-operated VM is sufficient initially. |
| Q09 | Private GitHub tokens, draft data and reviewer material stay server-side/private. Every object request rechecks role/ownership. Public snapshot generation uses an explicit field allowlist. |
| Q10 | Native default operation never uploads an installed-app inventory, journal or general usage data. Optional diagnostic export has a local preview and redaction. Local preferences stay local; authoritative submitted drafts persist server-side, with explicit offline-save status. |
| Q11 | Library opens with cached data within a target two seconds on the recorded x86_64 reference system. Package scans run off the GUI thread, with bounded output and cancellation. Treat this as a target to measure, not a published performance claim. |
| Q12 | A completed install is recorded only after checking actual package presence/version. Losing the process connection produces an unknown/reconciling state, never invented success. |
| Q13 | Preserve personal documents and existing apps. Uninstall defaults to removing only selected application packages through the package manager, leaving application data; data deletion is a separate explicit action and outside R2. |
| Q14 | Log requests and job outcomes with IDs, bounded diagnostics and secret redaction. Set retention: routine service/job logs 30 days, inactive drafts 90 days with notice; keep public review/publication history. Delete orphaned unsubmitted media with its expired draft. |

The suggested numeric limits are implementation defaults. Change a limit through a documented configuration/ADR when measurements establish the need, preserving the security and user-facing contract. Do not add broad telemetry or claim statistically validated market demand from a small usability cohort.

**10. Use the bundle catalogue below as the implementation order.**

Core dependency chain: B00 → B01 → B02 → B03 builds the native discovery preview. B04–B11 add author/reviewer services and native views; B12–B16 add planning, installation and public-pilot evidence. B17–B19 add selective settings. B20–B22 are optional managed commerce. The dependency table is authoritative and does not instruct an agent to spawn subagents.

| Bundle | Deliverable | Direct dependencies | Release |
| --- | --- | --- | --- |
| B00 | Repository checkout, runnable shell and handoff | None | Foundation |
| B01 | Catalogue v1 schema, validation and fixtures | B00 | R1 |
| B02 | Native catalogue client, cache and read API | B01 | R1 |
| B03 | Native discovery and truthful app details | B02 | R1 |
| B04 | Publisher identity and persistent workspaces | B01 | R2 |
| B05 | Submission drafts, media and preview | B04 | R2 |
| B06 | Bounded checks and evidence intake | B05 | R2 |
| B07 | Review workflow and approval records | B06 | R2 |
| B08 | GitHub publication bridge | B02, B07 | R2 |
| B09 | Release monitoring and suspensions | B08 | R2 |
| B10 | Makers, editorial selection and RSS | B03, B04, B08 | R2 |
| B11 | Setup catalogue and selective recipes | B03, B08 | R2 |
| B12 | Read-only installation planner | B02, B09 | R2 |
| B13 | Native library and identity handoff | B12 | R2 |
| B14 | Supported package execution | B12, B13 | R2 |
| B15 | Journal reconciliation and lifecycle | B11, B14 | R2 |
| B16 | Pilot operations and release readiness | B09, B10, B11, B15 | R2 |
| B17 | Supported settings schema and diffs | B11, B15, B16 | R3 |
| B18 | Setting apply, conflict and restore | B17 | R3 |
| B19 | Recipe remix and private-data-safe export | B18 | R3 |
| B20 | Commerce operating model and disabled integration | B16 | R4 gate |
| B21 | Checkout, delivery and entitlements | B20 | R4 |
| B22 | Refunds, disputes, payouts and commerce readiness | B21 | R4 |

**B00 creates the native application scaffold and development foundation.**

Outcome: a clean checkout builds a Qt/QML window and Rust core with a verified local handshake. Covers the execution contract and native architecture.

Scope: inspect the confirmed repository, preserve newer work, add the Rust workspace and pinned toolchain/lockfile, CMake/Ninja build, public Qt/QML interface, narrow C++ QProcess bridge, standard desktop entry, focused CI checks, MIT licence, contribution/submission stubs and bundle/handoff records. The protocol initially permits only `core.info`. It must reject malformed or oversized input, unsupported versions and unknown methods without executing anything. The window provides navigation and truthful empty states.

Likely files: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `CMakeLists.txt`, `native/core/*`, `native/ui/*`, `packaging/*`, `.github/workflows/*`, README and handoff/ADR documents. No catalogue ingestion, login, package execution, web frontend or cloud deployment belongs in this bundle.

Acceptance: B00-A1 clean build and documented run commands produce the native window and successful Rust handshake. B00-A2 dependency versions and missing future integrations are recorded. B00-A3 every bundle has a status and the next ready bundle is named. B00-A4 the checkout targets `tcballard/OmaStore`, preserves unrelated work and requires no production credentials.

Verification: Rust formatting/Clippy/tests, real child-process protocol tests, Qt compilation and offscreen startup, plus installation staging. Record a real Omarchy window/launcher exercise separately; an offscreen Linux pass does not establish it. Rollback: revert this scaffold change set and unregister only its own staged files; preserve unrelated application state.

**B01 creates catalogue v1 without inventing trust evidence.**

Outcome: the catalogue can represent the intended product consistently. Depends on B00. Covers P04, P07–P09, P11 and data contracts in section 6.

Scope: shared runtime schemas and types; release identity unions; makers, offers, media, test/evidence records and recipe primitives; unique IDs/slugs; referential integrity; structured installation routes. Start the new public catalogue at schema version 1 with no invented live listings. Support unknown values explicitly. Produce typed schemas and development fixtures shared by the Rust service and native core. Derive any display command from a validated route instead of accepting command text as authority.

Likely files: a shared Rust catalogue crate, `data/registry.json`, a Rust validation command, `tests/fixtures/*`, a complete submission example and focused contract tests. Keep one schema authority. Synthetic fixtures must live outside the public dataset and use a development-only loading path.

Acceptance: B01-A1 an empty public catalogue is valid and development fixtures cannot enter a public build through the fixture loader. B01-A2 missing evidence or a date alone produces no passing review. B01-A3 rejects duplicate identities, missing release references, wrong money types and unsafe package tokens. B01-A4 supports source-commit, publisher binary and repository-package records without claiming equivalent inspection. B01-A5 provides a complete valid submission example and a validator command with documented failure output.

Verification: positive/negative schema fixtures, empty-catalogue handling, deterministic serialization and a public-build fixture exclusion check. Rollback: revert the schema bundle before production content exists; once genuine records exist, preserve their identities and use an explicit version transition.

**B02 provides one public read contract for every client.**

Outcome: consumers see the same published facts and search results. Depends on B01. Covers UX02, P08, Q03, Q09 and section 8.

Scope: versioned catalogue/app/maker/setup read routes, ID/slug resolution, deterministic filtering/order, snapshot-bound pagination, ETags and a strict public-field projection. Define response fixtures and structured errors. Add the native HTTPS catalogue client and bounded cache, and test against the real local read service. Keep this bundle read-only; draft, identity and publication mutation endpoints follow later.

Likely files: Rust read-service routes, native cache/client modules, shared query functions, public serializers and API tests. Public catalogue reads must not require a provisioned private database.

Acceptance: B02-A1 native and API query fixtures return identical order. B02-A2 invalid filters and page cursors have bounded documented behaviour. B02-A3 concurrent snapshot changes never silently mix two snapshots in one page sequence. B02-A4 no test fixture containing a token, reviewer contact or draft field can leak through public serializers. B02-A5 unsupported schema versions fail explicitly.

Verification: HTTP contract tests through the actual Rust service entry, projection-leak tests, conditional-request tests and one measured 1,000-entry query sample. Rollback: preserve the B00 shell and use additive API versioning once clients depend on a published contract.

**B03 makes discovery and app details usable and accurate.**

Outcome: a newcomer can find and assess an application. Depends on B02. Covers P01–P04, UX01–UX03 and Q01–Q04.

Scope: build native Discover, Apps and app-detail views, retain local search/filter state, render type/price/licence/maturity independently, and show an action derived from real route eligibility. Build real media slots with accessible fallbacks and explanatory evidence states. Display extra service costs and seller/refund links for external purchases. Use clearly labelled development data for local review until genuine listings are ready.

Likely files: `native/ui/qml/*`, native catalogue models, reusable app/evidence/action controls and shared Qt style values. Extend the B00 primitives and show Tom concrete visual options for taste decisions. Do not fabricate application screenshots or imply the codename is a cleared launch brand.

Acceptance: B03-A1 preserves search/filter state across refresh, native back navigation and application restart. B03-A2 an unknown test state and an older tested release cannot render as current verification. B03-A3 AUR is labelled AUR; an unsupported managed route offers a useful external action. B03-A4 offline/service/activation/price information is readable before acquisition. B03-A5 keyboard-only and small-window flows succeed, including empty/error/media-failure states.

Verification: route/render tests, action-state fixtures and actual Qt interaction/accessibility checks for these flows. Record screenshots as QA evidence, not as proof of the listed apps' functionality. Rollback: revert presentation changes without rewriting catalogue data.

**B04 gives authors a private workspace and a scoped project claim.**

Outcome: someone can control only the submissions they are entitled to maintain. Depends on B01. Covers P03, P12, UX06–UX07 and Q09.

Scope: extend the Rust service with a private workflow database and migrations, implement a provider-supported OAuth flow using the system browser, and verify project control with least privilege. Bind sign-in to the initiating desktop session; keep provider secrets server-side and store desktop tokens in the system secret service, never plaintext preferences. Add a publisher-domain challenge for proprietary software. Claims carry target, nonce, expiry and revocation; revalidate on sensitive identity changes. B04 records the chosen maintained server/auth libraries and operating setup in an ADR.

Likely files: service migrations, auth/claim routes, native sign-in state and secret-service integration. Keep hosting visitor access independent from author identity. Do not place deployment/GitHub-connector tokens in product code or request unrestricted repository access merely to sign in.

Acceptance: B04-A1 another user's draft/claim cannot be read or changed by ID substitution. B04-A2 sign-in alone cannot claim an upstream project. B04-A3 expired/replayed/domain-mismatched challenges fail. B04-A4 revoking a role blocks the next privileged request. B04-A5 a nomination retains its unclaimed label and does not receive seller/update rights.

Verification: session/CSRF/ownership tests, challenge replay tests, database migration on a populated fixture and a real callback test with operator-provided credentials. Fixture sign-in stays confined to test builds. Rollback: additive migration and feature flag; preserve existing anonymous catalogue access.

**B05 turns submission into a persistent, previewable draft.**

Outcome: authors can prepare a complete listing and recover their work. Depends on B04. Covers UX06, P07 and Q05–Q07, Q09, Q14.

Scope: draft creation/import/editing, optimistic concurrency, validation feedback, autosave, native listing preview, media upload and immutable submit snapshots. Create the local media adapter and a deployable object-storage interface with separate draft-private and approved-public media access. Implement offer fields for every launch monetisation model and the draft/orphan-media retention job required by Q14. Default to a server-side upload route; use direct uploads only when the hosted capability can authorise them narrowly.

Likely files: native author/submit views, draft/media service routes, object-storage adapter and submission schemas. This bundle stores drafts; it does not approve or publish an application.

Acceptance: B05-A1 an application restart recovers the server draft. B05-A2 two editors cannot silently overwrite each other. B05-A3 malformed, oversized, falsely labelled or active-content media is rejected safely. B05-A4 a failed upload does not erase other fields; orphan cleanup obeys retention. B05-A5 submit freezes a candidate digest and later edits create a new revision. B05-A6 the author previews exactly which data will become public before opening public intake.

Verification: draft/reload/conflict flows, upload/SSRF negative cases and submission digest tests. Rollback: disable new submissions while preserving draft records and private media for recovery.

**B06 produces bounded automated findings without executing submissions.**

Outcome: authors and reviewers get useful evidence about the proposed record. Depends on B05. Covers P11, Q05–Q09 and section 7.

Scope: queue structural/provenance/link/manifest checks, compare source identity, apply network limits, record per-check tool/version/timing and distinguish failure from unavailable evidence. Import manual runtime-test records from disposable Omarchy VMs. Start with an operator-triggered or supported scheduled worker; document the actual deployment mechanism instead of assuming a queue product is configured.

Likely files: `lib/checks/*`, job endpoints, workflow runner adapter, check-record tables and fixtures. Reuse B01's validator in any GitHub structural-check workflow. Dependency inspection must not run installation or lifecycle hooks.

Acceptance: B06-A1 repeated delivery does not create duplicate active jobs or conflicting terminal results. B06-A2 malicious issue text is data; commands or instructions in it never run. B06-A3 redirect/DNS/decompression limits are enforced. B06-A4 timeouts, unavailable upstreams and missing runtime tests cannot become passes. B06-A5 a VM report records the tested bytes and actual environment and distinguishes automated from human observations.

Verification: deterministic job/retry tests, bounded-network fixtures and a reviewer-supplied runtime evidence sample. Rollback: pause job dispatch and retain findings; a missing check keeps approval unavailable.

**B07 makes review decisions specific, independent and auditable.**

Outcome: reviewers can approve the exact thing they inspected. Depends on B06. Covers P11–P12, UX07 and section 7.

Scope: reviewer queue, candidate diff, typed findings, test/environment records, needs-changes/reject/approve transitions, independent reviewer rules and append-only decision history. Store a frozen approval receipt linked to the candidate digest, policy and route/release identity. Model archived/withdrawn revisions separately from current candidates. Calculate queue age under the documented working-day rule.

Likely files: review UI/routes, workflow service, approval/audit tables and policy tests. No catalogue publication belongs here. A maintainer must not need to edit database rows to make an ordinary decision.

Acceptance: B07-A1 self-approval fails even for an operator account. B07-A2 high-impact candidates require two eligible reviewers. B07-A3 any material candidate edit invalidates approval eligibility. B07-A4 two concurrent decisions are serialised or return a stale-revision error. B07-A5 a check pass or GitHub label alone cannot issue approval. B07-A6 private evidence stays private in author/public views.

Verification: state-transition/role/concurrency tests and an end-to-end author revision followed by independent review. Rollback: make review read-only and retain historical receipts; do not rewrite past approvals.

**B08 establishes GitHub intake and the catalogue publication authority.**

Outcome: approved content reaches the native catalogue through a reviewable catalogue PR. Depends on B02 and B07. Covers P08, P13, UX06–UX07 and publication contracts.

Scope: configure the catalogue publication workflow for `tcballard/OmaStore` and its designated publication branch, create a public issue template, create redacted intake on explicit submission, reconcile issue submissions with native-submitted candidates, ingest signed webhooks, and request a catalogue PR from an exact approved revision. Add a trusted approval check for registry changes and document the required protected-branch configuration. Record candidate/issue/PR/commit/delivered-snapshot relationships. A direct contributor PR follows the same approval rules. Repository existence does not supply the deployed product's GitHub App credentials; complete the adapter and local tests while any real integration gate remains outstanding.

Likely files: GitHub adapter, webhook routes, publication jobs, `.github/workflows/*`, `.github/ISSUE_TEMPLATE/*`, `CONTRIBUTING.md`, `SUBMISSION.md` and approved receipt records. Product credentials are supplied by the operator; a coding agent's connector credentials are not product credentials.

Acceptance: B08-A1 duplicate webhook or publish requests yield one logical publication. B08-A2 a stale or tampered approval/PR digest is rejected. B08-A3 modifying CI or receipt files in a PR cannot self-authorise publication. B08-A4 missing provider credentials disables publication with a useful status while local draft/review work continues. B08-A5 the UI distinguishes approved, PR pending, merged and publicly delivered. B08-A6 a deployment failure leaves the last delivered catalogue usable.

Verification: forged/replayed webhook tests, a trusted-check fixture that modifies its own receipt/workflow, GitHub outage recovery, and a real test-repository issue/PR cycle when authorised. Rollback: disable publication writes, keep normal reviewed manual PR handling, and reconcile pending jobs rather than duplicating them.

**B09 keeps release evidence and distribution status current.**

Outcome: changed or withdrawn software does not retain misleading current evidence. Depends on B08. Covers P11, Q09, Q14 and status/freshness contracts.

Scope: six-hour source polling through bounded adapters, candidate creation, evidence ageing, route/owner/digest change detection, manual reports, operator suspension and appeals. Expose the public status overlay with a current server timestamp. Store reasoned changes and distinguish security/integrity incidents from ordinary updates. Suspension affects distribution eligibility; it does not delete users' installed applications.

Likely files: monitoring jobs, Rust status-service routes, report/operator views, the workflow database suspension tables and evidence selectors. Do not attempt to scrape arbitrary private provider APIs or promote candidate releases automatically.

Acceptance: B09-A1 a new release is a candidate with unknown current compatibility. B09-A2 an unchanged identity with different bytes suspends its route. B09-A3 a date boundary marks evidence due exactly once without erasing the original result. B09-A4 public status omits confidential report details. B09-A5 job retries and provider outages preserve known findings and visibly stale status. B09-A6 an authorised resolution restores eligibility through an auditable transition.

Verification: clock-controlled ageing, duplicate update events, simulated ownership/digest changes, role checks and status freshness tests. Rollback: pause polling while retaining operator suspension controls and the last observed status.

**B10 establishes makers, editorial discovery and useful release feeds.**

Outcome: people can understand who builds an app and return for meaningful changes. Depends on B03, B04 and B08. Covers P02, P12, UX01, UX04.

Scope: maker profiles and claimed/unclaimed attribution; RSS release feeds; reviewed editorial story/pick records; support links; explicit conflict-of-interest handling; a simple editorial scheduling interface that produces reviewed content changes. Weekly refresh is an operating target, not fabricated automated editorial judgement.

Likely files: maker/feed routes, editorial schemas/UI, app-to-maker relationships and RSS serializers. Account notification feeds, star ratings, recommendation tracking and advertising are outside this bundle.

Acceptance: B10-A1 an unclaimed listing cannot imply that the maintainer joined the store. B10-A2 unpublished updates generate no release-feed entries. B10-A3 RSS GUIDs survive title corrections and escaping prevents malformed feeds. B10-A4 a paid/sponsored flag cannot change the query ranking or review priority. B10-A5 Tom cannot feature his own app through an unreviewed editorial action. B10-A6 insufficient editorial content produces a smaller useful page rather than invented placeholders.

Verification: feed parse tests, publication-event idempotence, conflict-of-interest policy tests and maker-page keyboard flow. Rollback: retain app discovery and source links if editorial delivery is disabled.

**B11 makes a setup a useful, versioned selection of software.**

Outcome: a visitor can understand and select components of a real workflow. Depends on B03 and B08. Covers P05, UX05 and recipe contracts.

Scope: recipe schema, component references, dependency/conflict declarations, optional selections, individual costs, attribution and a native recipe detail view with stable shareable identity. Implement the native selection/export/handoff representation and author/reviewer flow for recipe revisions. Prepare three real editorial recipe candidates once tested media and components exist.

Likely files: `data/setups/*`, setup routes/components, shared recipe schema and selected-recipe export. Initial export contains only stable catalogue references and chosen options, not filesystem paths or machine inventory.

Acceptance: B11-A1 required dependencies are explained and cannot be silently deselected while their parent stays selected. B11-A2 dependency cycles/missing references fail validation. B11-A3 a removed or suspended item cannot be silently substituted. B11-A4 external purchases appear as separate steps with current item prices; unknown totals remain unknown. B11-A5 choosing a recipe does not write desktop settings or install anything. B11-A6 every selected revision retains creator/parent attribution and explicit sharing rights.

Verification: graph/selection fixtures, recipe-identity round-trip, unknown/mixed pricing, changed component and recipe-review tests. Rollback: recipes remain readable as native detail views if selection handoff is disabled.

**B12 builds the native core as a read-only planner first.**

Outcome: the target machine can explain a proposed install without changing itself. Depends on B02 and B09. Covers P10, UX08, Q10–Q12 and native plan contracts.

Scope: extend the existing Rust core with OS/architecture probes, package queries, typed plan generation and versioned plan messages. Reuse the B00 Qt/C++ process bridge and B02 catalogue client. Add preview rendering in the existing window; do not create a second application shell. Keep platform writes absent here.

Probe the actual current Omarchy package CLI and installed repository configuration. Prefer trusted configured repository packages for managed installs. AUR, source-only apps and unproven plugin operations remain external/manual steps. Build argument arrays from validated identifiers; a catalogue `command` field, URI or description can never become executable text.

Acceptance: B12-A1 supported/unsupported/unknown host profiles produce explicit eligibility. B12-A2 dry-run leaves filesystem/package state unchanged except private logs/cache. B12-A3 an already-installed app becomes a no-op or an explained lifecycle step. B12-A4 malicious IDs, leading-option tokens, changed source and stale status cannot enter a runnable plan. B12-A5 the same inputs produce the same normalised plan digest. B12-A6 plan/transport messages have version, size and item-count bounds; cap each JSON line at 256 KiB and a plan at 100 app operations initially.

Verification: Rust contract tests using shared catalogue fixtures, recorded package-query fixtures, a real read-only probe on Omarchy and a cross-client identity/digest test. Rollback: remove the experimental native package; no installed applications were changed.

**B13 extends the native window with its library and identity-only handoff.**

Outcome: users can inspect their library and review a proposed action in a normal desktop window. Depends on B12. Covers UX08, P03, Q01, Q02, Q10, Q11.

Scope: extend the B00/B03 window and B02 cache with installed/saved views, release detail, plan preview and `omastore://` registration. Extend the existing core bridge with operation IDs and event sequencing. Show process loss and reconnect/reconciliation states. The GUI remains unprivileged.

Likely files: `native/ui/*`, `native/qt/*`, desktop entry/icon integration and packaging skeleton. Carry forward the visual language established in B03 with documented Omarchy style integration and fallbacks. Do not assume Quickshell private imports work in a standalone Qt app.

Acceptance: B13-A1 opening a handoff URI only opens the referenced proposal. B13-A2 user-supplied origins, commands, paths and oversized URIs are rejected. B13-A3 library reading and installed-app launch work from cache when the network is absent. B13-A4 a second invocation safely focuses or opens the intended view without duplicating a write session. B13-A5 keyboard navigation, 200% text scaling and tiling resize remain usable. B13-A6 the UI never labels a proposal as installed.

Verification: protocol/process-failure tests and actual Omarchy GUI exercise with screenshots and environment recorded. Headless rendering alone leaves the bundle `implemented_unverified`. Rollback: unregister the application's own desktop/URI entry without touching other apps.

**B14 executes one supported package operation with visible consent.**

Outcome: a user-approved plan can install a supported application through the existing system mechanism. Depends on B12 and B13. Covers P10, Q12, Q13 and native execution boundaries.

Scope: allowlisted package adapter; pre-execution status/local-state refresh; privilege handoff to the actual operating-system flow; streamed bounded progress; per-operation results; package-manager locking and reconciliation. Begin with packages in trusted configured repositories. When the CLI requires an interactive terminal or polkit prompt, preserve that interaction and observe the real outcome. Do not create a password-capturing dialog, embed administrator credentials or run the whole GUI as root.

Use sequential package operations or a single supported package transaction after previewing its full effects. If a system upgrade is required, explain and hand off to the supported updater; never manufacture a partial repository upgrade to satisfy a store badge. Any new package route needs its own adapter proof.

Acceptance: B14-A1 execution needs a fresh explicit confirmation of the current plan. B14-A2 changed packages, dependencies, privileges or status invalidate that confirmation. B14-A3 cancelling before execution performs no install. B14-A4 privilege denial, repository outage and a package-manager lock have distinct recoverable outcomes. B14-A5 completion is established by querying installed version/source. B14-A6 user/OS credentials never enter diagnostic logs. B14-A7 arbitrary shell syntax in catalogue data remains inert.

Verification: one real clean-VM install and corresponding denial/cancellation/lock cases, plus focused adapter/injection tests. A simulated adapter is insufficient for `verified`. Rollback: disable managed install and retain the reviewed external/manual route; package removal is an explicit user choice.

**B15 makes interruption, update and removal behaviour trustworthy.**

Outcome: users can recover from incomplete work and manage the installed application afterwards. Depends on B11 and B14. Covers P05, UX08, Q10, Q12, Q13.

Scope: durable local SQLite journal; operation idempotency; unknown-outcome reconciliation; external installation/update/removal detection; supported removal/update handoff; selected-app setup execution using the B11 contract. Record whether an application pre-existed and which recipes reference it.

An operation moves through `planned`, `awaiting_user`, `running`, then `succeeded`, `failed` or `unknown`, followed by reconciliation where needed. Cancellation stops future work at safe boundaries. Once a package manager is mutating the system, show “Finishing the current operation” rather than killing it in an unsafe phase. Do not treat a surviving PID alone as proof the same job is still running.

Acceptance: B15-A1 restart after every durable transition neither duplicates installs nor invents success. B15-A2 a half-completed recipe reports each outcome and can resume selected unfinished work. B15-A3 removing a recipe never automatically removes pre-existing/shared applications. B15-A4 remove/update delegates to supported system behaviour and preserves documents/config by default. B15-A5 an external upgrade changes the library's installed version independently of catalogue evidence. B15-A6 credential/path redaction is applied to diagnostic exports.

Verification: interruption fault injection around journal commits, real package-state reconciliation, repeat execution and a real install/update-or-updater-handoff/remove cycle. Rollback: migrate journal additively and keep read-only recovery available if new execution is disabled.

**B16 supplies the operational and user evidence for a public pilot.**

Outcome: the first release is demonstrably usable and maintainable. Depends on B09, B10, B11 and B15. Covers all R1/R2 requirements and Q01–Q14.

Scope: production/private evidence separation, operator queue/status dashboard, retention jobs, alerting, install failure categories, support/report routing, dependency/secret audit of the actual implementation, packaging documentation, the selected hosting release procedure, rollback rehearsal and a small participant test plan. Document the actual native packaging and supporting-service release procedures; a website is not a release requirement. Record recurring costs and review time; keep managed commerce disabled.

Acceptance: B16-A1 five authors complete submission, including at least three outside Tom's projects, and one completes an update. B16-A2 20–30 real app pages and three tested recipes have appropriate evidence and truthful actions; do not pad the count with invented or unready apps. B16-A3 ten representative managed apps pass their lifecycle cases. B16-A4 at least 18 of 20 willing newcomers complete a selected supported find/install/launch task unaided, with download delays recorded separately. B16-A5 backup/restore, a failed catalogue delivery, a suspended route and stale status are rehearsed. B16-A6 the named operator/reviewer roles and support route exist before public launch.

Verification: a single release acceptance report with exact versions, screenshots, participant observations, failures and resolved/excluded cases. Recruitment and real Omarchy access are external evidence dependencies: finish implementable work, but never mark those gates passed from fixture data. Rollback: known working deployed version plus a status/managed-install disable procedure; do not erase user journals or install state.

**B17 defines the small set of settings OmaStore can safely understand.**

Outcome: a recipe can show a precise setting change before writing it. Depends on B11, B15 and B16. Covers P05, P10 and Q13.

Scope: a versioned settings-adapter interface, typed values, capability detection, read-only diff, preconditions and backup contract. First candidates are desktop theme selection and placement of an existing bar widget. Enable a candidate only after proving the current documented read/write behaviour and affected files on the target Omarchy release. Unsupported settings stay manual steps. Do not turn this into a general dotfile editor.

Each adapter declares its stable key, supported environment, value schema, read method, planned effects, conflict fingerprint and restoration limits. A recipe supplies desired values, never executable hooks or arbitrary paths. Theme values and widget choices belong to the recipe/user; the agent must not silently choose a new appearance for an existing desktop.

Acceptance: B17-A1 unknown settings and unsafe values cannot enter a write plan. B17-A2 a diff includes existing value, desired value, affected scope and restoration limits. B17-A3 missing plugins and changed desktop versions produce explicit unsupported/conflict states. B17-A4 no diff generation changes the user's configuration. B17-A5 the enabled initial adapters have measured before/after evidence and documented ownership boundaries.

Verification: adapter contract fixtures and real read-only comparisons on an unchanged and customised desktop. Rollback: disable the settings feature; application recipes continue to work.

**B18 applies and restores supported settings without overwriting later edits.**

Outcome: selected settings can be changed deliberately and recovered where supported. Depends on B17. Covers P05, P10, Q12–Q13.

Scope: compare-before-write preconditions, narrowly scoped backup, durable before/after journal, supported atomic file replacement where applicable, permission preservation, per-setting results and conflict-aware restore. Never modify packaged Omarchy source. Reject symlink/path escapes, and only clean up private backup files owned by OmaStore.

Restore automatically only if the current value still equals the value OmaStore applied. Otherwise explain the difference and let the user choose. An adapter with broader effects must state those effects and use its documented recovery behaviour; do not promise whole-system rollback.

Acceptance: B18-A1 a manual change after preview invalidates the write plan. B18-A2 a later manual change after apply blocks automatic restore. B18-A3 applying the same recipe twice is idempotent. B18-A4 interruption cannot turn a partial write into a successful record. B18-A5 permission, malformed-config and reload failures preserve recoverable evidence. B18-A6 disabling a recipe does not erase documents or unrelated settings.

Verification: real customised-desktop apply/restore tests, fault injection at journal/write boundaries, permissions/symlink tests and documented reload behaviour. Rollback: retain the recovery UI and backups even if new apply operations are disabled.

**B19 lets people remix and share recipes deliberately.**

Outcome: a working setup can become an attributed, reusable community contribution. Depends on B18. Covers P05, P08–P09 and Q10.

Scope: local recipe save, fork/remix identity, parent revision attribution, explicit selection of exportable components/settings, export preview and submission through the existing author/review process. Store secrets and local paths outside the export schema. Allow curators to attach their existing support link; recipes remain free in this release.

Likely files: native recipe editor/export, native recipe submission fields and schema validation. Full desktop inventory export, private file synchronisation, automatic publishing and paid bundle splitting remain outside scope.

Acceptance: B19-A1 credentials, machine paths, account IDs and unselected values cannot be serialised even if present in local state. B19-A2 a remix retains its parent attribution and supplied media rights. B19-A3 a local save does not publish. B19-A4 a recipe can be imported on another compatible machine and recomputed against that machine's state. B19-A5 missing components and user edits remain reviewable differences rather than silent substitutions.

Verification: adversarial export fixtures, cross-machine recipe round-trip and a real author/reviewer remix flow. Rollback: preserve local recipes and standard export reading while disabling new public submissions.

**B20 resolves the commercial operating model before enabling checkout.**

Outcome: the optional commerce feature has a specific responsible operator and a provider-supported design. Depends on B16. Covers P06–P07, P14 and the author-economics contract.

Scope: document provider selection and supported multi-author arrangement, seller identity, supported regions/currencies/product types, receipts/tax responsibilities, dispute/refund roles, payout schedule/reserves and licence-delivery contract. Propose the 5% service fee on the subtotal after discounts and before tax, with proportional reversal on refunds and separate provider charges. Prepare disabled provider interfaces and sandbox fixtures.

Entry evidence: at least five authors request managed checkout, an operator is assigned, operating costs are modelled and the provider supports the intended arrangement. These facts must be supplied or verified; the agent cannot create them through test fixtures. Continue supporting external checkout indefinitely.

Acceptance: B20-A1 buyer and seller terms identify the actual seller and responsibilities consistently with provider configuration. B20-A2 all fees and seller payout timing are concrete and documented. B20-A3 tax, refund, cancellation and consumer-rights handling are reviewed for the chosen operator/regions. B20-A4 a kill switch keeps real charges unavailable until the complete commerce release gate. B20-A5 an ADR records unresolved provider limitations with a named owner and explicit effect on scope.

Verification: provider sandbox capability proof and a reviewed responsibility/fee matrix. A payment processor integration alone is not evidence those obligations are satisfied. [Merchant-of-record responsibilities](https://docs.stripe.com/connect/merchant-of-record). Rollback: keep commerce disabled; free listing and external author income are unaffected.

**B21 implements purchases, delivery and recoverable entitlements.**

Outcome: a paid order produces the promised software or service entitlement exactly once. Depends on B20's completed gate. Covers P07, P14 and commercial delivery.

Scope: server-priced checkout sessions, seller/product/price snapshots, immutable order lines, provider event verification, delivery jobs, receipt history and licence/download recovery. Use integer minor units according to the currency. Associate buyer identity through the provider-supported flow; do not add an account requirement to free installations.

Use an order state machine that separates `created`, `payment_pending`, `paid`, `delivery_pending`, `delivered` and `delivery_failed`. A browser redirect is not proof of payment. Provider-confirmed events and reconciliation establish payment state. Lost/replayed/out-of-order events must converge correctly. The platform fee remains server-controlled and frozen on the order.

Acceptance: B21-A1 client price/seller/currency tampering cannot change a payable order. B21-A2 retries cannot charge or deliver twice. B21-A3 a paid-but-undelivered order enters support/retry flow and remains visible. B21-A4 receipts and licence recovery work without relying on the buyer's original browser. B21-A5 a local perpetual entitlement has an offline recovery/verification path; service dependencies are explicitly disclosed. B21-A6 open-source licence rights are not replaced by a store entitlement check.

Verification: provider sandbox purchase, delayed/missing/forged webhook cases, duplicate delivery and seller-secret isolation. Real-money availability remains off until B22 verifies the full lifecycle. Rollback: disable new checkout while preserving receipts, delivery retries and existing entitlements.

**B22 completes refunds, disputes, payouts and the commerce release gate.**

Outcome: the store can operate paid transactions through their whole lifecycle. Depends on B21. Covers P14 and R4 readiness.

Scope: buyer refund requests, provider execution/status reconciliation, proportional platform-fee reversal, published handling of retained processor fees, dispute evidence, seller payout reporting, reserve/negative-balance handling and support escalation. Subscription cancellation changes future billing under the actual product terms; it must not silently delete the buyer's local documents.

Model refund requested/pending/succeeded/failed separately from payment and entitlement state. A partial refund cannot be rounded into an unintended full refund. Preserve an auditable ledger or provider-backed reconciliation record; do not compute author income from browser checkout clicks.

Acceptance: B22-A1 full and partial refunds reconcile against the original order and fee basis. B22-A2 duplicate events never refund or pay out twice. B22-A3 seller balances reflect disputes, reserves, currency/provider charges and failed transfers as configured. B22-A4 delisting preserves existing buyer recovery where rights and safety conditions allow it. B22-A5 support can resolve payment/delivery/refund failures from an auditable record. B22-A6 a complete sandbox sale, delivery, refund and payout report passes before any authorised real-money launch.

Verification: currency rounding/property tests, provider reconciliation fixtures and the actual end-to-end sandbox lifecycle with operator sign-off. Rollback: stop new transactions while continuing existing buyer/seller obligations; never delete the commerce record to clear an error.

**11. Apply one definition of done to every bundle.**

| Gate | Required evidence |
| --- | --- |
| Scope | The change set names its bundle, requirements, dependencies and exclusions. Any scope adjustment has a written reason. |
| Code | Functional implementation in the architecture established by B00, meaningful failure states and no test-only behaviour enabled in production. |
| Contract | Shared schema/API changes have compatibility fixtures; consumers are updated together or a supported version transition is documented. |
| Validation | Required acceptance items are mapped to actual tests or observations. Preserve failures and missing runtime evidence explicitly. |
| Data | Initial schema creation works from empty storage. Any later migration is rehearsed against representative existing records; private/public separation, retention and recovery are checked where affected. |
| UX | User-visible work is exercised with keyboard and required responsive states. Preserve evidence for flows whose appearance or behaviour changed. |
| Operations | New jobs/secrets/bindings have an owner, documented setup and a failure/disable path. Never assume a fixture is a production integration. |
| Handoff | Update bundle status, exact revision, checks, risks, rollback and the next ready bundle. |

B00 establishes `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cmake --preset dev`, `cmake --build --preset dev` and `ctest --preset dev`. B01 adds and documents the catalogue-validator invocation. Use focused checks during implementation and the required gates once for the completed change. Do not claim a future command exists before adding it.

For runtime-dependent work, maintain three evidence columns: deterministic fixtures, integration/sandbox, and real Omarchy. Mark an item complete only at the level required by its bundle. An unavailable desktop or payment provider leaves the relevant gate outstanding; it does not justify skipping the implementation or fabricating a successful run.

**12. Use these cross-bundle scenarios as release acceptance.**

| Scenario | Expected result | Owning bundles |
| --- | --- | --- |
| Empty-project catalogue | Empty public content renders usefully; development fixtures and dated-only records cannot acquire public verification | B01, B03 |
| Free supported application | Discover, inspect source/limits, plan, authorise, install and launch without a store account | B03, B12–B15 |
| Proprietary external app | Publisher identity and licence limits are clear; purchase occurs at the named seller | B04–B08 |
| Unclaimed community nomination | Informational listing never implies maker participation or grants seller control | B04, B10 |
| Author submits and revises | Draft survives restart; findings refer to a specific revision; new content requires new checks | B05–B07 |
| Forged or stale approval | Catalogue publication fails even when a label/receipt/workflow is modified in the candidate PR | B07–B08 |
| Approved but not delivered | Author sees publication pending; users continue seeing the previous delivered catalogue | B08 |
| Upstream moves a release | Affected route is suspended/reviewed; prior test evidence is preserved and accurately labelled | B09 |
| Website-origin command injection | Native handoff resolves only trusted IDs and never executes supplied text | B12–B14 |
| Offline application | Cached library works; new managed install waits for current eligibility | B12–B13 |
| Interrupted package operation | Journal reconciles against real package state; future work can resume without duplicate actions | B14–B15 |
| Remove a selected setup | Shared/pre-existing apps and personal files remain; selected supported removals are explicit | B15 |
| Later manual setting edit | Restore shows a conflict instead of overwriting the edit | B17–B18 |
| Share a remix | Only selected supported fields leave the machine, with parent attribution | B19 |
| Paid order with lost callback | Provider reconciliation delivers once and preserves receipt/recovery | B21 |
| Partial refund and payout | Amounts reconcile with the frozen order and published fee basis | B22 |

**13. Preserve these scope and business boundaries.**

External author sales are available as links from R1. Their records describe the seller, model, price basis, entitlement, activation/online requirements and support/refund destination. Do not build a fake unified basket for purchases that occur at different sellers. A setup with paid components displays separate acquisition steps and does not claim purchase completion from returning to the page.

The native application manages discovery and supported lifecycle operations. Applications retain their upstream distribution; the store does not become a substitute system package manager. Adding an AUR builder, private binary host, universal Linux sandbox, background root daemon or automatic plugin updater requires a new scoped bundle and evidence. None is silently included in R2.

The initial user library is local. Account sync, broad social reviews, ranking telemetry, bounties, multi-author billing and collaborative desktop administration remain future product decisions. RSS, maker profiles, structured problem reports and the pilot feedback process satisfy the initial return/feedback loop.

Funding is a project operating decision. A proposed 5% commission on £10,000 of monthly eligible sales produces £500 before costs. Do not claim the store is commercially sustainable without observing hosting, testing, review, support and provider costs. Editorial selection and approval remain independent of sponsorship.

**14. Start the next coding-agent session with this instruction.**

```text
Work in https://github.com/tcballard/OmaStore. Read AGENTS.md, the current
native-first PRODUCT_SPEC.md, BUILD_HANDOFF.md, BUNDLE_STATUS.md and ADRs.

OmaStore itself is a native Omarchy desktop application: Qt/QML interface and
Rust core. Preserve the native scaffold and inspect subsequent work. Do not
replace it with a website, Electron app or webview. Shared services support
the native application; no website is needed to open or use the scaffold.

Consult BUNDLE_STATUS.md for current implementation progress. After the B01–B03 native discovery work, the next server bundle is B04; complete the separate real-Omarchy R1 exercise. B03b local author preparation does not replace B04–B07.
Keep each bundle reviewable. Establish catalogue types and real validation,
then the native catalogue client/cache and discovery views. Author services,
installation and payments follow their bundle dependencies and release gates.

Make routine engineering decisions, record material ones, and bring Tom
concrete options for visual and interaction taste. Complete useful work when
external evidence is unavailable, recording the exact outstanding gate.
Update docs/BUILD_HANDOFF.md and docs/BUNDLE_STATUS.md with actual checks and
results. Distinguish offscreen Linux from real Omarchy testing. Never invent
app listings, maker participation, compatibility tests or successful installs.
```

**15. Keep the DHH pitch as context, not an implementation requirement.**

The product argument is that people should be able to discover a useful app, see it working, adopt part of a real setup and support its maker. The unanswered relationship question is how an independent app storefront should fit alongside the current plugin marketplace. Official endorsement or integration is not assumed.

Draft message, retained from the storefront plan:

> David, I've been sketching OmaStore: a native Omarchy application for discovering useful software, seeing it work, and supporting its makers.
>
> The bit I'm most excited about is **Use this setup**.
>
> Someone shares their writing desk or development setup. You can see the apps, plugins and shortcuts behind it, choose the parts you want, and bring them onto your own machine. A great desktop screenshot becomes something you can actually use.
>
> I'd start with 20–30 apps, three setups, proper demos and compatibility results tied to a tested version. Submissions would use a public catalogue and a small review process, with existing plugin listings linked back to the current marketplace.
>
> Authors could give software away, accept support, sell an app outright or charge for a hosted service. Listing would be free, and they could keep their own checkout. OmaStore would be open source and run in its own native desktop window.
>
> Working on OmaChat, OmaSheets and the plugins is what got me thinking about this. I'd like people to find useful software quickly, and give its makers a reason to keep improving it.
>
> The first prototype would prove native discovery and app installation. Selective settings would follow once handling existing customisations is solid.
>
> Would you see this fitting best alongside the plugin marketplace, or growing out of it?
