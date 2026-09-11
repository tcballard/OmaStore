import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: window
    required property var core
    property int section: 0
    property bool filtersOpen: false
    property bool detailOpen: false
    property var filters: Object.assign({}, core.filters)
    readonly property var sections: ["Discover", "Apps", "Setups", "Library", "Makers", "Submit"]
    readonly property var d: core.detail
    readonly property var release: d.releases ? d.releases.find(r => r.id === d.currentRelease) : null
    visible: true
    width: 1040; height: 760
    minimumWidth: 800; minimumHeight: 600
    title: "OmaStore"
    function words(value) { return value ? String(value).replace(/_/g, " ") : "Unknown" }
    function updateFilter(key, value) {
        let next = Object.assign({}, filters); next[key] = value; filters = next
        queryDelay.restart()
    }
    function clearFilters() { filters = {}; queryDelay.stop(); core.search(filters); searchInput.forceActiveFocus() }
    function filterIndex(model, key) { let i = model.findIndex(x => x.value === (filters[key] || "")); return Math.max(0, i) }
    function openApp(id) { detailOpen = true; core.showApp(id) }
    function back() { detailOpen = false; core.clearDetail(); searchInput.forceActiveFocus() }
    Timer { id: queryDelay; interval: 60; onTriggered: window.core.search(window.filters) }
    Shortcut { sequence: "Ctrl+F"; onActivated: { window.detailOpen = false; window.section = 1; searchInput.forceActiveFocus() } }
    Shortcut { sequence: "Alt+Left"; enabled: window.detailOpen; onActivated: window.back() }
    Shortcut { sequence: "Escape"; enabled: window.detailOpen; onActivated: window.back() }
    component Copy: Label { textFormat: Text.PlainText; wrapMode: Text.Wrap; font.pixelSize: 15; Layout.fillWidth: true }
    component Heading: Copy { font.pixelSize: 24; font.bold: true }
    component Filter: ComboBox {
        required property string filterKey
        textRole: "text"; valueRole: "value"
        Layout.preferredWidth: 165; Layout.fillWidth: true
        currentIndex: window.filterIndex(model, filterKey)
        onActivated: window.updateFilter(filterKey, currentValue)
        Accessible.name: filterKey + " filter"
    }
    header: ToolBar {
        RowLayout {
            anchors.fill: parent; anchors.leftMargin: 20; anchors.rightMargin: 16
            Label { text: "OmaStore"; font.pixelSize: 22; font.bold: true }
            Label { text: "Discovery preview"; font.pixelSize: 13 }
            Item { Layout.fillWidth: true }
            BusyIndicator { running: window.core.busy; visible: running; Layout.preferredWidth: 24; Layout.preferredHeight: 24 }
            ToolButton { text: "Refresh"; enabled: window.core.ready && !window.core.busy; onClicked: window.core.refresh(); Accessible.name: "Refresh catalogue" }
            ToolButton { text: "About"; onClicked: about.open() }
        }
    }
    RowLayout {
        anchors.fill: parent; spacing: 0
        Pane {
            Layout.preferredWidth: window.width < 950 ? 145 : 175; Layout.fillHeight: true; padding: 10
            ColumnLayout {
                anchors.fill: parent; spacing: 5
                Repeater {
                    model: window.sections
                    ItemDelegate {
                        required property int index
                        required property string modelData
                        text: modelData; font.pixelSize: 16; Layout.fillWidth: true
                        highlighted: window.section === index
                        Accessible.role: Accessible.PageTab; Accessible.name: modelData; Accessible.selected: window.section === index
                        onClicked: { window.section = index; window.detailOpen = false; window.core.clearDetail() }
                    }
                }
                Item { Layout.fillHeight: true }
                Copy { text: "Independent\ncommunity project"; font.pixelSize: 12 }
            }
        }
        ToolSeparator { orientation: Qt.Vertical; Layout.fillHeight: true }
        ColumnLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.margins: window.width < 950 ? 16 : 28; spacing: 14
            Copy {
                objectName: "catalogueStatus"
                text: window.core.error || window.core.catalogueError ||
                    (window.core.catalogueState.stale ? "Browsing " + (window.core.catalogueState.source || "bundled") + " catalogue. " + (window.core.catalogueState.error ? "Refresh unavailable; saved information remains usable." : "Refresh to check for changes.") : "Catalogue refreshed.")
                font.pixelSize: 13
                Accessible.role: Accessible.StaticText
            }
            ColumnLayout {
                visible: window.section <= 1 && !window.detailOpen
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 12
                Heading { text: window.section === 0 ? "Find your next useful app." : "Applications" }
                Copy { text: "Explore software, inspect its source, and understand what is known before you get it."; visible: window.section === 0 }
                RowLayout {
                    Layout.fillWidth: true
                    TextField {
                        id: searchInput; objectName: "searchInput"
                        Layout.fillWidth: true; placeholderText: "Search apps, purpose or category"
                        text: window.filters.text || ""; maximumLength: 256
                        onTextEdited: window.updateFilter("text", text)
                        Accessible.name: "Search applications"
                    }
                    ToolButton { text: window.filtersOpen ? "Hide filters" : "Filters"; onClicked: window.filtersOpen = !window.filtersOpen }
                    ToolButton { text: "Clear"; onClicked: window.clearFilters(); Accessible.name: "Clear search and filters" }
                }
                GridLayout {
                    visible: window.filtersOpen; columns: window.width < 1100 ? 3 : 4; Layout.fillWidth: true
                    Filter { filterKey: "appType"; model: [{text:"All app types",value:""},{text:"Desktop",value:"desktop"},{text:"Terminal",value:"terminal"},{text:"Shell plugin",value:"shell_plugin"},{text:"Web",value:"web"},{text:"Service",value:"service"}] }
                    Filter { filterKey: "license"; model: [{text:"All licences",value:""},{text:"Open source",value:"open_source"},{text:"Source available",value:"source_available"},{text:"Proprietary",value:"proprietary"},{text:"Unknown",value:"unknown"}] }
                    Filter { filterKey: "architecture"; model: [{text:"All architectures",value:""},{text:"x86_64",value:"x86_64"},{text:"aarch64",value:"aarch64"},{text:"Unknown",value:"unknown"}] }
                    Filter { filterKey: "pricing"; model: [{text:"All pricing",value:""},{text:"Free",value:"free"},{text:"Voluntary support",value:"voluntary_support"},{text:"Pay what you want",value:"pay_what_you_want"},{text:"Paid app",value:"paid_app"},{text:"Paid upgrade",value:"paid_upgrade"},{text:"Paid features",value:"paid_features"},{text:"Subscription",value:"subscription"},{text:"Services",value:"professional_services"},{text:"Paid preview",value:"paid_preview"},{text:"Unknown",value:"unknown"}] }
                    Filter { filterKey: "offline"; model: [{text:"Any offline status",value:""},{text:"Works offline",value:"yes"},{text:"Needs connection",value:"no"},{text:"Unknown",value:"unknown"}] }
                    Filter { filterKey: "testResult"; model: [{text:"All test results",value:""},{text:"Passes recorded",value:"passes"},{text:"Limitations",value:"limitations"},{text:"Fails recorded",value:"fails"},{text:"Not tested",value:"not_tested"}] }
                    TextField { Layout.fillWidth: true; placeholderText: "Category"; text: window.filters.category || ""; maximumLength: 64; onTextEdited: window.updateFilter("category",text); Accessible.name: "Category filter" }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Copy { objectName: "resultCount"; text: window.core.total + (window.core.total === 1 ? " application" : " applications"); font.bold: true }
                    Label { text: "Name and relevance"; font.pixelSize: 12 }
                }
                ListView {
                    id: appList; objectName: "appList"
                    Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 8
                    model: window.core.apps
                    ScrollBar.vertical: ScrollBar {}
                    keyNavigationEnabled: true; activeFocusOnTab: true
                    Accessible.name: "Application results"
                    Keys.onReturnPressed: { if (currentIndex >= 0 && currentIndex < count) window.openApp(window.core.apps[currentIndex].id) }
                    delegate: ItemDelegate {
                        required property var modelData
                        required property int index
                        width: ListView.view.width; height: 104
                        highlighted: ListView.isCurrentItem && appList.activeFocus
                        onClicked: { appList.currentIndex = index; window.openApp(modelData.id) }
                        Accessible.name: modelData.name + ". " + modelData.summary + ". Informational listing."
                        contentItem: RowLayout {
                            spacing: 14
                            Rectangle {
                                Layout.preferredWidth: 48; Layout.preferredHeight: 48; radius: 8
                                color: window.palette.alternateBase; border.color: window.palette.mid
                                Label { anchors.centerIn: parent; text: modelData.name.charAt(0); font.pixelSize: 24; font.bold: true; textFormat: Text.PlainText }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 4
                                Label { text: modelData.name; textFormat: Text.PlainText; font.pixelSize: 18; font.bold: true; elide: Text.ElideRight; Layout.fillWidth: true }
                                Label { text: modelData.summary; textFormat: Text.PlainText; font.pixelSize: 14; elide: Text.ElideRight; Layout.fillWidth: true }
                                Label { text: modelData.category + "  ·  " + window.words(modelData.license.class) + "  ·  " + (modelData.informational ? "Informational" : "Listed"); textFormat: Text.PlainText; font.pixelSize: 12; elide: Text.ElideRight; Layout.fillWidth: true }
                            }
                            Label { text: "›"; font.pixelSize: 24 }
                        }
                    }
                    Label {
                        anchors.centerIn: parent; width: Math.min(parent.width - 32,420); horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap
                        visible: appList.count === 0 && !window.core.busy
                        text: "No applications match this view.\nClear filters or refresh the catalogue."
                    }
                }
                Button { visible: window.core.hasMore; text: "Load more"; enabled: !window.core.busy; onClicked: window.core.nextPage() }
            }
            ColumnLayout {
                visible: window.section <= 1 && window.detailOpen
                Layout.fillWidth: true; Layout.fillHeight: true
                Button { text: "‹ Back to applications"; onClicked: window.back(); Accessible.name: "Back to applications" }
                ScrollView {
                    objectName: "detailScroll"; Layout.fillWidth: true; Layout.fillHeight: true; clip: true
                    contentWidth: availableWidth
                    ColumnLayout {
                        width: parent.width; spacing: 16
                        Heading { objectName: "appTitle"; text: window.d.name || "Loading application…"; font.pixelSize: 30 }
                        Copy { text: window.d.summary || ""; font.pixelSize: 18 }
                        Copy { text: window.d.informational ? "INFORMATIONAL LISTING · Maker has not claimed this page" : "Application listing"; font.pixelSize: 12; font.bold: true }
                        Copy { text: window.d.description || "" }
                        GroupBox {
                            title: "Before you get it"; Layout.fillWidth: true
                            GridLayout {
                                anchors.fill: parent; columns: 2; columnSpacing: 20; rowSpacing: 8
                                Copy { text: "App type / maturity"; Layout.preferredWidth: 140 }
                                Copy { text: window.words(window.d.appType) + " / " + window.words(window.d.maturity) }
                                Copy { text: "Software licence" }
                                Copy { text: window.d.license ? window.words(window.d.license.class) + " · " + (window.d.license.identifier || "Identifier unknown") : "Unknown" }
                                Copy { text: "Recorded version" }
                                Copy { text: window.release ? window.release.version : "No release recorded" }
                                Copy { text: "Architectures" }
                                Copy { text: window.release ? window.release.architectures.join(", ") : "Unknown" }
                                Copy { text: "Works offline" }
                                Copy { text: window.release ? window.words(window.release.offline) : "Unknown" }
                                Copy { text: "Account / service" }
                                Copy { text: window.release ? window.words(window.release.accountRequired) + " / " + window.words(window.release.serviceRequired) : "Unknown" }
                                Copy { text: "Online activation" }
                                Copy { text: window.release ? window.words(window.release.activationRequired) : "Unknown" }
                                Copy { text: "Price / service costs" }
                                Copy { text: window.d.offers && window.d.offers.length ? window.d.offers.map(o => window.words(o.model) + (o.amountMinor !== null ? " · " + o.amountMinor + " minor units " + o.currency : " · Check developer for current price")).join("\n") : "Unknown. Check the developer's terms." }
                            }
                        }
                        GroupBox {
                            title: "Package source"; Layout.fillWidth: true
                            ColumnLayout {
                                anchors.fill: parent
                                Copy { text: window.release && window.release.identity.kind === "repository_package" ? window.release.identity.repository + " / " + window.release.identity.package + " / " + window.release.identity.version : "Source or external distribution" }
                                Copy { text: "This records a package definition. Availability on your machine has not been checked. Managed installation is not enabled in this preview." }
                                Button { text: window.d.actionLabel || "View source"; enabled: !!window.d.actionUrl; onClicked: window.core.openExternal(window.d.actionUrl); Layout.fillWidth: true }
                                Button { text: "Inspect listing provenance"; enabled: !!window.d.provenance; onClicked: window.core.openExternal(window.d.provenance) }
                            }
                        }
                        GroupBox {
                            title: "Compatibility evidence"; Layout.fillWidth: true
                            ColumnLayout {
                                anchors.fill: parent
                                Copy { objectName: "evidenceText"; font.bold: true; text: window.d.testResult === "not_tested" ? "This release has not been tested by OmaStore." : "Recorded result: " + window.words(window.d.testResult) + " · " + window.words(window.d.testFreshness) }
                                Copy { visible: window.d.testFreshness === "superseded"; text: "Existing evidence describes an older release, not this one." }
                                Copy { text: "A package listing is not a compatibility test, independent approval or security guarantee." }
                                Repeater { model: window.d.evidence || []; Copy { required property var modelData; text: window.words(modelData.result) + " · " + modelData.releaseId + " · " + modelData.environment + " · " + modelData.testedAt } }
                            }
                        }
                        GroupBox {
                            title: "Screenshots and demonstrations"; Layout.fillWidth: true
                            ColumnLayout {
                                anchors.fill: parent
                                Copy { text: "No application media is bundled with these informational listings. External media opens only when you choose it." }
                                Repeater { model: window.d.media || []; Button { required property var modelData; text: "View " + modelData.kind; onClicked: window.core.openExternal(modelData.url); Accessible.name: modelData.alt } }
                            }
                        }
                        Copy { text: (window.d.limitations || []).join("\n") }
                        RowLayout {
                            Layout.fillWidth: true
                            Button { text: "Project source"; enabled: !!window.d.source; onClicked: window.core.openExternal(window.d.source) }
                            Button { text: "Support"; enabled: !!window.d.support; onClicked: window.core.openExternal(window.d.support) }
                        }
                        Repeater { model: window.d.offers || []; ColumnLayout {
                            required property var modelData
                            Copy { text: "Seller: " + modelData.sellerId + " · " + modelData.entitlement + " · Tax included: " + window.words(modelData.taxIncluded) }
                            Button { text: "Developer offer and terms"; onClicked: window.core.openExternal(modelData.url) }
                            Button { visible: !!modelData.refund; text: "Refund terms"; onClicked: window.core.openExternal(modelData.refund) }
                            Button { visible: !!modelData.cancellation; text: "Cancellation terms"; onClicked: window.core.openExternal(modelData.cancellation) }
                        } }
                        Item { Layout.preferredHeight: 16 }
                    }
                }
            }
            ColumnLayout {
                visible: window.section > 1; Layout.fillWidth: true; Layout.fillHeight: true
                Heading { text: window.sections[window.section] }
                Copy { text: ["", "", "No setups have been published yet.", "Installed-app detection and library actions are planned for the next release.", "Maker accounts and project claims are not open yet.", "Submissions are not open yet."][window.section] }
                Item { Layout.fillHeight: true }
            }
        }
    }
    Dialog {
        id: about; anchors.centerIn: parent; width: Math.min(480,window.width-48); modal: true
        title: "About OmaStore"; standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            Copy { text: "Native discovery preview · Qt/QML and Rust" }
            Copy { text: window.core.ready ? "Local core " + window.core.version + " is connected." : window.core.error || "Starting the local core…" }
            Copy { text: "Catalogue revision: " + (window.core.catalogueState.revision || "Unknown") }
            Copy { text: "Independent community project. Managed installation, submissions and payments are not enabled." }
        }
    }
}
