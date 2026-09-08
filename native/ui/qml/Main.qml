import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    required property var core
    property int section: 0
    readonly property var sections: ["Discover", "Apps", "Setups", "Library", "Makers", "Submit"]
    readonly property var messages: [
        "The catalogue is empty.",
        "No applications are listed yet.",
        "No setups have been published yet.",
        "Library integration is coming next.",
        "No maker profiles have been published yet.",
        "Submissions are not open yet."
    ]
    visible: true
    width: 1040
    height: 700
    minimumWidth: 720
    minimumHeight: 460
    title: "OmaStore"

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 20
            anchors.rightMargin: 12
            Label { text: "OmaStore"; font.pixelSize: 20; font.bold: true }
            Item { Layout.fillWidth: true }
            ToolButton { text: "About"; onClicked: about.open() }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0
        Pane {
            Layout.preferredWidth: 180
            Layout.fillHeight: true
            padding: 12
            ColumnLayout {
                anchors.fill: parent
                spacing: 4
                Repeater {
                    model: window.sections
                    delegate: ItemDelegate {
                        required property int index
                        required property string modelData
                        text: modelData
                        font.pixelSize: 16
                        Layout.fillWidth: true
                        highlighted: window.section === index
                        Accessible.role: Accessible.PageTab
                        Accessible.name: modelData
                        Accessible.selected: window.section === index
                        onClicked: window.section = index
                    }
                }
                Item { Layout.fillHeight: true }
                Label { text: "Development build"; font.pixelSize: 14; wrapMode: Text.Wrap; Layout.fillWidth: true }
            }
        }
        ToolSeparator { orientation: Qt.Vertical; Layout.fillHeight: true }
        Pane {
            Layout.fillWidth: true
            Layout.fillHeight: true
            padding: 32
            ColumnLayout {
                anchors.fill: parent
                spacing: 20
                Label { text: window.sections[window.section]; font.pixelSize: 28; font.bold: true }
                Label {
                    text: window.messages[window.section]
                    font.pixelSize: 18
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
                Label {
                    text: "This is the initial native scaffold. App discovery, installation and author tools are still being built."
                    font.pixelSize: 16
                    wrapMode: Text.Wrap
                    Layout.maximumWidth: 580
                    Layout.fillWidth: true
                }
                Label {
                    visible: window.core.error.length > 0
                    text: "OmaStore's local service could not start. See About for details."
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }
                Item { Layout.fillHeight: true }
            }
        }
    }

    Dialog {
        id: about
        anchors.centerIn: parent
        width: Math.min(480, window.width - 48)
        modal: true
        title: "About OmaStore"
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            spacing: 16
            Label { text: "OmaStore 0.1.0 · Development scaffold"; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label {
                text: window.core.ready ? "Local core " + window.core.version + " is connected." :
                    (window.core.error.length ? window.core.error : "Starting the local core…")
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }
            Label { text: "Independent community project for Omarchy."; wrapMode: Text.Wrap; Layout.fillWidth: true }
        }
    }
}
