# ADR 0001: OmaStore is a native application

Status: accepted product direction; scaffold engineering choices recorded 8 September 2026.

Tom explicitly corrected the earlier web-first handoff: the storefront itself must be native to Omarchy. The earlier website-first order is superseded before implementation.

Use a standalone Qt 6/QML window and Rust core. A small C++ `QProcess` bridge connects them with bounded, versioned JSON-lines messages over child-process pipes. This avoids introducing a browser runtime and keeps future package operations outside the GUI. The scaffold uses Qt's public APIs and a standard desktop entry; it does not import Quickshell internals or run inside the desktop shell.

The GUI resolves the core as an absolute sibling executable. The initial protocol permits only `core.info`, with a 256 KiB line limit and a 128-byte request ID limit. Oversized input closes the read-only core stream after a bounded error. No catalogue field, URL or message becomes executable text. B12–B15 must extend this into typed plans and durable operation reconciliation before introducing writes.

Qt's Fusion controls and palette provide a modest development shell. Branding and final storefront composition remain matters for Tom's taste review. This scaffold is not a claim that Omarchy-specific colours, scaling, launcher or accessibility behaviour have been verified.

Native discovery and the local library are the primary product. A shared catalogue/status API and author/reviewer services can be added behind native views. A supporting website can be considered later; no frontend framework, hosting identity, domain or cloud database is part of B00.

Build with CMake/Ninja and Cargo. Qt 6.4 is the minimum; the chosen Rust compiler and crate resolution are pinned. CI builds on Linux and starts the actual Qt application offscreen. A real Omarchy desktop remains a separate acceptance environment.

References: [Qt QML module integration](https://doc.qt.io/qt-6/qt-add-qml-module.html), [QProcess](https://doc.qt.io/qt-6/qprocess.html), [Omarchy GUI applications](https://omarchy.org/manual/guis/).
