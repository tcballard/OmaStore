import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: header
    required property var theme
    required property int section
    signal navigate(int index)
    signal aboutRequested()
    spacing: 0
    RowLayout {
        Layout.fillWidth: true; Layout.margins: 22; spacing: 12
        ColumnLayout {
            spacing: 3
            RowLayout {
                Label { text: "OmaStore"; font.pixelSize: 26 * theme.scale; font.bold: true; color: theme.ink }
                Label { text: " BETA "; color: theme.accent; font.pixelSize: 11 * theme.scale; background: Rectangle { color: "transparent"; border.color: theme.accent } }
            }
            Label { text: "Unofficial App Store for Omarchy"; color: theme.muted; font.pixelSize: 11 * theme.scale }
        }
        Item { Layout.fillWidth: true }
        Repeater {
            model: [{name:"Discover",index:0},{name:"Browse",index:1},{name:"Library",index:3}]
            ToolButton {
                id: tab
                required property var modelData
                objectName: "nav" + (modelData.name === "Browse" ? "Apps" : modelData.name)
                text: modelData.name; font.pixelSize: 14 * theme.scale
                palette.buttonText: header.section === modelData.index ? theme.accent : theme.ink
                Accessible.role: Accessible.PageTab
                Accessible.selected: header.section === modelData.index
                background: Rectangle {
                    color: tab.hovered ? theme.surface : "transparent"
                    border.color: tab.activeFocus ? theme.accent : "transparent"
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 2; color: theme.accent; visible: header.section === tab.modelData.index }
                }
                onClicked: header.navigate(modelData.index)
            }
        }
        ToolButton {
            objectName: "moreNavigation"; text: "More"; onClicked: menu.open()
            Menu {
                id: menu
                MenuItem { objectName: "navSetups"; text: "Setups"; onTriggered: header.navigate(2) }
                MenuItem { objectName: "navMakers"; text: "Makers"; onTriggered: header.navigate(4) }
                MenuItem { objectName: "navSubmit"; text: "Submit"; onTriggered: header.navigate(5) }
                MenuItem { text: "About & shortcuts"; onTriggered: header.aboutRequested() }
            }
        }
    }
    Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
}
