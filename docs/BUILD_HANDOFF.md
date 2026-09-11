# Build handoff

11 September 2026, continuation recovery:
The active implementation is `design/storefront-polish`, PR #5, starting at
`641ab7a3c1ae1e27517fd7f1147eba1999b409d3`. Tom asked to restore the approved
design after a continuation incorrectly started from the B00 main branch.
Duplicate PRs #6–#8 are closed as superseded; their branches retain the work.
Continue from this branch and its existing PR #1–#4 dependencies. It already
contains catalogue search, bounded offline caching, six real repository
listings, the approved square-edged shelf and live Omarchy appearance.
Do not restart B01–B03 from main or replace this interface with the scaffold.
Native CI run 34522637713 and Arch package run 34522637670 both passed at the
starting revision. Real Omarchy desktop acceptance remains separate.
The restored source rebuilt locally with Qt 6.4.2 and the locked release Rust
workspace. Desktop-settings passed against Qt 6.4.2 (10.53 s). The remaining
native suite passed with the complete PySide6 Qt runtime (10 passes, one
D-Bus skip, 8.74 s), including search/shelf/save/blocked-plan, small/HiDPI and
light/dark theme journeys. The minimal local 6.4 runtime lacks QtQuick.Dialogs;
the complete runtime was used for UI checks and the fresh capture. Its QtTest
private ABI differs, so desktop-settings used its matching build runtime.
`screenshots/restored-storefront.png` is the fresh 1488×1056 native capture,
visually compared with the approved authored shelf. No app code was rewritten:
this recovery restores the existing implementation as the continuation base.


10 September 2026, B03 appearance integration, starting at `c94208131c1a9681063ac29203b150061bdb0d56`:
Tom authorised Follow Omarchy as the default, with the approved OmaStore
editorial style available through More → Appearance. The native settings reader
now follows Omarchy v4.0.3's active colors.toml and resolved Fontconfig monospace
family, debounces/rearms filesystem watches, recovers through a bounded polling
interval and persists only its own appearance preference. Theme data is inert;
desktop files, installation gates and payment capabilities are unchanged.
Category glyphs inherit the accent; screenshots retain their upstream appearance.
ADR 0013 records sources, fallback behaviour and the legacy-format boundary.
Arch preview revision 5 explicitly depends on Fontconfig. New isolated tests
cover live colour/font changes, atomic directory replacement, late creation,
missing/invalid/oversized files, contrast fallback and preference persistence.
Final ordinary suite: 11 passes and one local D-Bus skip in 11.98 seconds;
sample suite: 10 passes and one local D-Bus skip in 46.07 seconds. Ordinary
checks include full native journeys with both light and dark fixtures at
800×600; existing 200% and sample-workflow checks also pass. Actual 1488×1056
captures are `docs/screenshots/follow-omarchy-dark.png` and
`follow-omarchy-light.png`. Bash recipe syntax and `git diff --check` pass.
These are offscreen Linux checks, not real Omarchy/Wayland/theme/portal
acceptance, which remains open. Remote CI/package results remain pending.
Capture follow-up: inspection caught an offscreen capture occurring during the
font-triggered redraw, before the screenshot texture reached the frame. The QA
capture path now allows the layout/render to settle; both final captures were
regenerated and inspected. This follow-up changes QA timing only.

10 September 2026, approved authored shelf: Tom selected the final square-edged
dark editorial concept and authorised shipping it. ADR 0012 supersedes ADR 0011.
Native shell, bundled typefaces, app showcase, persistent keyboard shelf,
responsive search, secondary navigation and information disclosure are built.
Local `ac21ff4` is published as `5838ac788d616629749433e0d3f067e9e6c95c65`;
local `e7bc2c2` as `962cde14e914e3fb811bdd4eb468e9ed4b422ad7`, matching tree
`02445c7d41c66cafc514cdf026c9600fe1a809e0`. The following evidence commit also
scales display typography smoothly at intermediate widths.

Ordinary native suite: eight passes and one local D-Bus skip. Sample suite:
nine passes and one D-Bus skip, 35.04 seconds; final typography-only correction
was followed by ordinary tests and fresh captures. Formatting, deterministic
catalogue reproduction and diff checks pass. `design-qa.md` records visual
iterations and limits. All primary actions still use existing typed contracts.
The Arch recipe is revision 4 and includes Qt SVG for bundled licensed icons.
Remote native and Arch workflows started for the published implementation;
their final status must be checked on the PR. No release or merge is claimed.
Real Omarchy testing, live lifecycle and real payments remain separate gates.

