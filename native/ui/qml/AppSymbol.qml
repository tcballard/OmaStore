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
    color: theme.wash
    border.color: theme.line
    Image {
        anchors.centerIn: parent
        width: symbol.size * 0.55; height: width
        source: "qrc:/qt/qml/OmaStore/assets/icons/" + ({writing:"file-text",video:"scissors",games:"monitor",presentations:"monitor"}[symbol.category] || "grid") + ".svg"
    }
    Accessible.ignored: true
}
