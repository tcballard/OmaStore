import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: page
    required property var core
    required property var theme
    required property var worksheet
    property bool requested: false
    readonly property var account: core.workspace.actor || ({})
    readonly property bool signedIn: !!account.id
    property var claim: ({})
    function refresh() { requested = true; core.workspaceAction("state", {}); }
    function message(code) {
        const messages = {"new_publisher_intake_paused":"New publisher submissions are paused while the review queue recovers. Your draft remains saved.","stale_monitor_status":"An observation or operator changed this record. Refresh before deciding.","fresh_upstream_observation_required":"Refresh upstream evidence before restoring distribution.","artifact_mismatch_unresolved":"The observed artifact digest still differs. Distribution remains suspended.","listing_steward_required":"Only the recorded listing steward can appeal here; you can still send a private report.","delivery_in_progress":"Delivery is being reconciled. Refresh its status before withdrawing.","publication_busy":"The publication worker is active. Refresh status before retrying.","sign_in_unconfigured":"GitHub sign-in is not configured for this author service yet.","workspace_unconfigured":"The author service is not configured. You can still prepare and export a local worksheet.","workspace_unavailable":"Could not reach the author service. Your local worksheet is still available.","sign_in_required":"Your session has expired. Sign in again to continue.","claim_proof_mismatch":"The published proof does not match this account, project and challenge.","claim_challenge_unavailable":"This challenge expired or was already used. Create a new one.","sample_claim_has_no_public_proof":"Sample mode does not claim real projects. You can inspect the challenge format here.","browser_unavailable":"The system browser could not open. Check your default browser and try again.","role_required":"This account no longer has the role needed for that action.","rate_limited":"Too many requests. Wait a minute, then try again.","candidate_invalid":"Some required listing facts are missing or invalid. Review the exact preview for field errors.","stale_revision":"This record changed. Your edits remain on this device; compare the server copy before syncing.","media_count_exceeded":"This draft has reached the limit for that media kind.","candidate_cannot_self_verify":"Compatibility tests and control claims are assigned during independent review."};
        return messages[code] || "The action could not be completed. Refresh the workspace before retrying.";
    }
    Component.onCompleted: if (core.ready) refresh()
    Connections {
        target: core
        function onStateChanged() { if (core.ready && !page.requested) page.refresh(); }
        function onWorkspaceChanged() { if (core.workspaceReply.action === "claims.start" && core.workspaceReply.id) page.claim = core.workspaceReply; }
    }
    Timer { interval: 3000; repeat: true; running: !!core.workspace.signingIn; onTriggered: if (!core.loading) core.workspaceAction("auth.poll", {}) }
    TabBar {
        id: tabs
        Layout.fillWidth: true
        TabButton { objectName: "authorWorkspaceTab"; text: "Author workspace" }
        TabButton { objectName: "localWorksheetTab"; text: "Local worksheet" }
        TabButton { objectName: "reviewWorkspaceTab"; text: "Review" }
        TabButton {objectName:"operationsWorkspaceTab";text:"Operations"}
        TabButton {objectName:"readinessWorkspaceTab";text:"Readiness"}
    }
    StackLayout {
        currentIndex: tabs.currentIndex
        Layout.fillWidth: true
        Layout.fillHeight: true
        ScrollView {
            id: scroll
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: scroll.availableWidth
                spacing: 16
                Pane {
                    Layout.fillWidth: true
                    padding: 22
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 12
                        Label { text: page.signedIn ? "Your author workspace" : "Bring your software to OmaStore"; font.bold: true; font.pixelSize: 26 * theme.scale; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        Label { text: "Prepare a listing, explain its limits, and follow it through independent review. Listing and normal updates are free."; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        Label { visible: !!core.workspaceReply.error; text: page.message(core.workspaceReply.error); Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText; Accessible.role: Accessible.AlertMessage }
                        Label { visible: core.workspace.configured === false; text: "This preview has no author service configured. Use Local worksheet to prepare and export your listing."; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        Label { visible: core.workspace.storage === "session_only"; text: "Signed in for this session. The desktop keyring could not save your login."; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        RowLayout {
                            Layout.fillWidth: true
                            Label { text: page.signedIn ? (page.account.login + " · " + (page.account.roles || []).join(", ")) : "Not signed in"; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                            Button { text: "Refresh"; enabled: core.ready && !core.loading; onClicked: page.refresh() }
                            Button { visible: page.signedIn; text: "Sign out"; enabled: !core.loading; onClicked: core.workspaceAction("auth.logout", {}) }
                        }
                        Button { visible: !page.signedIn && !core.workspace.sandbox; text: core.workspace.signingIn ? "Waiting for GitHub…" : "Sign in with GitHub"; enabled: !!core.workspace.signInConfigured && !core.loading && !core.workspace.signingIn; onClicked: core.workspaceAction("auth.start", {}) }
                        Label { visible: !!core.workspace.sandbox; text: "Sample workspace · Fictional identities and private local records. Nothing here is published."; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        RowLayout {
                            visible: !!core.workspace.sandbox
                            Layout.fillWidth: true
                            ComboBox { id: sampleRole; objectName: "sampleRole"; model: ["author", "reviewer", "second-reviewer", "maintainer", "operator", "editor"]; Layout.fillWidth: true; Accessible.name: "Sample workspace role" }
                            Button { objectName: "sampleSignIn"; text: "Try this role"; enabled: !core.loading; onClicked: core.workspaceAction("auth.sandbox", {"name": sampleRole.currentText}) }
                        }
                    }
                }
                Pane {
                    visible: page.signedIn
                    Layout.fillWidth: true
                    padding: 22
                    DraftPanel { anchors.fill: parent; core: page.core; theme: page.theme; worksheet: page.worksheet }
                }
                Pane {
                    visible: page.signedIn
                    Layout.fillWidth: true
                    padding: 22
                    ColumnLayout {
                        anchors.fill: parent
                        spacing: 12
                        Label { text: "Project control"; font.bold: true; font.pixelSize: 22 * theme.scale }
                        Label { text: "Signing in identifies you. A separate published challenge proves control of a repository or publisher domain."; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        TextField { id: target; objectName: "claimTarget"; placeholderText: "https://github.com/owner/project or https://publisher.example"; maximumLength: 2048; Layout.fillWidth: true; Accessible.name: "Project or publisher origin" }
                        Button { text: "Create control challenge"; enabled: !core.loading && target.text.length > 0; onClicked: core.workspaceAction("claims.start", {"target": target.text}) }
                        Label { visible: !!page.claim.id; text: "Publish the following JSON at " + (page.claim.proofUrl || ""); Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                        TextArea { id: proof; visible: !!page.claim.id; readOnly: true; text: page.claim.proof ? JSON.stringify(page.claim.proof, null, 2) : ""; font.family: theme.mono; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText; Accessible.name: "Control challenge JSON" }
                        RowLayout {
                            visible: !!page.claim.id
                            Button { text: "Copy challenge"; onClicked: { proof.selectAll(); proof.copy(); } }
                            Button { text: "Check published proof"; enabled: !core.loading; onClicked: core.workspaceAction("claims.verify", {"id": page.claim.id}) }
                        }
                        Repeater {
                            model: core.workspace.claims || []
                            RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                Label { text: modelData.target + " · " + (modelData.active ? "Control verified" : "Expired or revoked"); Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                                Button { visible: modelData.active; text: "Revoke"; enabled: !core.loading; onClicked: core.workspaceAction("claims.revoke", {"target": modelData.target}) }
                            }
                        }
                    }
                }
            }
        }
        SubmitPage { core: page.core; theme: page.theme; worksheet: page.worksheet }
        ScrollView {id:reviewScroll;clip:true;contentWidth:availableWidth;ReviewPanel {width:reviewScroll.availableWidth;core:page.core;theme:page.theme}}
        ScrollView {id:operationsScroll;clip:true;contentWidth:availableWidth;MonitoringPanel {width:operationsScroll.availableWidth;core:page.core;theme:page.theme}}
        ScrollView {id:readinessScroll;clip:true;contentWidth:availableWidth;ReadinessPanel {width:readinessScroll.availableWidth;core:page.core;theme:page.theme}}
    }
}
