import QtQuick

QtObject {
    required property var desktop
    readonly property bool dark: desktop.dark
    readonly property color page: dark ? "#181d1b" : "#fafaf7"
    readonly property color surface: dark ? "#232b27" : "#ffffff"
    readonly property color sidebar: dark ? "#141a16" : "#eeefe8"
    readonly property color ink: dark ? "#f0f2e9" : "#202f26"
    readonly property color muted: dark ? "#b7c2b9" : "#58655d"
    readonly property color accent: dark ? "#b7d8a2" : "#315a3d"
    readonly property color onAccent: dark ? "#172619" : "#ffffff"
    readonly property color line: dark ? "#414c44" : "#dce1d8"
    readonly property color wash: dark ? "#304034" : "#e5ecdf"
    readonly property color feature: "#203d33"
    readonly property color onFeature: "#f4f5dc"
    readonly property color featureMuted: "#c5d3be"
    readonly property color featureAccent: "#d8e9b2"
    readonly property real scale: desktop.textScale
    readonly property string mono: desktop.monoFont
    readonly property int inset: 32
    readonly property int gap: 16
    readonly property int radius: 12
    readonly property int controlHeight: 38
    readonly property int titleSize: 34
    readonly property int sectionSize: 22
    readonly property int bodySize: 14
    // No animated movement: instant feedback also serves reduced-motion users.
}
