# Approved software shelf: native design QA

final result: passed

Scope: the approved storefront shell, app showcase, persistent shelf and
installation-review journey. This is the existing native Qt/Rust application;
the user's native-only requirement supersedes the web-template/browser steps.
No website or simulated browser implementation was substituted.

## Visual truth and evidence

Source: `/workspace/scratch/5ce02b8bcf2c/generated_images/exec-9e9aa41d-dd55-4a02-97ab-1a35019b7a88.png`,
the square-edged concept Tom approved immediately before asking to ship.
Source pixels: 1489×1056. Implementation: `docs/screenshots/authored-shelf.png`,
1488×1056 logical and physical pixels, Qt 6.8.3 offscreen/software, scale 1.
Both show real OmaCalc details with Browse selected. The one-pixel canvas
difference is immaterial; neither image was stretched. CSS size is not applicable.
The source and implementation were supplied together in the final comparison
input. Full-resolution text, action and shelf regions were readable without
additional crop enlargement. No generated image is evidence of a running app.

Additional captures: `authored-small.png` at 800×600 and
`authored-installation.png` at 1280×800. At narrow widths the hero stacks and
scrolls, search occupies its own row, and the persistent shelf scrolls
horizontally. Keyboard focus scrolls obscured controls into view.

## Comparison history

1. First native capture: P1 display type was too small; P2 preview placement
   and shelf proportions differed. Raised display hierarchy, constrained the
   two-column grid, and increased shelf depth.
2. Second comparison: P2 excessive font leading and header height shifted
   the main actions. Used explicit display leading and compact toolbar padding.
   Bundled typefaces load from resources; no runtime font download is needed.
3. Resize review: P2 the 1280px title wrapped prematurely. Display size now
   follows available width before the stacked breakpoint. Recaptured all three
   native views and reran the ordinary suite after this correction.
4. Final paired comparison: no remaining actionable P0/P1/P2 in this scope.

## Required fidelity surfaces

- Typography: Instrument Serif display, IBM Plex Sans body, explicit leading
  and responsive scale. Two-line main headline retained at the reference size.
  Standard platform rasterization differs slightly from the generated concept.
- Layout: zero-radius store surfaces, fine dividers, asymmetric app/preview
  composition, persistent six-app shelf, no clipping of persistent controls.
  The app information disclosure and saved action remain below the showcase.
- Tokens: graphite, warm white, amber actions and category surface roles.
  Text contrast: primary 15.24:1, secondary 8.02:1, action text 9.32:1.
  Selection uses an underline and a text label; keyboard focus has an outline.
- Assets: original attributed OmaCalc screenshot preserved at its true aspect
  ratio. Licensed Feather category symbols deliberately replace the concept's
  provisional app logos; they do not imply publisher-provided artwork. Bundled
  font and icon licences are installed with the package.
- Copy: unofficial/Beta positioning is explicit. Price, version, licence and
  evidence come from catalogue records. No ratings, compatibility passes or
  publisher participation were invented. More/Back and offline status are
  intentional production controls absent from the concept.

## Interaction evidence and limits

Ordinary native suite: eight passed, D-Bus instance check skipped locally.
Real catalogue keyboard journey covers discovery, saving, blocked plan and
disabled confirmation, shelf identity/position retention, empty search and
recovery at normal, 800×600 and 200% display scale. No QML runtime warnings.
Sample suite: nine passed, D-Bus skipped, 35.04s, including existing author,
maker, setup, settings, purchase/refund and finance paths across three sizes
and 200% scaling; this run preceded the final display-size-only adjustment.

Real Omarchy Wayland, portal and assistive-technology testing remains pending.
No live installation or payment gate was enabled. These are offscreen native
receipts, not claims of real Omarchy acceptance or browser testing.

## Follow-up polish

P3: replace category symbols with approved upstream app icons as those media
records are added. The shelf's long final app name elides, with a tooltip and
full accessible name. This is intentional and does not prevent selection.
