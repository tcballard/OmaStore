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
    property int destinationTab:0
    property int tab:0
    onDestinationTabChanged:tab=destinationTab
    property int inventoryOffset: 0
    readonly property var inventory: {
        const value=core.device;
        const items=(value.items || []).filter(i => page.tab===4 ? i.state==="update_available" : i.state==="installed" || i.state==="update_available");
        const offset=Math.min(page.inventoryOffset, Math.max(0, Math.floor((items.length-1)/30)*30));
        return Object.assign({}, value, {items:items.slice(offset,offset+30),offset:offset,nextOffset:offset+30<items.length?offset+30:null});
    }
    function refreshInventory(offset) { inventoryOffset=offset || 0; }
    onTabChanged: inventoryOffset=0
    property string pendingLaunch:""
    property string launchChoice:""
    property string notice:""
    property var pendingDetach:({})
    property bool detaching:false
    readonly property var setups:core.community["library.setups"] || ({})
    readonly property var library:core.community["library.list"] || ({})
    readonly property var launchers:core.community["library.launchers"] || ({})
    contentWidth:availableWidth;clip:true
    Component.onCompleted:{tab=destinationTab;core.communityAction("library.list");core.communityAction("library.setups");}
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
            const handoff=page.core.community["system.handoff"] || {};
            if(handoff.requested)page.notice=handoff.simulated?"Sample updater handoff. No system changes were made.":"Updater opened. Device status refreshes automatically when you return.";
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
            Label {text:page.tab===0?"Installed":page.tab===1?"Saved":page.tab===4?"Updates":"Your library.";font.pixelSize:32*page.theme.scale;font.bold:true}
            Label {visible:page.tab===0 || page.tab===4;text:page.inventory.notice || "Checking this device…";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!page.library.simulated;text:"Fictional device state · Package and launch rehearsals stay separate from this computer.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {visible:!!page.inventory.host && page.inventory.observationState!=="available";text:(page.inventory.host || {}).reason || "Device status unavailable";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!page.notice;text:page.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Flow {
                Layout.fillWidth:true;spacing:8
                Button {objectName:"openPurchases";text:"Purchases and licences";onClicked:page.purchasesRequested()}
                Button {objectName:"openRemixes";text:"My setup remixes";onClicked:page.remixesRequested()}
                Button {objectName:"openSettings";text:"Desktop settings";onClicked:page.settingsRequested()}
                Button {objectName:"libraryInstalled";text:"Installed";highlighted:page.tab===0;onClicked:page.tab=0}
                Button {objectName:"librarySaved";text:"Saved · "+page.core.saved.length;highlighted:page.tab===1;onClicked:page.tab=1}
                Button {objectName:"libraryUpdates";text:"Updates";highlighted:page.tab===4;onClicked:page.tab=4}
                Button {text:"Setups";highlighted:page.tab===3;onClicked:{page.tab=3;page.core.communityAction("library.setups");}}
                Button {objectName:"libraryActivity";text:"Proposals & activity";highlighted:page.tab===2;onClicked:page.tab=2}
                Button {text:"Refresh this device";enabled:!page.core.loading&&!page.core.deviceRefreshing;onClicked:{page.core.refreshDevice();page.core.communityAction("library.list");}}
            }
            ColumnLayout {
                visible:page.tab===0 || page.tab===4;Layout.fillWidth:true;spacing:12
                Label {visible:page.inventory.observationState==="available" && !(page.inventory.items || []).length;text:page.tab===4?"No app updates found in this device’s package databases.":"No apps from the current catalogue are installed.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Button {visible:page.tab===4;text:"Open Omarchy updater…";enabled:page.inventory.observationState==="available"&&!page.core.loading;onClicked:updatePrompt.open()}
                Repeater {
                    model:page.inventory.items || []
                    Frame {
                        required property var modelData
                        Layout.fillWidth:true
                        ColumnLayout {
                            anchors.fill:parent
                            Label {text:modelData.name;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            Label {text:"Installed: "+modelData.installedVersion+(modelData.availableVersion?" → Available: "+modelData.availableVersion:"")+"\n"+modelData.package+(modelData.updateIgnored?" · Held by package settings":"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            AppStateLabel {core:page.core;theme:page.theme;appId:modelData.id;Layout.fillWidth:true}
                            Label {text:"Checked: "+new Date(modelData.observedAt*1000).toLocaleString();Layout.fillWidth:true;wrapMode:Text.Wrap}
                            Flow {
                                Layout.fillWidth:true;spacing:8
                                Button {text:"App details";onClicked:page.core.showApp(modelData.id)}
                                Button {objectName:"remove-"+modelData.id;text:"Review removal";enabled:page.inventory.observationState==="available"&&!page.core.loading;onClicked:page.core.communityAction("system.plan",{kind:"remove",id:modelData.id})}
                                Button {text:"Open";enabled:page.inventory.observationState==="available"&&!page.core.loading;onClicked:{page.pendingLaunch=modelData.id;page.core.communityAction("library.launchers",{id:modelData.id});}}
                            }
                            Repeater {model:page.launchChoice===modelData.id?page.launchers.items:[];Button {required property string modelData;text:"Open "+modelData;onClicked:page.core.communityAction("library.launch",{id:page.launchChoice,desktop:modelData})}}
                        }
                    }
                }
                Flow {Layout.fillWidth:true;spacing:8;Button {text:"Previous";enabled:(page.inventory.offset || 0)>0&&!page.core.loading;onClicked:page.refreshInventory(Math.max(0,page.inventory.offset-30))} Button {text:"Next";enabled:page.inventory.nextOffset!==null&&page.inventory.nextOffset!==undefined&&!page.core.loading;onClicked:page.refreshInventory(page.inventory.nextOffset)}}
            }
            ColumnLayout {
                visible:page.tab===1;Layout.fillWidth:true;spacing:12
                Label {visible:page.core.saved.length===0;text:"Save an app from its detail page to keep it here.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {model:page.core.saved;Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent;Label {text:modelData.name;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText} AppStateLabel {core:page.core;theme:page.theme;appId:modelData.id;Layout.fillWidth:true} Flow {Layout.fillWidth:true;spacing:8;Button {text:"Inspect";onClicked:page.core.showApp(modelData.id)} Button {text:"Remove from saved";onClicked:page.core.removeSaved(modelData.id)}}}}}
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
    Dialog {
        id:updatePrompt;parent:Overlay.overlay;anchors.centerIn:parent;modal:true
        title:"Open Omarchy’s updater?";width:Math.min(560,parent?parent.width-32:560)
        standardButtons:Dialog.Ok|Dialog.Cancel
        contentItem:Label {text:"This opens the normal system updater in a terminal. It may update other packages and apply Omarchy migrations too. Review its prompts there.";wrapMode:Text.Wrap}
        onAccepted:page.core.communityAction("system.handoff",{kind:"update",confirmed:true})
    }
    Dialog {id:detachPrompt;parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Detach this setup?";implicitHeight:220*page.theme.scale;height:Math.min(implicitHeight,parent?parent.height-32:implicitHeight);width:Math.min(560,parent?parent.width-32:560);standardButtons:Dialog.Ok|Dialog.Cancel;contentItem:Label {text:"This removes the local setup references. Installed applications, documents and desktop settings stay in place.";wrapMode:Text.Wrap} onAccepted:{page.detaching=true;page.core.communityAction("library.detach_setup",{id:page.pendingDetach.id,revision:page.pendingDetach.revision,confirmed:true});}}

}
