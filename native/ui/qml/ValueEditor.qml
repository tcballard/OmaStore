import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: editor
    required property var value
    required property string field
    required property var theme
    property var path: []
    property int depth: 0
    signal edited(var path, var value)
    readonly property bool composite: value !== null && typeof value === "object"
    readonly property bool array: Array.isArray(value)
    readonly property string title: field.replace(/([A-Z])/g, " $1").replace(/_/g, " ")
    readonly property var choices: ({appType:["desktop","terminal","shell_plugin","web","service"],maturity:["development","preview","stable"],class:["open_source","source_available","proprietary","unknown"],offline:["yes","no","optional","unknown"],account:["yes","no","optional","unknown"],activation:["yes","no","optional","unknown"],model:["free","donation","pay_what_you_want","paid","upgrade","paid_features","subscription","service","working_preview"],tax:["included","excluded","unknown"]})[field] || []
    spacing: 5
    Layout.fillWidth: true
    Loader {
        Layout.fillWidth: true
        sourceComponent: editor.composite ? groupEditor : primitiveEditor
    }
    Component {
        id: primitiveEditor
        ColumnLayout {
            Label { text: editor.title; font.bold: true; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
            RowLayout {
                Layout.fillWidth: true
                ComboBox {
                    visible: editor.choices.length > 0
                    Layout.fillWidth: true
                    model: editor.choices
                    currentIndex: Math.max(0, editor.choices.indexOf(editor.value))
                    Accessible.name: editor.title
                    onActivated: editor.edited(editor.path,currentText)
                }
                CheckBox {
                    visible: typeof editor.value === "boolean"
                    checked: editor.value === true
                    Accessible.name: editor.title
                    onToggled: editor.edited(editor.path,checked)
                }
                TextField {
                    objectName: "field-" + editor.path.join("-")
                    visible: editor.choices.length === 0 && typeof editor.value !== "boolean"
                    Layout.fillWidth: true
                    text: editor.value === null ? "" : String(editor.value)
                    placeholderText: editor.value === null ? "Not supplied" : ""
                    maximumLength: 12000
                    Accessible.name: editor.title
                    onEditingFinished: {
                        const next = typeof editor.value === "number" ? Number(text) : text;
                        if (next !== editor.value && (typeof next !== "number" || (Number.isSafeInteger(next) && next >= 0))) editor.edited(editor.path,next);
                    }
                }
                Button { visible: editor.value === null; text: "Add value"; onClicked: editor.edited(editor.path, editor.field === "price" ? {currency:"GBP",minorUnits:0,exponent:2} : "") }
            }
        }
    }
    Component {
        id: groupEditor
        ColumnLayout {
            spacing: 8
            RowLayout {
                Layout.fillWidth: true
                ToolButton { id: expand; text: (checked ? "▾ " : "▸ ") + editor.title + (editor.array ? " · " + editor.value.length : ""); checkable: true; checked: editor.depth < 3; Layout.fillWidth: true; Accessible.name: editor.title }
                Button {
                    visible: editor.array && editor.value.length < 50
                    text: "Add"
                    onClicked: {
                        const copy = JSON.parse(JSON.stringify(editor.value));
                        const samples = {stories:{id:"",revision:"r1",kind:"story",title:"",summary:"",body:"",authorMakerId:"",appIds:[],publishAt:"",endAt:null,rights:""},media:{kind:"screenshot",url:"",alt:"",rights:"",sha256:""},components:{appId:"",releaseId:"",optional:true,dependsOn:[]}};
                        copy.push(copy.length ? JSON.parse(JSON.stringify(copy[copy.length-1])) : (samples[editor.field] || ""));
                        editor.edited(editor.path,copy);
                    }
                }
            }
            Loader {
                Layout.fillWidth: true
                active: expand.checked && editor.depth < 8
                sourceComponent: ColumnLayout {
                    spacing: 10
                    Repeater {
                        model: Object.keys(editor.value).slice(0,100)
                        RowLayout {
                            required property string modelData
                            Layout.fillWidth: true
                            Loader {
                                Layout.fillWidth: true
                                Component.onCompleted: setSource(Qt.resolvedUrl("ValueEditor.qml"), {
                                    value: Qt.binding(function() {return editor.value[modelData];}),
                                    field: editor.array ? String(Number(modelData)+1) : modelData,
                                    path: editor.path.concat([modelData]), depth: editor.depth+1, theme: editor.theme
                                })
                                onLoaded: item.edited.connect(function(path,value) {editor.edited(path,value);})
                            }
                            ToolButton {
                                visible: editor.array
                                text: "Remove"
                                Accessible.name: "Remove " + editor.title + " item " + (Number(modelData)+1)
                                onClicked: { const copy=JSON.parse(JSON.stringify(editor.value)); copy.splice(Number(modelData),1); editor.edited(editor.path,copy); }
                            }
                        }
                    }
                }
            }
        }
    }
}
