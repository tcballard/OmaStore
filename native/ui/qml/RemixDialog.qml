import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

Dialog {
    id:dialog
    required property var core
    required property var theme
    signal settingsRequested(var references)
    signal authorCreated()
    property var creationSelection:null
    property var selections:({})
    property var review:({})
    property var seen:({})
    property string notice:""
    property bool authorPending:false
    readonly property var recipes:(core.community["remixes.list"] || {}).items || []
    readonly property var definitions:(core.community["settings.adapters"] || {}).items || []
    function openFor(selection){creationSelection=selection;review=({});notice="";selections=({});name.text="My setup";core.communityAction("remixes.list",{});core.communityAction("settings.adapters",{});open();}
    function select(adapter,value,on){const copy=Object.assign({},selections);if(on)copy[adapter]={adapter:adapter,valueId:value,revision:"1"};else delete copy[adapter];selections=copy;}
    parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Your setup remixes"
    width:Math.min(800,parent?parent.width-32:800);height:Math.min(780,parent?parent.height-32:780);standardButtons:Dialog.Close
    Connections {target:dialog.core
        function onCommunityChanged(){for(const method of ["remixes.create","remixes.get","remixes.rename","remixes.import","remixes.export"]){const r=dialog.core.community[method];if(r&&dialog.seen[method]!==JSON.stringify(r)){dialog.seen[method]=JSON.stringify(r);if(r.remix){dialog.review=r;dialog.creationSelection=null;dialog.notice="Saved on this device. Nothing has been published.";dialog.core.communityAction("remixes.list",{});}else if(r.exported)dialog.notice="Selected recipe exported.";else if(r.error)dialog.notice=r.error.replace(/_/g," ");}}}
        function onWorkspaceChanged(){const r=dialog.core.workspaceReply;if(dialog.authorPending&&r.action==="drafts.remix"){dialog.authorPending=false;if(r.error)dialog.notice=r.error.replace(/_/g," ");else{dialog.authorCreated();dialog.close();}}}
    }
    ScrollView {anchors.fill:parent;clip:true;contentWidth:availableWidth;ColumnLayout {width:parent.width;spacing:14
        Label {text:"Keep a personal version, share only the pieces you choose, and retain the original creator's attribution. A local save does not publish or change your desktop.";Layout.fillWidth:true;wrapMode:Text.Wrap}
        Label {text:dialog.notice;visible:!!dialog.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
        ColumnLayout {visible:!!dialog.creationSelection;Layout.fillWidth:true
            Label {text:"Save the application selection you just reviewed. Dependencies stay included; unselected components stay out.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            TextField {id:name;objectName:"remixName";placeholderText:"Name your setup";maximumLength:160;Accessible.name:"Local setup name";Layout.fillWidth:true}
            Label {text:"Optional settings to include";font.bold:true;Layout.fillWidth:true}
            Repeater {model:dialog.definitions;RowLayout {required property var modelData;Layout.fillWidth:true
                CheckBox {id:choice;checked:!!dialog.selections[modelData.id];text:modelData.name;Layout.fillWidth:true;onToggled:dialog.select(modelData.id,values.currentText,checked)}
                ComboBox {id:values;model:modelData.values;Accessible.name:modelData.name+" to share";onActivated:if(choice.checked)dialog.select(modelData.id,currentText,true)}
            }}
            Button {objectName:"saveRemix";text:"Save my selected setup";enabled:!!dialog.creationSelection&&name.text.trim().length>0&&!dialog.core.loading;onClicked:dialog.core.communityAction("remixes.create",{selection:dialog.creationSelection,name:name.text.trim(),settings:Object.keys(dialog.selections).sort().map(k=>dialog.selections[k])})}
        }
        Flow {Layout.fillWidth:true;spacing:8
            Button {text:"Import a remix";enabled:!dialog.core.loading;onClicked:importFile.open()}
            Button {text:"Refresh my setups";enabled:!dialog.core.loading;onClicked:dialog.core.communityAction("remixes.list",{})}
        }
        Repeater {model:dialog.recipes;Button {required property var modelData;objectName:"localRemix-"+modelData.id;text:modelData.name+" · version "+modelData.revision;Layout.fillWidth:true;enabled:!dialog.core.loading;onClicked:dialog.core.communityAction("remixes.get",{id:modelData.id})}}
        ColumnLayout {visible:!!dialog.review.remix;Layout.fillWidth:true
            Label {objectName:"remixAttribution";text:dialog.review.remix?"Based on "+dialog.review.remix.parent.id+" · revision "+dialog.review.remix.parent.revision+" · by "+dialog.review.remix.parent.makerId:"";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {text:dialog.review.remix?"Sharing rights: "+dialog.review.remix.rights:"";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            RowLayout {Layout.fillWidth:true
                TextField {id:rename;text:(dialog.review.remix || {}).name || "";maximumLength:160;Accessible.name:"Rename local setup";Layout.fillWidth:true}
                Button {text:"Rename";enabled:!!dialog.review.remix&&rename.text.trim().length>0&&!dialog.core.loading;onClicked:dialog.core.communityAction("remixes.rename",{id:dialog.review.remix.id,version:dialog.review.remix.revision,name:rename.text.trim()})}
            }
            Label {text:dialog.review.ready?"Selected releases are available for a fresh plan on this machine.":"Review differences before proceeding. No release has been substituted.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {model:dialog.review.differences || [];Label {required property var modelData;text:modelData.id+" · "+modelData.kind.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Repeater {model:(dialog.review.remix || {}).components || [];RowLayout {required property var modelData;Layout.fillWidth:true
                Label {text:modelData.appId+" · "+modelData.releaseId;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Button {text:"Inspect app";onClicked:{dialog.core.showApp(modelData.appId);dialog.close();}}
            }}
            Label {text:"Selected settings: "+(((dialog.review.remix || {}).settings || []).map(r=>r.adapter+" = "+r.valueId).join(", ") || "none");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Repeater {model:(dialog.review.settingsPreview || {}).effects || [];Label {required property var modelData;text:modelData.reference.adapter+": "+JSON.stringify(modelData.before)+" → "+JSON.stringify(modelData.desired)+" · "+modelData.state;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
            Flow {Layout.fillWidth:true;spacing:8
                Button {objectName:"planRemix";text:"Review app installation";enabled:!!dialog.review.ready&&!dialog.core.loading;onClicked:{dialog.core.communityAction("system.plan",dialog.review.installationSelection);dialog.close();}}
                Button {text:"Review desktop settings";enabled:!!dialog.review.remix&&dialog.review.remix.settings.length>0;onClicked:{dialog.settingsRequested(dialog.review.remix.settings);dialog.close();}}
                Button {objectName:"exportRemix";text:"Export the preview below";enabled:!!dialog.review.digest&&!dialog.core.loading;onClicked:exportFile.open()}
            }
            Label {text:"Exact portable file · no local observations or account data";font.bold:true;Layout.fillWidth:true}
            TextArea {objectName:"remixExportPreview";text:JSON.stringify(dialog.review.export || {},null,2);readOnly:true;selectByMouse:true;wrapMode:TextEdit.Wrap;Layout.fillWidth:true;Layout.preferredHeight:220;font.family:dialog.theme.mono;Accessible.name:"Exact selected remix export"}
            Button {objectName:"authorRemix";text:"Create a private author draft";enabled:!!dialog.review.ready&&!!(dialog.core.workspace.actor || {}).id&&!dialog.core.loading;onClicked:{dialog.authorPending=true;dialog.core.workspaceAction("drafts.remix",{remix:dialog.review.export});}}
            Label {text:"Sign in under Submit to create an author draft. Confirm identity, rights, media and the exact public preview there; the normal independent review still applies.";Layout.fillWidth:true;wrapMode:Text.Wrap}
        }
        Button {objectName:"closeRemixes";text:"Done";onClicked:dialog.close()}
    }}
    FileDialog {id:exportFile;title:"Export selected setup remix";fileMode:FileDialog.SaveFile;defaultSuffix:"json";nameFilters:["Setup remix (*.json)"];onAccepted:dialog.core.communityAction("remixes.export",{id:dialog.review.remix.id,digest:dialog.review.digest,file:selectedFile.toString()})}
    FileDialog {id:importFile;title:"Import a setup remix";fileMode:FileDialog.OpenFile;nameFilters:["Setup remix (*.json)"];onAccepted:dialog.core.communityAction("remixes.import",{file:selectedFile.toString()})}
}
