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
    property string setupSelection:""
    property string setupRevision:""
    readonly property bool showingSubpage:showingDetail || (section===2 && !!setupSelection) || (section===4 && !!makerSelection)
    function back(){if(showingDetail)core.closeDetail();else if(section===2){setupSelection="";setupRevision="";}else if(section===4)makerSelection="";}
    property string makerSelection: ""
    function showMaker(id) { makerSelection=id; navigate(4); core.communityAction("makers.get",{id:id}); }
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
    font.family: storeTheme.body
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

    StoreTheme { id: storeTheme; desktop: window.desktop }
    function navigate(index) { core.closeDetail(); if(index !== section) browsePosition = 0; section = index; if (index === 0 && Object.keys(core.query).length) core.clearFilters(); }
    property real browsePosition: 0
    property string lastAppId: ""
    property string lastFocusName: ""
    function openApp(id, origin) { lastAppId = id; lastFocusName = origin || "app-" + id; if (section === 0) section = 1; core.showApp(id); }
    function readable(value) { return String(value || "unknown").replace(/_/g, " "); }
    function focusSearch() { navigate(1); topHeader.searchInput.forceActiveFocus(); topHeader.searchInput.selectAll(); }
    Shortcut { sequence: "Ctrl+K"; onActivated: window.focusSearch() }
    Shortcut { sequence: "Ctrl+F"; onActivated: window.focusSearch() }
    Shortcut { sequence: "Alt+Left"; enabled: window.showingSubpage && !planDialog.visible && !settingsDialog.visible && !remixDialog.visible && !purchasesDialog.visible; onActivated: window.back() }
    Shortcut { sequence: "Escape"; enabled: window.showingSubpage && !planDialog.visible && !settingsDialog.visible && !remixDialog.visible && !purchasesDialog.visible; onActivated: window.back() }
    Shortcut { sequence: "Ctrl+R"; onActivated: core.refresh() }
    Shortcut { sequence: "Ctrl+Q"; onActivated: window.close() }

    // Keep focused controls visible when the editorial view stacks under tiling.
    onActiveFocusItemChanged: Qt.callLater(function() {
        const item = window.activeFocusItem;
        if (!item) return;
        let ancestor = item.parent;
        while (ancestor) {
            if (typeof ancestor.contentY === "number" && ancestor.contentHeight > ancestor.height) {
                const point = item.mapToItem(ancestor, 0, 0);
                if (point.y < 0) ancestor.contentY += point.y - 8;
                else if (point.y + item.height > ancestor.height) ancestor.contentY += point.y + item.height - ancestor.height + 8;
                break;
            }
            ancestor = ancestor.parent;
        }
    })

    Connections {
        target:window.core
        function onHandoffReady(identity){if(identity.kind==="setup"){window.setupSelection=identity.id;window.setupRevision=identity.revision;window.navigate(2);}}
    }
    Component {id:libraryPage;LibraryPage {core:window.core;theme:storeTheme;onPurchasesRequested:purchasesDialog.openFor("");onRemixesRequested:remixDialog.openFor(null);onSettingsRequested:settingsDialog.openFor([])}}
    RemixDialog {id:remixDialog;core:window.core;theme:storeTheme;onSettingsRequested:(refs)=>settingsDialog.openFor(refs);onAuthorCreated:window.navigate(5)}
    PurchasesDialog {id:purchasesDialog;core:window.core;theme:storeTheme}
    SettingsDialog {id:settingsDialog;core:window.core;theme:storeTheme}
    PlanDialog {id:planDialog;core:window.core;theme:storeTheme}

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        StoreHeader {
            id: topHeader
            core: window.core; canGoBack: window.showingSubpage
            onBackRequested: window.back()
            onSearchRequested: (query) => { window.navigate(1); core.setFilter("q", query); }
            Layout.fillWidth: true; theme: storeTheme; section: window.section
            onNavigate: (index) => window.navigate(index)
            onAboutRequested: about.open()
        }
        ColumnLayout {
            Layout.fillHeight: true
            Layout.fillWidth: true
            spacing: 0
            Pane {
                visible: !!core.catalogue.demo || core.catalogue.source === "cached" || core.catalogue.source === "stale" || !!core.catalogue.warning
                Layout.fillWidth: true
                padding: 10
                background: Rectangle { color: storeTheme.wash }
                Label {
                    width: parent.width
                    text: core.catalogue.demo ? "Sample catalogue · All listings are fictional. No app or test claims are real." :
                          (core.catalogue.warning === "cache_write_failed" ? "Catalogue loaded, but it could not be saved for offline use." :
                          (core.catalogue.source === "stale" ? "Could not refresh. You can keep browsing the last available catalogue." : "Browsing a saved catalogue. Package updates are checked automatically."))
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
                sourceComponent: window.showingDetail ? detailPage : (window.section === 0 && core.apps.some(a => a.id === "repo-omacalc") ? showcasePage : (window.section === 3 ? libraryPage : window.section === 2 ? setupsPage : window.section === 4 ? makersPage : window.section === 5 ? submitPage : ((window.section === 0 || window.section === 1 || window.section === 3) ? browsePage : supportingPage)))
            }
            AppShelf {
                visible: (window.section === 0 || window.section === 1) && core.apps.length > 0
                Layout.fillWidth: true; theme: storeTheme; core: window.core
                selectedId: (core.detail.app || {}).id || (window.section === 0 ? "repo-omacalc" : "")
                onChosen: (id) => window.openApp(id, "shelf-" + id)
                onBrowseRequested: { window.navigate(1); window.showFilters = true; }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: storeTheme.line }
            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                Label { text: core.loading ? "Loading…" : (core.ready ? (core.catalogue.demo ? "SAMPLE MODE" : (core.catalogue.repositoryCheckedAt ? "Packages checked " + new Date(core.catalogue.repositoryCheckedAt).toLocaleString(Qt.locale(), Locale.ShortFormat) : "Catalogue available offline")) : "CONNECTING"); color: storeTheme.muted; font.family: storeTheme.mono; font.pixelSize: 10 * storeTheme.scale; textFormat: Text.PlainText }
                Item { Layout.fillWidth: true }
                Label { text: "Independent community project · Not affiliated with Omacom"; color: storeTheme.muted; font.family: storeTheme.mono; font.pixelSize: 10 * storeTheme.scale }
            }
        }
    }

    Component {
        id: showcasePage
        ScrollView {
            id: showcaseScroll
            objectName: "browseScroll"
            contentWidth: availableWidth; clip: true
            ColumnLayout {
                width: showcaseScroll.availableWidth
                AppShowcase {
                    Layout.fillWidth: true; theme: storeTheme; core: window.core
                    details: ({app: core.apps.find(a => a.id === "repo-omacalc") || {}})
                    overview: true
                    onInspectRequested: window.openApp("repo-omacalc", "discoverCalculator")
                }
            }
        }
    }

    Component {
        id: browsePage
        ScrollView {
            id: browseScroll
            objectName: "browseScroll"
            Component.onCompleted: Qt.callLater(function() {
                function locate(item) {
                    if (item.objectName === window.lastFocusName) return item;
                    const children = item.children || [];
                    for (let i=0; i<children.length; ++i) { const found=locate(children[i]); if(found) return found; }
                    return null;
                }
                if (window.lastFocusName) { const control=locate(browseScroll); if(control) control.forceActiveFocus(Qt.BacktabFocusReason); }
                contentItem.contentY = window.browsePosition;
            })
            Component.onDestruction: { if (contentItem) window.browsePosition = contentItem.contentY; }
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                width: browseScroll.availableWidth
                spacing: 22
                Item { Layout.preferredHeight: 2 }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 6
                        Label { text: window.section === 0 ? "Discover" : "Find your next app"; color: storeTheme.ink; font.pixelSize: storeTheme.titleSize * storeTheme.scale; font.weight: Font.Bold; Layout.fillWidth: true; wrapMode: Text.Wrap }
                        Label { text: window.section === 0 ? "Good tools. A desktop that feels like yours." : "Explore the catalogue, one useful tool at a time."; color: storeTheme.muted; font.pixelSize: 14 * storeTheme.scale; wrapMode: Text.Wrap; Layout.fillWidth: true }
                    }
                }
                DiscoveryFeature {
                    visible: window.section === 0 && core.apps.some(a => a.id === "repo-omacalc")
                    Layout.fillWidth: true; Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
                    theme: storeTheme
                    onChosen: window.openApp("repo-omacalc", "discoverCalculator")
                }
                ColumnLayout {
                    visible: window.section === 0 && (core.catalogue.categories || []).length > 0
                    Layout.fillWidth: true; Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
                    spacing: 12
                    Label { text: "What would you like to do?"; color: storeTheme.ink; font.pixelSize: storeTheme.sectionSize * storeTheme.scale; font.weight: Font.DemiBold }
                    Flow {
                        Layout.fillWidth: true; spacing: 8
                        Repeater {
                            model: core.catalogue.categories || []
                            ActionButton {
                                required property string modelData
                                objectName: "purpose-" + modelData
                                theme: storeTheme
                                text: ({writing:"Write something",video:"Work with video",presentations:"Tell a story",utilities:"Everyday essentials",games:"Take a break"})[modelData] || window.readable(modelData)
                                Accessible.name: "Browse " + window.readable(modelData)
                                onClicked: { window.navigate(1); core.clearFilters(); core.setFilter("category", modelData); }
                            }
                        }
                    }
                }
                EditorialPanel {visible:window.section===0 && ((core.community["editorial.list"] || {}).items || []).length > 0;Layout.leftMargin:storeTheme.inset;Layout.rightMargin:storeTheme.inset;core:window.core;theme:storeTheme;onMakerChosen:(id)=>window.showMaker(id)}
                RowLayout {
                    Layout.fillWidth: true; Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
                    Label { text: window.section === 0 ? "Explore the apps" : core.total + (core.total === 1 ? " application" : " applications"); font.pixelSize: storeTheme.sectionSize * storeTheme.scale; font.weight: Font.DemiBold; color: storeTheme.ink }
                    Item { Layout.fillWidth: true }
                    ActionButton { theme: storeTheme; text: window.showFilters ? "Hide filters" : "Filters"; visible: window.section !== 3; onClicked: window.showFilters = !window.showFilters }
                    ActionButton { theme: storeTheme; text: "Clear"; visible: window.section !== 3 && Object.keys(core.query).length > 0; onClicked: core.clearFilters() }
                }
                Flow {
                    visible: window.showFilters && window.section !== 3
                    Layout.fillWidth: true; Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
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
                    Layout.fillWidth: true; Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset
                    columns: width >= 710 ? 2 : 1
                    columnSpacing: 16; rowSpacing: 12
                    Repeater {
                        model: window.section === 3 ? core.saved : core.apps
                        AppCard {
                            required property var modelData
                            app: modelData
                            bookmark: window.section === 3
                            theme: storeTheme
                            Layout.fillWidth: true
                            Layout.preferredWidth: (cards.width - (cards.columns - 1) * cards.columnSpacing) / cards.columns
                            onChosen: window.openApp(app.id)
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
        DetailPage { onPurchasesRequested:(id)=>purchasesDialog.openFor(id);onMakerChosen:(id)=>window.showMaker(id); details: window.d; theme: storeTheme; core: window.core; mediaPreview: window.mediaPreview }
    }
    Component {id:setupsPage;SetupsPage {core:window.core;theme:storeTheme;mediaPreview:window.mediaPreview;selectedId:window.setupSelection;selectedRevision:window.setupRevision;onChosen:(id,revision)=>{window.setupSelection=id;window.setupRevision=revision;};onMakerChosen:(id)=>window.showMaker(id);onRemixRequested:(selection)=>remixDialog.openFor(selection);onSettingsRequested:(references)=>settingsDialog.openFor(references)}}
    Component {id:makersPage;MakersPage {core:window.core;theme:storeTheme;selectedId:window.makerSelection;onChosen:(id)=>window.makerSelection=id;onBrowseApps:(id)=>{window.navigate(1);core.clearFilters();core.setFilter("makerId",id);}}}
    Component { id: submitPage; WorkspacePage { worksheet: window.worksheet; core: window.core; theme: storeTheme } }
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
                    Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset; Layout.fillWidth: true
                    text: window.section === 2 ? "A good setup is more than a list of apps." : (window.section === 4 ? "Software has people behind it." : "Bring something useful to the shelf.")
                    font.pixelSize: 24 * storeTheme.scale; wrapMode: Text.Wrap
                }
                Label {
                    Layout.leftMargin: storeTheme.inset; Layout.rightMargin: storeTheme.inset; Layout.fillWidth: true
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
            Label { text: "Development build. App and setup discovery, private submissions and review workflows are available. Sample sessions and publication rehearsals are explicitly fictional. See the repository handoff for current verification."; Layout.fillWidth: true; wrapMode: Text.Wrap }
        }
    }
}