Storefront polish verification: implementation `b03cc345dff97d023f86ea6c858adaaaf971d857` is published as `1a00cdf8f8686536318c4d96e3ba8966e55d523c`, matching tree `e7a254b41bfdffadad93e736d50961570cd341d0`, in [draft PR 5](https://github.com/tcballard/OmaStore/pull/5), stacked on PR 4. Ordinary CTest: eight passes, D-Bus skipped, 5.59 seconds. Sample CTest: nine passes, D-Bus skipped, 36.89 seconds. The new real-catalogue keyboard journey verifies save, blocked installation, return with scroll/focus restoration and category navigation at normal/small/200% sizes. Existing sample author, settings and commerce interactions still pass. Formatting, catalogue reproduction and diff checks pass. Actual light/dark/detail/blocked-plan windows were captured and inspected; `docs/screenshots/README.md` records provenance, commands, contrast and limits. Native launch initially caught reserved QML token names, fixed before these receipts. The blocked sheet was simplified after visual inspection; actionable plans still expose privileges. Package revision 3 identifies the polish build; its remote Arch workflow is pending at this receipt. Real Omarchy acceptance remains separate. Undo through the three focused design/fix commits; user state and live gates were not changed.

Storefront polish continuation starts from `b0e3b28f0afae8f12f48a46bacb4d12dfefd315e` on `design/storefront-polish`. Tom authorised the Discover and detail/install journey visual pass. ADR 0011 defines the direction, purpose navigation, attributed upstream screenshot and unchanged live gates. Native rendering and interaction verification are in progress.

Arch package verification: [run 34398115220](https://github.com/tcballard/OmaStore/actions/runs/34398115220) passed at `94214915628ba532dc0d7262c442534785c1dbb7`, including the unprivileged package build, archive-content inspection and artifact upload. The `omastore-preview-arch-x86_64` artifact contains the installable preview, checksum and source revision. Disabling makepkg LTO resolved the observed SQLite linker failure. No package was installed on an actual Omarchy host.

Repository preview verification: the full native and isolated-media [CI run 34397410622](https://github.com/tcballard/OmaStore/actions/runs/34397410622) passed at published implementation `dd6313ac08716d2a6ddd3b45b3701f9cb71e511f`. This includes ordinary and sample builds, both native suites, keyboard/size exercises and desktop staging. After the packaging-only correction at published `94214915628ba532dc0d7262c442534785c1dbb7` (local `9dac46bb8c04cff537bcc5e62290905d4a779981`, matching tree `235d1168151565745fbb8ce22b51493f8697280d`), the local Qt 6.8.3 ordinary build also passed five CTest checks; the D-Bus instance check was skipped locally. An actual offscreen ordinary-window capture was inspected and displays the six real repository entries. The 90 Rust tests, Clippy and deterministic catalogue checks pass. Actual Omarchy desktop and live lifecycle acceptance remain unverified.

Arch packaging follow-up: run `34397410810` failed linking bundled SQLite symbols with Rust’s lld. The package now opts out of makepkg C/C++ LTO (`!lto`), and `!debug` keeps the intended single desktop archive instead of adding a split debug package. The replacement Arch run must confirm the linker diagnosis. This is a packaging-only correction; application code and the passing 90-test receipt are unchanged.

Packaging continuation from `eb2a8bd`: `packaging/PKGBUILD` revision 2 identifies this repository preview. A separate Arch CI workflow prepares dependencies, builds with an unprivileged account, checks archive contents and uploads the package with source commit and checksum. It does not publish a release or mutate Omarchy repositories. Local checks: PKGBUILD Bash syntax, package-checker Python syntax and workflow YAML parsing pass; all-feature Clippy passes with warnings denied. An actual Arch package build and real Omarchy acceptance remain outstanding. Native SDK verification is in progress.

Repository-preview continuation from `e2ad90e`: Tom authorised using existing Omarchy packages. Six stable x86_64 community entries now populate ordinary browsing, with exact observed package identities, unclaimed upstream attribution and no invented media/tests. Source-index archives, selected observations and a deterministic generator are checked in; `--check` verifies selected metadata and catalogue reproduction. ADR 0010 records the bootstrap decision and `unknown` maturity vocabulary. Historical empty-catalogue statements below describe earlier builds. Validation: all 90 workspace Rust tests pass (all features; two existing opt-in performance/media tests remain ignored), catalogue reproduction and selected-index-field checks pass. Publication tests exposed the trusted-base requirement; the bootstrap catalogue now lives separately in `data/repository/catalogue.json`, preserving the approved registry and synthetic empty fixture. Native and packaging receipts follow in the next continuation entry. No protected catalogue publication, real installation or payment was enabled.

9 September 2026 continuation, B20/P14 provider decision: Tom selected Stripe as the default payment provider. Starting revision `322d6f7c19f15c25828dc0b72fa040d938485e0f`; this documentation-only commit records the decision in the product specification, commerce guide, ADR 0009 and bundle status. The existing Stripe test adapter already matches this choice. Connect direct charges remains the proposed operating arrangement, not a separately confirmed commercial agreement. Validation: reviewed the documentation diff and `git diff --check`; runtime checks are not applicable because no code or configuration changed. Next commerce work is the real Stripe test-account lifecycle rehearsal with operator configuration. Live payments remain disabled. Undo by reverting this documentation commit.

B22d closes the final distribution-hold edge case: new paid delivery and fresh hosted recovery now respect open app holds, while existing receipts, issued licences, refunds and cancellation remain accessible. Orders project the hold explicitly in the native receipt view. The existing recovery integration test proves that offer withdrawal preserves recovery, an active hold blocks it, and the receipt/refund history remain available. All 88 Rust tests, both Clippy modes and nine native checks pass; the final native run completed in 37.11 seconds, with only the local D-Bus check skipped. No live capability was enabled. The PR records the latest published tree and final remote CI receipt.

B22c local `36080f1c9a57d6262db1d39cf82d6390bc112214` is published as `5153560503ae5da8146e7baa455236e7fb6be49d`, matching tree `3c1a310319485ce9b28b0b2bb303e5d7fdb7c7e3`. The full [native and isolated-media CI run passed](https://github.com/tcballard/OmaStore/actions/runs/34304788817).

B22c completes the native refund, subscription/cancellation, author finance, operator support/reserve/evidence and private packet-export flows. The exact retained refund intent survives lost client replies. All 88 Rust tests pass; both final Clippy modes are clean. Nine native checks pass in 36.72 seconds, including actual keyboard partial refund/approval, GBP 0.08 fee reversal, provider-cost reconciliation, failed payout reporting, dispute preview and actual private packet export at four sizes/scales. Testing caught a provider dispute ID incorrectly passed through the internal order-ID validator; the corrected provider-ID check passed the entire suite. Local D-Bus remains skipped. Final dependency audit and source credential-format scan report zero findings for the unchanged recorded lockfile.

All implementable B00–B22 software is now present. PLAYGROUND.md is the concrete native walkthrough. DHH_NOTE.md is a standalone, unsent message grounded in this implementation. README, API, commerce responsibilities and release/readiness documentation now reflect the full build. Actual Omarchy desktop/lifecycle, publisher/pilot and provider/operator evidence remain external gates; no live package/settings adapter or real-money switch was enabled.

B22b local `57f60111c48ee462dfa3b5506e3a2d68e2ca5ad0` is published as `45530478dddc57b788695d2dac100c0ac4cc9157`, matching tree `669020d337fba2b3dad18fbd342dc794152ca3ea`. Its full [CI run passed](https://github.com/tcballard/OmaStore/actions/runs/34304233375).

B22b completes immutable paid renewal cycles, current provider billing observations, durable future-billing cancellation, exact invoice/subscription identity checks, verified financial webhooks, periodic lifecycle recovery, native-service action contracts and support notes. Refunds preserve issued records while preventing new delivery/access through a fully refunded receipt; future periods wait, and elapsed periods require support. Provider reporting continues for deactivated sellers without impersonating a user. The test-commerce service cannot configure public catalogue publication/monitoring/check workers. All 88 Rust tests and both Clippy configurations pass, including the HTTP refund lifecycle and signed financial-event replay. Native buyer/operator finance screens are being verified in B22c.

B22a local `5020327c59542ff60509bce4d42140e49580c5a3` is published as `2b8326b89396ec8574349e20066427320eaab1bc`, matching tree `f2a60d0ab44fdd75d4b1c3164563be4be2d1913f`. Its full [CI run passed](https://github.com/tcballard/OmaStore/actions/runs/34302814706).

B22a implements durable refund requests/approval/reconciliation, cumulative exact platform-fee reversal, append-only proceeds/reserve records, actual provider fee observations, dispute movements/evidence, and account-level payout/failure reports. Repeated or ambiguous effects retain their original keys; expired provider retention never authorises a blind retry. Outside-store refund mismatches and unobserved processing costs remain explicit. Seller owner/account identities are frozen. All 85 Rust tests pass; both Clippy configurations pass. Native controls, recurring cycles and cancellation follow in B22b. The real provider sandbox lifecycle remains an external gate.

B21b local `32a4fe9377b39cbde1c67a5acb4bdb251fdf6c14` is published as `b4006849b9c7520e2a012fb7713252d58d711005`, matching tree `aa4856ed7ac9bd668fe91fecece9d6b3818966b1`. Its full [CI run passed](https://github.com/tcballard/OmaStore/actions/runs/34300696480).

B21b connects the service routes/workers, provider-bound development databases, operator-configured hosted fulfilment, exact service receipts, native purchase consent, persisted local request keys, recovery with a fresh login, licence export/offline verification, and native managed-offer/seller forms. All 81 Rust tests and both Clippy modes pass. Nine native checks pass in 34.81 seconds, covering keyboard purchase, paid-but-failed delivery, retry, actual private export and pinned-key verification at four sizes/scales. Resource aliases and the subsequent worksheet navigation were corrected from the native tests. Local D-Bus remains skipped. The updated lockfile again has zero audit findings. Provider and real Omarchy evidence remain external; no live key or charge is accepted. Next: B22 lifecycle/accounting.

B21a local `06eff965eea14ae9c961c1fe490ddae56edac1a1` is published as `4eab483760db312fac7800c6a94ce5b487a14289`, matching tree `ec4cf828e8807e70858a45d7c8f17a57f8107c4d`.

B21a adds immutable commercial prices/orders, current seller and listing authority checks, retained buyer request keys, durable checkout claims, exact provider amount/fee/account checks, verified raw webhooks, paid/delivery state separation, support history and pinned-key Ed25519 perpetual licences. A fixed-origin Stripe test adapter and persistent fictional provider implement creation/reconciliation; no live key or production database is accepted. All 79 Rust tests and both Clippy modes pass, including lost replies/restart, delayed/forged/duplicate events, price and buyer tampering, expired provider idempotency, failed/retried delivery and offline recovery. The updated lockfile has zero audit findings. Native/service wiring and hosted fulfilment follow in B21b.

B20 local `f64df8f3b3b1dec1a2cd4231e2867dbf28178853` is published as `f7ca9babe6897c7a2a9f9efbfafbb84c0612fb32`, matching tree `2d0d54fd40c9b635f0180b3de519c585ad33f8a7`.

B20 adds a closed commercial release gate, bounded operating facts, immutable versioned operator records, purchase pause, integer price/fee contracts, disabled provider interfaces, and a native Commerce view. All 72 Rust tests and both Clippy modes pass. Nine native checks pass in 30.87 seconds after fixing test response ordering; the local session bus skips. Missing operator/author/provider/legal facts remain explicit and do not stop disabled B21–B22 software under Tom’s full-scope instruction. No live key or charge is accepted.

B19 local `354e7a051cec24d130427b18dab459cbe85a22fe` is published as `78cb8508169dbf71d713226d35b18555a549ac69`, matching tree `2b8bfdeaf60f9124ffbf12ab53fac4c908232f5a`. CI run 34296407290 is in progress.

B19 implements strict attributed portable remixes, bounded local saves and optimistic rename/import conflicts, exact preview/export, current-machine differences, typed multi-app plans, and native remix/author-draft flows. Parent attribution cannot be erased from a completed remix draft; preparation rechecks published parent/media rights. All 70 Rust tests and both Clippy modes pass. Nine native checks pass in 31.25 seconds, including keyboard remix save, actual JSON export/import, selected installation planning and private author-draft creation. The real service fixture now takes an attributed remix through independent review and publication and rejects stripped attribution. Real Omarchy portal/author evidence remains external. Next: B20–B22 commerce.

The execution environment replaced the former scratch mount. A fresh checkout at `/tmp/omastore-worktree` recovered B18 from GitHub exactly: remote/local head `d344dfdad337e8c0b3b8a0a50c3ca6560b744ab1`, tree `92206f17b20a1a9b2d1fd285d95f5ba5a33a8790` (original local B18 `602a19fd21c6c99dafa99a67515c983f32ae7e9c`). Uncommitted B19 work was reconstructed and its gates rerun. Build tools and logs survived. Development cache now uses `/tmp/omastore-target`; native release cache remains `/tmp/omastore-release-demo`. B16f CI completed successfully for both native and isolated-media jobs.

B18 adds private per-setting journals and events, explicit apply and restore consent, atomic directory-relative replacement with permission preservation, stale-preview checks, interrupted outcome reconciliation, and native history/restoration previews. All 67 Rust tests and both Clippy configurations pass. Nine native checks pass, including actual keyboard apply/restore at four sizes/scales; local D-Bus remains skipped. Native testing caught and fixed a JSON property-order comparison that hid valid proposals, and checks now require the consent view to be visible. B18-A1–A6 are covered by deterministic file/fault fixtures; actual Omarchy reload/ACL/customised-desktop evidence remains an external gate. No live setting adapter is enabled. Next: B19 local attributed remixes, then B20–B22.

The isolated-video integration now passes on B16f, [run 34292283886, job 102281219317](https://github.com/tcballard/OmaStore/actions/runs/34292283886/job/102281219317). It proves actual Bubblewrap/FFmpeg startup, private-file/environment isolation and metadata stripping. Remote `aa064aa69d71cb001a3ba363c70d95f291400e1f` matches local `592452f`, tree `425826db9957d29e089d85f5dda6585bbb9b3946`. Native CI for that revision is still running.

B16f resolves the diagnosed Debian loader failure: the isolated FFprobe could not resolve `libblas.so.3`, whose system-package symlink passes through `/etc/alternatives`. The fixed alternatives directory is now mounted read-only when present, alongside the loader cache; arbitrary `/etc` remains unavailable. B16e CI 34291762913 supplied this exact diagnostic. Pending the positive integration result. B17 remote `b60c7c5527a68e2728b644c18e345118172bc055` matches local `4eb07ad`, tree `45aaff00639b20de0a3493f9ad82a6473c818d87`. B18 remains a separate uncommitted changeset.

B17 implements the versioned settings registry, read-only fingerprints and before/desired/scope/restoration proposals, private plan storage, and native selection/diff views from Library and Setups. Requirements B17-A1–A4 pass in fixtures; A5 remains a live-adapter gate. All 64 Rust tests and both Clippy configurations pass. Nine native checks pass in 28.56 seconds, including actual keyboard settings preview at four sizes/scales; the local bus test skips. No host desktop setting was read or changed in sample mode. Initial live-version allowlists remain empty. Next: B18 apply/recovery/restore, B19 remixes, B20–B22 commerce.

B16e remote `517ddd63fb2c425e30b50aa5ee5d29d891cd0c10` matches local `9a2738e`, tree `da62b2549ef112face94d5d348b4543e3bd954d4`. The separate isolated-media CI result remains pending.

B16e makes the system dynamic-loader cache available read-only to the isolated decoder and adds a bounded fixed-FFprobe startup diagnostic. The supported-host B16d run passed namespace/environment/private-file isolation, then failed during video normalization. B16d remote `601db8506fa14a07fa04cdddd7194edb2ffb3558` matches local `9692ad4`, tree `9fc3ad1715f8bd1c52fa88cef15de7603bc0d2b7`. Actual video integration remains pending until its positive job passes.

B16d moves the mandatory isolated-video integration to a supported Ubuntu 22.04 worker host while retaining native Ubuntu 24.04 coverage. B16c CI 34290845779 diagnosed namespace loopback setup as the failure before any decoder ran. No host AppArmor/sysctl policy is weakened and no test is skipped. The replacement job must pass before media isolation is accepted. B16c remote `40306e78fc3c9a96378dfb617310ab7ca82212ae` matches local `9383c28c18f46fbe21349ea91134427ef560d814`, tree `c255f3d1f945978724447472c208b235003565c5`.

B16c removes the advisory-bearing RustCrypto RSA dependency by selecting jsonwebtoken's supported AWS-LC backend. The actual GitHub App RS256 signing/verification/tamper test passes with a generated temporary key. cargo-audit 0.22.2 reports zero vulnerabilities/warnings across 249 dependencies against advisory database `bf25f6575a93a35f30796c65c0ed91bee7fa19fd`; the source credential-format scan has zero matches. The receipt binds the lockfile digest. B16b CI run 34288947027 failed at sandbox startup before the video test; the positive test now emits a bounded fixed-program startup diagnostic so the environment failure can be resolved without weakening the sandbox.

B16b is published as `ce41725a28a36db35ec7037c0122230c7a1287be`, matching local `dae59d81a4d561b9ce18134f133ed8234d42fd4b` and tree `d44d05aed303b42c5b53a5d83904bed1b19a2ca4`. B17 settings work is in progress separately.

B16b connects native readiness/evidence and review-start controls, hourly maintenance, subscription addresses and isolated video normalization. All 60 ordinary Rust tests, both Clippy modes and nine native checks pass (29.17 seconds); the session-bus check skips locally. The positive Bubblewrap/FFmpeg test is explicitly ignored in the ordinary suite and required separately in CI; this container lacks Bubblewrap. A credential-format scan of tracked/unignored source found zero matches. The dependency advisory tool is compiling separately; record its result in the next receipt. No public release gate was marked passed. B17–B22 implementation continues.

B16a is published as `ca197e26d6bdaad9142f4c4895233441f3195b72`, matching local `bf56f8cbe869becb2681efc0e063efe284ba7e96` and tree `a0ae000811c73062b91e3db692d5301fb5fbca9a`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34287878156).

B16a adds operator queue/readiness projections, current-role evidence intake, actual first-response tracking, consecutive-week expansion pause, draft notice/grace retention, active-selection media quota, and private database/object backup and restoration to new paths. All 60 Rust tests and all-feature Clippy pass. Populated backup restoration, public object recovery, environment isolation, tamper rejection, retention and revoked roles are exercised. Native dashboard, hourly worker and media sandbox hardening follow in B16b.

B15 is published as `fb07b56045f51875be3fc3c4fd9ebf571d58909d`, matching local `eb1fa84ca478392a3cf76786bfef7ed4bd493685` and tree `b3eb52436b8986e6f9564cad1125bf252753da09`.

B15 completes read-only interruption reconciliation, fresh partial-setup replanning, shared-reference-aware removal, safe setup detachment and exact-preview diagnostic export. All 56 Rust tests and both Clippy modes pass. Nine native checks pass in 27.54 seconds; the bus test skips locally. The real subprocess test now covers install, coordinator loss, recovery, explicit removal and replay. Real Omarchy lifecycle evidence remains an external gate; no host package or settings changed. Next: B16 operations and readiness, then B17–B22.

B14 is published as `37f053021c800ca3f12c0c230e73c3658f167809`, matching local `6f4e57228dc316fd13a38e43ba7714eadaff85b7` and tree `01da93a3aaf2d154cd57252eae657bc6d28ae381`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34284929806).

B14 implements exact explicit consent, fresh preflight after system authentication, independent workers, kernel locks/atomic claims, bounded output draining, package/source/version outcome checks, cancellation boundaries and native operation progress. All 52 Rust tests and both Clippy modes pass. Nine native checks pass (27.07 seconds), including a real core-loss/subprocess-survival/replay test and keyboard sample installation at three sizes and 200% scaling. The session-bus test still skips locally. Live package writes remain unavailable: the verified Omarchy release list is empty pending actual VM lifecycle evidence. B15 adds interruption reconciliation, removal, selected retry and diagnostics.

B13 is published as `5396535d0babb79977e1121a591b079eb25a58a3`, matching local `43ddbfdef836b9394e18462a220e0eb3910e4441` and tree `de597806329ac9b5955cfb570be9e663c2e13cd7`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34282128450), including the actual two-process D-Bus test in both ordinary and sample builds. B12 [CI also passed](https://github.com/tcballard/OmaStore/actions/runs/34280853990).

B13 adds a private local SQLite library, persistent exact proposals/events, external installed-version observations, cached reads, package-owned desktop launcher selection, strict identity-only URI handling and a per-user single-window lock/session-bus handoff. All 49 Rust tests and ordinary/development Clippy pass. Eight native checks pass (23.14 seconds); the additional two-process D-Bus test skipped because this environment cannot create the isolated bus. CI must run that test. Real Omarchy focus/launcher/offline-launch evidence remains outstanding. B14–B15 consent, independent execution and recovery follow; no host package mutation occurred.

B12 is published as `e90a4393322745a2e94d93699b6a6a7898ff7930`, matching local `82789a8c35d2728170bb78b25ec9db725f8cc28c` and tree `6bec1d582354017b9681d54eb332d13510727eb5`.

B12 implements bounded read-only Omarchy/Arch probing, signature-policy checks, installed/update queries, dependency resolution, deterministic typed proposals and native plan review from app/setup details. Exact repository package versions are separate from display release versions. Unknown hosts, stale or changed status, signature exceptions, replacements/conflicts and system upgrades remain explicit blockers. All 47 Rust tests, ordinary/development Clippy and 8 native checks pass (22.00 seconds). A transient empty C++ object was rebuilt serially; the resulting full native suite passed. No real Omarchy package probe or package mutation was performed. B13 is next.

B11b is published as `dbe284298e523cd6d21a822fc266c25b3db0b134`, matching local `8ee22d0777663e65f74cf1923bf1439c1d2d69c7` and tree `2374bd67937dbb253629a693e9a8fa7dd58b211f`.

B11b completes native setup browsing, optional component selection/dependency explanations, price grouping, current status, creator/media actions, exact selection preview, native import/export and published-app selection in author drafts. All 45 Rust tests, workspace Clippy and 8 native checks pass (21.24 seconds), including keyboard selection and export/import at three sizes and 200% scaling. Portal dialogs and three real tested recipes still require Omarchy/content evidence. B12 read-only planning is next; no package writes were performed.

The separate Qt 6.4 RSS comparison correction is published as `a62a621bfc5289ffca080dd25e328d00f52cedbc`, matching local `ad1e23c8075de8218758945d5b5f34836a0cd666` and tree `006414ced999984bf93a0803bab87035fe7a2c49`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34277144978), including the ordinary and demo native suites.

## Full-scope continuation

Tom authorised implementing all scoped bundles on 8 September 2026, making engineering decisions autonomously and committing regularly. The restored starting revision is `1bc14e0edd512c21af364a42e5c180193f40ad9d`; B04–B09 are on `build/author-workflows`; the next stack continues on `build/native-marketplace`. The earlier preview receipt below remains historical evidence, not a description of the full implementation's completion.

B04a adds the private SQLite identity/role/session/claim authority, GitHub authorization-code/PKCE adapter, desktop proof binding, single-use callbacks, expiring scoped control challenges, DNS-pinned bounded public reads and a production/development database boundary. Four focused tests pass, including populated-database restart, cross-account/replay rejection and session/role revocation. Formatting and Clippy with all features and warnings denied pass. Native/service wiring follows in B04b. A registered provider and real callback remain live integration evidence; they do not block subsequent implementation.

B04a implementation commit: local `c33c6f4e025d5d336d278ab4b730863aa8fd05cc`, published as `c5d8dab4dde416bd63ffee087544eead616d0ea3`; both have tree `eafd7fa4d2c755f9063461df4092f886c35ef601`.

B04b starts from B04a and adds the Axum HTTP boundary, bounded request handling, authenticated claim actions, operator role CLI and the native author workspace. The Qt bridge never receives tokens; Secret Service storage has an explicit session-only fallback. The separate sample build supports fictional local role sessions in a development-marked SQLite database. QA isolates that database as well as preferences. A reusable Cargo cache path and a single Ring TLS provider keep native/core/service builds consistent.

B04b validation: all 19 Rust tests pass (plus the previously separate performance sample), full-workspace Clippy with all features and warnings denied passes, and all 8 demo CTest checks pass. These include actual HTTP/core parity, the native sample-author sign-in, worksheet keyboard flow, three logical sizes and 200% scaling. A dialog sizing loop introduced by the nested author tabs was found and fixed. The local SDK is Qt 6.8.3; CMake/Cargo build resources were bounded after recovering a damaged generated dependency artifact. Real GitHub callback/keyring/Omarchy evidence is still outstanding. Next implementation: B05.

Rollback: revert the B04a implementation and retain private database files for recovery. No external provider registration, author account, machine package or desktop setting has been changed.

## Marketplace continuation receipts

CI follow-up: B10b's ordinary Qt 6.4 build caught an ambiguous QStringView/ASCII comparison in the QA RSS parser (run 34275044638). The comparison now uses QString/QStringLiteral, preserving compatibility with the declared minimum SDK. Local B10 evidence above remains Qt 6.8.3; the updated CI run must verify the older SDK. B11a is published as `da92184fc35006a595f1998fbb1df2dc15c83012`, matching local `a8b4244bf6e4e375b91336fc5e8127bf41d50a2a` and tree `a2ab6ce0c72e37ef43a89b68b11ae1f56bef30c7`.

B11a adds bounded setup metadata/dependency/conflict validation, selective dependency closure, separated/unknown price estimates, stable identity export/import and exact recipe media/review/publication. A local end-to-end test normalises a setup icon, rejects premature reviewer access, freezes app context, independently approves and observes publication without creating component release-feed entries. Proprietary app submissions now require the publisher-domain challenge; control of a source fork cannot substitute. All 45 Rust tests and workspace Clippy with warnings denied pass. Native setup selection follows in B11b.

B10b is published as `5d41ae738c7a5e2de9e284ea045ca60bb8ea91d7`, matching local `ece33add6c8af1d021c508a6db43089535b56d28` and tree `103119a0ccc4b278c2daf5934bfe4d483fe1c991`. The standalone DHH conversation starter is in DHH_NOTE.md; it is a draft and has not been sent. Its wording distinguishes the running development workflows from the remaining build.

B10b connects native maker discovery, attribution, support links, editorial reading and author scheduling, plus RSS export from observed deliveries. `prepare_draft` saves exact published context before confirmation; review/publication preserve referenced app evidence/media and reject changed facts. Current scoped control is projected by the authority, and existing entity ownership is a reviewer conflict. The development seed cannot impersonate new author submissions; those use separate IDs/slugs. Native rehearsal found and fixed canonical catalogue ordering and a delegate lifetime issue. All 40 Rust tests and Clippy with warnings denied pass. All 8 native CTest checks pass in 20.02 seconds, including maker navigation by keyboard, populated RSS parsing after actual local delivery, conditional HTTP feed reads, the author/reviewer/operator rehearsal, three logical sizes and 200% scaling. These are offscreen and fictional-provider checks; real Omarchy and provider evidence remain separate.

B10a is published as `e8f39bd888eb8cdc6a66f647db37f8756861d0e2`, matching local `82d2afab08393c77a86989327cf3bd996608616b` and tree `0d15fc2ebac46b818f50df2c0ed10e21400faf39`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34272125374). This continuation is [draft PR #3](https://github.com/tcballard/OmaStore/pull/3), stacked on #2. [B09 CI also passed](https://github.com/tcballard/OmaStore/actions/runs/34269507535).

B10a adds bounded maker profiles, scoped claim labels with expiry, scheduled editorial records, independent editor approval checks, and delivered-release RSS. Release title corrections retain their original GUID and first delivery date. No feed entry is created by approval, PR creation or an unavailable public deployment. Existing catalogue and approval bytes remain stable when the new optional fields are absent. All 38 Rust tests and workspace Clippy with warnings denied pass. Native maker/editorial authoring and frozen published context follow in B10b.

B09 is published as `7a55710df30bce03f8e88e19a7681411a8354495`, matching local `9fe06645236976d2928d1b1bda79918640c6bccf` and tree `eefe47cde5e29163785f4b3907abf2dd23255adf`.

## Author continuation receipts

B09 adds six-hour leased observations, immutable update candidates with unknown compatibility, once-only evidence ageing, integrity/ownership holds, private reports and appeals, fresh operator role/version checks and audited restoration. The public status overlay separates its five-minute response validity from upstream freshness, binds the catalogue snapshot and excludes private incident details. Native detail acquisition respects suspension/status availability; the Operations workspace provides the complete fictional suspension/restoration rehearsal. All 36 Rust tests and Clippy with warnings denied pass; all 8 native checks pass in 19.23 seconds. The final metadata-reader change was checked by the Rust gate; live upstream reads remain unverified. Database environment is checked before applying future migrations. No installed package was modified. See MONITORING.md for adapter scope and remaining live evidence.

B08b is published as `41ed1b5fd25d10cee9dddea60aebc1692d7276e1`, matching local `c99510d983d3a2b1cf94b70fc79774e3d6f64c32` and tree `fbce332d923232a60293437c86b2e9d7d817eb0d`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34266967804).

B08b adds the native publication status, exact-file export, issue/PR attachment, retry, base update and local provider rehearsal. The rehearsal uses the real approval/publication/reconciliation functions against an in-memory provider and labels every result as simulated. The native delivery and media origin now use `catalogue-live`; an unavailable branch preserves truthful empty/cached browsing. Authors can withdraw pending submissions before delivery work starts. All 33 Rust tests and workspace Clippy pass. All 8 native checks pass in 19.36 seconds, including author-to-reviewed-to-delivered rehearsal at three sizes and 200% scale. B08a is published as `b83fa5569bc7449a2b375b335901f014428c9a82`, local `b444dcacf748ca4371ef133f7ceb981b7bd77f3d`, matching tree `8f9a35b8625942ad3d46d6d8fb0aaad2539a5f94`.

B08a implements the operator-owned GitHub App adapter, signed webhook intake, exact private approval checks, durable issue/PR checkpoints, source PR attachment and non-forced rebase recovery, isolated live-branch delivery and atomic catalogue replacement. Provider errors are redacted and explicit; no App credentials or live deployment have been created. B08a verification: all 31 Rust tests pass, including forged approval content, role revocation, raw-body HMAC/delivery deduplication, lost-issue reconciliation and atomic replacement preservation; workspace Clippy with warnings denied passes. B08b connects native recovery and a local rehearsal. Configuration and remaining live gates are in PUBLICATION.md.

B07 is published as `9a8dd9c923d8df3edf890a388890ef50d7692669`, matching local `ee0d87270ee43a219e8f22cf8992f5acfaaedb13` and tree `e82109e1204db3ba5e5026eb57348eff5bab8624`. [CI passed](https://github.com/tcballard/OmaStore/actions/runs/34259543054).

B07 verification: all 27 Rust tests and Clippy with warnings denied pass. All 8 native CTest checks pass in 16.89 seconds, including sample submission, checks, role switch, evidence import and independent approval via keyboard at three sizes and 200% scaling. No production approval or public listing was created.

B07 adds the independent reviewer queue and native workspace, field-level revision differences, author-visible decisions, current role checks, exact approval payloads and append-only review history. Sensitive capabilities require two distinct eligible reviewers. Project-control history remains a conflict even after a claim is revoked. Submitted media can be inspected without exposing later private draft files. Core media previews verify bytes and use owned temporary files; demo video playback uses the native system player. Policy tests cover self-approval, missing checks/evidence, two votes, stale concurrent decisions, replay, immutable receipts, private findings and London DST. B06 is published as `edcd37d8287cefbea413349cab8dd38fef3a7e29`, matching local `51fa9bffb7cad4120bb9b9c86c9a1c01575c033e` and tree `5b6d649d0ee52d6c9c7482c6d7668bc04c151305`.



B06 adds a durable SQLite checks queue, expiring leases, bounded source/link/provenance reads, explicit unavailable findings, retry commands and immutable independent VM-report intake. The service dispatcher and pause mechanism are documented in REVIEW_OPERATIONS.md. Sample checks use the same leases and completion transitions in a development-only database. Seven workflow tests pass, including stale-worker rejection and lease recovery; no actual submitted program has been executed as a check. B05b is published as `d6f1aaa9246d58ac532b71ac9ea4c3812518083b`, matching local `0e2ad5a6b86da6b17b2ef93a04490e349bc496c2` and tree `7d4b94867f0e9482313c73894bb2a4fdb210c1bf`.



B05b adds the native full-field draft editor, local recovery scoped to account and service, optimistic server sync, an explicit conflict merge acknowledgement, media selection, exact preview, submission and revision status. HTTP mutation retries use a persisted key; a failed subsequent refresh cannot misreport a committed action. Expired login polling clears correctly. Local edits survive network failure without becoming public. The template contains empty author facts, not demo claims.

B05a commit: `79129ba`. B05b verification: all 22 Rust tests passed, including authenticated HTTP upload/private-access checks; all 8 native CTest checks passed in 15.40 seconds, including creating a draft, keyboard typing and private autosave at three sizes and 200% scale. The recursive native editor uses dynamic QML loading after the runtime rejected static recursion. Media replacement retains historical bytes and counts the current draft selection. The checked UI is offscreen Linux; real Omarchy accessibility/media-dialog evidence remains outstanding.



B04b is committed locally as `d2e341a4ee3d8363f5397a6d2a0173c6d92ae308` and published as `d17855a5ab5968882f8cce41bb486f229cecf8c9`, with identical tree `3750cc37a612fe44047fdc3f5b5844cc027af955`. The author continuation is [draft PR #2](https://github.com/tcballard/OmaStore/pull/2), stacked on PR #1.

B05a adds private versioned drafts, atomic idempotent commands, immutable submitted candidates, revision-scoped findings, and authenticated media upload/read endpoints. Images are decoded and normalised; videos are bounded and metadata-stripped by resource-limited FFmpeg subprocesses. Files live behind an explicit private/public object-storage interface. The default persistent-volume implementation works without an external object provider. Public catalogue writes remain unavailable until independent review and publication. Drafts are bounded to 80 KiB to leave room for recovery data inside the native 256 KiB transport. Six workflow unit tests pass; the expanded HTTP/native acceptance checks are running in B05b. Retention and operational media cleanup remain tracked for B16.

## Earlier native preview receipt

Date: 8 September 2026. Branch: `build/native-discovery`. Scope: B01, B02, B03 and bounded extension B03b, followed by the executed-byte evidence correction. Review: [draft PR #1](https://github.com/tcballard/OmaStore/pull/1).

Verified implementation: [`887d44947beee29ca45c89b7ff5f6888542d3ae6`](https://github.com/tcballard/OmaStore/commit/887d44947beee29ca45c89b7ff5f6888542d3ae6), with Git tree `3ddb5cd7b037d8838d3dd72fab187700109e633f`. The published tree exactly matches the locally checked tree. The following documentation receipt commit does not change implementation files.

## Original scaffold grounding

Started from the actual `tcballard/OmaStore` native scaffold at `53b8957335e0842289abc2966d762fe09f7bcded`. No unrelated repository was used as an application baseline. The authoritative native product spec is version 0.2. [ADR 0002](adr/0002-native-discovery.md) records pinned Omawrite/Omacalc sources inspected for desktop settings, Qt windows, shortcuts and Arch packaging conventions.

The application now supports native discovery/search/filter/detail, independent price/type/licence/evidence labels, explicit external acquisition/source/support links, local saved items and a local author worksheet. The Rust crate is the single catalogue schema/query authority, reused by the core and actual public read service. The HTTPS client validates before replacing its bounded atomic XDG cache. Public catalogue data remains empty. Only the separately compiled demo embeds synthetic listings; no real publisher participation or test result was invented.

The worksheet saves partial work privately on this device, rejects concurrent overwrite, checks a typed candidate, previews the exact export and uses a native file dialog. It always exports an unclaimed development candidate. B03b does not complete B04 identity or B05 server drafts. No public intake, authentication, package writes, installed-app scanning, managed checkout or desktop configuration mutation is implemented.

## Earlier preview validation evidence

Local environment: Linux x86_64, Ubuntu 24.04.3, Qt 6.8.3, GCC 13.3, Rust 1.98.1. Isolated toolchain paths are verification-environment details, not project dependencies.

- [GitHub CI run 34218944477](https://github.com/tcballard/OmaStore/actions/runs/34218944477) passed all native gates for the implementation revision above: Rust checks, public catalogue validation, ordinary and separate demo builds, both CTest suites, desktop staging and an actual preview screenshot. The suites contain 5 ordinary-build checks and 8 preview checks. CI uses Ubuntu 24.04; it is not a real Omarchy desktop exercise.
- Rust catalogue, evidence, query, cache and local-preparation tests pass; 14 tests total, plus one deliberately ignored performance sample run separately. Formatting and Clippy with warnings denied pass. Test records require the SHA-256 of the executed bytes, including source/package releases, and app details expose the recorded environment, date, actor, limitations and evidence link.
- Actual Qt keyboard flow passes at 800×600, 1280×800 and 1920×1080 logical sizes, plus 800×600 at 200% DPI. It exercises search focus/typing, card activation, accessible card name, app detail, saving, restored preferences, back navigation, worksheet typing/saving and field-error feedback.
- Native media tests check byte digest, inert image format, decoded dimensions and disallowed origins. Worksheet tests cover restart recovery, optimistic conflict, invalid saved-file preservation, checked-result invalidation and local-only export.
- Actual Rust HTTP service and core process tests compare query outputs and check conditional GET, invalid filters/methods, snapshot changes and invalid catalogue replacement. These ran successfully here; an earlier socket-restriction assumption was not applicable to this run.
- Ordinary-build tests check that the demo option is rejected and fictional catalogue text is absent from the core executable. CMake staging installs only owned desktop artifacts. The Arch PKGBUILD has syntax validation only until exercised on Arch.
- A separate local Release build with `BUILD_TESTING=OFF` and development data disabled built successfully. Its help omits QA/demo switches and its dynamic dependencies omit Qt Test.

Performance sample: Intel Xeon Platinum 8370C at 2.80 GHz, release Rust build, 1,000 synthetic entries, 100 samples. Shared query p95 10.312 ms; pure HTTP handler p95 10.527 ms. These exclude disk/socket/proxy time and do not establish GUI typing latency with 1,000 entries. The numeric target remains a target until that broader exercise is measured.

## Current boundaries and remaining gates

Real Omarchy acceptance still needs launcher identity, Wayland tiling, live portal/theme/text updates, native file dialogs, screen-reader/keyboard behaviour, actual package/settings lifecycle evidence and visual review. The offscreen receipts above are a separate form of evidence. The theme icon remains provisional.

The private author/reviewer, publication, monitoring, operations and commerce software now exists. Actual operator-owned OAuth/GitHub App/provider registrations, deployed service configuration, genuine listings/recipes, independent staffing, participant observations and commercial responsibilities remain external inputs. Keep connector credentials out of product configuration. EXECUTION.md, SETTINGS.md, COMMERCE.md and RELEASE_READINESS.md record the corresponding closed gates.

Public catalogue and normalized media delivery use the protected `catalogue-live` branch after exact approved publication has been observed. The public catalogue is still empty; ordinary browsing retains its bundled/cached state when delivery is unavailable. The optional service has not been remotely deployed and requires the documented HTTPS proxy/limits. Cached browsing data never authorises installation.

## Continue and recover

The full continuation branch is `build/native-marketplace`, stacked on `build/author-workflows` and `build/native-discovery` in PRs 3, 2 and 1 respectively. The current local checkout is `/tmp/omastore-worktree`. Local and connector-created commit hashes can differ; the recorded Git tree hashes match exactly. Start from the current branch and receipts at the top of this handoff, preserving later user work.

B00–B22 implementation is complete. Read the spec, status and applicable ADRs before additional changes. The next release work is gathering the actual desktop, deployment, author/pilot and provider evidence; the software must not label those gates passed from fixture success. PLAYGROUND.md explains how to exercise the implemented sample flows, and DHH_NOTE.md is prepared but unsent.

Undo code through explicit bundle reversions while preserving durable private records. Use the backup/new-destination restore procedure before service migrations or deployment. Never clear commerce, installation or settings errors by deleting their journals. Do not remove broad XDG directories or buyer documents. Packaging stages under `build/stage` for reversible rehearsal. No host application package, Omarchy setting or real payment was changed by this build session.
