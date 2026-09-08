import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

ColumnLayout {
    id: panel
    required property var core
    required property var theme
    required property var worksheet
    property var draft: ({})
    property var candidate: ({})
    property var revision: ({})
    property var errors: []
    property bool dirty: false
    property bool localSaved: false
    property bool previewReady: false
    property string owner: ""
    property string notice: ""
    property int remoteVersion: 0
    function command(value) { core.workspaceAction("command",value); }
    function open(id) { core.workspaceAction("drafts.get",{id:id}); }
    function change(path,value) {
        const copy=JSON.parse(JSON.stringify(candidate)); let cursor=copy;
        for (let i=0;i<path.length-1;i++) cursor=cursor[path[i]];
        cursor[path[path.length-1]]=value; candidate=copy; dirty=true; localSaved=false; previewReady=false; autosave.restart();
    }
    function save() { command({command:"save_draft",id:draft.id,version:draft.version,candidate:candidate}); }
    Connections {
        target: core
        function onWorkspaceChanged() {
            const current=(core.workspace.actor || {}).id || "";
            if (panel.owner !== current) { panel.owner=current; panel.draft={}; panel.candidate={}; panel.revision={}; panel.dirty=false; panel.previewReady=false; panel.notice=""; }
            const r=core.workspaceReply;
            if (r.error) { if (r.error === "stale_revision") panel.notice="The server has a newer revision. Your local edits are retained. Reopen the draft to compare before saving."; return; }
            if (r.action === "drafts.get" && r.id) {
                panel.draft=r; panel.remoteVersion=r.version; panel.candidate=r.candidate; panel.dirty=false; panel.previewReady=false; panel.localSaved=false; panel.notice="";
                if (r.localRecovery && r.localRecovery.candidate) {
                    panel.candidate=r.localRecovery.candidate; panel.dirty=true; panel.localSaved=true;
                    panel.notice=r.localRecovery.version === r.version ? "Recovered unsynced edits from this device." : "Recovered local edits. The server changed too; compare the server copy before saving.";
                    if (r.localRecovery.version !== r.version) panel.draft.version=r.localRecovery.version;
                }
            } else if ((r.action === "command" || r.action === "drafts.new" || r.action === "drafts.sample") && r.command === "create_draft") panel.open(r.id);
            else if (r.action === "command" && r.command === "save_draft") {panel.draft.version=r.version;panel.dirty=false;panel.notice="Saved to your private workspace.";}
            else if (r.action === "drafts.cache") panel.localSaved=true;
            else if (r.action === "drafts.preview") {panel.errors=r.errors || [];panel.previewReady=true;preview.open();}
            else if (r.action === "media.upload") panel.open(r.draftId);
            else if (r.action === "revisions.get") panel.revision=r;
            else if (r.action === "command" && r.command === "submit_draft") {panel.notice="Revision submitted for checks. Further edits create a new immutable revision.";core.workspaceAction("revisions.get",{id:r.id});}
        }
    }
    Timer { id: autosave; interval: 650; onTriggered: {
        if (!panel.dirty || !panel.draft.id) return;
        if (core.loading) {restart();return;}
        core.workspaceAction("drafts.cache",{id:panel.draft.id,version:panel.draft.version,candidate:panel.candidate});
    }}
    RowLayout {
        Layout.fillWidth: true
        Label { text:"Private drafts"; font.bold:true; font.pixelSize:22*theme.scale; Layout.fillWidth:true }
        Button { objectName:"newDraft"; text:"New listing"; enabled:!core.loading; onClicked:core.workspaceAction("drafts.new",{}) }
        Button { text:"From worksheet"; enabled:!core.loading && !!worksheet.result.candidate; onClicked:panel.command({command:"create_draft",kind:"app",candidate:worksheet.result.candidate,base_revision:null}) }
    }
    Button { visible:!!core.workspace.sandbox;text:"Use a filled fictional listing to try submission";enabled:!core.loading;onClicked:core.workspaceAction("drafts.sample",{}) }
    Label { visible:!(core.workspace.drafts || []).length; text:"Start with a few facts. Drafts stay private until you confirm the exact submission preview."; Layout.fillWidth:true; wrapMode:Text.Wrap }
    Repeater {
        model:core.workspace.drafts || []
        RowLayout {
            required property var modelData
            Layout.fillWidth:true
            Label {text:(modelData.name || "Untitled listing") + " · revision " + modelData.version;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!modelData.expiryNoticeAt;text:"Inactive draft · save to retain";wrapMode:Text.Wrap}
            Button {text:"Open";enabled:!core.loading;onClicked:panel.open(modelData.id)}
        }
    }
    Frame {
        visible:!!panel.draft.id
        Layout.fillWidth:true
        ColumnLayout {
            anchors.fill:parent
            spacing:12
            Label { text:"Listing editor";font.bold:true;font.pixelSize:22*theme.scale }
            Label { text:panel.dirty ? (panel.localSaved ? "Edits saved on this device · sync when ready" : "Saving edits on this device…") : "Private workspace copy"; Layout.fillWidth:true;wrapMode:Text.Wrap }
            Label { visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.role:Accessible.AlertMessage }
            RowLayout {
                Layout.fillWidth:true
                Button {text:"Save workspace";enabled:panel.dirty && !core.loading;onClicked:panel.save()}
                Button {text:"Compare server copy";enabled:!!panel.draft.localRecovery;onClicked:serverCopy.open()}
                Button {text:"Preview submission";enabled:!panel.dirty && !core.loading;onClicked:core.workspaceAction("drafts.preview",{candidate:panel.candidate})}
            }
            Label {text:"Purpose, release identity, acquisition route and price are separate facts. Expand each section to edit. Public test results and verified control are assigned during review.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            ValueEditor { value:panel.candidate;field:"Listing fields";theme:panel.theme;onEdited:(path,value)=>panel.change(path,value) }
            Label {text:"Upload media";font.bold:true}
            Label {text:"PNG or WebP icon ≤1 MiB; up to five screenshots ≤5 MiB; one 15–45 second MP4/WebM demo ≤30 MiB. Supply alt text and rights for each item.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            ComboBox {id:mediaKind;model:["icon","screenshot","demo"];Accessible.name:"Media kind"}
            TextField {id:mediaAlt;placeholderText:"Describe what the image or demo shows";maximumLength:2000;Layout.fillWidth:true;Accessible.name:"Media alt text"}
            TextField {id:mediaRights;placeholderText:"Creator, permission or licence";maximumLength:2000;Layout.fillWidth:true;Accessible.name:"Media rights"}
            Button {text:"Choose file and upload";enabled:!panel.dirty && !core.loading && mediaAlt.text.length>0 && mediaRights.text.length>0;onClicked:mediaFile.open()}
        }
    }
    RowLayout {
        Layout.fillWidth:true
        Label {text:"Submitted revisions";font.bold:true;font.pixelSize:22*theme.scale;Layout.fillWidth:true}
        Button {visible:!!core.workspace.sandbox;text:"Run sample checks";enabled:!core.loading;onClicked:core.workspaceAction("checks.run_sample",{})}
    }
    Repeater {
        model:core.workspace.revisions || []
        RowLayout {
            required property var modelData
            Layout.fillWidth:true
            Label {text:"Revision " + modelData.number + " · " + modelData.state.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap}
            Button {text:"View status";enabled:!core.loading;onClicked:core.workspaceAction("revisions.get",{id:modelData.id})}
        }
    }
    Frame {
        visible:!!panel.revision.id
        Layout.fillWidth:true
        ColumnLayout {
            anchors.fill:parent
            Label {text:"Revision · " + (panel.revision.state || "").replace(/_/g," ");font.bold:true}
            Label {text:"Content digest: " + (panel.revision.digest || "");Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}
            Repeater {model:panel.revision.findings || [];Label {required property var modelData;text:modelData.check+" · "+modelData.result+" · "+modelData.detail;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Repeater {model:panel.revision.decisions || [];Label {required property var modelData;text:modelData.decision+": "+modelData.reason;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            TextField {id:withdrawReason;placeholderText:"Reason for withdrawing";Layout.fillWidth:true;maximumLength:2000;Accessible.name:"Withdrawal reason"}
            Button {text:"Withdraw revision";enabled:!core.loading && withdrawReason.text.length>0 && ["submitted","checking","in_review","needs_changes","approved"].indexOf(panel.revision.state)>=0;onClicked:panel.command({command:"withdraw_revision",id:panel.revision.id,version:panel.revision.version,reason:withdrawReason.text})}
        }
    }
    FileDialog { id:mediaFile;title:"Choose listing media";fileMode:FileDialog.OpenFile;nameFilters:["Images and video (*.png *.jpg *.jpeg *.webp *.mp4 *.webm)"];onAccepted:core.workspaceAction("media.upload",{draftId:panel.draft.id,version:panel.draft.version,kind:mediaKind.currentText,alt:mediaAlt.text,rights:mediaRights.text,file:selectedFile.toString()}) }
    Dialog {
        id:preview
        parent:Overlay.overlay
        title:"Exact submission preview"
        modal:true
        width:Math.min(760,parent ? parent.width-32 : 760)
        height:Math.min(620,parent ? parent.height-32 : 620)
        anchors.centerIn:parent
        standardButtons:Dialog.Close
        ColumnLayout {
            anchors.fill:parent
            Label {text:panel.errors.length ? "Resolve " + panel.errors.length + " field issues before submission." : "This snapshot is frozen when submitted. Reviewers can request a new revision.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            ScrollView {Layout.fillWidth:true;Layout.fillHeight:true;TextArea {text:JSON.stringify(panel.candidate,null,2);readOnly:true;wrapMode:Text.WrapAnywhere;font.family:theme.mono;textFormat:Text.PlainText;Accessible.name:"Exact public candidate"}}
            Label {visible:panel.errors.length>0;text:panel.errors.slice(0,8).map(e=>e.path+": "+e.code).join("\n");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            CheckBox {id:confirm;checked:false;text:"I confirm the content and rights in this exact preview";Accessible.name:text}
            Button {text:"Submit for independent review";enabled:confirm.checked && !panel.errors.length && !panel.dirty && !core.loading;onClicked:{panel.command({command:"submit_draft",id:panel.draft.id,version:panel.draft.version,confirm_public_preview:true});preview.close();confirm.checked=false;}}
        }
    }
    Dialog {
        id:serverCopy
        parent:Overlay.overlay
        title:"Server copy · local edits are retained"
        modal:true
        width:Math.min(700,parent ? parent.width-32 : 700)
        height:Math.min(560,parent ? parent.height-32 : 560)
        anchors.centerIn:parent
        standardButtons:Dialog.Close
        ColumnLayout {
            anchors.fill:parent
            ScrollView {Layout.fillWidth:true;Layout.fillHeight:true;TextArea {text:JSON.stringify(panel.draft.candidate || {},null,2);readOnly:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText}}
            Button {text:"I have merged both copies";enabled:panel.draft.version !== panel.remoteVersion;onClicked:{panel.draft.version=panel.remoteVersion;panel.dirty=true;panel.notice="Merged edits are ready to sync. A further server change will be checked again.";autosave.restart();serverCopy.close();}}
        }
    }
}
