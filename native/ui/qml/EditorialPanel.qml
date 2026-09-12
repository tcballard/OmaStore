import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id:panel
    required property var core
    required property var theme
    signal makerChosen(string id)
    property string selectedId:""
    readonly property var feed:panel.core.community["editorial.list"] || ({})
    readonly property var detail:panel.core.community["editorial.get"] || ({})
    readonly property var story:detail.story || ({})
    Layout.fillWidth:true
    spacing:12
    Connections {target:core;function onCommunityChanged(){if(panel.story.id===panel.selectedId && panel.selectedId.length) reading.open();}}
    Label {text:"From the editors";font.pixelSize:22*theme.scale;font.bold:true}
    Label {text:(panel.feed.items || []).length ? panel.feed.notice : "Stories and picks will appear after independent review. Explore the catalogue while the shelves take shape.";Layout.fillWidth:true;wrapMode:Text.Wrap;color:theme.muted}
    Repeater {
        model:panel.feed.items || []
        Frame {
            required property var modelData
            Layout.fillWidth:true
            ColumnLayout {
                anchors.fill:parent
                Label {text:modelData.kind.toUpperCase()+" · "+modelData.publishAt.slice(0,10);font.family:theme.mono;color:theme.muted;textFormat:Text.PlainText}
                Label {text:modelData.title;font.pixelSize:20*theme.scale;font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Label {text:modelData.summary;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Button {objectName:"story-"+modelData.id;text:"Read story";onClicked:{panel.selectedId=modelData.id;panel.core.communityAction("editorial.get",{id:modelData.id});}}
            }
        }
    }
    Dialog {
        id:reading;parent:Overlay.overlay;anchors.centerIn:parent
        width:Math.min(760,parent?parent.width-32:760);height:Math.min(650,parent?parent.height-32:650)
        modal:true;standardButtons:Dialog.Close;title:panel.story.title || "Story"
        onClosed:panel.selectedId=""
        ScrollView {
            id:storyScroll
            anchors.fill:parent;contentWidth:availableWidth;clip:true
            ColumnLayout {
                width:storyScroll.availableWidth;spacing:16
                Label {text:panel.story.body || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
                Button {text:"By "+((panel.detail.maker || {}).name || "the maker");onClicked:{reading.close();panel.makerChosen(panel.story.authorMakerId);}}
                Repeater {model:panel.detail.apps || [];AppCard {core:panel.core;required property var modelData;app:modelData;theme:panel.theme;Layout.fillWidth:true;onChosen:{reading.close();panel.core.showApp(app.id);}}}
                Label {text:"Rights: "+(panel.story.rights || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;color:theme.muted}
            }
        }
    }
}
