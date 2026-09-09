# Native preview evidence

These are actual Qt window captures from the Linux verification environment, not mockups or screenshots of the listed applications.

- `native-discovery.png`: 1280×800 logical pixels, explicitly synthetic catalogue. The letter blocks are fallback initials; no application media or maker participation is implied.
- `native-empty.png`: 800×600 logical pixels, ordinary build with the intentionally empty public catalogue.

Reproduce after building:

```sh
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software ./build-demo/bin/omastore --demo --screenshot build-demo/native-discovery.png --window-size 1280x800
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software ./build/bin/omastore --screenshot build/native-empty.png --window-size 800x600
```

The capture/interaction switches are available only with BUILD_TESTING enabled. The release packaging configuration excludes the test harness. These images establish rendered layout in the recorded environment; real Omarchy theme/tiling/portal behaviour and final visual taste still need review.

The native interaction harness now also exercises actual keyboard purchase consent, paid/failed delivery recovery, signed licence export and offline verification, partial refund request/operator approval, exact fee reversal, author balances, a failed provider payout and a reviewed private dispute-packet export. Settings/remix/lifecycle and author/review flows remain in the same four-size/scale checks. These are fictional/offscreen integration receipts; they do not establish real money, actual Omarchy package/settings effects, final portals or accessibility acceptance.
