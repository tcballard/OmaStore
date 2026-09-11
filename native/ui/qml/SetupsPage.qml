import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

ScrollView {
    id:page
    signal settingsRequested(var references)
    signal remixRequested(var selection)
    required property var core
    required property var theme
    required property var mediaPreview
    property string selectedId:""
    property string selectedRevision:""
    signal chosen(string id,string revision)
    signal makerChosen(string id)
    readonly property var list:core.community["setups.list"] || ({})
    readonly property var selection:core.community["setups.select"] || ({})
    readonly property var recipe:selection.recipe || ({})
    readonly property bool showing:!!selectedId && recipe.id===selectedId && recipe.revision===selectedRevision
    readonly property bool current:showing && selection.snapshot===core.catalogue.snapshot
    property var distribution:({})
    property string statusKey:""
    property string notice:""
    property real nowSeconds:Date.now()/1000
    property int mediaIndex:0
    contentWidth:availableWidth
    clip:true
    ScrollBar.horizontal.policy:ScrollBar.AlwaysOff
    function choose(appId,on) {let ids=(selection.requested || []).filter(id=>id!==appId);if(on)ids.push(appId);core.communityAction("setups.select",{id:recipe.id,revision:recipe.revision,chosen:ids});}
    function checkStatus(){if(showing)core.workspaceAction("status.get",{ids:(selection.components || []).map(c=>c.appId)});}
    function status(row) {
        if(!current || distribution.catalogueSnapshot!==selection.snapshot || distribution.validUntil<=nowSeconds || distribution.validUntil>distribution.generatedAt+300 || distribution.generatedAt>nowSeconds+30 || distribution.generatedAt<nowSeconds-300)return "Current distribution status unavailable";
        const found=(distribution.items || []).find(x=>x.appId===row.appId);
        if(!found || found.releaseId!==row.releaseId)return "This release needs a current check";
        return found.distribution==="suspended"?"Distribution suspended":found.sourceCurrent?"Source observation current":"Source observation needs refresh";
    }
    function showMedia(){const asset=(recipe.media || [])[mediaIndex];mediaPreview.load(asset || {});}
    function amount(g){return g.currency+" "+(g.minorUnits/Math.pow(10,g.exponent)).toFixed(g.exponent)+(g.billingInterval?" / "+g.billingInterval:"")+" · tax "+g.tax;}
    Component.onCompleted:{if(showing){checkStatus();showMedia();}}
    Component.onDestruction:mediaPreview.load({})
    Connections {
        target:page.core
        function onCommunityChanged() {
            const r=page.core.community["setups.select"] || {};
            if(r.imported && r.recipe && (page.selectedId!==r.recipe.id || page.selectedRevision!==r.recipe.revision))page.chosen(r.recipe.id,r.recipe.revision);
            if(page.showing) {
                const key=r.snapshot+"|"+r.recipe.id+"|"+r.recipe.revision+"|"+(r.selected || []).join(",");
                if(page.statusKey!==key){page.statusKey=key;page.checkStatus();page.showMedia();}
            }
            const exported=page.core.community["setups.export"] || {};
            if(exported.exported)page.notice="Selection exported. It contains public references and choices.";
            if(exported.error)page.notice="Export unavailable. Reload this setup and check the selection.";
        }
        function onWorkspaceChanged(){const r=page.core.workspaceReply;if(r.action==="status.get"&&!r.error){page.distribution=r;page.nowSeconds=Date.now()/1000;}}
    }
    Timer {interval:10000;running:page.showing;repeat:true;onTriggered:page.nowSeconds=Date.now()/1000}
    ColumnLayout {
        width:page.availableWidth;spacing:18
        Item {Layout.preferredHeight:8}
        ColumnLayout {
            Layout.fillWidth:true;Layout.margins:28;spacing:18
            Label {text:"A WORKFLOW, PIECE BY PIECE";font.family:page.theme.mono;color:page.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {objectName:"setupTitle";text:page.showing?page.recipe.name:"Make room for a better setup.";font.pixelSize:32*page.theme.scale;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Flow {
                Layout.fillWidth:true;spacing:8
                Button {text:"All setups";visible:!!page.selectedId;onClicked:page.chosen("","")}
                Button {text:"Import a selection";enabled:!page.core.loading;onClicked:importFile.open()}
                Button {text:"Copy setup link";visible:page.showing;onClicked:{page.core.copySetupLink();page.notice="Setup link copied.";}}
                Button {text:"Reload setup";visible:page.showing;enabled:!page.core.loading;onClicked:page.core.communityAction("setups.select",{id:page.recipe.id,revision:page.recipe.revision,chosen:page.selection.requested || []})}
            }
            Label {visible:!!page.notice;text:page.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            TextField {id:search;visible:!page.selectedId;placeholderText:"Find a workflow";maximumLength:200;Accessible.name:"Search setups";Layout.fillWidth:true;onTextEdited:searchTimer.restart()}
            Timer {id:searchTimer;interval:180;onTriggered:page.core.communityAction("setups.list",{q:search.text})}
            Label {visible:!page.selectedId && !(page.list.items || []).length;text:"Reviewed setups will explain a workflow and let you choose what belongs on your machine.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {
                model:page.selectedId?[]:(page.list.items || [])
                Frame {
                    required property var modelData
                    Layout.fillWidth:true
                    ColumnLayout {
                        anchors.fill:parent
                        Label {text:modelData.name;font.bold:true;font.pixelSize:22*page.theme.scale;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {text:modelData.summary;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Button {objectName:"setup-"+modelData.id;text:"Explore "+modelData.components+" components";enabled:!page.core.loading;onClicked:{page.core.communityAction("setups.select",{id:modelData.id,revision:modelData.revision,chosen:null});page.chosen(modelData.id,modelData.revision);}}
                    }
                }
            }
            RowLayout {
                visible:!page.selectedId;Layout.fillWidth:true
                Button {text:"Previous";enabled:(page.list.offset || 0)>0&&!page.core.loading;onClicked:page.core.communityAction("setups.list",{q:search.text,offset:Math.max(0,page.list.offset-30),snapshot:page.list.snapshot})}
                Label {text:(page.list.total || 0)+" setups";Layout.fillWidth:true}
                Button {text:"Next";enabled:page.list.nextOffset!==null && page.list.nextOffset!==undefined&&!page.core.loading;onClicked:page.core.communityAction("setups.list",{q:search.text,offset:page.list.nextOffset,snapshot:page.list.snapshot})}
            }
            ColumnLayout {
                visible:page.showing;Layout.fillWidth:true;spacing:16
                Label {text:page.recipe.summary || "";font.pixelSize:18*page.theme.scale;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Label {visible:!!page.recipe.description;text:page.recipe.description || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Button {text:"By "+((page.selection.maker || {}).name || "the creator")+" · revision "+(page.recipe.revision || "");onClicked:page.makerChosen(page.recipe.makerId)}
                Label {text:(page.selection.maker || {}).claimLabel || "";color:page.theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Label {text:"Sharing rights: "+(page.recipe.rights || "")+(page.recipe.parent?"\nBased on "+page.recipe.parent+" · "+(page.recipe.parentRevision || "revision not recorded"):"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                ComboBox {visible:(page.recipe.media || []).length>0;model:(page.recipe.media || []).map(m=>m.kind+" · "+m.alt);Layout.fillWidth:true;Accessible.name:"Setup media";onActivated:{page.mediaIndex=currentIndex;page.showMedia();}}
                Image {visible:page.mediaPreview.status==="ready";source:page.mediaPreview.source;Layout.fillWidth:true;Layout.preferredHeight:visible?260:0;fillMode:Image.PreserveAspectFit;Accessible.name:((page.recipe.media || [])[page.mediaIndex] || {}).alt || "Setup media"}
                Button {visible:!!(page.recipe.media || [])[page.mediaIndex];text:"Open selected media";onClicked:page.core.openSetupLink("media",page.mediaIndex)}
                Label {text:"Choose the pieces you want.";font.bold:true;font.pixelSize:22*page.theme.scale}
                Label {text:page.selection.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;color:page.theme.muted}
                Label {visible:!page.current;text:"The catalogue changed. Reload this setup before exporting or planning.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Repeater {
                    model:page.selection.components || []
                    Frame {
                        required property var modelData
                        required property int index
                        Layout.fillWidth:true
                        ColumnLayout {
                            anchors.fill:parent
                            CheckBox {objectName:"component-"+modelData.appId;text:modelData.name+" · "+modelData.version;checked:modelData.selected;enabled:!modelData.required && page.current && !page.core.loading;onToggled:page.choose(modelData.appId,checked)}
                            Label {text:modelData.summary;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            Label {text:modelData.requiredBy.length?"Required by: "+modelData.requiredBy.join(", "):(!modelData.optional?"Required component":"Optional component");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;color:page.theme.muted}
                            Label {text:modelData.priceLabel+" · "+modelData.routeLabel+"\n"+modelData.evidenceLabel+"\n"+page.status(modelData);Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                            Label {visible:!modelData.currentRelease;text:"The selected release is no longer current. It will not be substituted.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                            Flow {Layout.fillWidth:true;spacing:8;Button {text:"Inspect app";onClicked:page.core.showApp(modelData.appId)} Button {visible:modelData.externalPurchase && !!modelData.offer;text:"View seller's offer";onClicked:page.core.openSetupLink("offer",index)}}
                        }
                    }
                }
                Label {visible:(page.selection.conflicts || []).length>0;text:"Resolve these conflicts: "+(page.selection.conflicts || []).map(p=>p.join(" / ")).join("; ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Label {text:"Costs for this selection";font.bold:true;font.pixelSize:22*page.theme.scale}
                Repeater {model:(page.selection.costs || {}).groups || [];Label {required property var modelData;text:page.amount(modelData);Layout.fillWidth:true;wrapMode:Text.Wrap}}
                Label {text:(page.selection.costs || {}).unknownItems>0 ? "Total remains unknown · "+page.selection.costs.unknownItems+" item(s) need a current quote." : ((page.selection.costs || {}).groups || []).length===0 ? "No required purchase amount in this selection." : "Amounts above are separate estimates.";Layout.fillWidth:true;wrapMode:Text.Wrap}
                Label {text:(page.selection.costs || {}).notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;color:page.theme.muted}
                Label {visible:(page.recipe.settings || []).length>0;text:"Setting references: "+(page.recipe.settings || []).map(s=>s.adapter+" = "+s.valueId).join(", ")+". This selection does not apply them.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Flow {Layout.fillWidth:true;spacing:8;Button {objectName:"remixSetup";text:"Remix this selection";enabled:page.current&&page.selection.valid&&!page.core.loading;onClicked:page.remixRequested({id:page.recipe.id,revision:page.recipe.revision,chosen:page.selection.requested || []})} Button {text:"Review desktop settings";enabled:page.current&&!page.core.loading;onClicked:page.settingsRequested(page.recipe.settings || [])} Button {text:"Check distribution status";enabled:!page.core.loading;onClicked:page.checkStatus()} Button {objectName:"previewSetupPlan";text:"Review installation plan";enabled:page.current&&page.selection.valid&&!page.core.loading;onClicked:page.core.communityAction("system.plan",{kind:"setup",id:page.recipe.id,revision:page.recipe.revision,chosen:page.selection.requested || []})} Button {objectName:"previewSetupExport";text:"Preview selection export";enabled:page.current && page.selection.valid && !page.core.loading;onClicked:exportPreview.open()} Button {visible:!!(page.selection.maker || {}).support;text:"Support the creator";onClicked:page.core.openSetupLink("support")}}
            }
        }
    }
    Dialog {
        id:exportPreview;parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Selection to share";standardButtons:Dialog.Close
        width:Math.min(740,parent?parent.width-32:740);height:Math.min(600,parent?parent.height-32:600)
        ColumnLayout {
            anchors.fill:parent
            Label {text:"Public references, selected components and attribution. No machine inventory or local paths.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            ScrollView {Layout.fillWidth:true;Layout.fillHeight:true;TextArea {text:JSON.stringify(page.selection.export || {},null,2);readOnly:true;wrapMode:Text.WrapAnywhere;textFormat:Text.PlainText;Accessible.name:"Exact setup selection export"}}
            Button {text:"Choose export file";enabled:page.current && page.selection.valid&&!page.core.loading;onClicked:exportFile.open()}
        }
    }
    FileDialog {id:exportFile;title:"Export setup selection";fileMode:FileDialog.SaveFile;nameFilters:["OmaStore selection (*.json)"];defaultSuffix:"json";onAccepted:{page.core.communityAction("setups.export",{selection:{id:page.recipe.id,revision:page.recipe.revision,chosen:page.selection.requested || []},snapshot:page.selection.snapshot,file:selectedFile.toString()});exportPreview.close();}}
    FileDialog {id:importFile;title:"Import setup selection";fileMode:FileDialog.OpenFile;nameFilters:["OmaStore selection (*.json)"];onAccepted:page.core.communityAction("setups.import",{file:selectedFile.toString()})}
}
