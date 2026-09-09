import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ItemDelegate {
    id: feature
    required property var theme
    signal chosen()
    objectName: "discoverCalculator"
    implicitHeight: Math.max(300, copy.implicitHeight + 64)
    hoverEnabled: true
    padding: 0
    Accessible.name: "Explore OmaCalc. A small calculator for everyday arithmetic."
    background: Rectangle {
        radius: theme.radius
        color: theme.feature
        border.width: feature.activeFocus ? 3 : 1
        border.color: feature.activeFocus ? theme.accent : theme.feature
    }
    contentItem: Item {
        ColumnLayout {
            id: copy
            anchors.left: parent.left; anchors.leftMargin: 30
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width * 0.56 - 30
            spacing: 14
            Label { text: "IN THE CATALOGUE"; color: theme.featureMuted; font.family: theme.mono; font.pixelSize: 10 * theme.scale; font.letterSpacing: 1.2 }
            Label { text: "A little app.\nA clear answer."; color: theme.featureInk; font.pixelSize: (feature.width > 720 ? 38 : 30) * theme.scale; font.weight: Font.Bold; lineHeight: 0.98; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label { text: "Meet OmaCalc. Everyday arithmetic,\nin its own native window."; color: theme.featureMuted; font.pixelSize: 13 * theme.scale; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label { text: "Explore OmaCalc  →"; color: theme.featureAccent; font.weight: Font.DemiBold; font.pixelSize: 13 * theme.scale; Layout.topMargin: 6; font.underline: feature.hovered }
        }
        Image {
            anchors.right: parent.right; anchors.rightMargin: 30
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width * 0.32
            height: parent.height - 44
            source: "qrc:/qt/qml/OmaStore/assets/omacalc-upstream.png"
            fillMode: Image.PreserveAspectFit
            sourceSize.width: 480
            Accessible.ignored: true
        }
    }
    onClicked: chosen()
}
