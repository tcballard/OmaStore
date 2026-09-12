import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ItemDelegate {
    id: card
    required property var core
    required property var app
    required property var theme
    signal chosen()
    signal removeRequested()
    property bool bookmark: false
    objectName: "app-" + app.id
    implicitHeight: contentItem.implicitHeight + 32
    padding: 16
    hoverEnabled: true
    Accessible.name: app.name + ". " + app.summary + ". " + app.priceLabel + ". " + app.evidenceLabel
    background: Rectangle {
        color: card.down || card.hovered ? theme.wash : theme.surface
        radius: theme.radius
        border.width: card.activeFocus ? 2 : 1
        border.color: card.activeFocus ? theme.accent : theme.line
    }
    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            spacing: 14
            AppSymbol { theme: card.theme; category: card.app.category || "utilities"; size: 54 }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 4
                Label { text: card.app.name; font.weight: Font.DemiBold; font.pixelSize: 16 * theme.scale; color: theme.ink; textFormat: Text.PlainText; elide: Text.ElideRight; Layout.fillWidth: true }
                Label { text: card.app.summary; color: theme.muted; font.pixelSize: 12 * theme.scale; wrapMode: Text.Wrap; maximumLineCount: 2; elide: Text.ElideRight; textFormat: Text.PlainText; Layout.fillWidth: true }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: card.app.evidenceLabel; color: theme.muted; font.pixelSize: 10 * theme.scale; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: card.app.priceLabel + "  →"; font.weight: Font.DemiBold; color: theme.accent; font.pixelSize: 12 * theme.scale; textFormat: Text.PlainText }
        }
        AppStateLabel { core: card.core; theme: card.theme; appId: card.app.id; Layout.fillWidth: true }
        RowLayout {
            visible: card.bookmark
            Label { text: "Saved on this device"; color: theme.muted; Layout.fillWidth: true; font.pixelSize: 11 * theme.scale }
            ToolButton { text: "Remove"; Accessible.name: "Remove " + card.app.name + " from saved"; onClicked: card.removeRequested() }
        }
    }
    onClicked: chosen()
}
