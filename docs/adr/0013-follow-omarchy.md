# ADR 0013: follow the user's Omarchy appearance

Accepted 10 September 2026. Tom authorised following the active Omarchy theme
and fonts by default, retaining the approved square-edged layout. Supersedes
ADR 0012's always-dark policy, not its geometry or information architecture.

`More → Appearance` offers **Follow Omarchy** (default) and **OmaStore**
(the approved graphite/amber, Instrument Serif/IBM Plex Sans presentation).
The choice is a local QSettings preference. App screenshots are never tinted.

Source conventions checked against Omarchy v4.0.3:

- [theme-set](https://github.com/omacom/omarchy/blob/v4.0.3/bin/omarchy-theme-set)
  replaces `~/.local/state/omarchy/current/theme`; we read `colors.toml` there.
  The current script uses this home-relative state path, not XDG_STATE_HOME.
- [font-set](https://github.com/omacom/omarchy/blob/v4.0.3/bin/omarchy-font-set)
  writes Fontconfig's monospace selection.
- [font-current](https://github.com/omacom/omarchy/blob/v4.0.3/bin/omarchy-font-current)
  resolves it with `fc-match monospace`. The store uses that resolved family
  for interface, headings and technical text while following an available theme.
- [Tokyo Night palette](https://github.com/omacom/omarchy/blob/v4.0.3/themes/tokyo-night/colors.toml)
  establishes the flat colour keys. `muted` is a border/surface colour, not a
  readable text role. Secondary text uses `dark_foreground` only when readable.

The reader accepts bounded, inert quoted six-digit RGB values, requires valid
background/foreground, derives optional roles and corrects unreadable text.
It does not evaluate TOML expressions or source shell files. Unsupported/missing
palettes fall back to OmaStore without changing the saved Follow preference.
No desktop configuration is written and no local settings adapter is enabled.

Filesystem notifications are debounced for 180 ms and rearmed across directory
replacement. A five-second read-only recovery poll covers symlink changes,
missing ancestors and included Fontconfig files. Font resolution is an async
fixed `/usr/bin/fc-match` invocation with fixed arguments and a one-second
timeout, never a theme-provided command. Fontconfig is an explicit Arch runtime
dependency. The last available resolved font survives resolution failure.

Tests isolate preference/theme files and fontconfig selection, check live
replacement, late creation, deletion, malformed/oversized data, light contrast,
font changes and preference persistence. QA-only `--theme-fixture` enables full
native journey checks without touching a host's active theme. Offscreen Linux
evidence is not real Omarchy/Wayland or portal acceptance. Legacy pre-v4 theme
layouts and arbitrary theme formats are not claimed as supported.
