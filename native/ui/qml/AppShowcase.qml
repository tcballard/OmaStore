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
    readonly property var app: details.app || ({})
    readonly property bool calculator: app.id === "repo-omacalc"
    readonly property bool compact: width < 1000 || theme.scale > 1.4
    implicitHeight: Math.max(compact ? 510 : 620, composition.implicitHeight + 80)
    GridLayout {
        id: composition
        anchors.left: parent.left; anchors.right: parent.right
        anchors.margins: hero.compact ? 28 : 64
        anchors.verticalCenter: parent.verticalCenter
        columns: hero.compact ? 1 : 2
        columnSpacing: 70; rowSpacing: 30
        ColumnLayout {
            Layout.fillWidth: true; Layout.preferredWidth: 640
            spacing: 22
            Label { text: (hero.app.category || "APP").toUpperCase() + "  /  " + (hero.app.name || "").toUpperCase(); color: theme.muted; font.pixelSize: 10 * theme.scale; font.letterSpacing: 2; textFormat: Text.PlainText; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label {
                text: hero.calculator ? "A little space\nfor everyday maths." : hero.app.name || ""
                font.family: theme.display; font.pixelSize: (hero.compact ? 48 : 72) * theme.scale
                color: theme.ink; lineHeight: 0.98; wrapMode: Text.Wrap; textFormat: Text.PlainText
                Layout.fillWidth: true
            }
            Label { text: hero.calculator ? "A focused calculator with keyboard input\nand your expression above the result." : hero.app.summary || ""; color: theme.muted; font.pixelSize: 18 * theme.scale; lineHeight: 1.2; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
            RowLayout {
                Layout.topMargin: 10; spacing: 18
                AppSymbol { theme: hero.theme; category: hero.app.category || "utilities"; size: 64 }
                ColumnLayout {
                    Layout.fillWidth: true
                    Label { text: hero.app.name || ""; color: theme.ink; font.pixelSize: 24 * theme.scale; font.bold: true; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                    Label { text: hero.overview ? "Community listing" : (hero.details.makers || []).map(m => m.name).join(" · ") + " · Community listing"; color: theme.muted; font.pixelSize: 13 * theme.scale; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                }
            }
            Flow {
                Layout.fillWidth: true; spacing: 16
                ActionButton {
                    theme: hero.theme; primary: true
                    objectName: hero.overview ? "discoverCalculator" : "previewAppPlan"
                    text: hero.overview ? "Explore OmaCalc" : core.loading ? "Preparing…" : "Review installation"
                    enabled: core.ready && !core.loading
                    onClicked: hero.overview ? hero.inspectRequested() : core.communityAction("system.plan", {kind:"app",id:hero.app.id})
                }
                Label { text: hero.overview ? (hero.app.priceLabel || "") : (hero.details.summary || {}).priceLabel || ""; color: theme.ink; font.pixelSize: 14 * theme.scale; topPadding: 10; textFormat: Text.PlainText }
            }
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
            Layout.preferredWidth: hero.compact ? 280 : 390
            Layout.alignment: Qt.AlignHCenter
            spacing: 12
            Image {
                source: "qrc:/qt/qml/OmaStore/assets/omacalc-upstream.png"
                Layout.preferredWidth: hero.compact ? 260 : Math.min(390, hero.width * 0.30)
                Layout.preferredHeight: width * 1.55
                fillMode: Image.PreserveAspectFit; sourceSize.width: 800
                Accessible.role: Accessible.Graphic
                Accessible.name: "Upstream OmaCalc screenshot showing its keypad and the result 133"
            }
            Label { text: "Upstream preview · Tokyo Night"; color: theme.muted; font.pixelSize: 11 * theme.scale; Layout.alignment: Qt.AlignHCenter }
        }
    }
}
