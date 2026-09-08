# ADR 0006: exact published context in editorial and setup revisions

Status: accepted for B10/B11.

Stories and recipes need existing app and maker facts for schema validation and an intelligible exact preview. Treating those objects as fresh author submissions would allow a curator to rewrite someone else's evidence, media or ownership.

The private authority resolves referenced IDs against its last publicly observed delivered catalogue during `prepare_draft`. It rejects modified supplied context, adds missing referenced objects, projects the author's current scoped control where applicable, bounds the expanded result to 80 KiB and saves a new private draft version. The user confirms those exact bytes. Submit recomputes normalization and rejects a changed preview; immutable revision context records the app/maker IDs and source snapshot.

Review and publication compare referenced facts with the current delivered catalogue. A change requires a new preview/revision. Unchanged context keeps its original evidence and media, is excluded from new media publication and release announcements, and never transfers ownership. Existing ownership and scoped claim history remain reviewer conflicts. Editor roles are checked at the decision and again before publication.

A development-only seed supplies fictional read-only context to the local playground. Fictional author submissions use separate stable IDs/slugs. This seed cannot be used with a production workflow database or the real GitHub adapter.

The tradeoff is deliberate rebasing when a referenced listing changes and a practical limit on large recipes. The native transport remains 256 KiB and carries no executable command text.
