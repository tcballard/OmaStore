import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

ScrollView {
    id: page
    required property var core
    required property var theme
    property string selectedId: ""
    property string notice: ""
    signal chosen(string id)
    signal browseApps(string makerId)
    readonly property var results: page.core.community["makers.list"] || ({})
    readonly property var profile: page.core.community["makers.get"] || ({})
    readonly property bool showing: selectedId.length>0 && profile.id===selectedId
    contentWidth: availableWidth
    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
    Component.onCompleted: {if(selectedId.length) page.core.communityAction("makers.get",{id:selectedId});}
    Connections {target:core;function onWorkspaceChanged(){const r=page.core.workspaceReply;if(r.action==="feed.export") page.notice=r.error ? "Feed export unavailable: "+r.error.replace(/_/g," ") : "RSS exported. Import it into your feed reader.";}}
    ColumnLayout {
        width:page.availableWidth
        spacing:18
        Item {Layout.preferredHeight:8}
        ColumnLayout {
            Layout.fillWidth:true;Layout.margins:28;spacing:18
            Label {text:"THE PEOPLE BEHIND THE TOOLS";font.family:theme.mono;color:theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap}
            Label {objectName:"makerTitle";text:page.showing ? page.profile.name : "Software has people behind it.";font.pixelSize:32*theme.scale;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label {visible:!!page.notice;text:page.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Flow {
                Layout.fillWidth:true;spacing:8
                Button {text:"All makers";visible:page.selectedId.length>0;onClicked:page.chosen("")}
                Button {text:page.showing?"Export maker release feed":"Export all releases";onClicked:feedFile.open()}
                Button {text:"Project website";visible:page.showing;onClicked:page.core.openMakerLink("homepage")}
                Button {text:"Support the maker";visible:page.showing && !!page.profile.support;onClicked:page.core.openMakerLink("support")}
            }
            Label {visible:page.showing;text:page.profile.bio || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;font.pixelSize:18*theme.scale}
            Label {visible:page.showing;text:page.profile.claimLabel || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;color:theme.muted}
            Label {visible:page.showing && !!page.profile.claimVerifiedAt;text:"Recorded verification: "+(page.profile.claimVerifiedAt || "")+" · expires "+(page.profile.claimExpiresAt || "")+". Current control can change.";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;color:theme.muted}
            Label {visible:page.showing;text:page.profile.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap;color:theme.muted}
            TextField {id:search;visible:!page.selectedId.length;placeholderText:"Find a maker";maximumLength:200;Layout.fillWidth:true;Accessible.name:"Search makers";onTextEdited:makerSearch.restart()}
            Timer {id:makerSearch;interval:150;onTriggered:page.core.communityAction("makers.list",{q:search.text})}
            Label {visible:!page.selectedId.length && !(page.results.items || []).length;text:"Maker profiles appear with reviewed listings. Community nominations keep their unclaimed status until control is verified.";Layout.fillWidth:true;wrapMode:Text.Wrap}
            Repeater {
                model:page.selectedId.length ? [] : (page.results.items || [])
                Frame {
                    required property var modelData
                    Layout.fillWidth:true
                    ColumnLayout {
                        anchors.fill:parent
                        Button {objectName:"maker-"+modelData.id;text:modelData.name;flat:true;font.bold:true;onClicked:{page.core.communityAction("makers.get",{id:modelData.id});page.chosen(modelData.id);}}
                        Label {text:modelData.bio;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                        Label {text:modelData.claimLabel;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;color:theme.muted}
                    }
                }
            }
            RowLayout {
                visible:!page.selectedId.length;Layout.fillWidth:true
                Button {text:"Previous";enabled:(page.results.offset || 0)>0 && !page.core.loading;onClicked:page.core.communityAction("makers.list",{q:search.text,offset:Math.max(0,page.results.offset-30),snapshot:page.results.snapshot})}
                Label {text:(page.results.total || 0)+" makers";Layout.fillWidth:true}
                Button {text:"Next";enabled:page.results.nextOffset!==null && page.results.nextOffset!==undefined && !page.core.loading;onClicked:page.core.communityAction("makers.list",{q:search.text,offset:page.results.nextOffset,snapshot:page.results.snapshot})}
            }
            Label {visible:page.showing;text:"Applications · "+(page.profile.appCount || 0);font.bold:true}
            Repeater {model:page.showing?(page.profile.apps || []):[];AppCard {required property var modelData;app:modelData;theme:page.theme;Layout.fillWidth:true;onChosen:page.core.showApp(app.id)}}
            Button {visible:page.showing && page.profile.appCount>30;text:"Browse all applications from this maker";onClicked:page.browseApps(page.profile.id)}
            Label {visible:page.showing && (page.profile.stories || []).length>0;text:"Editorial contributions";font.bold:true}
            Repeater {model:page.showing?(page.profile.stories || []):[];Label {required property var modelData;text:modelData.title+"\n"+modelData.summary;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
        }
    }
    FileDialog {id:feedFile;title:"Export RSS release feed";fileMode:FileDialog.SaveFile;nameFilters:["RSS feed (*.xml)"];defaultSuffix:"xml";onAccepted:page.core.workspaceAction("feed.export",{makerId:page.selectedId.length?page.selectedId:null,file:selectedFile.toString()})}
}
