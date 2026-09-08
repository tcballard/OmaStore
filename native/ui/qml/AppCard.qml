import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ItemDelegate {
    id: card
    required property var app
    required property var theme
    signal chosen()
    signal removeRequested()
    property bool bookmark: false
    objectName: "app-" + app.id
    implicitHeight: contentItem.implicitHeight + 36
    padding: 18
    hoverEnabled: true
    Accessible.name: app.name + ". " + app.summary + ". " + app.priceLabel + ". " + app.evidenceLabel
    background: Rectangle {
        color: card.hovered ? theme.wash : theme.surface
        radius: 7
        border.width: card.activeFocus ? 2 : 1
        border.color: card.activeFocus ? theme.accent : theme.line
    }
    contentItem: ColumnLayout {
        spacing: 12
        RowLayout {
            spacing: 12
            Rectangle {
                width: 44; height: 44; radius: 8; color: theme.wash
                Label { anchors.centerIn: parent; text: card.app.name.substring(0, 1); color: theme.accent; font.pixelSize: 25 * theme.scale; font.family: theme.mono; font.bold: true }
                Accessible.ignored: true
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                Label { text: card.app.name; font.bold: true; font.pixelSize: 17 * theme.scale; color: theme.ink; textFormat: Text.PlainText; elide: Text.ElideRight; Layout.fillWidth: true }
                Label { text: String(card.app.appType).replace(/_/g, " ") + " · " + card.app.maturity; color: theme.muted; font.pixelSize: 11 * theme.scale; textFormat: Text.PlainText; Layout.fillWidth: true; wrapMode: Text.Wrap }
            }
        }
        Label { text: card.app.summary; color: theme.ink; wrapMode: Text.Wrap; textFormat: Text.PlainText; Layout.fillWidth: true; Layout.minimumHeight: 42 * theme.scale }
        Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
        RowLayout {
            Layout.fillWidth: true
            Label { text: card.app.priceLabel; font.bold: true; color: theme.ink; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "View →"; color: theme.accent; font.bold: true }
        }
        Label { text: card.app.evidenceLabel; color: theme.muted; font.pixelSize: 11 * theme.scale; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
        RowLayout {
            visible: card.bookmark
            Label { text: "Saved copy · open for current details"; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; font.pixelSize: 11 * theme.scale }
            ToolButton { text: "Remove"; Accessible.name: "Remove " + card.app.name + " from saved"; onClicked: card.removeRequested() }
        }
    }
    onClicked: chosen()
}
