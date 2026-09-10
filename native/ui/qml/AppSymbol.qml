import QtQuick
import QtQuick.Controls

// Category symbols, not publisher logos. Shapes remain sharp at desktop scaling.
Rectangle {
    id: symbol
    required property var theme
    property string category: "utilities"
    property int size: 56
    implicitWidth: size; implicitHeight: size
    radius: 0
    color: theme.symbolSurface(category)
    border.color: theme.line
    // Public Controls icon tinting keeps the bundled Feather glyphs theme-aware.
    ToolButton {
        anchors.centerIn: parent
        width: symbol.size * 0.55; height: width
        enabled: false; focusPolicy: Qt.NoFocus
        padding: 0; display: AbstractButton.IconOnly
        icon.width: width; icon.height: height; icon.color: theme.accent
        icon.source: "qrc:/qt/qml/OmaStore/assets/icons/" + ({writing:"file-text",video:"scissors",games:"monitor",presentations:"monitor"}[symbol.category] || "grid") + ".svg"
        background: null
        Accessible.ignored: true
    }
    Accessible.ignored: true
}
