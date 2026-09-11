import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id:panel
    required property var core
    required property var theme
    property var queue: ({})
    property string notice: ""
    readonly property var selected: (queue.apps || [])[apps.currentIndex] || ({})
    readonly property bool operator: (core.workspace.actor && (core.workspace.actor.roles || []).indexOf("operator")>=0) || false
    function refresh() {core.workspaceAction("monitor.queue",{});}
    function action(value) {core.workspaceAction("command",{command:"distribution",operation:value});}
    spacing:14
    Connections {
        target:core
        function onWorkspaceChanged() {
            if(!panel.operator) panel.queue={};
            const r=core.workspaceReply;
            if(r.error) {panel.notice=r.error.replace(/_/g," ");return;}
            if(r.action==="monitor.queue") panel.queue=r;
            else if(r.action==="monitor.sample") {panel.notice=r.notice;panel.refresh();}
            else if(r.action==="command" && r.command==="distribution") {panel.notice="Recorded with an audit trail.";panel.refresh();}
        }
    }
    Label {text:"Distribution operations";font.pixelSize:26*theme.scale;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {text:"Review upstream changes, private reports and author appeals. Suspensions affect distribution; they never delete installed applications.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {visible:!panel.operator;text:"An operator role is required to inspect private reports and change distribution status.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
    Flow {
        Layout.fillWidth:true;spacing:8
        Button {objectName:"loadMonitoringQueue";text:"Refresh operations";enabled:panel.operator && !core.loading;onClicked:panel.refresh()}
        Button {objectName:"sampleMonitoring";visible:!!core.workspace.sandbox;text:"Simulate upstream observations";enabled:panel.operator && !core.loading;onClicked:core.workspaceAction("monitor.sample",{})}
    }
    ComboBox {id:apps;objectName:"monitoredApplication";model:panel.queue.apps || [];textRole:"name";Layout.fillWidth:true;Accessible.name:"Application to inspect"}
    Label {text:panel.selected.appId ? "Observation: " + (panel.selected.error || (panel.selected.lastSuccessAt ? new Date(panel.selected.lastSuccessAt*1000).toLocaleString() : "not yet observed")) : "No delivered applications to monitor.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
    TextArea {id:reason;objectName:"monitoringReason";placeholderText:"Private reason and evidence reviewed";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.name:"Distribution decision reason"}
    CheckBox {id:reviewed;objectName:"monitoringAcknowledgement";text:"I reviewed the incident, current observation and relevant evidence";Accessible.name:text}
    Flow {
        Layout.fillWidth:true;spacing:8
        Button {objectName:"suspendDistribution";text:"Suspend distribution";enabled:panel.operator && !!panel.selected.appId && reason.text.length>0 && reason.text.length<=2000 && !core.loading;onClicked:panel.action({action:"suspend",app_id:panel.selected.appId,version:panel.selected.version,code:"operator_hold",reason:reason.text})}
        Button {objectName:"restoreDistribution";text:"Resolve and restore distribution";enabled:panel.operator && !!panel.selected.appId && reason.text.length>0 && reason.text.length<=2000 && reviewed.checked && !core.loading;onClicked:panel.action({action:"resolve",app_id:panel.selected.appId,version:panel.selected.version,reason:reason.text,confirm_reviewed:reviewed.checked})}
    }
    Label {text:"Active suspensions";font.bold:true}
    Repeater {model:panel.queue.holds || [];Label {required property var modelData;text:modelData.appId+" · "+modelData.code.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
    Label {text:"Upstream release candidates · compatibility unknown";font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
    Repeater {model:panel.queue.candidates || [];Label {required property var modelData;text:modelData.appId+" · "+modelData.version;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
    Label {text:"Private reports";font.bold:true}
    Repeater {
        model:panel.queue.reports || []
        ColumnLayout {
            required property var modelData
            Layout.fillWidth:true
            Label {text:modelData.appId+" · "+modelData.kind+" · "+modelData.message;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Button {text:"Close report with reason above";enabled:reason.text.length>0 && reason.text.length<=2000 && !core.loading;onClicked:panel.action({action:"close_report",id:modelData.id,reason:reason.text})}
        }
    }
    Label {text:"Author appeals";font.bold:true}
    Repeater {model:panel.queue.appeals || [];Label {required property var modelData;text:modelData.appId+" · "+modelData.message;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
}
