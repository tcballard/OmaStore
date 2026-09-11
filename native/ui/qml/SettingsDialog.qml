import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id:dialog
    required property var core
    required property var theme
    property var selections:({})
    property var initialReferences:[]
    readonly property var definitions:core.community["settings.adapters"] || ({})
    readonly property var plan:core.community["settings.preview"] || ({})
    property string requestedDigest:""
    property var historyItems:[]
    property var seenReplies:({})
    property string restoreId:""
    readonly property var restoration:core.community["settings.restore_preview"] || ({})
    readonly property bool currentPlan:!!requestedDigest && JSON.stringify(((plan.request || {}).settings || []).map(r=>[r.adapter,r.valueId,r.revision]))===JSON.stringify(Object.keys(selections).sort().map(k=>[selections[k].adapter,selections[k].valueId,selections[k].revision]))
    function choose(adapter,value,enabled){const copy=Object.assign({},selections);if(enabled)copy[adapter]={adapter:adapter,valueId:value,revision:"1"};else delete copy[adapter];selections=copy;requestedDigest="";applyConsent.checked=false;}
    function openFor(references){initialReferences=references;selections=({});requestedDigest="";restoreId="";applyConsent.checked=false;restoreConsent.checked=false;core.communityAction("settings.adapters",{});core.communityAction("settings.history",{});open();}
    parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Review desktop settings"
    width:Math.min(780,parent?parent.width-32:780);height:Math.min(720,parent?parent.height-32:720);standardButtons:Dialog.Close
    Connections {target:dialog.core;function onCommunityChanged(){
        if(dialog.plan.digest)dialog.requestedDigest=dialog.plan.digest;
        for(const method of ["settings.history","settings.apply","settings.restore","settings.reconcile"]){const reply=dialog.core.community[method];if(reply && reply.items && dialog.seenReplies[method]!==JSON.stringify(reply)){dialog.seenReplies[method]=JSON.stringify(reply);dialog.historyItems=reply.items;applyConsent.checked=false;restoreConsent.checked=false;}}
    }}
    ScrollView {
        anchors.fill:parent;contentWidth:availableWidth;clip:true
        ColumnLayout {width:parent.width;spacing:14
            Label {text:dialog.definitions.notice || "Loading supported choices…";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:dialog.initialReferences.length>0;text:"Recipe references: "+dialog.initialReferences.map(r=>r.adapter+" = "+r.valueId+" (revision "+r.revision+")").join(", ")+". Choose explicitly below; unknown choices remain manual.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Repeater {model:dialog.definitions.items || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
                CheckBox {id:enabledChoice;objectName:"settingEnabled-"+modelData.id;text:modelData.name;checked:!!dialog.selections[modelData.id];onToggled:dialog.choose(modelData.id,values.currentText,checked);Layout.fillWidth:true}
                ComboBox {id:values;objectName:"settingValue-"+modelData.id;model:modelData.values;Layout.fillWidth:true;Accessible.name:modelData.name+" value";onActivated:if(enabledChoice.checked)dialog.choose(modelData.id,currentText,true)}
                Label {text:modelData.scope+"\n"+modelData.restoration;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            }}}
            Button {objectName:"previewSettings";text:"Preview selected changes";enabled:Object.keys(dialog.selections).length>0&&!dialog.core.loading;onClicked:{dialog.requestedDigest="";dialog.core.communityAction("settings.preview",{settings:Object.keys(dialog.selections).sort().map(k=>dialog.selections[k])});}}
            Label {visible:!!dialog.plan.error;text:(dialog.plan.error || "").replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            ColumnLayout {visible:dialog.currentPlan;Layout.fillWidth:true;spacing:12
                Label {objectName:"settingsPlanNotice";text:dialog.plan.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Repeater {model:dialog.plan.effects || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
                    Label {text:modelData.reference.adapter+" · "+modelData.state;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    Label {text:"Current: "+JSON.stringify(modelData.before)+"\nSelected: "+JSON.stringify(modelData.desired)+"\n"+modelData.reason;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                    Label {text:modelData.scope+"\n"+modelData.restoration;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                }}}
                CheckBox {id:applyConsent;objectName:"settingsConsent";text:dialog.plan.simulated?"Apply these selected values to the fictional desktop files":"I reviewed these exact desktop setting changes";Layout.fillWidth:true}
                Button {objectName:"applySettings";text:"Apply selected settings";enabled:dialog.currentPlan && dialog.plan.canApply && applyConsent.checked && !dialog.core.loading && !dialog.historyItems.some(r=>r.id===dialog.plan.digest);onClicked:dialog.core.communityAction("settings.apply",{id:dialog.plan.digest,digest:dialog.plan.digest,accepted:true})}
            }
            Label {text:dialog.core.error;visible:!!dialog.core.error;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {text:"Setting history on this device";font.bold:true;Layout.fillWidth:true}
            Button {text:"Refresh history";enabled:!dialog.core.loading;onClicked:dialog.core.communityAction("settings.history",{})}
            Repeater {model:dialog.historyItems;Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
                Label {text:(modelData.simulated?"Fictional desktop · ":"")+new Date(modelData.createdAt*1000).toLocaleString();Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {model:modelData.steps;Label {required property var modelData;text:modelData.reference.adapter+" · "+modelData.state+"\n"+JSON.stringify(modelData.before)+" → "+JSON.stringify(modelData.after)+"\n"+modelData.code.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
                Flow {Layout.fillWidth:true;spacing:8
                    Button {text:"Check interrupted change";visible:modelData.steps.some(s=>s.state==="unknown");enabled:!dialog.core.loading;onClicked:dialog.core.communityAction("settings.reconcile",{id:modelData.id})}
                    Button {objectName:"previewRestore-"+modelData.id;text:"Preview restoration";enabled:!dialog.core.loading && modelData.steps.some(s=>s.state==="applied"||s.state==="unknown");onClicked:{dialog.restoreId=modelData.id;restoreConsent.checked=false;dialog.core.communityAction("settings.restore_preview",{id:modelData.id});}}
                }
            }}}
            ColumnLayout {visible:!!dialog.restoreId && dialog.restoration.id===dialog.restoreId;Layout.fillWidth:true
                Label {text:"Restore only the selected values below. Later changes to the same setting block restoration. Current unrelated settings and documents are preserved.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {model:dialog.restoration.effects || [];Label {required property var modelData;text:modelData.reference.adapter+": "+JSON.stringify(modelData.current)+" → "+JSON.stringify(modelData.restore)+" · "+modelData.code.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
                CheckBox {id:restoreConsent;objectName:"settingsRestoreConsent";text:"I reviewed these exact restoration changes";Layout.fillWidth:true}
                Button {objectName:"restoreSettings";text:"Restore previous values";enabled:!!dialog.restoration.canRestore && restoreConsent.checked && !dialog.core.loading;onClicked:dialog.core.communityAction("settings.restore",{id:dialog.restoreId,digest:dialog.restoration.digest,accepted:true})}
            }
            Button {objectName:"closeSettings";text:"Done";onClicked:dialog.close()}
        }
    }
}
