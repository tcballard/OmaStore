import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

ScrollView {
    id: page
    required property var worksheet
    required property var core
    required property var theme
    contentWidth: availableWidth
    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    readonly property var groups: [
        { title: "1. What are you making?", rows: [
            { key: "id", label: "Listing ID", hint: "lowercase-name" },
            { key: "name", label: "App name", hint: "Name people will recognise" },
            { key: "summary", label: "Purpose", hint: "The useful thing it helps someone do", max: 240 },
            { key: "description", label: "Description", hint: "What it does, who it helps, and its limits", max: 12000 },
            { key: "category", label: "Category", options: ["productivity", "writing", "development", "design", "media", "games", "utilities"] },
            { key: "appType", label: "App type", options: ["desktop", "terminal", "shell_plugin", "web", "service"] },
            { key: "maturity", label: "Maturity", options: ["unknown", "development", "preview", "stable"] },
            { key: "homepage", label: "App website", hint: "https://…" },
            { key: "support", label: "Support destination", hint: "https://…" }
        ]},
        { title: "2. Maker & licence", rows: [
            { key: "makerName", label: "Maker name", hint: "Person or organisation" },
            { key: "makerHomepage", label: "Maker website", hint: "https://…" },
            { key: "licence", label: "Licence class", options: ["unknown", "open_source", "source_available", "proprietary"] },
            { key: "licenceId", label: "Licence identifier", hint: "For example, MIT" },
            { key: "licenceUrl", label: "Licence terms", hint: "https://… (optional)" }
        ]},
        { title: "3. The exact release", rows: [
            { key: "version", label: "Upstream version", hint: "The author's version, exactly as published" },
            { key: "identity", label: "Release identity", options: ["source_commit", "binary_artifact", "repository_package"] },
            { key: "source", label: "Source repository", hint: "https://… (required for source commits)" },
            { key: "commit", label: "Full source commit", hint: "Full 40- or 64-character commit hash" },
            { key: "sha256", label: "Binary SHA-256", hint: "Required for publisher binary releases" },
            { key: "repository", label: "Package repository", hint: "Required for repository package identities" },
            { key: "package", label: "Package name", hint: "The package identity, never a shell command" },
            { key: "architecture", label: "Architecture", options: ["x86_64", "aarch64", "any"] },
            { key: "offline", label: "Works offline", options: ["unknown", "yes", "no", "optional"] },
            { key: "account", label: "Account required", options: ["unknown", "yes", "no", "optional"] },
            { key: "activation", label: "Activation required", options: ["unknown", "yes", "no", "optional"] },
            { key: "serviceCosts", label: "Additional service costs", hint: "State none, unknown, or the actual recurring costs" },
            { key: "removal", label: "Removal instructions", hint: "How to remove the app while preserving personal files" }
        ]},
        { title: "4. How can people support you?", rows: [
            { key: "model", label: "Offer model", options: ["free", "donation", "pay_what_you_want", "paid", "upgrade", "paid_features", "subscription", "service", "working_preview"] },
            { key: "offerUrl", label: "Developer or seller destination", hint: "https://…" },
            { key: "currency", label: "Currency", options: ["USD", "GBP", "EUR", "CAD", "AUD", "CHF", "JPY", "KWD"] },
            { key: "price", label: "Price, if supplied", hint: "For example, 19.99. Leave blank if not supplied." },
            { key: "billingInterval", label: "Subscription interval", options: ["", "month", "year"] },
            { key: "terms", label: "Offer terms", hint: "https://…" },
            { key: "refund", label: "Refund terms", hint: "https://… (optional)" },
            { key: "cancellation", label: "Cancellation terms", hint: "https://… (optional)" }
        ]}
    ]
    ColumnLayout {
        width: page.availableWidth
        spacing: 20
        Item { Layout.preferredHeight: 8 }
        ColumnLayout {
            Layout.fillWidth: true; Layout.leftMargin: 28; Layout.rightMargin: 28
            spacing: 16
            Label { text: "Bring something useful."; font.pixelSize: 32 * theme.scale; font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Prepare a listing on this device. Save your work, check the fields and review an export. Author sign-in and public submissions are still being built."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: "Free submissions. No OmaStore fee on external sales or support. Buying placement will not buy approval."; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { objectName: "worksheetStatus"; text: worksheet.status; color: theme.accent; wrapMode: Text.Wrap; Layout.fillWidth: true; Accessible.role: Accessible.StaticText }
            Flow {
                Layout.fillWidth: true; spacing: 10
                Button { objectName: "saveWorksheet"; text: "Save on this device"; enabled: worksheet.dirty; onClicked: worksheet.save() }
                Button { objectName: "checkWorksheet"; text: "Check fields"; enabled: core.ready && !core.loading; onClicked: worksheet.check() }
                Button { text: "Review export"; enabled: !!worksheet.result.valid; onClicked: exportPreview.open() }
                Button { text: "Reload saved"; onClicked: reloadDialog.open() }
            }
            Repeater {
                model: worksheet.result.errors || []
                Label { required property var modelData; text: modelData.path + ": " + modelData.code.replace(/_/g, " "); color: theme.ink; textFormat: Text.PlainText; wrapMode: Text.Wrap; Layout.fillWidth: true }
            }
            Repeater {
                model: page.groups
                ColumnLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: 12
                    Label { text: modelData.title; font.pixelSize: 22 * theme.scale; font.bold: true; Layout.topMargin: 20 }
                    Repeater {
                        model: modelData.rows
                        ColumnLayout {
                            id: row
                            required property var modelData
                            visible: (modelData.key !== "commit" || worksheet.fields.identity === "source_commit")
                                && (modelData.key !== "sha256" || worksheet.fields.identity === "binary_artifact")
                                && (["repository", "package"].indexOf(modelData.key) < 0 || worksheet.fields.identity === "repository_package")
                            Layout.fillWidth: true
                            spacing: 5
                            Label { text: row.modelData.label; font.bold: true; textFormat: Text.PlainText }
                            TextField {
                                objectName: "field-" + row.modelData.key
                                visible: !row.modelData.options
                                Layout.fillWidth: true
                                text: worksheet.fields[row.modelData.key] || ""
                                placeholderText: row.modelData.hint || ""
                                maximumLength: row.modelData.max || 2048
                                Accessible.name: row.modelData.label
                                onTextEdited: worksheet.setField(row.modelData.key, text)
                            }
                            ComboBox {
                                visible: !!row.modelData.options
                                Layout.fillWidth: true
                                model: row.modelData.options || []
                                displayText: currentText.replace(/_/g, " ")
                                currentIndex: Math.max(0, (row.modelData.options || []).indexOf(worksheet.fields[row.modelData.key] || ""))
                                Accessible.name: row.modelData.label
                                onActivated: worksheet.setField(row.modelData.key, currentText)
                            }
                        }
                    }
                }
            }
            Label { text: "This worksheet is a starting point. Real screenshots, capabilities, release notes, project-control verification and independent review are required before publication. Nothing here authorises installation."; color: theme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.topMargin: 12 }
        }
        Item { Layout.preferredHeight: 32 }
    }
    Dialog {
        id: reloadDialog
        parent: Overlay.overlay
        anchors.centerIn: Overlay.overlay
        width: Math.min(460, Math.max(320, Overlay.overlay.width - 32))
        modal: true
        title: "Reload the saved worksheet?"
        standardButtons: Dialog.Ok | Dialog.Cancel
        contentItem: Label { text: "This replaces the fields in this window with the last saved version. Unsaved edits will be discarded."; wrapMode: Text.Wrap }
        onAccepted: worksheet.reload()
    }
    Dialog {
        id: exportPreview
        parent: Overlay.overlay
        anchors.centerIn: Overlay.overlay
        width: Math.min(720, Math.max(320, Overlay.overlay.width - 32))
        height: Math.min(620, Math.max(240, Overlay.overlay.height - 32))
        modal: true
        title: "Review the candidate export"
        standardButtons: Dialog.Close
        ColumnLayout {
            anchors.fill: parent
            Label { text: "This file contains the listing fields shown below. It stays marked as a development candidate, with an unclaimed maker and no compatibility evidence. Export does not submit or publish it."; Layout.fillWidth: true; wrapMode: Text.Wrap }
            ScrollView { Layout.fillWidth: true; Layout.fillHeight: true; TextArea { text: worksheet.candidateJson; readOnly: true; wrapMode: TextEdit.Wrap; textFormat: TextEdit.PlainText; font.family: theme.mono; Accessible.name: "Candidate export contents" } }
            Button { text: "Choose export file…"; onClicked: exportFile.open() }
        }
    }
    FileDialog { id: exportFile; title: "Export listing candidate"; fileMode: FileDialog.SaveFile; defaultSuffix: "json"; nameFilters: ["JSON files (*.json)"]; onAccepted: worksheet.exportCandidate(selectedFile) }
}
