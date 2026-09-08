import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id:dialog
    required property var core
    required property var theme
    readonly property var plan:core.community["system.plan"] || ({})
    property string shownDigest:""
    readonly property var operation:!!plan.digest && (core.community["operations.status"] || {}).id===plan.digest?(core.community["operations.status"] || {}):({})
    readonly property string phase:operation.state || "planned"
    property real nowSeconds:Date.now()/1000
    readonly property bool canConfirm:!!operation.enabled && (phase==="planned" || (phase==="awaiting_user"&&!operation.claimed)) && plan.expiresAt>nowSeconds && !(plan.blockers || []).length && (plan.operations || []).some(o=>o.action==="install")
    property string systemKind:"update"
    parent:Overlay.overlay;anchors.centerIn:parent
    width:Math.min(800,parent?parent.width-32:800);height:Math.min(720,parent?parent.height-32:720)
    modal:true;title:plan.simulated?"Sample installation plan":"Review installation plan"
    standardButtons:Dialog.Close
    Connections {
        target:dialog.core
        function onCommunityChanged(){
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
            Label {objectName:"planNotice";text:dialog.phase==="planned"?(dialog.plan.notice || ""):(dialog.plan.simulated?"Sample operation · No host package changes":"The recorded package outcome appears below");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {text:((dialog.plan.host || {}).omarchyVersion || "Host not verified")+" · "+((dialog.plan.host || {}).architecture || "")+"\n"+((dialog.plan.host || {}).reason || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Repeater {
                model:dialog.plan.operations || []
                Frame {
                    required property var modelData
                    Layout.fillWidth:true
                    ColumnLayout {
                        anchors.fill:parent
                        Label {text:modelData.name+" · "+modelData.version;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {text:modelData.action.toUpperCase()+" · "+modelData.reason;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {visible:!!modelData.package;text:(modelData.repository || "")+" / "+(modelData.package || "")+(modelData.installedVersion?" · installed "+modelData.installedVersion:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {text:"Declared privileges: "+(modelData.privileges.length?modelData.privileges.join(", "):"none recorded")+"\nServices: "+(modelData.services.length?modelData.services.join(", "):"none recorded");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    }
                }
            }
            Label {visible:(dialog.plan.packages || []).length>0;text:"Resolved package effects";font.bold:true}
            Repeater {model:dialog.plan.packages || [];Label {required property var modelData;text:modelData.repository+" / "+modelData.name+" · "+modelData.version+" · "+modelData.downloadBytes+" download bytes";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Repeater {model:dialog.plan.blockers || [];Label {required property string modelData;text:modelData;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Label {text:"Proposal expires: "+new Date((dialog.plan.expiresAt || 0)*1000).toLocaleString()+(dialog.phase==="planned"?"\nNo package changes have been authorised.":"");Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {objectName:"operationState";text:dialog.phase==="running"&&dialog.operation.cancelRequested?"Finishing the current transaction":dialog.phase.replace(/_/g," ");font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!dialog.operation.enabled;text:dialog.operation.enablementNotice || "Checking execution availability…";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {model:dialog.phase!=="planned"?(dialog.operation.items || []):[];Label {required property var modelData;text:modelData.name+" · "+modelData.state.replace(/_/g," ")+(modelData.version?" · "+modelData.version:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Label {visible:(dialog.operation.items || []).some(o=>o.state==="manual");text:"External components remain manual steps. This does not mean the whole setup is installed.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!!(dialog.core.community["operations.confirm"] || {}).error;text:((dialog.core.community["operations.confirm"] || {}).error || "").replace(/_/g," ")+". Review a fresh proposal before continuing.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            CheckBox {id:consent;objectName:"planConsent";text:dialog.plan.simulated?"I approve this sample change":"I approve these package changes";enabled:dialog.canConfirm;Layout.fillWidth:true}
            Button {objectName:"confirmPlan";text:dialog.plan.simulated?"Run sample installation":"Continue in the system terminal";enabled:dialog.canConfirm&&consent.checked&&!dialog.core.loading;onClicked:dialog.core.communityAction("operations.confirm",{id:dialog.plan.digest,digest:dialog.plan.digest,accepted:true})}
            Button {text:dialog.phase==="running"?"Stop after the current transaction":"Cancel this operation";visible:dialog.phase==="planned"||dialog.phase==="awaiting_user"||dialog.phase==="running";enabled:!dialog.core.loading&&!dialog.operation.cancelRequested;onClicked:dialog.core.communityAction("operations.cancel",{id:dialog.plan.digest})}
            Flow {Layout.fillWidth:true;spacing:8;Button {text:"Open Omarchy's updater";onClicked:{dialog.systemKind="update";systemPrompt.open();}} Button {text:"Open system package chooser";onClicked:{dialog.systemKind="install";systemPrompt.open();}}}
            Button {objectName:"closePlan";text:"Done";onClicked:dialog.close()}
            Label {text:"Plan: "+(dialog.plan.digest || "");font.family:dialog.theme.mono;font.pixelSize:10*dialog.theme.scale;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
        }
    }
    Dialog {id:systemPrompt;implicitHeight:240*dialog.theme.scale;height:Math.min(implicitHeight,parent?parent.height-32:implicitHeight);parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:dialog.systemKind==="update"?"Open Omarchy's updater":"Open the system package chooser";standardButtons:Dialog.Ok|Dialog.Cancel;width:Math.min(560,parent?parent.width-32:560);contentItem: Label {text:dialog.plan.simulated?"This will rehearse the handoff without opening a system tool.":"The normal system tool will open in a separate terminal and own any package changes. Refresh the library when it finishes.";wrapMode:Text.Wrap} onAccepted:dialog.core.communityAction("system.handoff",{kind:dialog.systemKind,confirmed:true})}
}
