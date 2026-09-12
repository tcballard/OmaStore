#include "CoreBridge.h"
#include "Instance.h"
#include "DesktopSettings.h"
#include "MediaPreview.h"
#include "Preparation.h"

#include <QCommandLineParser>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
#include <QTimer>
#include <QVariant>
#include <QFont>
#include <QPalette>
#include <QQuickWindow>
#include <QQuickItem>
#include <QSettings>
#ifdef OMASTORE_QA
#include <QTemporaryDir>
#include <QFile>
#include <QXmlStreamReader>
#include <QTest>
#include <memory>
#include <functional>
#include <QAccessible>
#endif
#include <QStandardPaths>
#include <QIcon>

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    QCoreApplication::setApplicationName("OmaStore");
    QCoreApplication::setApplicationVersion("0.1.0");
    QCoreApplication::setOrganizationName("OmaStore");
    QGuiApplication::setDesktopFileName("io.github.tcballard.OmaStore");
    QGuiApplication::setWindowIcon(QIcon::fromTheme("system-software-install"));
    QQuickStyle::setStyle("Fusion");

    QCommandLineParser parser;
    parser.setApplicationDescription("Native application storefront for Omarchy");
    parser.addHelpOption();
    parser.addVersionOption();
#ifdef OMASTORE_QA
    const QCommandLineOption smokeTest("smoke-test", "Exit after verifying the window and local core startup.");
    parser.addOption(smokeTest);
    const QCommandLineOption uiTest("ui-test", "Exercise native discovery interaction and quit.");
    const QCommandLineOption screenshot("screenshot", "Save an actual window capture after loading.", "path");
    const QCommandLineOption size("window-size", "Initial logical dimensions, for desktop QA.", "WIDTHxHEIGHT");
    const QCommandLineOption storefrontTest("storefront-test", "Exercise the real catalogue browse/detail/plan journey.");
    const QCommandLineOption captureView("capture-view", "Capture discover, detail or plan in the real catalogue.", "view", "discover");
    const QCommandLineOption darkAppearance("dark-appearance", "Use a dark palette for offscreen QA.");
    const QCommandLineOption themeFixture("theme-fixture", "Read an isolated Omarchy theme fixture for QA.", "directory");
    parser.addOptions({uiTest, screenshot, size, storefrontTest, captureView, darkAppearance, themeFixture});
#endif
#ifdef OMASTORE_DEVELOPMENT_DATA
    const QCommandLineOption demoOption("demo", "Show explicitly fictional development listings.");
    parser.addOption(demoOption);
#endif
    parser.addPositionalArgument("uri","Open an OmaStore app or exact setup revision.","[omastore://…]");
    parser.process(app);
    if(parser.positionalArguments().size()>1)return 2;
    const QString handoff=parser.positionalArguments().value(0);
    if(handoff.size()>400 || (!handoff.isEmpty() && !handoff.startsWith("omastore://")))return 2;

    bool demo = false;
#ifdef OMASTORE_DEVELOPMENT_DATA
    demo = parser.isSet(demoOption);
#endif
    if (demo) QCoreApplication::setApplicationName("OmaStoreDevelopment");
    QString dataDirectory = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
#ifdef OMASTORE_QA
    QTemporaryDir testSettings;
    if (parser.isSet(smokeTest) || parser.isSet(uiTest) || parser.isSet(screenshot) || parser.isSet(storefrontTest)) {
        dataDirectory = testSettings.filePath("data");
        qputenv("XDG_DATA_HOME",testSettings.filePath("xdg-data").toUtf8());
        qputenv("XDG_STATE_HOME",testSettings.filePath("xdg-state").toUtf8());
        QSettings::setDefaultFormat(QSettings::IniFormat);
        QSettings::setPath(QSettings::IniFormat, QSettings::UserScope, testSettings.path());
    }
    #endif
    Instance instance(demo);
    const auto acquisition=instance.acquire(handoff);
    if(acquisition==Instance::Forwarded)return 0;
    if(acquisition==Instance::Unavailable){qWarning("OmaStore is already running but its window could not be reached.");return 2;}
#ifdef OMASTORE_QA
    if (parser.isSet(darkAppearance)) {
        auto palette = app.palette();
        palette.setColor(QPalette::Window, QColor("#181d1b"));
        app.setPalette(palette);
    }
#endif
#ifdef OMASTORE_QA
    DesktopSettings desktop(parser.isSet(themeFixture) ? parser.value(themeFixture) : QDir::homePath() + "/.local/state/omarchy/current/theme",
        QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation) + "/fontconfig/fonts.conf");
#else
    DesktopSettings desktop;
