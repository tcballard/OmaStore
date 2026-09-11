# Native storefront verification

## Approved authored shelf, 10 September 2026

`authored-shelf.png` is the actual 1488×1056 native implementation of Tom's
approved square-edged concept. `authored-small.png` captures the 800×600
stacked layout, and `authored-installation.png` the blocked plan at 1280×800.
These supersede the earlier visual direction below. See `../../design-qa.md`
for comparison iterations, asset decisions, contrast and keyboard evidence.
All three are ordinary Qt 6.8.3 captures with fresh temporary application state.
The small view scrolls vertically while the app shelf remains visible.

## Earlier polish pass (superseded design)

These are actual Qt 6.8.3 offscreen captures of the ordinary six-app catalogue,
not mockups. They were captured at local implementation `b03cc345dff97d023f86ea6c858adaaaf971d857`,
published as `1a00cdf8f8686536318c4d96e3ba8966e55d523c`; both have tree
`e7a254b41bfdffadad93e736d50961570cd341d0`.

- `discover.png`: 1280×800, light appearance.
- `discover-dark.png`: 1280×800, dark appearance.
- `detail.png`: real OmaCalc details, 1280×800.
- `detail-small.png`: real OmaCalc details, 800×600.
- `installation.png`: blocked installation review on this non-Omarchy host.

Capture with the QA build, for example:

```sh
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software \
  ./build/bin/omastore --screenshot discover.png --window-size 1280x800
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software \
  ./build/bin/omastore --screenshot detail.png --capture-view detail --window-size 1280x800
```

Use `--capture-view plan` for the installation sheet and `--dark-appearance`
for dark QA. These switches are excluded from non-testing release builds.
Captures and journey tests use temporary application state. The capture
itself makes no installation attempt.

Final local verification: ordinary CTest has eight passes and one D-Bus skip
(5.59 seconds); sample CTest has nine passes and one D-Bus skip (36.89 seconds).
The real-catalogue journey verifies feature activation, save, blocked plan,
disabled confirmation, return, retained scroll/focus and category navigation.
It runs at normal size, 800×600, and 800×600 with 200% display scaling. The
sample suite covers existing lifecycle, author and commerce paths at three
window sizes and 200% scaling. No QML runtime warnings occurred.

Measured text contrast ratios: light secondary text 5.85:1; primary button
7.89:1; dark secondary text 9.29:1; dark primary button 10.06:1; feature body
7.56:1. This is colour-pair evidence, not blanket accessibility certification.

Actual Omarchy Wayland, portal, assistive-technology and package lifecycle
acceptance remain pending. Initial-letter placeholders have been replaced by
category symbols; these are not publisher logos. The OmaCalc preview image
is attributed upstream artwork, as documented in `native/ui/assets/README.md`.
