# Release monitoring and distribution status

B09 monitors delivered catalogue entries. It does not run a submitted program, package build or installation hook. New upstream versions become separate candidates with unknown compatibility; they do not replace a reviewed release or copy its runtime result.

The service reconciles the delivered catalogue once per worker minute, leases one due application and bounds the observation to 45 seconds. A successful or failed attempt schedules its next poll six hours later. A changed catalogue identity invalidates the old lease and its fresh-observation state immediately. An unavailable provider retains the previous observation and reports degradation rather than a new success. Evidence crossing 90 elapsed days records one retest-due event without erasing the test.

Supported readers are public GitHub repository/commit/release metadata, official Arch `core`, `extra` and `multilib` x86_64 package metadata, and an exact GitHub release-asset URL with the provider's SHA-256 digest. A general publisher website is not treated as artifact bytes. Other sources report an unavailable adapter. Arch metadata does not establish the SHA-256 of executed package bytes. The actual Omarchy/package-profile and byte checks belong to the native planner and execution gates.

GitHub responses bind repository ID and owner ID. A changed observed owner or changed binary digest creates a distribution hold. A reused catalogue release ID with a different identity, or a changed project origin, also requires resolution. Ordinary new versions remain update candidates. No listing, installed program or personal document is deleted by monitoring.

`OMASTORE_MONITORING_PAUSED=1` disables polling. Signed-in operators can still review reports, suspend distribution and resolve incidents. Provider rate limits and outages appear as unavailable observations; they cannot keep a source current indefinitely. The current public readers use anonymous provider requests. A larger catalogue needs measured request budgeting or a separately scoped operator-owned read adapter; no connector credential is reused. Real upstream availability and rate-limit behaviour remain release evidence to collect on the configured service.

## Public status contract

`GET /api/v1/status?ids=app-one,app-two` accepts at most 100 validated IDs. Unknown IDs are omitted. It returns the exact catalogue revision/snapshot, `generatedAt` and `validUntil` as Unix seconds, a five-minute response validity period, and separate last-attempt and last-successful-observation times. `sourceCurrent` requires matching current material, a successful observation within six hours and no subsequent failed attempt.

Each item includes `distribution`, public `reasonCodes`, `sourceAvailable`, `sourceCurrent`, `evidence`, `distributionEligible` and `runtimeEvidencePassing`. These facts do not authorise installation on an arbitrary host. The planner must independently match its Omarchy environment and release bytes, supported repository, local state and unexpired status snapshot before requesting consent. A fresh service heartbeat never refreshes old upstream evidence.

Reports, reporter identity, appeal text, operator reasoning and raw upstream observations are excluded from this endpoint. Native app details check response time and catalogue snapshot; an expired/mismatched overlay disables the acquisition action. Source and support remain available for inspection. Refreshing status does not upload a local package inventory.

## Reports, suspension and appeal

A signed-in visitor can send a private report. Public reports do not automatically suspend an app. A recorded listing steward may appeal. Operators inspect the Operations workspace and make version-checked, idempotent decisions with private reasons. Every suspension and resolution has an audit trail. Restoring distribution requires a fresh successful observation and an explicit confirmation that the incident and relevant evidence were reviewed; an unresolved artifact mismatch cannot be overridden by that confirmation. Restoration does not turn old or failing runtime evidence into a pass.

The native preview has a separate “Simulate upstream observations” action for fictional local data. These records are labelled simulation and never become production status. Its operator flow can suspend and restore a sample route without modifying a machine package.

Sources: [GitHub repository metadata](https://docs.github.com/en/rest/repos/repos#get-a-repository), [latest releases](https://docs.github.com/en/rest/releases/releases#get-the-latest-release), [release asset digests](https://docs.github.com/en/rest/releases/assets#get-a-release-asset), and the [official Arch package metadata endpoint](https://archlinux.org/packages/core/x86_64/pacman/json/). The live Arch endpoint could not be verified in this build session; fixture tests and an implemented adapter are not that integration evidence.
