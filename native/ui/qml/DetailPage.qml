import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ScrollView {
    id: page
    required property var details
    required property var theme
    required property var core
    required property var mediaPreview
    property int savedRevision: 0
    readonly property var app: details.app || ({})
    readonly property var release: details.release || ({})
    readonly property var summary: details.summary || ({})
    contentWidth: availableWidth
    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    Connections { target: core; function onSavedChanged() { page.savedRevision++; } }
    function readable(value) { return String(value || "unknown").replace(/_/g, " "); }
    Component.onDestruction: mediaPreview.load({})
    ColumnLayout {
        width: page.availableWidth
        spacing: 20
        Item { Layout.preferredHeight: 8 }
        ColumnLayout {
            Layout.fillWidth: true; Layout.leftMargin: 28; Layout.rightMargin: 28
            spacing: 14
            Label { text: page.app.name || ""; font.pixelSize: 36 * theme.scale; font.bold: true; wrapMode: Text.Wrap; textFormat: Text.PlainText; Layout.fillWidth: true }
            Label { text: page.app.summary || ""; font.pixelSize: 18 * theme.scale; color: theme.muted; wrapMode: Text.Wrap; textFormat: Text.PlainText; Layout.fillWidth: true }
            Flow {
                Layout.fillWidth: true; spacing: 10
                Repeater {
                    model: [page.readable(page.app.appType), page.readable(page.app.maturity), page.readable((page.app.licence || {}).class), page.release.version || ""]
                    Label { required property string modelData; text: modelData; padding: 8; color: theme.ink; textFormat: Text.PlainText; background: Rectangle { color: theme.wash; radius: 4 } }
                }
            }
            Label { text: (page.details.makers || []).map(function(m) { return m.name + " · " + m.claim; }).join("\n"); color: theme.muted; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: page.summary.priceLabel || "Price not supplied"; font.pixelSize: 22 * theme.scale; font.bold: true; textFormat: Text.PlainText; Layout.fillWidth: true }
            Flow {
                Layout.fillWidth: true; spacing: 10
                Button { objectName: "acquireButton"; text: (page.details.acquisition || {}).label + " ↗"; onClicked: core.openLink("acquisition") }
                Button { objectName: "saveButton"; text: { page.savedRevision; return core.isSaved(page.app.id || "") ? "Remove from saved" : "Save for later"; } onClicked: core.toggleSaved() }
                Button { text: "Source ↗"; visible: !!page.app.source; onClicked: core.openLink("source") }
                Button { text: "Support ↗"; onClicked: core.openLink("support") }
            }
            Label { text: (page.summary.routeLabel || "") + ". Opens in your browser. Final availability, price and terms belong to the seller."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
            Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
            Label { text: "Before you get it"; font.pixelSize: 22 * theme.scale; font.bold: true }
            GridLayout {
                Layout.fillWidth: true
                columns: 2; columnSpacing: 24; rowSpacing: 10
                Repeater {
                    model: ["Works offline", page.readable(page.release.offline), "Account required", page.readable(page.release.account), "Activation required", page.readable(page.release.activation), "Architecture", (page.release.architectures || []).join(", "), "Service costs", page.release.serviceCosts || "Unknown", "Omarchy evidence", page.summary.evidenceLabel || "Not tested"]
                    Label { required property int index; required property string modelData; text: modelData; color: index % 2 ? theme.ink : theme.muted; font.bold: index % 2 === 0; Layout.fillWidth: true; Layout.preferredWidth: index % 2 ? 400 : 150; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                }
            }
            Repeater {
                model: page.app.tests || []
                ColumnLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    Label { text: "Recorded test · " + page.readable(modelData.result); font.bold: true; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Label { text: modelData.environment + " · " + modelData.testedAt + "\nRelease: " + modelData.releaseId + "\nTested by: " + modelData.actor + " · " + modelData.toolVersion; color: theme.muted; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Label { text: modelData.limitations || "No limitations recorded."; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Button { text: "Read test evidence ↗"; onClicked: core.openLink("evidence", index) }
                }
            }
            Label { text: "About this app"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Label { text: page.app.description || ""; wrapMode: Text.Wrap; textFormat: Text.PlainText; Layout.fillWidth: true }
            Label { text: "See it in use"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Pane {
                Layout.fillWidth: true
                padding: 20
                background: Rectangle { color: theme.wash; radius: 5 }
                Label { width: parent.width; text: (page.app.media || []).length === 0 ? "No screenshots or demonstration have been supplied. This is not evidence that the app works." : "Media opens on demand in your browser. App details remain available if media cannot load."; wrapMode: Text.Wrap; color: theme.muted }
            }
            Repeater {
                model: page.app.media || []
                RowLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    Button { text: modelData.kind === "demo" ? "Play demonstration in browser ↗" : "Preview image"; onClicked: modelData.kind === "demo" ? core.openLink("media", index) : mediaPreview.load(modelData) }
                    Label { text: modelData.alt; Layout.fillWidth: true; textFormat: Text.PlainText; wrapMode: Text.Wrap }
                    Button { text: "Open ↗"; Accessible.name: "Open " + modelData.kind + ": " + modelData.alt; onClicked: core.openLink("media", index) }
                }
            }
            Image { visible: mediaPreview.status === "ready"; source: mediaPreview.source; Layout.fillWidth: true; Layout.preferredHeight: visible ? Math.min(380, page.width * 0.6) : 0; fillMode: Image.PreserveAspectFit; Accessible.role: Accessible.Graphic; Accessible.name: "Application media preview" }
            Label { visible: mediaPreview.status !== "idle" && mediaPreview.status !== "ready"; text: mediaPreview.status === "loading" ? "Loading image…" : (mediaPreview.status === "external" ? "This media is hosted by the publisher. Use Open to view it in your browser." : "This image could not be loaded or verified. App information remains available above."); color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Offers & terms"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Repeater {
                model: page.app.offers || []
                ColumnLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    Label { text: page.readable(modelData.model) + " · tax " + page.readable(modelData.tax); font.bold: true; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Label { text: modelData.entitlement; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Flow {
                        Layout.fillWidth: true; spacing: 8
                        Button { text: "Seller ↗"; onClicked: core.openLink("offer", index) }
                        Button { text: "Terms ↗"; onClicked: core.openLink("terms", index) }
                        Button { text: "Refunds ↗"; visible: !!modelData.refund; onClicked: core.openLink("refund", index) }
                        Button { text: "Cancellation ↗"; visible: !!modelData.cancellation; onClicked: core.openLink("cancellation", index) }
                    }
                }
            }
            Label { text: "Release & removal"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Label { text: page.release.notes || "No release notes supplied."; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: page.release.removal || "Removal instructions not supplied."; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Managed installation is not available in this preview."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
        }
        Item { Layout.preferredHeight: 28 }
    }
}
