import QtQuick

QtObject {
    required property var desktop
    readonly property bool dark: true
    readonly property color page: dark ? "#171817" : "#fafaf7"
    readonly property color surface: dark ? "#202120" : "#ffffff"
    readonly property color sidebar: dark ? "#141514" : "#eeefe8"
    readonly property color ink: dark ? "#f1ede4" : "#202f26"
    readonly property color muted: dark ? "#acafa9" : "#58655d"
    readonly property color accent: dark ? "#e5b36a" : "#315a3d"
    readonly property color accentInk: dark ? "#171817" : "#ffffff"
    readonly property color line: dark ? "#444741" : "#dce1d8"
    readonly property color wash: dark ? "#302b23" : "#e5ecdf"
    readonly property color feature: "#203d33"
    readonly property color featureInk: "#f4f5dc"
    readonly property color featureMuted: "#c5d3be"
    readonly property color featureAccent: "#d8e9b2"
    readonly property real scale: desktop.textScale
    readonly property string mono: desktop.monoFont
    readonly property int inset: 32
    readonly property int gap: 16
    readonly property int radius: 0
    readonly property string display: displayFont.status === FontLoader.Ready ? displayFont.name : "serif"
    readonly property string body: bodyFont.status === FontLoader.Ready ? bodyFont.name : "sans-serif"
    property FontLoader displayFont: FontLoader { source: "qrc:/qt/qml/OmaStore/assets/fonts/InstrumentSerif-Regular.ttf" }
    property FontLoader bodyFont: FontLoader { source: "qrc:/qt/qml/OmaStore/assets/fonts/IBMPlexSans-Regular.ttf" }
    readonly property int controlHeight: 38
    readonly property int titleSize: 34
    readonly property int sectionSize: 22
    readonly property int bodySize: 14
    // No animated movement: instant feedback also serves reduced-motion users.
}
