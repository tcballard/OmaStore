import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: hero
    required property var theme
    required property var core
    required property var details
    property bool overview: false
    signal inspectRequested()
    readonly property var deviceState: core.appStates[app.id] || ({})
    readonly property var appAction: { core.actionRevision; return core.actionForApp(app.id || ""); }
    readonly property var app: details.app || ({})
    readonly property bool calculator: app.id === "repo-omacalc"
    readonly property bool compact: width < 1000 || theme.scale > 1.4
    readonly property real displaySize: theme.following ? (compact ? 36 : 54) : (compact ? 48 : Math.max(56, Math.min(96, (width - 680) * 0.119)))
    implicitHeight: Math.max(compact ? 510 : 620, composition.implicitHeight + 80)
    GridLayout {
        id: composition
        anchors.left: parent.left; anchors.right: parent.right
        anchors.leftMargin: hero.compact ? 28 : 92
        anchors.rightMargin: hero.compact ? 28 : 136
        anchors.verticalCenter: parent.verticalCenter
        columns: hero.compact ? 1 : 2
        columnSpacing: 90; rowSpacing: 30
        ColumnLayout {
            Layout.fillWidth: true; Layout.preferredWidth: 640
            spacing: 22
            Label { text: (hero.app.category || "APP").toUpperCase() + "  /  " + (hero.app.name || "").toUpperCase(); color: theme.muted; font.pixelSize: 10 * theme.scale; font.letterSpacing: 2; textFormat: Text.PlainText; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label {
                text: hero.calculator ? "A little space\nfor everyday maths." : hero.app.name || ""
                font.family: theme.display; font.pixelSize: hero.displaySize * theme.scale
                font.weight: Font.DemiBold
                color: theme.ink; lineHeightMode: Text.FixedHeight; lineHeight: hero.displaySize * (theme.following ? 1.15 : 0.96) * theme.scale; wrapMode: Text.Wrap; textFormat: Text.PlainText
                Layout.fillWidth: true
            }
            Label { text: hero.calculator ? "A focused calculator with keyboard input\nand your expression above the result." : hero.app.summary || ""; color: theme.muted; font.pixelSize: (hero.compact ? 17 : 24) * theme.scale; lineHeight: 1.2; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
            RowLayout {
                Layout.topMargin: 10; spacing: 26
                AppSymbol { theme: hero.theme; category: hero.app.category || "utilities"; size: 80 }
                ColumnLayout {
                    Layout.fillWidth: true
                    Label { text: hero.app.name || ""; color: theme.ink; font.pixelSize: 30 * theme.scale; font.bold: true; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                    Label { text: hero.overview ? "Community listing" : (hero.details.makers || []).map(m => m.name + (m.claim === "unclaimed" ? " · Community listing" : "")).join(" · "); color: theme.muted; font.pixelSize: 13 * theme.scale; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                }
            }
            Flow {
                Layout.fillWidth: true; spacing: 16
                ActionButton {
                    theme: hero.theme; primary: true
                    textSize: 18; implicitWidth: 296; implicitHeight: 56
                    objectName: hero.overview ? "discoverCalculator" : "previewAppPlan"
                    text: hero.overview ? "Explore OmaCalc" : hero.appAction.label
                    enabled: hero.overview ? core.ready && !core.loading : !!hero.appAction.enabled
                    onClicked: hero.overview ? hero.inspectRequested() : core.activateAppAction(hero.app.id)
                }
                AppActionButton {core:hero.core;theme:hero.theme;appId:hero.app.id || "";secondary:true;visible:!hero.overview&&!!hero.appAction.secondaryKind}
                Label { text: hero.overview ? (hero.app.priceLabel || "") : (hero.details.summary || {}).priceLabel || ""; color: theme.ink; font.pixelSize: 14 * theme.scale; topPadding: 10; textFormat: Text.PlainText }
            }
            AppStateLabel { id: statusLabel; core: hero.core; theme: hero.theme; appId: hero.app.id || ""; Layout.fillWidth: true }
            Label {visible:!hero.overview && !!hero.deviceState.installedVersion;text:"Installed: "+hero.deviceState.installedVersion+(hero.deviceState.availableVersion?" · Available: "+hero.deviceState.availableVersion:"");color:theme.muted;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
            Label { text: (hero.details.summary || {}).evidenceLabel || hero.app.evidenceLabel || "Not tested on Omarchy"; color: theme.muted; font.pixelSize: 12 * theme.scale; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Rectangle { visible: !hero.overview; Layout.fillWidth: true; height: 1; color: theme.line; Layout.topMargin: 12 }
            Flow {
                visible: !hero.overview; Layout.fillWidth: true; spacing: 18
                Label { text: "Version " + ((hero.details.release || {}).version || "Unknown"); color: theme.muted; font.pixelSize: 12 * theme.scale; topPadding: 10 }
                Label { text: (hero.app.licence || {}).identifier || "Unknown licence"; color: theme.muted; font.pixelSize: 12 * theme.scale; topPadding: 10 }
                ToolButton { text: "View source"; visible: !!hero.app.source; onClicked: core.openLink("source") }
            }
        }
        ColumnLayout {
            visible: hero.calculator
            Layout.preferredWidth: hero.compact ? 280 : 400
            Layout.alignment: Qt.AlignHCenter
            spacing: 12
            Image {
                source: "qrc:/qt/qml/OmaStore/assets/omacalc-upstream.png"
                Layout.preferredWidth: hero.compact ? 260 : Math.min(400, hero.width * 0.30)
                Layout.preferredHeight: width * 1.55
                fillMode: Image.PreserveAspectFit; sourceSize.width: 800
                Accessible.role: Accessible.Graphic
                Accessible.name: "Upstream OmaCalc screenshot showing its keypad and the result 133"
            }
            Label { text: "Upstream preview · Tokyo Night"; color: theme.muted; font.pixelSize: 11 * theme.scale; Layout.alignment: Qt.AlignHCenter }
        }
    }

}
