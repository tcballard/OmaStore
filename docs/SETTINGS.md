# Selective desktop settings

Open Library → Desktop settings, or Review desktop settings on a setup. Choose supported fields explicitly, then preview the current value, desired value, affected scope and restoration limits. Previewing saves a local proposal; it does not authorise a desktop change. Unknown versions, custom bars, missing/duplicate clock widgets and unsafe configuration files are explicit unavailable states.

The initial registry contains `omarchy-theme` revision `1` and `omarchy-clock-placement` revision `1`. Theme values are `tokyo-night` and `catppuccin` for public references; sample mode offers only `sample-ink` and `sample-sand`. Clock values `left-start`, `center-start` and `right-start` select index zero in that section. The widget's options and all other shell fields remain outside the requested change.

Native IPC: `settings.adapters` takes empty parameters. `settings.preview` takes `{settings:[{adapter,valueId,revision}]}` with one or two distinct known adapters. `settings.get` accepts a saved proposal `{id}`. Proposals expire after ten minutes, bind the current environment/file identity and remain local. The live version allowlist is empty until real Omarchy evidence passes. See ADR 0008 for exact source grounding and ownership boundaries.
