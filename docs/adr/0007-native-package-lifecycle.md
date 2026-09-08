# ADR 0007: Resolve typed packages through the existing system tools

Status: accepted implementation boundary; real Omarchy adapter evidence outstanding.

The local Rust core owns read-only host probing and plan generation. The GUI supplies a stable app ID or an exact setup selection. Catalogue text, URLs and author fields never become shell source. Plans last ten minutes and bind catalogue, current distribution eligibility, package/configuration state and resolved effects. Package source changes, system upgrades, signature exceptions and unresolved conflicts block execution.

Use root-owned absolute `pacman` and `pacman-conf` executables. Query resolved configuration with pacman-conf so Include directives and repository precedence follow the package manager. Prefer configured repositories requiring trusted package signatures. Query installed packages and resolve the exact repository/name/version using pacman's print mode; retain dependency hashes and download sizes. This initial adapter declines transactions involving replacements, declared conflicts or dependency upgrades. It never refreshes repository databases independently of a system upgrade.

Omarchy's current `omarchy-pkg-install` is an interactive package chooser. `omarchy-launch-terminal` preserves the normal terminal session, and `omarchy update` owns system snapshots, package upgrades and migrations. Later execution uses an independent terminal worker with durable SQLite transitions and an explicit OS privilege prompt. Closing the GUI or its pipe core must not kill a package transaction. Package removal will use an explicit target, preserving configuration and unrelated dependencies.

Grounding: inspected Omarchy commit `5b91db503c904bbfc5f34bdaaa9c708814958f3d`, particularly `bin/omarchy-pkg-install`, `bin/omarchy-pkg-remove`, `bin/omarchy-launch-terminal`, `bin/omarchy-version` and `bin/omarchy-update-pacman-guard`. The hardware setup can include a repository with `SigLevel = Never`; being configured alone does not make that a managed route.

References: [Omarchy source](https://github.com/omacom/omarchy/tree/5b91db503c904bbfc5f34bdaaa9c708814958f3d), [pacman](https://man.archlinux.org/man/pacman.8.en), [pacman-conf](https://man.archlinux.org/man/pacman-conf.8.en). Live read-only and lifecycle cases must be recorded on an actual supported Omarchy release before enabling the release adapter. The separately compiled development build uses a plainly labelled fictional host and never invokes package writes.
