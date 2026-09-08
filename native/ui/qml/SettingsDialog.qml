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
    readonly property bool currentPlan:!!requestedDigest && JSON.stringify((plan.request || {}).settings || [])===JSON.stringify(Object.keys(selections).sort().map(k=>selections[k]))
    function choose(adapter,value,enabled){const copy=Object.assign({},selections);if(enabled)copy[adapter]={adapter:adapter,valueId:value,revision:"1"};else delete copy[adapter];selections=copy;requestedDigest="";}
    function openFor(references){initialReferences=references;selections=({});requestedDigest="";core.communityAction("settings.adapters",{});open();}
    parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Review desktop settings"
    width:Math.min(780,parent?parent.width-32:780);height:Math.min(720,parent?parent.height-32:720);standardButtons:Dialog.Close
    Connections {target:dialog.core;function onCommunityChanged(){if(dialog.plan.digest)dialog.requestedDigest=dialog.plan.digest;}}
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
                Label {text:"This is a saved proposal. No desktop setting has been authorised.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            }
            Button {objectName:"closeSettings";text:"Done";onClicked:dialog.close()}
        }
    }
}
