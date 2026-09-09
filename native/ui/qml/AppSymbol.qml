import QtQuick
import QtQuick.Controls

// Category symbols, not publisher logos. Shapes remain sharp at desktop scaling.
Rectangle {
    id: symbol
    required property var theme
    property string category: "utilities"
    property int size: 56
    implicitWidth: size; implicitHeight: size
    radius: size * 0.23
    color: theme.wash
    border.color: theme.line
    Label {
        anchors.centerIn: parent
        text: symbol.category === "writing" ? "¶" : symbol.category === "video" ? "▷" : symbol.category === "games" ? "✣" : symbol.category === "presentations" ? "▤" : "▦"
        font.family: symbol.theme.mono
        font.pixelSize: symbol.size * 0.48
        color: symbol.theme.accent
    }
    Accessible.ignored: true
}
