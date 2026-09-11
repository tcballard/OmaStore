# ADR 0002: Read-only catalogue and discovery

Status: accepted engineering direction, 11 September 2026.

B01 uses a shared Rust crate as the schema authority. Public records use explicit
Serde structures with unknown-field rejection, typed routes and release identities.
No private workflow object is serialized into a snapshot. Validation errors contain
field paths and codes, never supplied values. The catalogue is bounded to 1 MiB.
Evidence describes exact executed identities; it never grants approval or execution.

The checked-in public registry begins empty. Synthetic records are a separate
`development` channel and the fixture loader is compiled only with `dev-fixtures`.
Real informational nominations can follow in a separate data change with cited facts,
no implied maker participation, no invented media, no passing compatibility evidence.
Ordinary submission and independent approval requirements still apply to approved listings.

B02 will share query and pagination logic between native and HTTP consumers.
B03 extends the existing Fusion/palette interface with standard native controls;
it is a functional discovery preview, with final art direction left for visual review.
Managed installation is outside these bundles. Catalogue data never becomes a command.
