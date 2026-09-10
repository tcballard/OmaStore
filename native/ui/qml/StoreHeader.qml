import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: header
    required property var theme
    required property int section
    required property var core
    property bool canGoBack: false
    readonly property var searchInput: width >= 1100 ? searchWide : searchNarrow
    signal navigate(int index)
    signal aboutRequested()
    signal backRequested()
    signal searchRequested(string query)
    spacing: 0
    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: header.width < 1100 ? 22 : 32
        Layout.rightMargin: header.width < 1100 ? 22 : 32
        Layout.topMargin: 16; Layout.bottomMargin: 16; spacing: 12
        ColumnLayout {
            spacing: 3
            RowLayout {
                Label { text: "Oma<font color='" + theme.accent + "'>Store</font>"; textFormat: Text.RichText; font.pixelSize: 30 * theme.scale; font.bold: true; color: theme.ink }
                Label { text: " BETA "; color: theme.accent; font.pixelSize: 11 * theme.scale; background: Rectangle { color: "transparent"; border.color: theme.accent } }
            }
            Label { text: "Unofficial App Store for Omarchy"; color: theme.muted; font.pixelSize: 13 * theme.scale }
        }
        Item { Layout.fillWidth: true }
        ToolButton { objectName: "backButton"; text: "Back"; visible: header.canGoBack; onClicked: header.backRequested() }
        Repeater {
            model: [{name:"Discover",index:0},{name:"Browse",index:1},{name:"Library",index:3}]
            ToolButton {
                id: tab
                required property var modelData
                objectName: "nav" + (modelData.name === "Browse" ? "Apps" : modelData.name)
                text: modelData.name; font.pixelSize: 16 * theme.scale
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
                MenuItem { text: "Refresh catalogue"; enabled: core.ready && !core.loading; onTriggered: core.refresh() }
                MenuItem { text: "About & shortcuts"; onTriggered: header.aboutRequested() }
            }
        }
        TextField {
            id: searchWide
            objectName: visible ? "searchField" : "inactiveSearchField"
            visible: header.width >= 1100
            Layout.preferredWidth: Math.min(390, header.width * 0.27)
            implicitHeight: 42
            text: core.query.q || ""; maximumLength: 200
            font.pixelSize: 14 * theme.scale
            placeholderText: "Search apps, tools and more…"
            Accessible.name: "Search applications"
            background: Rectangle { color: theme.surface; border.color: searchWide.activeFocus ? theme.accent : theme.line; border.width: searchWide.activeFocus ? 2 : 1 }
            onTextEdited: header.searchRequested(text)
        }
    }
    TextField {
        id: searchNarrow
        objectName: visible ? "searchField" : "inactiveSearchField"
        visible: header.width < 1100
        Layout.fillWidth: true; Layout.leftMargin: 22; Layout.rightMargin: 22; Layout.bottomMargin: 12
        implicitHeight: 38
        text: core.query.q || ""; maximumLength: 200
        font.pixelSize: 14 * theme.scale
        placeholderText: "Search apps, tools and more…"
        Accessible.name: "Search applications"
        background: Rectangle { color: theme.surface; border.color: searchNarrow.activeFocus ? theme.accent : theme.line; border.width: searchNarrow.activeFocus ? 2 : 1 }
        onTextEdited: header.searchRequested(text)
    }
    Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
}
