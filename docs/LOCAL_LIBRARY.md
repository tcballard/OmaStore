# Local library and identity handoff

The native library separates observed installations, saved listings and operation proposals. It lives under `$XDG_STATE_HOME/omastore/native` (or `~/.local/state/omastore/native`) in a private SQLite database with full synchronous WAL transactions. Sample mode uses `omastore-sample` and a checked database mode marker. Opening a database through a symlink or with exposed permissions fails explicitly.

An observation records installed version and time, while keeping the catalogue's version and test evidence distinct. An external upgrade changes the observed version and marks the source as externally changed. If a probe is unavailable, cached observations remain dated observations. They are never uploaded to the service. Delisting does not remove the local record or installed application.

Each saved proposal has a content-derived operation ID and a monotonic event sequence. Duplicate saves do not create another operation. Only a planned record exists before consent. Retain up to 100 unexecuted proposals and discard unexecuted proposals older than seven days when saving another; execution history follows the lifecycle retention rules. Library pages contain at most 30 apps and the 50 most recent operations. Event reads return at most 100 events after an explicit sequence.

Opening an installed app queries its package's current presence and desktop files. Supported launchers are root-owned regular files directly under `/usr/share/applications`, with an Application entry and no Hidden/NoDisplay flag. GIO receives that exact desktop file, never an author command, URI-supplied executable or a higher-precedence user desktop override. A launch response means the request reached the system launcher, not proof that the app completed startup. Missing or multiple launchers remain explicit choices.

`omastore://app/ID` and `omastore://setup/ID?revision=REVISION` carry identity only. The strict Rust parser rejects extra parameters, credentials, ports, fragments, percent encoding, normalised traversal, paths and external origins. The current catalogue must contain the exact ID/revision. A handoff navigates; it cannot confirm, install or apply settings.

The desktop entry registers `x-scheme-handler/omastore`. A per-user process lock prevents another GUI write session. Where the session bus is available, a second invocation forwards the identity and requests focus on the existing window. Without it, a second invocation exits with a clear unavailable result. Wayland compositor focus policy still applies.

Verification: persistent observation/version-change tests, mode and symlink rejection, identity-injection cases, actual core/Qt flow, and an isolated session-bus two-process test. The latter skips only when this verification environment cannot create a bus; CI must exercise it. Real Omarchy launcher, focus, accessibility and offline app-launch evidence remain release gates.
