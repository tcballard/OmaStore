import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ScrollView {
    id:page
    signal settingsRequested()
    signal remixesRequested()
    signal purchasesRequested()
    required property var core
    required property var theme
    property int tab:0
    property string pendingLaunch:""
    property string launchChoice:""
    property string notice:""
    property var pendingDetach:({})
    property bool detaching:false
    readonly property var setups:core.community["library.setups"] || ({})
    readonly property var library:core.community["library.list"] || ({})
    readonly property var launchers:core.community["library.launchers"] || ({})
    contentWidth:availableWidth;clip:true
    Component.onCompleted:{core.communityAction("library.list");core.communityAction("library.refresh");core.communityAction("library.setups");}
    Connections {
        target:page.core
        function onCommunityChanged(){
            const detached=page.core.community["library.detach_setup"] || {};
            if(page.detaching && detached.detached && page.pendingDetach.id){page.detaching=false;page.pendingDetach=({});page.notice=detached.notice;page.core.communityAction("library.setups");}

            if(page.pendingLaunch && page.launchers.id===page.pendingLaunch){
                const id=page.pendingLaunch;page.pendingLaunch="";
                if(page.launchers.items.length===1)page.core.communityAction("library.launch",{id:id,desktop:page.launchers.items[0]});
                else if(page.launchers.items.length>1)page.launchChoice=id;
                else page.notice="This package has no supported desktop launcher. Use its normal system entry.";
            }
            const result=page.core.community["library.launch"] || {};
            if(result.requested)page.notice=result.simulated?"Sample launch requested. No real application was opened.":"Open request sent to the system desktop launcher.";
        }
    }
    ColumnLayout {
        width:page.availableWidth;spacing:18
        Item {Layout.preferredHeight:8}
        ColumnLayout {
            Layout.fillWidth:true;Layout.margins:28;spacing:18
            Label {text:"YOUR TOOLS, ON THIS DEVICE";font.family:page.theme.mono;color:page.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {text:"Your library.";font.pixelSize:32*page.theme.scale;font.bold:true}
            Label {text:page.library.notice || "Loading local observations…";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!page.library.simulated;text:"Fictional device state · Package and launch rehearsals stay separate from this computer.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!!page.library.host;text:((page.library.host || {}).state || "unknown")+" · "+((page.library.host || {}).reason || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!page.notice;text:page.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Flow {
                Layout.fillWidth:true;spacing:8
                Button {objectName:"openPurchases";text:"Purchases and licences";onClicked:page.purchasesRequested()}
                Button {objectName:"openRemixes";text:"My setup remixes";onClicked:page.remixesRequested()}
                Button {objectName:"openSettings";text:"Desktop settings";onClicked:page.settingsRequested()}
                Button {objectName:"libraryInstalled";text:"Installed";highlighted:page.tab===0;onClicked:page.tab=0}
                Button {objectName:"librarySaved";text:"Saved · "+page.core.saved.length;highlighted:page.tab===1;onClicked:page.tab=1}
                Button {text:"Setups";highlighted:page.tab===3;onClicked:{page.tab=3;page.core.communityAction("library.setups");}}
                Button {objectName:"libraryActivity";text:"Proposals & activity";highlighted:page.tab===2;onClicked:page.tab=2}
                Button {text:"Refresh this device";enabled:!page.core.loading;onClicked:page.core.communityAction("library.refresh")}
            }
            ColumnLayout {
                visible:page.tab===0;Layout.fillWidth:true;spacing:12
                Label {visible:!(page.library.items || []).length;text:"No supported installations have been observed yet. Saved items and plans are kept in their own views.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {
                    model:page.library.items || []
                    Frame {
                        required property var modelData
                        Layout.fillWidth:true
                        ColumnLayout {
                            anchors.fill:parent
                            Label {text:modelData.name;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            Label {text:(modelData.present?"Observed installed · ":"Last observed version · ")+modelData.installedVersion+"\n"+modelData.package+" · "+modelData.sourceState.replace(/_/g," ")+"\nCatalogue package version: "+modelData.catalogueVersion;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            Label {text:(modelData.preexisting?"Present before an OmaStore installation was recorded. ":"")+"Observed: "+new Date(modelData.observedAt*1000).toLocaleString();Layout.fillWidth:true;wrapMode:Text.Wrap}
                            Flow {
                                Layout.fillWidth:true;spacing:8
                                Button {text:"App details";onClicked:page.core.showApp(modelData.id)}
                                Button {objectName:"remove-"+modelData.id;text:"Review removal";enabled:modelData.present&&!page.core.loading;onClicked:page.core.communityAction("system.plan",{kind:"remove",id:modelData.id})}
                                Button {text:"Open";enabled:modelData.present&&!page.core.loading;onClicked:{page.pendingLaunch=modelData.id;page.core.communityAction("library.launchers",{id:modelData.id});}}
                            }
                            Repeater {model:page.launchChoice===modelData.id?page.launchers.items:[];Button {required property string modelData;text:"Open "+modelData;onClicked:page.core.communityAction("library.launch",{id:page.launchChoice,desktop:modelData})}}
                        }
                    }
                }
                Flow {Layout.fillWidth:true;spacing:8;Button {text:"Previous";enabled:(page.library.offset || 0)>0&&!page.core.loading;onClicked:page.core.communityAction("library.list",{offset:Math.max(0,page.library.offset-30)})} Button {text:"Next";enabled:page.library.nextOffset!==null&&page.library.nextOffset!==undefined&&!page.core.loading;onClicked:page.core.communityAction("library.list",{offset:page.library.nextOffset})}}
            }
            ColumnLayout {
                visible:page.tab===1;Layout.fillWidth:true;spacing:12
                Label {visible:page.core.saved.length===0;text:"Save an app from its detail page to keep it here.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {model:page.core.saved;Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent;Label {text:modelData.name;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText} Flow {Layout.fillWidth:true;spacing:8;Button {text:"Inspect";onClicked:page.core.showApp(modelData.id)} Button {text:"Remove from saved";onClicked:page.core.removeSaved(modelData.id)}}}}}
            }
            ColumnLayout {
                visible:page.tab===2;Layout.fillWidth:true;spacing:12
                Label {text:"Recent activity · sequence "+(page.library.lastSequence || 0);Layout.fillWidth:true;wrapMode:Text.Wrap}
                Label {visible:!(page.library.operations || []).length;text:"No local operations yet. An installation plan starts a reviewable proposal.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {
                    model:page.library.operations || []
                    Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent;Label {text:modelData.state.replace(/_/g," ")+" · "+new Date(modelData.updatedAt*1000).toLocaleString();Layout.fillWidth:true;wrapMode:Text.Wrap} Label {text:modelData.id;font.family:page.theme.mono;font.pixelSize:10*page.theme.scale;Layout.fillWidth:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText} Button {objectName:"operation-"+modelData.id;text:"Review proposal";enabled:!page.core.loading;onClicked:page.core.communityAction("operations.get",{id:modelData.id})}}}
                }
            }
            ColumnLayout {
                visible:page.tab===3;Layout.fillWidth:true;spacing:12
                Label {text:"These are references to observed components. Detaching a setup preserves its apps and settings.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Label {visible:!(page.setups.items || []).length;text:"No local setup references yet."}
                Repeater {model:page.setups.items || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent;Label {text:modelData.id+" · revision "+modelData.revision+" · "+modelData.componentCount+" observed component(s)";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText} Flow {Layout.fillWidth:true;spacing:8;Button {text:"Open exact setup";onClicked:page.core.communityAction("handoff.open",{uri:"omastore://setup/"+modelData.id+"?revision="+modelData.revision})} Button {text:"Detach setup";onClicked:{page.pendingDetach={id:modelData.id,revision:modelData.revision};detachPrompt.open();}}}}}}
                Flow {Layout.fillWidth:true;spacing:8;Button {text:"Previous";enabled:(page.setups.offset || 0)>0&&!page.core.loading;onClicked:page.core.communityAction("library.setups",{offset:Math.max(0,page.setups.offset-30)})} Button {text:"Next";enabled:!!page.setups.hasMore&&!page.core.loading;onClicked:page.core.communityAction("library.setups",{offset:(page.setups.offset || 0)+30})}}
            }
            TextField {id:shared;Layout.fillWidth:true;maximumLength:400;placeholderText:"omastore://app/… or an exact setup link";Accessible.name:"Shared OmaStore identity"}
            Button {text:"Open shared link";enabled:shared.text.length>0&&!page.core.loading;onClicked:page.core.communityAction("handoff.open",{uri:shared.text})}
        }
    }
    Dialog {id:detachPrompt;parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Detach this setup?";implicitHeight:220*page.theme.scale;height:Math.min(implicitHeight,parent?parent.height-32:implicitHeight);width:Math.min(560,parent?parent.width-32:560);standardButtons:Dialog.Ok|Dialog.Cancel;contentItem:Label {text:"This removes the local setup references. Installed applications, documents and desktop settings stay in place.";wrapMode:Text.Wrap} onAccepted:{page.detaching=true;page.core.communityAction("library.detach_setup",{id:page.pendingDetach.id,revision:page.pendingDetach.revision,confirmed:true});}}

}