#endif
    const QFont baseFont = app.font();
    QObject::connect(&desktop, &DesktopSettings::changed, &app, [&] {
        QFont font = baseFont;
        font.setPointSizeF((baseFont.pointSizeF() > 0 ? baseFont.pointSizeF() : 10) * desktop.textScale());
        app.setFont(font);
    });
    CoreBridge core(demo);
    MediaPreview mediaPreview;
    Preparation worksheet(dataDirectory);
    QObject::connect(&worksheet, &Preparation::checkRequested, &core, &CoreBridge::prepareCandidate);
    QObject::connect(&core, &CoreBridge::candidatePrepared, &worksheet, &Preparation::acceptResult);
    QQmlApplicationEngine engine;
    bool qmlWarning = false;
    QObject::connect(&engine, &QQmlApplicationEngine::warnings, &app, [&](const QList<QQmlError> &) { qmlWarning = true; });
    engine.setInitialProperties({{"core", QVariant::fromValue(&core)}, {"desktop", QVariant::fromValue(&desktop)}, {"mediaPreview", QVariant::fromValue(&mediaPreview)}, {"worksheet", QVariant::fromValue(&worksheet)}});
    engine.load(QUrl(QStringLiteral("qrc:/qt/qml/OmaStore/Main.qml")));
    if (engine.rootObjects().isEmpty()) return 1;
    auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
    QObject::connect(&instance,&Instance::requested,&app,[&](const QString &uri){window->show();window->raise();window->requestActivate();core.openHandoff(uri);});
    instance.ready();
    if (!window) return 1;
