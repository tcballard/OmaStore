import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: shelf
    required property var theme
    required property var core
    property string selectedId: ""
    signal chosen(string id)
    signal browseRequested()
    spacing: 8
    Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
    RowLayout {
        Layout.fillWidth: true; Layout.leftMargin: 32; Layout.rightMargin: 32
        Label { text: "Browse apps"; color: theme.ink; font.pixelSize: 13 * theme.scale }
        Item { Layout.fillWidth: true }
        ToolButton { text: "All apps / filters"; onClicked: shelf.browseRequested() }
    }
    ListView {
        id: list
        objectName: "appShelf"
        Layout.fillWidth: true; Layout.leftMargin: 32; Layout.rightMargin: 32
        Layout.preferredHeight: 88 * theme.scale
        orientation: ListView.Horizontal; clip: true; spacing: 16
        model: core.apps
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.horizontal: ScrollBar { }
        delegate: ItemDelegate {
            id: entry
            required property var modelData
            required property int index
            objectName: "shelf-" + modelData.id
            width: Math.max(196 * theme.scale, (list.width - 80) / 6)
            height: list.height - 10
            padding: 0
            Accessible.name: modelData.name + (shelf.selectedId === modelData.id ? ", selected" : "")
            Accessible.selected: shelf.selectedId === modelData.id
            onActiveFocusChanged: if (activeFocus) list.positionViewAtIndex(index, ListView.Contain)
            Keys.onRightPressed: { list.currentIndex = Math.min(index + 1, list.count - 1); list.itemAtIndex(list.currentIndex).forceActiveFocus(); }
            Keys.onLeftPressed: { list.currentIndex = Math.max(index - 1, 0); list.itemAtIndex(list.currentIndex).forceActiveFocus(); }
            background: Rectangle {
                color: entry.hovered ? theme.surface : "transparent"
                border.color: entry.activeFocus ? theme.accent : "transparent"
                Rectangle { anchors.bottom: parent.bottom; height: 2; width: parent.width; color: theme.accent; visible: shelf.selectedId === entry.modelData.id }
            }
            contentItem: RowLayout {
                spacing: 12
                AppSymbol { theme: shelf.theme; category: entry.modelData.category; size: 48 * theme.scale }
                ColumnLayout {
                    Layout.fillWidth: true; spacing: 3
                    Label { text: entry.modelData.name; color: theme.ink; font.pixelSize: 12 * theme.scale; elide: Text.ElideRight; Layout.fillWidth: true; textFormat: Text.PlainText }
                    Label { text: entry.modelData.category; color: theme.muted; font.pixelSize: 11 * theme.scale; textFormat: Text.PlainText }
                    Label { text: (entry.modelData.priceLabel || "View details") + (shelf.selectedId === entry.modelData.id ? " · Selected" : ""); color: theme.accent; font.pixelSize: 10 * theme.scale }
                }
            }
            onClicked: shelf.chosen(modelData.id)
        }
    }
}
