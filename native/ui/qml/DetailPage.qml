import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ScrollView {
    id: page
    required property var details
    required property var theme
    required property var core
    required property var mediaPreview
    signal makerChosen(string id)
    signal purchasesRequested(string id)
    property int savedRevision: 0
    property bool reporting: false
    property bool moreDetails: false
    readonly property var app: details.app || ({})
    readonly property var release: details.release || ({})
    readonly property var summary: details.summary || ({})
    contentWidth: availableWidth
    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    Connections { target: core; function onSavedChanged() { page.savedRevision++; } function onWorkspaceChanged() {const r=core.workspaceReply;if(r.command==="distribution" && r.appId===page.app.id && r.private && !r.error) reportMessage.text="";} }
    function readable(value) { return String(value || "unknown").replace(/_/g, " "); }
    Component.onDestruction: mediaPreview.load({})
    ColumnLayout {
        width: page.availableWidth
        spacing: 20
        AppShowcase { Layout.fillWidth: true; details: page.details; theme: page.theme; core: page.core }
        Flow {
            Layout.fillWidth: true; Layout.leftMargin: theme.inset; Layout.rightMargin: theme.inset; spacing: 12
            ActionButton { theme: page.theme; objectName: "saveButton"; text: { page.savedRevision; return core.isSaved(page.app.id || "") ? "Saved" : "Save for later"; } onClicked: core.toggleSaved() }
            ActionButton { theme: page.theme; text: page.moreDetails ? "Hide app information" : "More app information"; onClicked: page.moreDetails = !page.moreDetails }
        }
        ColumnLayout {
            visible: page.moreDetails
            Layout.fillWidth: true; Layout.leftMargin: theme.inset; Layout.rightMargin: theme.inset; spacing: 14
            Label { text: "About this app"; font.pixelSize: theme.sectionSize * theme.scale; font.weight: Font.DemiBold; Layout.topMargin: 8 }
            Label { text: page.app.description || ""; wrapMode: Text.Wrap; textFormat: Text.PlainText; Layout.fillWidth: true; font.pixelSize: 14 * theme.scale; lineHeight: 1.3 }
            Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }
            Label { text: "Before you get it"; font.pixelSize: 22 * theme.scale; font.bold: true }
            GridLayout {
                Layout.fillWidth: true
                columns: 2; columnSpacing: 24; rowSpacing: 10
                Repeater {
                    model: ["App type", page.readable(page.app.appType), "Release maturity", page.readable(page.app.maturity), "Works offline", page.readable(page.release.offline), "Account required", page.readable(page.release.account), "Activation required", page.readable(page.release.activation), "Architecture", (page.release.architectures || []).join(", "), "Service costs", page.release.serviceCosts || "Unknown", "Omarchy evidence", page.summary.evidenceLabel || "Not tested"]
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
                    ActionButton { theme: page.theme; text: "Read test evidence ↗"; onClicked: core.openLink("evidence", index) }
                }
            }
            Label { visible: (page.app.media || []).length > 0; text: "See it in use"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Pane {
                Layout.fillWidth: true
                visible: page.app.id !== "repo-omacalc" || (page.app.media || []).length > 0
                padding: 20
                background: Rectangle { color: theme.wash; radius: 5 }
                Label { width: parent.width; text: (page.app.media || []).length === 0 ? "The maker has not supplied additional screenshots. You can explore the project through its source link." : "Media opens on demand in your browser. App details remain available if media cannot load."; wrapMode: Text.Wrap; color: theme.muted }
            }
            Repeater {
                model: page.app.media || []
                RowLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    ActionButton { theme: page.theme; text: modelData.kind === "demo" ? "Play demonstration in browser ↗" : "Preview image"; onClicked: modelData.kind === "demo" ? core.openLink("media", index) : mediaPreview.load(modelData) }
                    Label { text: modelData.alt; Layout.fillWidth: true; textFormat: Text.PlainText; wrapMode: Text.Wrap }
                    ActionButton { theme: page.theme; text: "Open ↗"; Accessible.name: "Open " + modelData.kind + ": " + modelData.alt; onClicked: core.openLink("media", index) }
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
                        ActionButton { theme: page.theme; text: "Seller ↗"; onClicked: core.openLink("offer", index) }
                        ActionButton { theme: page.theme; text: "Terms ↗"; onClicked: core.openLink("terms", index) }
                        ActionButton { theme: page.theme; text: "Refunds ↗"; visible: !!modelData.refund; onClicked: core.openLink("refund", index) }
                        ActionButton { theme: page.theme; text: "Cancellation ↗"; visible: !!modelData.cancellation; onClicked: core.openLink("cancellation", index) }
                    }
                }
            }
            Label { text: "Release & removal"; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 12 }
            Label { text: page.release.notes || "No release notes supplied."; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: page.release.removal || "Removal instructions not supplied."; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Project & support"; font.pixelSize: theme.sectionSize * theme.scale; font.weight: Font.DemiBold; Layout.topMargin: 12 }
            Label { text: (page.details.makers || []).map(m => m.name + " · " + (m.claimLabel || m.claim)).join("\n"); color: theme.muted; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Flow {
                Layout.fillWidth: true; spacing: 8
                Repeater { model: page.details.makers || []; ActionButton { required property var modelData; theme: page.theme; text: "Meet " + modelData.name; onClicked: page.makerChosen(modelData.id) } }
                ActionButton { theme: page.theme; text: "Support ↗"; onClicked: core.openLink("support") }
                ActionButton { theme: page.theme; objectName: "acquireButton"; enabled: core.distributionCurrent && core.distribution.distribution !== "suspended"; text: ((page.details.acquisition || {}).label || "Visit publisher") + " ↗"; onClicked: core.openLink("acquisition") }
                ActionButton { theme: page.theme; text: "Purchases and licences"; onClicked: page.purchasesRequested(page.app.id) }
            }
            Label { text: (page.summary.routeLabel || "") + ". External acquisition opens in your browser; final price and terms belong to the seller."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; textFormat: Text.PlainText }
            ActionButton { theme: page.theme; text: page.reporting ? "Close reporting tools" : "Report a problem"; onClicked: page.reporting = !page.reporting }
            ColumnLayout {
                visible: page.reporting; Layout.fillWidth: true; spacing: 12
                ActionButton { theme: page.theme; text: "Refresh distribution status"; enabled: !core.loading; onClicked: core.workspaceAction("status.get", {ids:[page.app.id]}) }
            ComboBox {id:reportKind;model:["integrity","security","compatibility","availability","ownership"];Accessible.name:"Report category"}
            TextArea {id:reportMessage;placeholderText:"Describe a security, integrity or compatibility concern privately";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.name:"Private distribution report"}
            Flow {
                Layout.fillWidth:true;spacing:8
                ActionButton { theme: page.theme;text:core.workspace.actor ? "Send private report" : "Sign in from Submit to report";enabled:!!core.workspace.actor && reportMessage.text.length>0 && reportMessage.text.length<=2000 && !core.loading;onClicked:core.workspaceAction("command",{command:"distribution",operation:{action:"report",app_id:page.app.id,kind:reportKind.currentText,message:reportMessage.text}})}
                ActionButton { theme: page.theme;text:"Appeal as listing steward";visible:!!core.workspace.actor && core.distribution.distribution==="suspended";enabled:reportMessage.text.length>0 && reportMessage.text.length<=2000 && !core.loading;onClicked:core.workspaceAction("command",{command:"distribution",operation:{action:"appeal",app_id:page.app.id,message:reportMessage.text}})}
            }
            Label {visible:core.workspaceReply.command==="distribution" && core.workspaceReply.appId===page.app.id && !!core.workspaceReply.id;text:"Your private request was recorded. Reference: " + (core.workspaceReply.id || "");Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
            }
            Label { text: "Managed installation is not available in this preview."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
        }
        Item { Layout.preferredHeight: 28 }
    }
}
