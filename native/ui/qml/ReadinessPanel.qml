import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id:panel
    required property var core
    required property var theme
    property var report:({})
    property string notice:""
    readonly property bool operator:!!core.workspace.actor && (core.workspace.actor.roles || []).indexOf("operator")>=0
    spacing:14
    function refresh(){core.workspaceAction("operations.dashboard",{});}
    Connections {target:panel.core;function onWorkspaceChanged(){
        if(!panel.operator)panel.report=({});
        const r=panel.core.workspaceReply;
        if(r.error){panel.notice=r.error.replace(/_/g," ");return;}
        if(r.action==="operations.dashboard"){if(r.error)panel.notice=r.error.replace(/_/g," ");else panel.report=r;}
        if(r.action==="command"&&r.command==="release_evidence"){panel.notice=r.notice || (r.error || "").replace(/_/g," ");ack.checked=false;panel.refresh();}
    }}
    Label {text:"Ready to operate?";font.pixelSize:26*theme.scale;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {text:"Review service health and the reports needed for a real pilot. A passing rehearsal does not stand in for independent authors or an Omarchy desktop run.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {visible:!panel.operator;text:"The operator role is required to inspect readiness evidence.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Button {objectName:"loadReadiness";text:"Refresh readiness";enabled:panel.operator&&!core.loading;onClicked:panel.refresh()}
    Label {objectName:"readinessEnvironment";text:panel.report.environment?"Evidence environment: "+panel.report.environment:"No report loaded";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {text:panel.report.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
    Label {text:panel.report.expansionPaused?"New publisher intake is paused after consecutive weekly breaches.":"Expansion pause: "+(panel.report.environment?"not recorded":"unknown");Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {text:"Oldest unanswered submission: "+(panel.report.oldestWorkingDays || 0)+" working days"+(panel.report.operatorAlert?" · operator attention required":"")+"\n"+(panel.report.calendar || "");Layout.fillWidth:true;wrapMode:Text.Wrap}
    Repeater {model:Object.keys(panel.report.counts || {});Label {required property string modelData;text:modelData.replace(/([A-Z])/g," $1")+": "+panel.report.counts[modelData];Layout.fillWidth:true;wrapMode:Text.Wrap}}
    Label {text:"Release evidence";font.bold:true}
    Repeater {model:panel.report.gates || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent;Label {text:modelData.gate.replace(/_/g," ")+" · "+modelData.outcome;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText} TextField {visible:!!modelData.reportUrl;text:modelData.reportUrl || "";readOnly:true;selectByMouse:true;Layout.fillWidth:true;Accessible.name:"Evidence report address"}}}}
    Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
    ComboBox {id:gate;model:panel.report.gates || [];textRole:"gate";Layout.fillWidth:true;Accessible.name:"Release gate"}
    ComboBox {id:outcome;model:["pending","failed","passed"];Layout.fillWidth:true;Accessible.name:"Evidence outcome"}
    TextField {id:reportUrl;placeholderText:"HTTPS report address";maximumLength:2048;Layout.fillWidth:true;Accessible.name:placeholderText}
    TextField {id:reportDigest;placeholderText:"Report SHA-256";maximumLength:64;Layout.fillWidth:true;Accessible.name:placeholderText}
    CheckBox {id:ack;text:"I reviewed this report and its actual environment";enabled:panel.operator;Layout.fillWidth:true}
    Button {text:"Record operator evidence";enabled:panel.operator&&ack.checked&&gate.currentIndex>=0&&reportUrl.text.length>0&&reportDigest.text.length===64&&!core.loading;onClicked:core.workspaceAction("command",{command:"release_evidence",evidence:{gate:panel.report.gates[gate.currentIndex].gate,outcome:outcome.currentText,report_url:reportUrl.text,report_digest:reportDigest.text}})}
    Button {text:"Open OmaStore's issue tracker";onClicked:Qt.openUrlExternally("https://github.com/tcballard/OmaStore/issues")}
}
