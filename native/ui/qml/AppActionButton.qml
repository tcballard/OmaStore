import QtQuick

ActionButton {
    id: control
    required property var core
    required property string appId
    property bool secondary: false
    readonly property var action: { core.actionRevision; return core.actionForApp(appId); }
    objectName: (secondary ? "secondary-action-" : "app-action-") + appId
    text: (secondary ? action.secondaryLabel : action.label) || ""
    visible: !secondary || !!action.secondaryKind
    enabled: !!action.enabled
    onClicked: core.activateAppAction(appId, secondary)
}
