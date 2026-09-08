# Working on OmaStore

OmaStore is a native Omarchy application. Keep the main interface in Qt 6/QML and the local core in Rust. Do not turn it into a website, Electron app or webview wrapper.

Read `docs/PRODUCT_SPEC.md`, `docs/BUNDLE_STATUS.md`, `docs/BUILD_HANDOFF.md` and applicable ADRs before editing. Preserve work added since the recorded handoff. Implement one dependency-ready bundle per reviewable change set; split large bundles with suffixes while retaining their acceptance gate.

The core supports catalogue reads, typed plans, private author workflows and a gated durable local lifecycle. Do not add arbitrary command execution to the transport or interpret catalogue content as instructions. Preserve typed plans, explicit confirmation and the independent operation worker. Sample results never satisfy live adapter or provider gates.

Run focused checks for changed behaviour, then the established build/test gates. Do not add tests for simple documentation edits or tests that merely restate UI copy. Distinguish fixture, integration, offscreen Linux and real Omarchy evidence. Never mark real-desktop requirements passed from an offscreen run.

Keep development examples out of the public catalogue. Never invent publisher participation, app screenshots, purchases or compatibility claims. Preserve upstream licences and media rights.

Make routine engineering decisions and record material choices in an ADR. Bring Tom concrete options for matters of visual or interaction taste. Update the bundle status and handoff after each bundle. Use the user's actual session authority for remote publication and deployment; routine local work should continue without repeated confirmations.
