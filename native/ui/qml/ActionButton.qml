import QtQuick
import QtQuick.Controls

Button {
    id: control
    required property var theme
    property bool primary: false
    hoverEnabled: true
    implicitHeight: Math.max(theme.controlHeight, contentItem.implicitHeight + 18)
    implicitWidth: contentItem.implicitWidth + 32
    leftPadding: 16; rightPadding: 16
    contentItem: Text {
        text: control.text
        font.pixelSize: 13 * control.theme.scale
        font.family: control.theme.body
        font.weight: Font.DemiBold
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        color: !control.enabled ? control.theme.muted : control.primary ? control.theme.accentInk : control.theme.accent
    }
    background: Rectangle {
        radius: control.theme.radius
        color: !control.enabled ? control.theme.sidebar : control.primary ? control.theme.accent : control.down || control.hovered ? control.theme.wash : control.theme.surface
        border.color: control.activeFocus ? control.theme.accent : control.primary && control.enabled ? control.theme.accent : control.theme.line
        border.width: control.activeFocus ? 2 : 1
        Rectangle { anchors.fill: parent; anchors.margins: -3; color: "transparent"; visible: control.activeFocus; border.width: 2; border.color: control.theme.accent }
    }
}
