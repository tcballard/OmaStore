import QtQuick

QtObject {
    required property var desktop
    readonly property var colors: desktop.themeColors
    readonly property bool following: desktop.followOmarchy && colors.page !== undefined
    readonly property bool dark: following ? colors.dark : true
    readonly property color page: following ? colors.page : "#171817"
    readonly property color surface: following ? colors.surface : "#202120"
    readonly property color sidebar: following ? colors.sidebar : "#141514"
    readonly property color ink: following ? colors.ink : "#f1ede4"
    readonly property color muted: following ? colors.muted : "#acafa9"
    readonly property color accent: following ? colors.accent : "#e5b36a"
    readonly property color accentInk: following ? colors.accentInk : "#171817"
    readonly property color line: following ? colors.line : "#444741"
    readonly property color wash: following ? colors.wash : "#302b23"
    readonly property color feature: following ? surface : "#203d33"
    readonly property color featureInk: following ? ink : "#f4f5dc"
    readonly property color featureMuted: following ? muted : "#c5d3be"
    readonly property color featureAccent: following ? accent : "#d8e9b2"
    readonly property real scale: desktop.textScale
    readonly property string mono: desktop.monoFont
    readonly property int inset: 32
    readonly property int gap: 16
    readonly property int radius: 0
    function symbolSurface(category) {
        if (following) return surface;
        return ({writing:"#273a34",utilities:"#282b42",video:"#352445",games:"#252a28",presentations:"#71422d"})[category] || surface;
    }
    readonly property string display: following ? desktop.monoFont : (displayFont.status === FontLoader.Ready ? displayFont.name : "serif")
    readonly property string body: following ? desktop.monoFont : (bodyFont.status === FontLoader.Ready ? bodyFont.name : "sans-serif")
    property FontLoader displayFont: FontLoader { source: "qrc:/qt/qml/OmaStore/assets/fonts/InstrumentSerif-Regular.ttf" }
    property FontLoader bodyFont: FontLoader { source: "qrc:/qt/qml/OmaStore/assets/fonts/IBMPlexSans-Regular.ttf" }
    readonly property int controlHeight: 38
    readonly property int titleSize: 34
    readonly property int sectionSize: 22
    readonly property int bodySize: 14
    // No animated movement: instant feedback also serves reduced-motion users.
}
