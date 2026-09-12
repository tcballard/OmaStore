# ADR 0016: Shared storefront session

Accepted 12 September 2026; B13b follows the device-state foundation in PR #10.
Tom authorised automatic device refresh, shared status, persistent operation
progress/recovery and navigation memory before the screenshot-led visual pass.

The C++ bridge owns one device projection for every QML surface. It gathers all
30-item inventory pages into a staging map and publishes only a complete,
snapshot-consistent result. A changed snapshot or unavailable probe clears action
intents and publishes unavailable status; it never silently retains an old Open
button. Core process loss also invalidates the projection. Background requests
have separate routing and do not set the foreground loading flag or overwrite
another page's latest-request reply. No per-card subprocess queries are issued.

Device observations refresh on catalogue delivery, application activation,
explicit refresh and operation/handoff responses, and every 15 seconds. Polling
waits behind foreground requests and permits only one background request at a
time. Package database synchronisation remains the existing repository/updater
contract; observing the device never synchronises pacman databases.

`library.activity` accepts an empty object only. It returns at most five operation
summaries, prioritising running/awaiting/unknown work over recent outcomes, with
at most 100 app identities/actions per operation. Unconfirmed proposals stay in
existing library history. The existing worker inspection detects interrupted
work; the tray never confirms, replans, retries or reconciles automatically.
Full per-component results and recovery controls remain in the existing review.
Activity polls every second while active, otherwise every 15 seconds. Changes
trigger inventory refresh. On disconnection the recorded activity remains visible
but explicitly unavailable. A session tray is a recent view, not the full journal.

App cards (including maker/editorial cards), search, shelf, detail, Installed and
Saved bind to this shared state. Operation state and installed state are separate;
failed/unknown operations offer review, and a proposal never means installed.
The persistent activity entry remains accessible after the review closes and on
restart. The external Omarchy updater reports a handoff, never invented completion.

App detail overlays a mounted destination so Back preserves scroll, local tabs
and controls. Switching destinations remembers scroll position, library tab/page
and the Browse query within this window; existing QSettings persist search/saved
preferences. This is session navigation memory, not cross-launch scroll recovery.
The approved appearance and native architecture remain intact. Real Omarchy
install/update/remove and screenshot-led visual acceptance are still outstanding.
