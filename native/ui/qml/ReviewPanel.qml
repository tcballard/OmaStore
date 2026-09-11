import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

ColumnLayout {
    id: panel
    required property var core
    required property var theme
    property var queue: []
    property var selected: ({})
    property string account: ""
    property string notice: ""
    property string previewUrl: ""
    property string previewAlt: ""
    readonly property bool reviewer: ((core.workspace.actor || {}).roles || []).indexOf("reviewer") >= 0
    onReviewerChanged: if(!reviewer) {queue=[];selected={};previewUrl="";}
    function refresh() {if(reviewer)core.workspaceAction("review.queue",{});}
    function open(id) {core.workspaceAction("review.get",{id:id});}
    function decide(decision) {core.workspaceAction("command",{command:"review_decision",id:selected.id,version:selected.version,decision:decision,reason:reason.text,acknowledge_limits:ack.checked});}
    function error(code) {
        const messages={independent_reviewer_required:"Your relationship to this project prevents approval. Choose an independent reviewer.",required_checks_missing:"A required check has not passed. Inspect findings and retry after correcting the cause.",runtime_evidence_missing:"Independent runtime evidence is missing or no longer current.",stale_revision:"Another action changed this revision. Reload it before deciding.",role_required:"This account no longer has the required role.",evidence_invalid:"The report does not match the declared release identity or evidence schema.",evidence_candidate_mismatch:"The report belongs to different candidate content."};
        return messages[code] || "The action was not completed. Check the current revision and try again.";
    }
    Connections {
        target:core
        function onWorkspaceChanged() {
            const current=(core.workspace.actor || {}).id || "";
            if(current !== panel.account) {panel.account=current;panel.queue=[];panel.selected={};panel.notice="";reason.text="";ack.checked=false;}
            const r=core.workspaceReply;
            if(r.error) {panel.notice=panel.error(r.error);return;}
            if(r.action === "review.queue") panel.queue=r.items || [];
            else if(r.action === "media.preview") {if(r.contentType === "image/png") {panel.previewUrl=r.url;mediaDialog.open();}else panel.notice="Opened the normalised demo in your native video player.";}
            else if(r.action === "review.get") {panel.selected=r;ack.checked=false;panel.notice="";}
            else if(r.action === "review.sample_evidence" || r.action === "evidence.import") {if(panel.selected.id)panel.open(panel.selected.id);}
            else if(r.action === "command" && r.command === "review_decision") {panel.notice="Decision recorded · " + r.state.replace(/_/g," ");panel.open(r.id);}
            else if(r.action === "command" && (r.command === "retry_checks" || r.command === "start_review")) panel.open(r.id);
        }
    }
    RowLayout {
        Layout.fillWidth:true
        Label {text:"Independent review";font.bold:true;font.pixelSize:22*theme.scale;Layout.fillWidth:true}
        Button {objectName:"loadReviewQueue";text:"Load queue";enabled:panel.reviewer && !core.loading;onClicked:panel.refresh()}
    }
    Label {text:"First response target: three working days, Monday–Friday in Europe/London. Public holidays are not yet modelled. Payments do not affect priority.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {visible:!panel.reviewer;text:"A reviewer role is required. In the sample workspace, switch to reviewer or second-reviewer to try this flow.";Layout.fillWidth:true;wrapMode:Text.Wrap}
    Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.role:Accessible.AlertMessage}
    Repeater {
        model:panel.queue
        Frame {
            required property var modelData
            Layout.fillWidth:true
            RowLayout {
                anchors.fill:parent
                ColumnLayout {
                    Layout.fillWidth:true
                    Label {text:modelData.name;Layout.fillWidth:true;font.bold:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    Label {text:modelData.state.replace(/_/g," ")+" · "+modelData.workingDays+" working days · "+modelData.requiredReviewers+" reviewer(s)";Layout.fillWidth:true;wrapMode:Text.Wrap}
                    Label {visible:!modelData.independent;text:"Project relationship · independent reviewer required";Layout.fillWidth:true;wrapMode:Text.Wrap}
                }
                Button {objectName:"inspectReview";text:"Inspect";enabled:!core.loading;onClicked:panel.open(modelData.id)}
            }
        }
    }
    Frame {
        visible:!!panel.selected.id
        Layout.fillWidth:true
        ColumnLayout {
            anchors.fill:parent
            spacing:12
            Label {text:"Revision under review";font.bold:true;font.pixelSize:22*theme.scale}
            Label {text:(panel.selected.state || "").replace(/_/g," ")+" · "+(panel.selected.requiredReviewers || 1)+" independent reviewer(s) required";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {text:"Candidate digest: "+(panel.selected.digest || "");Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
            Label {text:"Findings";font.bold:true}
            Repeater {model:panel.selected.findings || [];Label {required property var modelData;text:modelData.check+" · "+modelData.result+"\n"+modelData.detail;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Button {text:"Start independent review";enabled:!!panel.selected.independent && !core.loading && ["submitted","checking","in_review"].indexOf(panel.selected.state)>=0;onClicked:core.workspaceAction("command",{command:"start_review",id:panel.selected.id,version:panel.selected.version})}
            Button {text:"Retry bounded checks";enabled:!core.loading && ["in_review","needs_changes"].indexOf(panel.selected.state)>=0;onClicked:core.workspaceAction("command",{command:"retry_checks",id:panel.selected.id,version:panel.selected.version})}
            Label {text:"Changes from the previous submitted revision";font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {model:panel.selected.diff || [];Label {required property var modelData;text:(modelData.path || "Whole candidate")+" · "+modelData.change+(modelData.beforePreview ? "\nBefore: "+modelData.beforePreview : "")+(modelData.afterPreview ? "\nAfter: "+modelData.afterPreview : "");Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}}
            Button {text:"Inspect exact candidate";onClicked:candidateDialog.open()}
            Repeater {
                model:panel.selected.media || []
                ColumnLayout {
                    required property var modelData
                    Layout.fillWidth:true
                    Label {text:modelData.kind+" · "+modelData.alt;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    Label {text:"Rights: "+modelData.rights;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    Button {text:"Preview private media";enabled:!core.loading;onClicked:{panel.previewAlt=modelData.alt;core.workspaceAction("media.preview",{id:modelData.id,sha256:modelData.sha256});}}
                }
            }
            Label {text:"Runtime evidence";font.bold:true}
            Label {visible:!(panel.selected.runtimeEvidence || []).length;text:"No independent runtime report has been imported. Automated checks do not establish compatibility.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {model:panel.selected.runtimeEvidence || [];Label {required property var modelData;text:modelData.record.result+" · "+modelData.record.environment+"\n"+modelData.record.testedAt+" · "+modelData.record.actor+"\nExecuted bytes: "+modelData.record.executedSha256+"\n"+modelData.record.limitations;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}}
            Button {text:"Import VM evidence JSON";enabled:!!panel.selected.independent && !core.loading;onClicked:evidenceFile.open()}
            Button {objectName:"sampleEvidence";visible:!!core.workspace.sandbox;text:"Add explicitly simulated observations";enabled:!!panel.selected.independent && !core.loading;onClicked:core.workspaceAction("review.sample_evidence",{id:panel.selected.id})}
            Label {text:"Decision history";font.bold:true}
            Repeater {model:panel.selected.decisions || [];Label {required property var modelData;text:modelData.actor+" · "+modelData.decision+"\n"+modelData.reason;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            TextArea {id:reason;objectName:"reviewReason";placeholderText:"Explain the decision, evidence and remaining limits";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.name:"Review decision reason"}
            CheckBox {id:ack;objectName:"reviewAcknowledgement";text:"I reviewed the exact candidate, rights, routes and limits";Accessible.name:text}
            Flow {
                Layout.fillWidth:true
                spacing:8
                Button {text:"Request changes";enabled:!!panel.selected.independent && reason.text.length>0 && !core.loading;onClicked:panel.decide("needs_changes")}
                Button {text:"Reject";enabled:!!panel.selected.independent && reason.text.length>0 && !core.loading;onClicked:panel.decide("reject")}
                Button {objectName:"approveReview";text:"Approve exact revision";enabled:!!panel.selected.independent && ack.checked && reason.text.length>0 && !core.loading;onClicked:panel.decide("approve")}
            }
        }
    }
    Dialog {
        id:mediaDialog
        parent:Overlay.overlay
        title:"Private media preview"
        modal:true
        width:Math.min(760,parent ? parent.width-32 : 760)
        height:Math.min(620,parent ? parent.height-32 : 620)
        anchors.centerIn:parent
        standardButtons:Dialog.Close
        ColumnLayout {
            anchors.fill:parent
            Image {source:panel.previewUrl;Layout.fillWidth:true;Layout.fillHeight:true;fillMode:Image.PreserveAspectFit;sourceSize.width:1440;sourceSize.height:900;Accessible.name:panel.previewAlt}
            Label {text:panel.previewAlt;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
        }
    }
    FileDialog {id:evidenceFile;title:"Import independent VM evidence";fileMode:FileDialog.OpenFile;nameFilters:["Evidence JSON (*.json)"];onAccepted:core.workspaceAction("evidence.import",{file:selectedFile.toString()})}
    Dialog {
        id:candidateDialog
        parent:Overlay.overlay
        title:"Exact immutable candidate"
        modal:true
        width:Math.min(760,parent ? parent.width-32 : 760)
        height:Math.min(620,parent ? parent.height-32 : 620)
        anchors.centerIn:parent
        standardButtons:Dialog.Close
        ScrollView {anchors.fill:parent;TextArea {text:JSON.stringify(panel.selected.candidate || {},null,2);readOnly:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText;font.family:theme.mono;Accessible.name:"Immutable candidate JSON"}}
    }
}