#ifdef OMASTORE_QA
    if (parser.isSet(size)) {
        const auto dimensions = parser.value(size).split('x');
        if (dimensions.size() != 2) return 2;
        const int width = dimensions[0].toInt(), height = dimensions[1].toInt();
        if (width < 800 || height < 600 || width > 3840 || height > 2160) return 2;
        window->resize(width, height);
    }

    if (parser.isSet(smokeTest) || parser.isSet(uiTest) || parser.isSet(screenshot) || parser.isSet(storefrontTest)) {
        const auto scheduled = std::make_shared<bool>(false);
        QObject::connect(&core, &CoreBridge::dataChanged, &app, [&, scheduled] {
            if (!core.loaded() || *scheduled) return;
            *scheduled = true;
            QTimer::singleShot(300, &app, [&] {
                bool ok = true;
                const auto check = [&](bool condition, const char *stage) { if (!condition) { qWarning() << "Native interaction failed:" << stage; ok = false; } };
                std::function<QQuickItem *(QQuickItem *, const QString &)> findItem;
                findItem = [&](QQuickItem *parent, const QString &name) -> QQuickItem * {
                    if (parent->objectName() == name) return parent;
                    for (auto *child : parent->childItems()) if (auto *found = findItem(child, name)) return found;
                    return nullptr;
                };
                const auto openNavigation = [&] {
                    auto *more = findItem(window->contentItem(), "moreNavigation");
                    check(more != nullptr, "secondary navigation menu");
                    if (more) { more->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); QTest::qWait(80); }
                };
                if (parser.isSet(uiTest)) {
                    QTest::keyClick(window, Qt::Key_K, Qt::ControlModifier);
                    QTest::qWait(100);
                    auto *search = window->findChild<QObject *>("searchField");
                    check(search && search->property("activeFocus").toBool(), "search focus");
                    for (const auto character : QByteArray("fieldnotes")) QTest::keyClick(window, character);
                    QTest::qWait(300);
                    check(core.query().value("q").toString() == "fieldnotes", "search text");
                    if (demo && core.apps().size() == 1) {
                        const auto id = core.apps().first().toMap().value("id").toString();
                        auto *card = findItem(window->contentItem(), "app-" + id);
                        if (card) {
                            auto *accessible = QAccessible::queryAccessibleInterface(card);
                            check(accessible && !accessible->text(QAccessible::Name).isEmpty(), "accessible app name");
                            card->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space);
                        }
                        else check(false, "missing app card");
                        QTest::qWait(200);
                        check(!core.detail().isEmpty(), "open app with keyboard");
                        auto *save = findItem(window->contentItem(), "saveButton");
                        if (save) { save->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                        else check(false, "missing save button");
                        QTest::qWait(100); check(core.isSaved(id), "save with keyboard");
                        CoreBridge restored(demo);
                        check(restored.isSaved(id) && restored.query().value("q").toString() == "fieldnotes", "restore preferences");
                        QTest::keyClick(window, Qt::Key_Escape); QTest::qWait(100);
                        check(core.detail().isEmpty() && core.query().value("q").toString() == "fieldnotes", "back preserves query");
                    } else if (demo) { check(false, "search result count"); }
                    core.clearFilters(); QTest::qWait(200);
                    if(demo) {
                    openNavigation();
                        auto *makers=findItem(window->contentItem(),"navMakers");
                        if(makers){makers->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}QTest::qWait(100);
                        const auto makersResult=core.community().value("makers.list").toMap().value("items").toList();
                        check(!makersResult.isEmpty(),"maker discovery");
                        if(!makersResult.isEmpty()) {
                            const auto makerId=makersResult.first().toMap().value("id").toString();
                            auto *maker=findItem(window->contentItem(),"maker-"+makerId);
                            if(maker){maker->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                            for(int i=0;i<40 && core.loading();++i)QTest::qWait(50);
                            check(core.community().value("makers.get").toMap().value("id").toString()==makerId,"maker profile by keyboard");
                            check(core.community().value("makers.get").toMap().value("claim").toString()=="unclaimed","fictional maker remains unclaimed");
                        }
                    }
                    if(demo) {
                        auto pressSetup=[window](QQuickItem *item){if(item){item->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}};
                    openNavigation();
                        pressSetup(findItem(window->contentItem(),"navSetups"));QTest::qWait(100);
                        pressSetup(findItem(window->contentItem(),"setup-demo-writing-desk"));
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        auto selection=core.community().value("setups.select").toMap();
                        check(selection.value("recipe").toMap().value("id").toString()=="demo-writing-desk","native setup detail");
                        auto *required=findItem(window->contentItem(),"component-demo-fieldnotes");
                        check(required && !required->isEnabled() && required->property("checked").toBool(),"required setup component cannot be deselected");
                        pressSetup(findItem(window->contentItem(),"component-demo-papertrail"));
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        selection=core.community().value("setups.select").toMap();check(selection.value("selected").toList().size()==2,"select optional setup component");
                        pressSetup(findItem(window->contentItem(),"previewSetupPlan"));
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        const auto proposal=core.community().value("system.plan").toMap();
                        check(proposal.value("simulated").toBool() && proposal.value("operations").toList().size()==2,"typed setup installation proposal");
                        check(!proposal.value("digest").toString().isEmpty(),"proposal has a stable content digest");
                        auto *planNotice=findItem(window->contentItem(),"planNotice");
                        check(planNotice && planNotice->isVisible(),"native plan review is visible");
                        pressSetup(findItem(window->contentItem(),"closePlan"));QTest::qWait(50);
                        QTemporaryDir selectionDirectory;const auto selectionFile=QUrl::fromLocalFile(selectionDirectory.filePath("selection.json")).toString();
                        const QVariantMap intent{{"id","demo-writing-desk"},{"revision","1"},{"chosen",selection.value("requested")}};
                        core.communityAction("setups.export",{{"selection",intent},{"snapshot",selection.value("snapshot")},{"file",selectionFile}});
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        check(core.community().value("setups.export").toMap().value("exported").toBool(),"native selection file export");
                        core.communityAction("setups.import",{{"file",selectionFile}});
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        check(core.community().value("setups.select").toMap().value("selected").toList().size()==2,"native selection identity round trip");
                        pressSetup(findItem(window->contentItem(),"remixSetup"));for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        auto *saveRemix=findItem(window->contentItem(),"saveRemix");check(saveRemix&&saveRemix->isVisible(),"native selected remix editor");pressSetup(saveRemix);for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        const auto remix=core.community().value("remixes.create").toMap();const auto portable=remix.value("export").toMap();
                        check(remix.value("ready").toBool()&&portable.value("parent").toMap().value("id").toString()=="demo-writing-desk"&&portable.value("settings").toList().isEmpty(),"local remix keeps attribution and only selected settings");
                        const auto remixFile=QUrl::fromLocalFile(selectionDirectory.filePath("remix.json")).toString();core.communityAction("remixes.export",{{"id",portable.value("id")},{"digest",remix.value("digest")},{"file",remixFile}});for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(core.community().value("remixes.export").toMap().value("exported").toBool(),"reviewed portable remix export");
                        core.communityAction("remixes.import",{{"file",remixFile}});for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(core.community().value("remixes.import").toMap().value("ready").toBool(),"local remix import recomputes current availability");
                        pressSetup(findItem(window->contentItem(),"planRemix"));for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(core.community().value("system.plan").toMap().value("selection").toMap().value("kind").toString()=="apps","local remix enters typed installation planning");pressSetup(findItem(window->contentItem(),"closePlan"));QTest::qWait(50);
                        QTest::keyClick(window,Qt::Key_Escape);QTest::qWait(50);
                        check(window->property("setupSelection").toString().isEmpty(),"back from setup selection");
                    }
                    if(demo) {
                        auto *library=findItem(window->contentItem(),"navLibrary");
                        if(library){library->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        const auto local=core.community().value("library.list").toMap();
                        check(local.value("items").toList().isEmpty(),"proposal is never labelled installed");
                        check(!local.value("operations").toList().isEmpty(),"proposal survives in the local journal");
                        for(int i=0;i<100&&core.deviceRefreshing();++i)QTest::qWait(50);
                        check(core.device().value("installedCount",-1).toInt()==0,"installed destination reads device inventory");
                        auto *updates=findItem(window->contentItem(),"navUpdates");check(updates!=nullptr,"updates destination exists");
                        if(updates){updates->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(window->property("section").toInt()==7 && core.device().value("updateCount",-1).toInt()==0,"updates destination reads update inventory");
                        auto *saved=findItem(window->contentItem(),"navSaved");check(saved!=nullptr,"saved destination exists");
                        if(saved){saved->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(window->property("section").toInt()==6,"saved destination remains separate");
                        core.openHandoff("omastore://setup/demo-writing-desk?revision=1");
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        check(window->property("setupSelection").toString()=="demo-writing-desk","identity handoff opens exact setup");
                        core.communityAction("system.plan",{{"kind","app"},{"id","demo-fieldnotes"}});
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        auto *approve=findItem(window->contentItem(),"confirmPlan");
                        check(approve&&!approve->isEnabled(),"installation requires an explicit consent gesture");
                        auto *consent=findItem(window->contentItem(),"planConsent");
                        if(consent){consent->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        if(approve){approve->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        const auto operationId=core.community().value("system.plan").toMap().value("digest").toString();
                        auto *during=findItem(window->contentItem(),"closePlan");if(during){during->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        auto *discover=findItem(window->contentItem(),"navDiscover");if(discover){discover->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<200 && (core.activity().isEmpty() || core.activity().first().toMap().value("state").toString()!="succeeded" || core.appStates().value("demo-fieldnotes").toMap().value("state").toString()!="installed");++i)QTest::qWait(50);
                        check(!core.activity().isEmpty() && core.activity().first().toMap().value("state").toString()=="succeeded","sample worker progress continues with review closed");
                        check(core.appStates().value("demo-fieldnotes").toMap().value("state").toString()=="installed","completion refreshes shared device state automatically");
                        check(findItem(window->contentItem(),"openActivity")!=nullptr,"operation recovery remains accessible while browsing");
                        core.communityAction("operations.get",{{"id",operationId}});
                        for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        auto *previewDiagnostics=findItem(window->contentItem(),"previewDiagnostics");
                        if(previewDiagnostics){previewDiagnostics->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        check(findItem(window->contentItem(),"diagnosticDocument")!=nullptr,"native diagnostic preview before export");
                        auto *closeDiagnostics=findItem(window->contentItem(),"closeDiagnostics");if(closeDiagnostics){closeDiagnostics->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}QTest::qWait(50);
                        auto *done=findItem(window->contentItem(),"closePlan");if(done){done->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        core.communityAction("library.refresh");for(int i=0;i<60 && core.loading();++i)QTest::qWait(50);
                        check(!core.community().value("library.list").toMap().value("items").toList().isEmpty(),"sample installation appears in observed library");
                        auto *librarySettings=findItem(window->contentItem(),"navLibrary");if(librarySettings){librarySettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        auto *openSettings=findItem(window->contentItem(),"openSettings");if(openSettings){openSettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        auto *clockSetting=findItem(window->contentItem(),"settingEnabled-omarchy-clock-placement");check(clockSetting!=nullptr,"native supported settings choices");if(clockSetting){clockSetting->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        auto *previewSettings=findItem(window->contentItem(),"previewSettings");if(previewSettings){previewSettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        const auto settingsPlan=core.community().value("settings.preview").toMap();check(settingsPlan.value("canApply").toBool()&&settingsPlan.value("simulated").toBool(),"native settings diff remains an explicit fictional proposal");
                        auto *applySettings=findItem(window->contentItem(),"applySettings");check(applySettings&&applySettings->isVisible()&&!applySettings->isEnabled(),"setting changes require a visible separate consent flow");
                        auto *settingsConsent=findItem(window->contentItem(),"settingsConsent");if(settingsConsent){settingsConsent->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        if(applySettings){applySettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        auto settingsHistory=core.community().value("settings.apply").toMap().value("items").toList();check(!settingsHistory.isEmpty()&&settingsHistory.first().toMap().value("steps").toList().first().toMap().value("state").toString()=="applied","native setting apply records the sample file outcome");
                        auto *previewRestore=findItem(window->contentItem(),"previewRestore-"+settingsPlan.value("digest").toString());if(previewRestore){previewRestore->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(core.community().value("settings.restore_preview").toMap().value("canRestore").toBool(),"native conflict-aware restoration preview");
                        auto *restoreConsent=findItem(window->contentItem(),"settingsRestoreConsent");if(restoreConsent){restoreConsent->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        auto *restoreSettings=findItem(window->contentItem(),"restoreSettings");if(restoreSettings){restoreSettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        settingsHistory=core.community().value("settings.restore").toMap().value("items").toList();check(!settingsHistory.isEmpty()&&settingsHistory.first().toMap().value("steps").toList().first().toMap().value("state").toString()=="restored","native restoration preserves a durable result");
                        auto *closeSettings=findItem(window->contentItem(),"closeSettings");if(closeSettings){closeSettings->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}


                        QTest::keyClick(window,Qt::Key_Escape);QTest::qWait(50);
                    }
                    openNavigation();
                    auto *submit = findItem(window->contentItem(), "navSubmit");
                    if (submit) { submit->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    QTest::qWait(150);
                    if (demo) {
                        for (int i=0;i<40 && core.loading();++i) QTest::qWait(50);
                        auto *sampleSignIn=findItem(window->contentItem(),"sampleSignIn");
                        if (sampleSignIn) {sampleSignIn->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for (int i=0;i<40 && core.loading();++i) QTest::qWait(50);
                        check(core.workspace().value("actor").toMap().value("id").toString()=="development:author","sample author sign-in");
                        const auto remixForAuthor=core.community().value("remixes.create").toMap().value("export").toMap();
                        core.workspaceAction("drafts.remix",{{"remix",remixForAuthor}});for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(!core.workspaceReply().value("id").toString().isEmpty()&&core.workspace().value("revisions").toList().isEmpty(),"remix creates a private author draft without submitting");

                        auto *newDraft=findItem(window->contentItem(),"newDraft");
                        if (newDraft) {newDraft->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for (int i=0;i<60 && (core.loading() || !findItem(window->contentItem(),"field-apps-0-name"));++i) QTest::qWait(50);
                        auto *draftName=findItem(window->contentItem(),"field-apps-0-name");
                        check(draftName!=nullptr,"native draft editor created");
                        if (draftName) {draftName->forceActiveFocus();for (const auto character:QByteArray("My draft")) QTest::keyClick(window,character);QTest::keyClick(window,Qt::Key_Tab);}
                        QTest::qWait(850);
                        check(core.workspaceReply().value("localSaved").toBool(),"private draft autosave");
                        core.workspaceAction("drafts.sample",{});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        const auto sampleDraft=core.workspaceReply();
                        core.workspaceAction("command",{{"command","submit_draft"},{"id",sampleDraft.value("id")},{"version",sampleDraft.value("version")},{"confirm_public_preview",true}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        const auto revisions=core.workspace().value("revisions").toList();
                        check(!revisions.isEmpty(),"immutable sample submission");
                        core.workspaceAction("checks.run_sample",{});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        core.workspaceAction("auth.sandbox",{{"name","reviewer"}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        auto press=[window](QQuickItem *item) {if(item){item->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}};
                        press(findItem(window->contentItem(),"reviewWorkspaceTab"));
                        QTest::qWait(50);press(findItem(window->contentItem(),"loadReviewQueue"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        press(findItem(window->contentItem(),"inspectReview"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("independent").toBool(),"reviewer independence");
                        press(findItem(window->contentItem(),"sampleEvidence"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        auto *reviewReason=findItem(window->contentItem(),"reviewReason");
                        if(reviewReason){reviewReason->forceActiveFocus();for(const auto character:QByteArray("Fictional QA review")) QTest::keyClick(window,character);}
                        press(findItem(window->contentItem(),"reviewAcknowledgement"));
                        press(findItem(window->contentItem(),"approveReview"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("state").toString()=="approved","independent sample approval");
                        const auto approvedRevision=core.workspaceReply().value("id");
                        core.workspaceAction("auth.sandbox",{{"name","author"}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        press(findItem(window->contentItem(),"authorWorkspaceTab"));
                        core.workspaceAction("revisions.get",{{"id",approvedRevision}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        press(findItem(window->contentItem(),"requestPublication"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("state").toString()=="publication_pending","publication requested");
                        press(findItem(window->contentItem(),"samplePublication"));
                        for(int i=0;i<80 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("state").toString()=="published","local provider delivery observed");
                        QTemporaryDir feedDirectory;
                        const auto feedPath=feedDirectory.filePath("releases.xml");
                        core.workspaceAction("feed.export",{{"makerId",QVariant()},{"file",QUrl::fromLocalFile(feedPath).toString()}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        QFile feed(feedPath);check(feed.open(QIODevice::ReadOnly),"native RSS export");
                        QXmlStreamReader rss(feed.readAll());int entries=0;
                        while(!rss.atEnd()){rss.readNext();if(rss.isStartElement() && rss.name().toString()==QStringLiteral("item"))++entries;}
                        check(!rss.hasError() && entries==1,"observed release RSS parses once");
                        core.workspaceAction("auth.sandbox",{{"name","operator"}});
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        press(findItem(window->contentItem(),"operationsWorkspaceTab"));
                        press(findItem(window->contentItem(),"loadMonitoringQueue"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        press(findItem(window->contentItem(),"sampleMonitoring"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        auto *monitorReason=findItem(window->contentItem(),"monitoringReason");
                        if(monitorReason){monitorReason->forceActiveFocus();for(const auto character:QByteArray("Simulated operator review"))QTest::keyClick(window,character);}
                        press(findItem(window->contentItem(),"suspendDistribution"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(!core.workspaceReply().value("holds").toList().isEmpty(),"audited distribution suspension");
                        press(findItem(window->contentItem(),"monitoringAcknowledgement"));
                        press(findItem(window->contentItem(),"restoreDistribution"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("holds").toList().isEmpty(),"reviewed distribution restoration");
                        press(findItem(window->contentItem(),"readinessWorkspaceTab"));press(findItem(window->contentItem(),"loadReadiness"));
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);
                        check(core.workspaceReply().value("environment").toString()=="development"&&!core.workspaceReply().value("pilotEvidenceComplete").toBool(),"native readiness separates sample and public evidence");
                        check(core.workspaceReply().value("gates").toList().size()==12,"native operator report exposes every pilot gate");
                        press(findItem(window->contentItem(),"commerceWorkspaceTab"));press(findItem(window->contentItem(),"loadCommerce"));
                        for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                        check(core.workspaceReply().value("environment").toString()=="development"&&!core.workspaceReply().value("realCheckoutEnabled").toBool()&&!core.workspaceReply().value("missing").toList().isEmpty(),"native commerce shows missing facts with real charges disabled");

                        auto *pauseCommerce=findItem(window->contentItem(),"commercePauseReason");if(pauseCommerce){pauseCommerce->forceActiveFocus();for(const auto character:QByteArray("Fictional purchase rehearsal"))QTest::keyClick(window,character);}
                        press(findItem(window->contentItem(),"commercePause"));for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"navLibrary"));for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"openPurchases"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"reviewPurchase-sample-perpetual"));for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        auto *purchaseButton=findItem(window->contentItem(),"confirmPurchase");check(purchaseButton&&purchaseButton->isVisible()&&!purchaseButton->isEnabled(),"native purchase displays exact terms and requires consent");
                        press(findItem(window->contentItem(),"purchaseConsent"));press(purchaseButton);for(int i=0;i<120&&core.loading();++i)QTest::qWait(50);
                        auto *purchases=window->findChild<QObject*>("purchasesDialog");auto purchaseOrder=purchases?purchases->property("order").toMap():QVariantMap{};
                        check(purchaseOrder.value("paymentState").toString()=="payment_pending","native checkout is not labelled paid from creation");
                        press(findItem(window->contentItem(),"sampleDeliveryFailure"));press(findItem(window->contentItem(),"sampleCapture"));for(int i=0;i<120&&core.loading();++i)QTest::qWait(50);
                        purchaseOrder=purchases?purchases->property("order").toMap():QVariantMap{};check(purchaseOrder.value("paymentState").toString()=="paid"&&purchaseOrder.value("deliveryState").toString()=="delivery_failed","native paid order retains a failed delivery");
                        press(findItem(window->contentItem(),"retryPurchaseDelivery"));for(int i=0;i<120&&core.loading();++i)QTest::qWait(50);
                        purchaseOrder=purchases?purchases->property("order").toMap():QVariantMap{};check(purchaseOrder.value("deliveryState").toString()=="delivered"&&!purchaseOrder.value("licenceDigest").toString().isEmpty(),"native retry recovers the signed licence");
                        QTemporaryDir licenceDirectory;const auto licencePath=licenceDirectory.filePath("licence.json");
                        core.workspaceAction("commerce.licence.export",{{"id",purchaseOrder.value("id")},{"digest",purchaseOrder.value("licenceDigest")},{"file",QUrl::fromLocalFile(licencePath).toString()}});for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        check(QFileInfo::exists(licencePath),"actual private licence export");
                        core.workspaceAction("commerce.licence.verify",{{"file",QUrl::fromLocalFile(licencePath).toString()}});for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        check(core.workspaceReply().value("verifiedOffline").toBool(),"native offline verification uses the pinned sample issuer");
                        auto *refundPanel=purchases?purchases->findChild<QQuickItem*>("refundsPanel"):nullptr;
                        check(refundPanel&&refundPanel->isVisible(),"native paid receipt exposes refund controls");
                        if(refundPanel){
                            auto *amount=findItem(refundPanel,"refundAmount");if(amount){amount->forceActiveFocus();for(const auto character:QByteArray("2.00"))QTest::keyClick(window,character);}
                            auto *reason=findItem(refundPanel,"refundReason");if(reason){reason->forceActiveFocus();for(const auto character:QByteArray("Fictional partial refund"))QTest::keyClick(window,character);}
                            press(findItem(refundPanel,"refundConsent"));press(findItem(refundPanel,"requestRefund"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                            const auto refunds=refundPanel->property("report").toMap().value("items").toList();check(refunds.size()==1,"native refund request retains one intent");
                            if(!refunds.isEmpty()){press(findItem(refundPanel,"approveRefundConsent"));press(findItem(refundPanel,"executeRefund-"+refunds.first().toMap().value("id").toString()));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                                const auto report=refundPanel->property("report").toMap();check(report.value("refunded").toInt()==200,"native operator approval reconciles exact partial refund");
                                const auto items=report.value("items").toList();check(!items.isEmpty()&&items.first().toMap().value("feeAmount").toInt()==8&&items.first().toMap().value("feeState").toString()=="succeeded","native refund reports separate exact fee reversal");}
                        }
                        press(findItem(window->contentItem(),"closePurchases"));
                    openNavigation();
                        press(findItem(window->contentItem(),"navSubmit"));for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"commerceWorkspaceTab"));
                        press(findItem(window->contentItem(),"loadFinanceSellers"));for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"reconcileFinances"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        auto *finances=window->findChild<QQuickItem*>("financesPanel");auto financeReport=finances?finances->property("report").toMap():QVariantMap{};
                        check(financeReport.value("providerFeesPending").toInt()==0&&!financeReport.value("currencies").toList().isEmpty(),"native author finance report reconciles provider costs");
                        auto *orderRef=findItem(window->contentItem(),"financeOrderReference");if(orderRef){orderRef->forceActiveFocus();for(const auto character:purchaseOrder.value("id").toString().toLatin1())QTest::keyClick(window,character);}
                        auto *financeScenario=findItem(window->contentItem(),"financeScenario");if(financeScenario){financeScenario->forceActiveFocus();QTest::keyClick(window,Qt::Key_Home);QTest::keyClick(window,Qt::Key_Down);}
                        press(findItem(window->contentItem(),"simulateFinance"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        financeReport=finances?finances->property("report").toMap():QVariantMap{};const auto payouts=financeReport.value("payouts").toList();check(!payouts.isEmpty()&&payouts.first().toMap().value("state").toString()=="failed","native report retains failed provider payout");
                        if(financeScenario){financeScenario->forceActiveFocus();QTest::keyClick(window,Qt::Key_Home);for(int i=0;i<3;++i)QTest::keyClick(window,Qt::Key_Down);}
                        press(findItem(window->contentItem(),"simulateFinance"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        press(findItem(window->contentItem(),"previewFinancePacket"));for(int i=0;i<100&&core.loading();++i)QTest::qWait(50);
                        const auto packet=finances?finances->property("packet").toMap():QVariantMap{};check(!packet.value("digest").toString().isEmpty(),"native operator previews private dispute packet");
                        const auto packetPath=licenceDirectory.filePath("dispute.json");const auto dispute=packet.value("packet").toMap().value("dispute").toMap();
                        core.workspaceAction("commerce.packet.export",{{"id",dispute.value("id")},{"digest",packet.value("digest")},{"file",QUrl::fromLocalFile(packetPath).toString()}});for(int i=0;i<80&&core.loading();++i)QTest::qWait(50);check(QFileInfo::exists(packetPath),"actual reviewed private dispute packet export");

                        core.refresh();
                        for(int i=0;i<60 && core.loading();++i) QTest::qWait(50);

                    }
                    auto *localTab=findItem(window->contentItem(),"localWorksheetTab");
                    if (localTab) {localTab->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                    QTest::qWait(100);
                    auto *name = findItem(window->contentItem(), "field-name");
                    if (name) {
                        name->forceActiveFocus();
                        for (const auto character : QByteArray("My tool")) QTest::keyClick(window, character);
                    }
                    check(worksheet.fields().value("name").toString() == "My tool", "worksheet typing");
                    auto *saveWorksheet = findItem(window->contentItem(), "saveWorksheet");
                    if (saveWorksheet) { saveWorksheet->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    check(!worksheet.dirty(), "worksheet save");
                    auto *checkWorksheet = findItem(window->contentItem(), "checkWorksheet");
                    if (checkWorksheet) { checkWorksheet->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    QTest::qWait(200);
                    check(!worksheet.result().value("valid").toBool() && !worksheet.result().value("errors").toList().isEmpty(), "worksheet validation feedback");
                    auto *discover = findItem(window->contentItem(), "navDiscover");
                    if (discover) { discover->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    QTest::qWait(100);
                }
                if (!demo && (parser.isSet(storefrontTest) || parser.value(captureView) != "discover")) {
                    auto *originalBrowse = findItem(window->contentItem(), "browseScroll");
                    auto *feature = findItem(window->contentItem(), "discoverCalculator");
                    check(feature != nullptr, "repository discovery feature");
                    if (feature) { feature->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    for (int i=0; i<100 && core.detail().isEmpty(); ++i) QTest::qWait(50);
                    check(core.detail().value("app").toMap().value("id").toString()=="repo-omacalc", "feature opens real OmaCalc");
                    QTest::qWait(150);
                    if (parser.isSet(storefrontTest)) {
                        auto *save = findItem(window->contentItem(), "saveButton");
                        check(save != nullptr, "repository save action");
                        if (save) { save->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                        check(core.isSaved("repo-omacalc"), "save real app");
                    }
                    if (parser.isSet(storefrontTest) || parser.value(captureView)=="plan") {
                        auto *preview = findItem(window->contentItem(), "previewAppPlan");
                        for(int i=0;i<100 && core.loading();++i)QTest::qWait(50);
                        check(preview && preview->isEnabled(), "repository plan action available");
                        if(preview){preview->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for(int i=0;i<100 && core.community().value("system.plan").toMap().isEmpty();++i)QTest::qWait(50);
                        const auto plan=core.community().value("system.plan").toMap();
                        check(!plan.value("digest").toString().isEmpty(), "real package plan received");
                        check(!plan.value("blockers").toList().isEmpty(), "unverified live install remains blocked");
                        QTest::qWait(150);
                        auto *confirm=findItem(window->contentItem(),"confirmPlan");
                        check(confirm && !confirm->isEnabled(), "blocked plan cannot be confirmed");
                        if(parser.isSet(storefrontTest)) {
                            auto *close=findItem(window->contentItem(),"closePlan");
                            if(close){close->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                            QTest::qWait(100);
                            QTest::keyClick(window,Qt::Key_Escape);
                            QTest::qWait(100);
                            check(core.detail().isEmpty(), "return to discovery");
                            check(window->property("section").toInt()==0 && originalBrowse==findItem(window->contentItem(),"browseScroll"), "detail Back preserves its Discover origin and mounted scroll view");
                            check(core.isSaved("repo-omacalc"), "saved app survives return");
                            auto *shelf = findItem(window->contentItem(), "appShelf");
                            check(shelf != nullptr, "persistent catalogue shelf");
                            const qreal shelfX = shelf ? shelf->property("contentX").toReal() : 0;
                            auto *entry = findItem(window->contentItem(), "shelf-repo-omacalc");
                            check(entry != nullptr, "real app shelf selector");
                            if (entry) { entry->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                            for (int i=0;i<60 && core.detail().isEmpty();++i) QTest::qWait(50);
                            check(core.detail().value("app").toMap().value("id").toString()=="repo-omacalc", "shelf selects real app");
                            check(shelf == findItem(window->contentItem(), "appShelf"), "shelf survives app selection");
                            check(!shelf || qAbs(shelf->property("contentX").toReal()-shelfX)<1, "shelf position preserved");
                            QTest::keyClick(window, Qt::Key_K, Qt::ControlModifier);
                            for (const auto character : QByteArray("nomatchingapp")) QTest::keyClick(window, character);
                            QTest::qWait(300);
                            check(core.total()==0, "empty search results");
                            core.clearFilters();
                            for(int i=0;i<60&&core.loading();++i)QTest::qWait(50);
                            check(core.total()>0, "search recovery restores catalogue");
                        }
                    }
                }
                if (parser.isSet(screenshot)) {
                    // Let async font resolution and its resulting layout reach a frame.
                    QTest::qWait(250);
                    window->requestUpdate();
                    QTest::qWait(100);
                    ok = window->grabWindow().save(parser.value(screenshot)) && ok;
                }
                app.exit(ok && !qmlWarning ? 0 : 1);
            });
        });
        QObject::connect(&core, &CoreBridge::stateChanged, &app, [&] { if (!core.ready() && !core.error().isEmpty()) app.exit(1); });
        QTimer::singleShot(20000, &app, [&app] { app.exit(1); });
    }
#endif
    core.start();
    return app.exec();
}
