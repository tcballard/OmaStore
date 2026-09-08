import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    required property var core
    required property var desktop
    required property var mediaPreview
    required property var worksheet
    property bool discardClose: false
    property int section: 0
    property bool showFilters: false
    readonly property bool showingDetail: !!core.detail.app
    readonly property var sections: ["Discover", "Apps", "Setups", "Library", "Makers", "Submit"]
    readonly property var d: core.detail
    visible: true
    width: 1180
    height: 820
    minimumWidth: 800
    minimumHeight: 600
    title: (d.app || {}).name ? d.app.name + " · OmaStore" : "OmaStore"
    onClosing: function(event) { if (worksheet.dirty && !discardClose) { event.accepted = false; unsaved.open(); } }
    color: storeTheme.page
    palette.window: storeTheme.page
    palette.base: storeTheme.surface
    palette.button: storeTheme.surface
    palette.windowText: storeTheme.ink
    palette.text: storeTheme.ink
    palette.buttonText: storeTheme.ink
    palette.highlight: storeTheme.accent
    palette.highlightedText: storeTheme.page
    palette.mid: storeTheme.line
    palette.placeholderText: storeTheme.muted

    QtObject {
        id: storeTheme
        readonly property color page: desktop.dark ? "#1d2420" : "#f7f7f2"
        readonly property color surface: desktop.dark ? "#262e28" : "#ffffff"
        readonly property color sidebar: desktop.dark ? "#19201b" : "#edefe7"
        readonly property color ink: desktop.dark ? "#edf0e7" : "#243429"
        readonly property color muted: desktop.dark ? "#b6c0b7" : "#546458"
        readonly property color accent: desktop.dark ? "#b2d495" : "#375c33"
        readonly property color line: desktop.dark ? "#455147" : "#c4ccc0"
        readonly property color wash: desktop.dark ? "#354536" : "#e2ead9"
        readonly property real scale: desktop.textScale
        readonly property string mono: desktop.monoFont
    }
    function navigate(index) { core.closeDetail(); section = index; }
    function readable(value) { return String(value || "unknown").replace(/_/g, " "); }
    function focusSearch() { navigate(1); searchField.forceActiveFocus(); searchField.selectAll(); }
    Shortcut { sequence: "Ctrl+K"; onActivated: window.focusSearch() }
    Shortcut { sequence: "Ctrl+F"; onActivated: window.focusSearch() }
    Shortcut { sequence: "Alt+Left"; enabled: window.showingDetail; onActivated: core.closeDetail() }
    Shortcut { sequence: "Escape"; enabled: window.showingDetail; onActivated: core.closeDetail() }
    Shortcut { sequence: "Ctrl+R"; onActivated: core.refresh() }
    Shortcut { sequence: "Ctrl+Q"; onActivated: window.close() }

    RowLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.preferredWidth: window.width < 950 ? 156 : 196
            Layout.fillHeight: true
            color: storeTheme.sidebar
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 6
                Label { text: "OmaStore"; font.family: storeTheme.mono; font.pixelSize: 23 * storeTheme.scale; font.bold: true; color: storeTheme.ink; Layout.topMargin: 20; Layout.bottomMargin: 4 }
                Label { text: "A place for good tools."; font.pixelSize: 11 * storeTheme.scale; color: storeTheme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.bottomMargin: 32 }
                Repeater {
                    model: window.sections
                    ItemDelegate {
                        required property int index
                        required property string modelData
                        objectName: "nav" + modelData
                        Layout.fillWidth: true
                        implicitHeight: Math.max(42, contentItem.implicitHeight + 20)
                        text: modelData
                        highlighted: window.section === index
                        palette.highlightedText: storeTheme.ink
                        font.bold: highlighted
                        Accessible.role: Accessible.PageTab
                        Accessible.name: modelData
                        Accessible.selected: highlighted
                        background: Rectangle { radius: 5; color: parent.highlighted ? storeTheme.wash : (parent.hovered ? storeTheme.surface : "transparent"); border.color: parent.activeFocus ? storeTheme.accent : "transparent"; border.width: 2 }
                        onClicked: window.navigate(index)
                    }
                }
                Item { Layout.fillHeight: true }
                Label { text: "FOR OMARCHY"; font.family: storeTheme.mono; font.pixelSize: 11 * storeTheme.scale; color: storeTheme.muted; Layout.bottomMargin: 3 }
                Label { text: "Independent community\npreview · 0.1.0"; color: storeTheme.muted; font.pixelSize: 11 * storeTheme.scale; wrapMode: Text.Wrap; Layout.fillWidth: true }
                Button { text: "About & shortcuts"; flat: true; Layout.fillWidth: true; onClicked: about.open() }
            }
        }
        Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; color: storeTheme.line }
        ColumnLayout {
            Layout.fillHeight: true
            Layout.fillWidth: true
            spacing: 0
            Pane {
                Layout.fillWidth: true
                padding: 18
                background: Rectangle { color: storeTheme.page }
                RowLayout {
                    anchors.fill: parent
                    ToolButton { objectName: "backButton"; text: "← Back"; visible: window.showingDetail; onClicked: core.closeDetail() }
                    TextField {
                        id: searchField
                        objectName: "searchField"
                        Layout.fillWidth: true
                        placeholderText: "Search for an app or a purpose…"
                        text: core.query.q || ""
                        maximumLength: 200
                        Accessible.name: "Search applications"
                        onTextEdited: { window.navigate(1); core.setFilter("q", text); }
                    }
                    ToolButton { text: "Refresh"; enabled: core.ready && !core.loading; onClicked: core.refresh(); ToolTip.visible: hovered; ToolTip.text: "Refresh the catalogue · Ctrl+R" }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: storeTheme.line }
            Pane {
                visible: !!core.catalogue.demo || core.catalogue.source === "cached" || core.catalogue.source === "stale" || !!core.catalogue.warning
                Layout.fillWidth: true
                padding: 10
                background: Rectangle { color: storeTheme.wash }
                Label {
                    width: parent.width
                    text: core.catalogue.demo ? "Sample catalogue · All listings are fictional. No app or test claims are real." :
                          (core.catalogue.warning === "cache_write_failed" ? "Catalogue loaded, but it could not be saved for offline use." :
                          (core.catalogue.source === "stale" ? "Could not refresh. You can keep browsing the last available catalogue." : "Browsing a saved catalogue. Refresh to check for changes."))
                    color: storeTheme.ink; wrapMode: Text.Wrap; textFormat: Text.PlainText
                }
            }
            Pane {
                visible: core.error.length > 0
                Layout.fillWidth: true
                padding: 12
                RowLayout {
                    anchors.fill: parent
                    Label { text: core.error; Layout.fillWidth: true; wrapMode: Text.Wrap; textFormat: Text.PlainText }
                    Button { text: core.ready ? "Clear filters" : "Reconnect"; onClicked: core.ready ? core.clearFilters() : core.start() }
                }
            }
            Loader {
                id: body
                Layout.fillWidth: true
                Layout.fillHeight: true
                sourceComponent: window.showingDetail ? detailPage : (window.section === 5 ? submitPage : ((window.section === 0 || window.section === 1 || window.section === 3) ? browsePage : supportingPage))
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: storeTheme.line }
            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                Label { text: core.loading ? "Loading…" : (core.ready ? (core.catalogue.demo ? "SAMPLE MODE" : "CATALOGUE · " + (core.catalogue.revision || "")) : "CONNECTING"); color: storeTheme.muted; font.family: storeTheme.mono; font.pixelSize: 10 * storeTheme.scale; textFormat: Text.PlainText }
                Item { Layout.fillWidth: true }
                Label { text: "Ctrl+K  Search"; color: storeTheme.muted; font.family: storeTheme.mono; font.pixelSize: 10 * storeTheme.scale }
            }
        }
    }

    Component {
        id: browsePage
        ScrollView {
            id: browseScroll
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                width: browseScroll.availableWidth
                spacing: 22
                Item { Layout.preferredHeight: 2 }
                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: 28; Layout.rightMargin: 28
                    spacing: 10
                    Label { text: window.section === 3 ? "YOUR OWN SHORTLIST" : (window.section === 0 ? "THE COMMUNITY STOREFRONT" : "FIND SOMETHING USEFUL"); color: storeTheme.muted; font.family: storeTheme.mono; font.pixelSize: 11 * storeTheme.scale; font.letterSpacing: 1 }
                    Label { text: window.section === 3 ? "Saved for later." : (window.section === 0 ? "Find your next\nfavourite tool." : "Apps, on your terms."); color: storeTheme.ink; font.pixelSize: (window.width < 950 ? 32 : 42) * storeTheme.scale; font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Label { text: window.section === 3 ? "A local shortlist on this device. Installed-app management is still to come." : "See what it does. Know what it needs. Support the people who make it."; color: storeTheme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.maximumWidth: 630 }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.leftMargin: 28; Layout.rightMargin: 28
                    Label { text: window.section === 3 ? core.saved.length + " saved" : core.total + (core.total === 1 ? " application" : " applications"); font.bold: true; color: storeTheme.ink }
                    Item { Layout.fillWidth: true }
                    Button { text: window.showFilters ? "Hide filters" : "Filters"; visible: window.section !== 3; onClicked: window.showFilters = !window.showFilters }
                    Button { text: "Clear"; visible: window.section !== 3 && Object.keys(core.query).length > 0; onClicked: core.clearFilters() }
                }
                Flow {
                    visible: window.showFilters && window.section !== 3
                    Layout.fillWidth: true; Layout.leftMargin: 28; Layout.rightMargin: 28
                    spacing: 10
                    Repeater {
                        model: [
                            { key: "category", label: "Category", values: core.catalogue.categories || [] },
                            { key: "appType", label: "App type", values: ["desktop", "terminal", "shell_plugin", "web", "service"] },
                            { key: "price", label: "Price", values: ["free", "donation", "paid", "pay_what_you_want", "subscription", "upgrade", "paid_features", "service", "working_preview"] },
                            { key: "licence", label: "Licence", values: ["open_source", "source_available", "proprietary", "unknown"] },
                            { key: "architecture", label: "Architecture", values: ["x86_64", "aarch64", "any"] },
                            { key: "offline", label: "Works offline", values: ["yes", "no", "optional", "unknown"] },
                            { key: "evidence", label: "Omarchy test", values: ["not_tested", "passes", "limitations", "fails", "retest_due"] }
                        ]
                        ComboBox {
                            required property var modelData
                            width: Math.max(168, implicitWidth)
                            model: [modelData.label + ": any"].concat(modelData.values.map(function(v) { return window.readable(v); }))
                            currentIndex: Math.max(0, modelData.values.indexOf(core.query[modelData.key]) + 1)
                            Accessible.name: modelData.label
                            onActivated: core.setFilter(modelData.key, currentIndex === 0 ? "" : modelData.values[currentIndex - 1])
                        }
                    }
                }
                GridLayout {
                    id: cards
                    Layout.fillWidth: true; Layout.leftMargin: 28; Layout.rightMargin: 28
                    columns: width >= 710 ? 2 : 1
                    columnSpacing: 14; rowSpacing: 14
                    Repeater {
                        model: window.section === 3 ? core.saved : core.apps
                        AppCard {
                            required property var modelData
                            app: modelData
                            bookmark: window.section === 3
                            theme: storeTheme
                            Layout.fillWidth: true
                            Layout.preferredWidth: (cards.width - (cards.columns - 1) * cards.columnSpacing) / cards.columns
                            onChosen: core.showApp(app.id)
                            onRemoveRequested: core.removeSaved(app.id)
                        }
                    }
                }
                ColumnLayout {
                    visible: core.loaded && (window.section === 3 ? core.saved.length === 0 : core.total === 0)
                    Layout.fillWidth: true; Layout.margins: 28
                    spacing: 16
                    Label { text: window.section === 3 ? "Keep a few good possibilities." : (core.catalogue.appCount > 0 ? "Nothing matches those filters." : "The shelves are ready."); font.pixelSize: 25 * storeTheme.scale; font.bold: true; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    Label { text: window.section === 3 ? "Open an app and choose Save for later. Your shortlist stays on this device." : (core.catalogue.appCount > 0 ? "Try a broader search, or clear the filters to see everything." : "OmaStore is being built in the open. Real listings will appear after their makers, sources and evidence have been reviewed."); color: storeTheme.muted; wrapMode: Text.Wrap; Layout.fillWidth: true; Layout.maximumWidth: 600 }
                    Button { text: window.section === 3 ? "Explore apps" : "Clear filters"; visible: window.section === 3 || Object.keys(core.query).length > 0; onClicked: window.section === 3 ? window.navigate(1) : core.clearFilters() }
                }
                Button { text: "Load more"; visible: core.hasMore && window.section !== 3; enabled: !core.loading; Layout.alignment: Qt.AlignHCenter; onClicked: core.nextPage() }
                Item { Layout.preferredHeight: 24 }
            }
        }
    }

    Component {
        id: detailPage
        DetailPage { details: window.d; theme: storeTheme; core: window.core; mediaPreview: window.mediaPreview }
    }
    Component { id: submitPage; SubmitPage { worksheet: window.worksheet; core: window.core; theme: storeTheme } }
    Dialog {
        id: unsaved
        anchors.centerIn: parent
        width: Math.min(520, window.width - 64)
        modal: true
        title: "Keep your worksheet changes?"
        contentItem: ColumnLayout {
            spacing: 14
            Label { text: "The submission worksheet has unsaved changes on this device."; wrapMode: Text.Wrap; Layout.fillWidth: true }
            Label { text: worksheet.status; wrapMode: Text.Wrap; Layout.fillWidth: true }
            RowLayout {
                Button { text: "Save & quit"; onClicked: { if (worksheet.save()) { window.discardClose = true; window.close(); } } }
                Button { text: "Discard & quit"; onClicked: { window.discardClose = true; window.close(); } }
                Button { text: "Keep editing"; onClicked: unsaved.close() }
            }
        }
    }
    Component {
        id: supportingPage
        ScrollView {
            id: supportingScroll
            contentWidth: availableWidth
            clip: true
            ColumnLayout {
                width: supportingScroll.availableWidth
                spacing: 20
                Item { Layout.preferredHeight: 24 }
                Label { text: window.sections[window.section]; font.pixelSize: 36 * storeTheme.scale; font.bold: true; Layout.margins: 28 }
                Label {
                    Layout.leftMargin: 28; Layout.rightMargin: 28; Layout.fillWidth: true
                    text: window.section === 2 ? "A good setup is more than a list of apps." : (window.section === 4 ? "Software has people behind it." : "Bring something useful to the shelf.")
                    font.pixelSize: 24 * storeTheme.scale; wrapMode: Text.Wrap
                }
                Label {
                    Layout.leftMargin: 28; Layout.rightMargin: 28; Layout.fillWidth: true
                    text: window.section === 2 ? "Published setups will explain a workflow, show each component and let you choose what belongs on your machine. Setup publication and installation are still being built." :
                          (window.section === 4 ? "Maker profiles will arrive with reviewed listings. A community nomination will stay clearly unclaimed until project control is verified." :
                          "Submissions will be free. Authors can offer free software, receive support or sell through their own checkout with no OmaStore fee. Author sign-in and private submission workspaces are still being built.")
                    color: storeTheme.muted; wrapMode: Text.Wrap
                }
                Label { visible: window.section === 5; text: "The first review will ask for purpose, source or publisher identity, exact release, price and limits, real media, and Omarchy evidence. Paid placement will not buy approval."; Layout.margins: 28; Layout.fillWidth: true; wrapMode: Text.Wrap; color: storeTheme.muted }
                Item { Layout.preferredHeight: 30 }
            }
        }
    }
    Dialog {
        id: about
        anchors.centerIn: parent
        width: Math.min(520, window.width - 64)
        modal: true
        title: "OmaStore · Native preview"
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            spacing: 16
            Label { text: "An independent community storefront for Omarchy.\nQt Quick interface · Rust catalogue core " + core.version; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label { text: "Ctrl+K / Ctrl+F   Search\nAlt+Left / Escape   Back from an app\nCtrl+R   Refresh catalogue\nCtrl+Q   Quit"; font.family: storeTheme.mono; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label { text: "This preview browses listings and saves a local shortlist. Package installation, author accounts and managed checkout are still to come."; Layout.fillWidth: true; wrapMode: Text.Wrap }
        }
    }
}
