import QtQuick
import QtQuick.Controls

Label {
    id: label
    required property var core
    required property var theme
    required property string appId
    readonly property var deviceState: core.appStates[appId] || ({})
    readonly property var operation: core.activity.find(o => o.apps.some(a => a.appId === label.appId && (a.action === "install" || a.action === "remove"))) || ({})
    readonly property string phase: operation.state || ""
    readonly property bool needsReview: phase === "running" || phase === "awaiting_user" || phase === "failed" || phase === "unknown"
    text: (deviceState.stale ? "Last checked · " : "") + (needsReview ? (!core.activityCurrent ? "Progress unavailable · Reconnect" : phase === "running" ? (operation.cancelRequested ? "Finishing current transaction…" : "Package operation in progress…") : phase === "awaiting_user" ? "Waiting for system approval" : phase === "unknown" ? "Outcome needs checking" : "Operation failed · Review available") : deviceState.state === "installed" ? "Installed" : deviceState.state === "update_available" ? "Update available" : deviceState.state === "not_installed" ? "Not installed" : deviceState.state === "external" ? "From developer" : "Device status unavailable")
    color: theme.muted
    font.pixelSize: 11 * theme.scale
    wrapMode: Text.Wrap
    textFormat: Text.PlainText
}
