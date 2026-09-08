# ADR 0003: Prepare a submission before author services exist

Accepted bounded extension B03b, 8 September 2026. Depends on B01 and the B03 native window. This adds a useful local author workflow while preserving B04–B07's identity, private-service, media and review gates.

A native form collects purpose, maker, exact release identity, account/offline/service requirements and an external offer. It supports every planned launch offer model and exact decimal-to-minor-unit conversion. The core's `candidate.prepare` command passes the result through the shared catalogue validator. Its output always uses the development channel, an unclaimed maker and no test evidence. The public loader rejects it. Structural validity is not readiness for publication.

The editable worksheet is local data, not an authoritative server draft. Save uses QSaveFile with owner-only permissions, a QLockFile and comparison against the bytes originally loaded. A stale editor cannot overwrite another window. An invalid saved file is preserved. Unsaved-close choices are explicit. A checked candidate can be reviewed and exported through the native file dialog; edits invalidate that checked result. No upload, OAuth, publisher claim, public intake or payment occurs.

The first form prepares one external route and one offer. It does not yet author media, capabilities, privileged services, release notes, multiple architectures/offers or submission evidence. These remain disclosed readiness work. B05 must eventually import the candidate into a real authenticated private draft, revalidate it and add the missing fields. It must not treat an exported file or its digest as approval.

This is a separate B03b change set. It does not mark B04 or B05 complete and does not change their dependency/acceptance contracts. Next server work remains B04, including a provider-supported browser flow, secret-service token storage, scoped claims and an actual callback using operator-owned configuration.
