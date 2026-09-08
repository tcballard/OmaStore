import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id:dialog
    required property var core
    required property var theme
    readonly property var plan:core.community["system.plan"] || ({})
    property string shownDigest:""
    parent:Overlay.overlay;anchors.centerIn:parent
    width:Math.min(800,parent?parent.width-32:800);height:Math.min(720,parent?parent.height-32:720)
    modal:true;title:plan.simulated?"Sample installation plan":"Review installation plan"
    standardButtons:Dialog.Close
    Connections {
        target:dialog.core
        function onCommunityChanged(){if(dialog.plan.digest && dialog.plan.digest!==dialog.shownDigest){dialog.shownDigest=dialog.plan.digest;dialog.open();}}
    }
    ScrollView {
        anchors.fill:parent;contentWidth:availableWidth;clip:true
        ColumnLayout {
            width:parent.width;spacing:16
            Label {objectName:"planNotice";text:dialog.plan.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
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
            Label {text:"Proposal expires: "+new Date((dialog.plan.expiresAt || 0)*1000).toLocaleString()+"\nNo package changes have been authorised.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Button {objectName:"closePlan";text:"Done";onClicked:dialog.close()}
            Label {text:"Plan: "+(dialog.plan.digest || "");font.family:dialog.theme.mono;font.pixelSize:10*dialog.theme.scale;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
        }
    }
}
