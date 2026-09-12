import QtQuick

ActionButton {
    id: control
    required property var core
    required property string appId
    property bool secondary: false
    readonly property var appAction: { core.actionRevision; return core.actionForApp(appId); }
    objectName: (secondary ? "secondary-action-" : "app-action-") + appId
    text: (secondary ? appAction.secondaryLabel : appAction.label) || ""
    visible: !secondary || !!appAction.secondaryKind
    enabled: !!appAction.enabled
    onClicked: core.activateAppAction(appId, secondary)
}
