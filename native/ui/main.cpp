#include "CoreBridge.h"
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
#include <QQuickWindow>
#include <QQuickItem>
#include <QSettings>
#ifdef OMASTORE_QA
#include <QTemporaryDir>
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
    parser.addOptions({uiTest, screenshot, size});
#endif
#ifdef OMASTORE_DEVELOPMENT_DATA
    const QCommandLineOption demoOption("demo", "Show explicitly fictional development listings.");
    parser.addOption(demoOption);
#endif
    parser.process(app);

    bool demo = false;
#ifdef OMASTORE_DEVELOPMENT_DATA
    demo = parser.isSet(demoOption);
#endif
    if (demo) QCoreApplication::setApplicationName("OmaStoreDevelopment");
    QString dataDirectory = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
#ifdef OMASTORE_QA
    QTemporaryDir testSettings;
    if (parser.isSet(smokeTest) || parser.isSet(uiTest) || parser.isSet(screenshot)) {
        dataDirectory = testSettings.filePath("data");
        qputenv("XDG_DATA_HOME",testSettings.filePath("xdg-data").toUtf8());
        QSettings::setDefaultFormat(QSettings::IniFormat);
        QSettings::setPath(QSettings::IniFormat, QSettings::UserScope, testSettings.path());
    }
    #endif
    DesktopSettings desktop;
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
    if (!window) return 1;
#ifdef OMASTORE_QA
    if (parser.isSet(size)) {
        const auto dimensions = parser.value(size).split('x');
        if (dimensions.size() != 2) return 2;
        const int width = dimensions[0].toInt(), height = dimensions[1].toInt();
        if (width < 800 || height < 600 || width > 3840 || height > 2160) return 2;
        window->resize(width, height);
    }

    if (parser.isSet(smokeTest) || parser.isSet(uiTest) || parser.isSet(screenshot)) {
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
                    auto *submit = findItem(window->contentItem(), "navSubmit");
                    if (submit) { submit->forceActiveFocus(); QTest::keyClick(window, Qt::Key_Space); }
                    QTest::qWait(150);
                    if (demo) {
                        for (int i=0;i<40 && core.loading();++i) QTest::qWait(50);
                        auto *sampleSignIn=findItem(window->contentItem(),"sampleSignIn");
                        if (sampleSignIn) {sampleSignIn->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for (int i=0;i<40 && core.loading();++i) QTest::qWait(50);
                        check(core.workspace().value("actor").toMap().value("id").toString()=="development:author","sample author sign-in");
                        auto *newDraft=findItem(window->contentItem(),"newDraft");
                        if (newDraft) {newDraft->forceActiveFocus();QTest::keyClick(window,Qt::Key_Space);}
                        for (int i=0;i<60 && (core.loading() || !findItem(window->contentItem(),"field-apps-0-name"));++i) QTest::qWait(50);
                        auto *draftName=findItem(window->contentItem(),"field-apps-0-name");
                        check(draftName!=nullptr,"native draft editor created");
                        if (draftName) {draftName->forceActiveFocus();for (const auto character:QByteArray("My draft")) QTest::keyClick(window,character);QTest::keyClick(window,Qt::Key_Tab);}
                        QTest::qWait(850);
                        check(core.workspaceReply().value("localSaved").toBool(),"private draft autosave");
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
                if (parser.isSet(screenshot)) ok = window->grabWindow().save(parser.value(screenshot)) && ok;
                app.exit(ok && !qmlWarning ? 0 : 1);
            });
        });
        QObject::connect(&core, &CoreBridge::stateChanged, &app, [&] { if (!core.ready() && !core.error().isEmpty()) app.exit(1); });
        QTimer::singleShot(10000, &app, [&app] { app.exit(1); });
    }
#endif
    core.start();
    return app.exec();
}
