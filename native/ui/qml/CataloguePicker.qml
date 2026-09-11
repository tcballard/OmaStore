import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id:picker
    required property var core
    property var selectedIds:[]
    signal toggled(string appId,string releaseId,bool selected)
    readonly property var result:core.community["apps.pick"] || ({})
    property var cursors:[]
    property string cursor:""
    function load(value) {cursor=value;core.communityAction("apps.pick",{q:search.text,cursor:value.length?value:null});}
    Component.onCompleted:load("")
    TextField {id:search;placeholderText:"Find a published app to include";maximumLength:200;Accessible.name:"Find setup components";Layout.fillWidth:true;onTextEdited:{picker.cursors=[];debounce.restart();}}
    Timer {id:debounce;interval:180;onTriggered:picker.load("")}
    Repeater {
        model:picker.result.items || []
        CheckBox {
            required property var modelData
            objectName:"pick-"+modelData.id
            Layout.fillWidth:true
            text:modelData.name+" · "+modelData.priceLabel
            checked:picker.selectedIds.indexOf(modelData.id)>=0
            enabled:!picker.core.loading
            onToggled:picker.toggled(modelData.id,modelData.currentReleaseId,checked)
        }
    }
    RowLayout {
        Layout.fillWidth:true
        Button {text:"Previous";enabled:picker.cursors.length>0&&!picker.core.loading;onClicked:{const stack=picker.cursors.slice();const prior=stack.pop();picker.cursors=stack;picker.load(prior);}}
        Label {text:(picker.result.total || 0)+" matching apps";Layout.fillWidth:true}
        Button {text:"Next";enabled:!!picker.result.nextCursor&&!picker.core.loading;onClicked:{picker.cursors=picker.cursors.concat([picker.cursor]);picker.load(picker.result.nextCursor);}}
    }
}
