# ADR 0015: Device state behind the storefront destinations

Accepted 12 September 2026; B13a continuation. Tom approved Discover, Installed,
Saved and Updates and authorised wiring these before a later UX/visual pass.
The Mac App Store experience specification is a behavioural reference. Existing
native QML appearance, theme support, author workflows and lifecycle gates remain.

`library.inventory` accepts `filter` (`all`, `installed`, `updates`), `offset`,
optional `id` and `snapshot`. Pages contain at most 30 current catalogue apps;
subsequent pages require the previous snapshot, binding catalogue and device state.
`library.app_status` accepts one app ID and uses the same projection. Each item
has state, primaryAction, installedVersion, availableVersion, updateIgnored and
observedAt. Counts cover catalogue apps, not every package or dependency installed
on the computer. Saved items remain independent QSettings bookmarks.

Only bounded local pacman observations establish installation and updates.
`pacman -Qu` records contain name, installed version, `->`, available version,
and optionally `[ignored]`. Do not parse them as `pacman -Q` rows, compare version
strings lexically, or use the online community index as a device upgrade list.
Reject duplicate/malformed rows and disagreement between the observed installed
version and update row. Preserve epoch/version/release strings and ignored state.
[Pacman query documentation](https://man.archlinux.org/man/pacman.8.en#QUERY_OPTIONS_(APPLY_TO_-Q)).

Unavailable/unsupported/locked hosts yield unknown status and null counts, not
zero installed apps or an up-to-date claim. Existing last-known journal records
are preserved. A successful observation reconciles externally installed, updated
or removed apps and updates changed catalogue package mappings. The inventory
excludes apps no longer in the current catalogue; their durable journal remains.
No inventory leaves the device. Results are refreshed on entering the device
views and app details, or explicitly with Refresh this device. Continuous device
polling, operation badges on all discovery cards and inventory-wide search are
later slices; repository startup/hourly synchronisation is unchanged.

Action values are intents, not execution authority. `open` resolves and validates
an actual desktop launcher using the existing local adapter; multiple launchers
require selection. `review_install` still enters a typed plan and keeps the live
execution gate. `system_update` opens the existing explicit-confirmation terminal
handoff. It may update the whole system and apply Omarchy migrations. Never refresh
package databases independently or implement partial upgrades for store badges.
Update results disclose that they use local databases; older databases carry a
staleness warning. No invented update completion or download percentage is shown.

Primary destinations are Discover, Installed, Saved and Updates. Browse remains
available through search, the app shelf and More. Existing secondary library
controls remain accessible pending the screenshot-led experience pass. This is a
functional foundation, not final visual acceptance or evidence of upstream release.
