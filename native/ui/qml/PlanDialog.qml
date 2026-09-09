import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Dialog {
    id:dialog
    required property var core
    required property var theme
    readonly property var plan:core.community["system.plan"] || ({})
    property string shownDigest:""
    property bool advanced: false
    readonly property var operation:!!plan.digest && (core.community["operations.status"] || {}).id===plan.digest?(core.community["operations.status"] || {}):({})
    readonly property string phase:operation.state || "planned"
    property real nowSeconds:Date.now()/1000
    readonly property bool canConfirm:!!operation.enabled && (phase==="planned" || (phase==="awaiting_user"&&!operation.claimed)) && plan.expiresAt>nowSeconds && !(plan.blockers || []).length && (plan.operations || []).some(o=>o.action==="install" || o.action==="remove")
    property string systemKind:"update"
    property bool awaitingDiagnostics:false
    readonly property var diagnostics:core.community["operations.diagnostics"] || ({})
    readonly property bool removing:(plan.selection || {}).kind==="remove"
    parent:Overlay.overlay;anchors.centerIn:parent
    width:Math.min(720,parent?parent.width-32:720);height:Math.min((plan.blockers || []).length && !advanced ? 520 : 720,parent?parent.height-32:720)
    padding:24
    background: Rectangle { color: dialog.theme.page; radius: dialog.theme.radius; border.color: dialog.theme.line }
    modal:true;title:removing?(plan.simulated?"Sample removal plan":"Review package removal"):(plan.simulated?"Sample installation plan":"Review installation plan")
    standardButtons:Dialog.NoButton
    header: Label { text:dialog.title; padding:24; bottomPadding:12; color:dialog.theme.ink; font.pixelSize:18*dialog.theme.scale; font.weight:Font.DemiBold }
    footer: Pane {
        padding:16
        background:Rectangle {color:dialog.theme.page;Rectangle {width:parent.width;height:1;color:dialog.theme.line}}
        RowLayout {
            anchors.fill:parent
            Label {text:dialog.plan.simulated?"Sample operation": "Changes require your approval";color:dialog.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap;font.pixelSize:11*dialog.theme.scale}
            ActionButton {theme:dialog.theme;objectName:"closePlan";text:"Done";onClicked:dialog.close()}
        }
    }
    Connections {
        target:dialog.core
        function onCommunityChanged(){
            if(dialog.awaitingDiagnostics && (dialog.diagnostics.document || {}).operationId===dialog.plan.digest){dialog.awaitingDiagnostics=false;diagnosticPreview.open();}
            if(!dialog.plan.digest)dialog.shownDigest="";
            if(dialog.plan.digest && dialog.plan.digest!==dialog.shownDigest){
                dialog.shownDigest=dialog.plan.digest;consent.checked=false;dialog.nowSeconds=Date.now()/1000;
                dialog.core.communityAction("operations.status",{id:dialog.plan.digest});dialog.open();
            }
        }
    }
    Timer {interval:750;running:dialog.visible;repeat:true;onTriggered:{dialog.nowSeconds=Date.now()/1000;if(dialog.plan.digest&&!dialog.core.loading&&(dialog.phase==="awaiting_user"||dialog.phase==="running"))dialog.core.communityAction("operations.status",{id:dialog.plan.digest});}}
    ScrollView {
        anchors.fill:parent;contentWidth:availableWidth;clip:true
        ColumnLayout {
            width:parent.width;spacing:16
            Pane {
                Layout.fillWidth:true; padding:18
                background: Rectangle { color:dialog.theme.wash; radius:dialog.theme.radius }
                ColumnLayout {
                    width:parent.width;spacing:7
                    Label { text:(dialog.plan.blockers || []).length ? "Installation needs attention" : dialog.phase === "planned" ? "Your changes, before they happen" : "Package progress"; color:dialog.theme.ink; font.pixelSize:22*dialog.theme.scale; font.weight:Font.DemiBold; Layout.fillWidth:true;wrapMode:Text.Wrap }
                    Label { text:(dialog.plan.blockers || []).length ? "Nothing has been installed. Review the requirements below, or return to the app." : "Review the selected packages and their effects. Changes require your approval."; color:dialog.theme.muted; Layout.fillWidth:true;wrapMode:Text.Wrap }
                }
            }
            Label {visible:dialog.plan.simulated || !(dialog.plan.blockers || []).length;objectName:"planNotice";text:dialog.phase==="planned"?(dialog.plan.notice || ""):(dialog.plan.simulated?"Sample operation · No host package changes":"The recorded package outcome appears below");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:dialog.advanced;text:((dialog.plan.host || {}).omarchyVersion || "Host not verified")+" · "+((dialog.plan.host || {}).architecture || "")+"\n"+((dialog.plan.host || {}).reason || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Repeater {
                model:dialog.plan.operations || []
                Frame {
                    required property var modelData
                    padding:16
                    background:Rectangle {color:dialog.theme.surface;radius:dialog.theme.radius;border.color:dialog.theme.line}
                    Layout.fillWidth:true
                    ColumnLayout {
                        anchors.fill:parent;spacing:10
                        Label {font.pixelSize:16*dialog.theme.scale;text:modelData.name+" · "+modelData.version;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {visible:!(dialog.plan.blockers || []).length;text:modelData.action.toUpperCase()+" · "+modelData.reason;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {visible:!!modelData.package;text:(modelData.repository || "")+" / "+(modelData.package || "")+(modelData.installedVersion?" · installed "+modelData.installedVersion:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {visible:dialog.advanced;text:"Declared privileges: "+(modelData.privileges.length?modelData.privileges.join(", "):"none recorded")+"\nServices: "+(modelData.services.length?modelData.services.join(", "):"none recorded");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {visible:!!modelData.disclosure && (dialog.advanced || !(dialog.plan.blockers || []).length);text:((modelData.disclosure || {}).evidence || "")+"\nAccounts: "+((modelData.disclosure || {}).account || "not part of this operation")+" · activation: "+((modelData.disclosure || {}).activation || "not part of this operation")+"\n"+((modelData.disclosure || {}).serviceCosts || "")+"\n"+((modelData.disclosure || {}).removal || (modelData.disclosure || {}).restoration || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}

                    }
                }
            }
            Label {visible:(dialog.plan.packages || []).length>0;text:"Resolved package effects";font.bold:true}
            Repeater {model:dialog.plan.packages || [];Label {required property var modelData;text:modelData.repository+" / "+modelData.name+" · "+modelData.version+" · "+modelData.downloadBytes+" download bytes";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Label {visible:(dialog.plan.blockers || []).length>0;text:"Before you can continue";font.weight:Font.DemiBold;color:dialog.theme.ink}
            Repeater {model:dialog.plan.blockers || [];Label {required property string modelData;text:"• " + modelData.replace(/^([^:]+):/, function(match,id) {const item=(dialog.plan.operations || []).find(o=>o.appId===id || o.id===id);return item ? item.name + ":" : "";});Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Label {visible:!(dialog.plan.blockers || []).length || dialog.advanced;text:"Proposal expires: "+new Date((dialog.plan.expiresAt || 0)*1000).toLocaleString()+(dialog.phase==="planned"?"\nNo package changes have been authorised.":"");Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:dialog.phase!=="planned";objectName:"operationState";text:dialog.phase==="running"&&dialog.operation.cancelRequested?"Finishing the current transaction":dialog.phase.replace(/_/g," ");font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!dialog.operation.enabled;text:!dialog.plan.simulated && !dialog.advanced ? "Installing through OmaStore is not enabled in this preview. You can save the app and return to it later." : dialog.operation.enablementNotice || "Checking execution availability…";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {model:dialog.phase!=="planned"?(dialog.operation.items || []):[];Label {required property var modelData;text:modelData.name+" · "+modelData.state.replace(/_/g," ")+(modelData.version?" · "+modelData.version:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Label {visible:(dialog.operation.items || []).some(o=>o.state==="manual");text:"External components remain manual steps. This does not mean the whole setup is installed.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!!(dialog.core.community["operations.confirm"] || {}).error;text:((dialog.core.community["operations.confirm"] || {}).error || "").replace(/_/g," ")+". Review a fresh proposal before continuing.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            CheckBox {visible:!(dialog.plan.blockers || []).length;id:consent;objectName:"planConsent";text:dialog.plan.simulated?"I approve this sample change":"I approve these package changes";enabled:dialog.canConfirm;Layout.fillWidth:true}
            ActionButton {theme:dialog.theme;primary:true;visible:!(dialog.plan.blockers || []).length;objectName:"confirmPlan";text:dialog.plan.simulated?(dialog.removing?"Run sample removal":"Run sample installation"):"Continue in the system terminal";enabled:dialog.canConfirm&&consent.checked&&!dialog.core.loading;onClicked:dialog.core.communityAction("operations.confirm",{id:dialog.plan.digest,digest:dialog.plan.digest,accepted:true})}
            Button {text:dialog.phase==="running"?"Stop after the current transaction":"Cancel this operation";visible:dialog.phase==="awaiting_user"||dialog.phase==="running";enabled:!dialog.core.loading&&!dialog.operation.cancelRequested;onClicked:dialog.core.communityAction("operations.cancel",{id:dialog.plan.digest})}
            ActionButton {theme:dialog.theme;text:dialog.advanced?"Hide package details":"Package details & system tools";onClicked:dialog.advanced=!dialog.advanced}
            Flow {visible:dialog.advanced;Layout.fillWidth:true;spacing:8;Button {text:"Open Omarchy's updater";onClicked:{dialog.systemKind="update";systemPrompt.open();}} Button {text:"Open system package chooser";onClicked:{dialog.systemKind="install";systemPrompt.open();}}}
            Flow {visible:dialog.advanced || dialog.phase!=="planned";Layout.fillWidth:true;spacing:8;Button {objectName:"reconcileOperation";text:"Reconcile package state";visible:dialog.phase==="unknown";enabled:!dialog.core.loading;onClicked:dialog.core.communityAction("operations.reconcile",{id:dialog.plan.digest})} Button {objectName:"replanOperation";text:"Review a fresh plan";enabled:!!dialog.plan.digest&&!dialog.core.loading&&dialog.phase!=="running"&&!dialog.operation.workerActive;onClicked:dialog.core.communityAction("operations.replan",{id:dialog.plan.digest})} Button {objectName:"previewDiagnostics";text:"Preview diagnostics";enabled:!!dialog.plan.digest&&!dialog.core.loading;onClicked:{dialog.awaitingDiagnostics=true;dialog.core.communityAction("operations.diagnostics",{id:dialog.plan.digest});}}}
            Button {text:"Keep observed components as a setup";visible:(dialog.plan.selection || {}).kind==="setup";enabled:!dialog.core.loading&&dialog.plan.expiresAt>dialog.nowSeconds;onClicked:dialog.core.communityAction("library.remember_setup",{id:dialog.plan.digest})}
            Label {visible:dialog.advanced;text:"Plan: "+(dialog.plan.digest || "");font.family:dialog.theme.mono;font.pixelSize:10*dialog.theme.scale;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
        }
    }
    Dialog {id:systemPrompt;implicitHeight:240*dialog.theme.scale;height:Math.min(implicitHeight,parent?parent.height-32:implicitHeight);parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:dialog.systemKind==="update"?"Open Omarchy's updater":"Open the system package chooser";standardButtons:Dialog.Ok|Dialog.Cancel;width:Math.min(560,parent?parent.width-32:560);contentItem: Label {text:dialog.plan.simulated?"This will rehearse the handoff without opening a system tool.":"The normal system tool will open in a separate terminal and own any package changes. Refresh the library when it finishes.";wrapMode:Text.Wrap} onAccepted:dialog.core.communityAction("system.handoff",{kind:dialog.systemKind,confirmed:true})}
    Dialog {
        id:diagnosticPreview;parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Diagnostic export preview";standardButtons:Dialog.Close
        width:Math.min(720,parent?parent.width-32:720);height:Math.min(600,parent?parent.height-32:600)
        ColumnLayout {anchors.fill:parent;Label {text:"Review this exact report before saving or sharing it.";Layout.fillWidth:true;wrapMode:Text.Wrap} ScrollView {Layout.fillWidth:true;Layout.fillHeight:true;TextArea {objectName:"diagnosticDocument";text:JSON.stringify(dialog.diagnostics.document || {},null,2);readOnly:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}} Button {objectName:"saveDiagnostics";text:"Choose export file";onClicked:diagnosticFile.open()} Button {objectName:"closeDiagnostics";text:"Done";onClicked:diagnosticPreview.close()}}
    }
    FileDialog {id:diagnosticFile;title:"Save diagnostic report";fileMode:FileDialog.SaveFile;nameFilters:["Diagnostic report (*.json)"];defaultSuffix:"json";onAccepted:dialog.core.communityAction("operations.diagnostics.export",{id:dialog.plan.digest,digest:dialog.diagnostics.digest,file:selectedFile.toString()})}

}
