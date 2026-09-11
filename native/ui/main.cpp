#include "CoreBridge.h"
#ifdef OMASTORE_TESTING
#include "DiscoveryChecks.h"
#endif

#include <QCommandLineParser>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QTimer>
#include <QVariant>

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    QCoreApplication::setApplicationName("OmaStore");
    QCoreApplication::setApplicationVersion("0.1.0");
    QCoreApplication::setOrganizationName("OmaStore");
    QGuiApplication::setDesktopFileName("io.github.tcballard.OmaStore");
    QQuickStyle::setStyle("Fusion");

    QCommandLineParser parser;
    parser.setApplicationDescription("Native application storefront for Omarchy");
    parser.addHelpOption();
    parser.addVersionOption();
    const QCommandLineOption smokeTest("smoke-test", "Exit after verifying the window and local core startup.");
    parser.addOption(smokeTest);
    parser.addOption(QCommandLineOption("screenshot", "Save a QA screenshot and exit.", "file"));
    parser.addOption(QCommandLineOption("show-app", "Open a catalogue app ID for QA.", "id"));
    parser.addOption(QCommandLineOption("window-size", "QA window size WIDTHxHEIGHT.", "size"));
#ifdef OMASTORE_TESTING
    parser.addOption(QCommandLineOption("interaction-test", "Run native interaction checks and exit."));
    parser.addOption(QCommandLineOption("expect-search", "Check persisted search after restart.", "text"));
#endif
    parser.process(app);

    CoreBridge core;
    QQmlApplicationEngine engine;
    engine.setInitialProperties({{"core", QVariant::fromValue(&core)}});
    engine.load(QUrl(QStringLiteral("qrc:/qt/qml/OmaStore/Main.qml")));
    if (engine.rootObjects().isEmpty()) return 1;

    auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
    if (parser.isSet("window-size")) {
        auto parts=parser.value("window-size").split('x');
        if(parts.size()!=2 || parts[0].toInt()<800 || parts[1].toInt()<600 || parts[0].toInt()>3840 || parts[1].toInt()>2160) return 2;
        window->resize(parts[0].toInt(),parts[1].toInt());
    }
    if (parser.isSet("screenshot")) {
        auto *capture = new QTimer(&app); capture->setInterval(100);
        bool *opened = new bool(false);
        QObject::connect(capture,&QTimer::timeout,&app,[&,opened,capture] {
            if(!core.error().isEmpty()){app.exit(1);return;}
            if(!core.ready() || core.busy())return;
            if(parser.isSet("show-app") && !*opened){
                *opened=true;
                QMetaObject::invokeMethod(window,"openApp",Q_ARG(QVariant,parser.value("show-app")));
                return;
            }
            capture->stop();
            QTimer::singleShot(250,&app,[&]{app.exit(window->grabWindow().save(parser.value("screenshot")) ? 0 : 1);});
        });
        capture->start();
        QObject::connect(&app,&QCoreApplication::aboutToQuit,&app,[opened]{delete opened;});
        QTimer::singleShot(15000,&app,[&app]{app.exit(1);});
    }
    if (parser.isSet(smokeTest)) {
        QObject::connect(&core, &CoreBridge::stateChanged, &app, [&] {
            if (core.ready()) QTimer::singleShot(250, &app, &QCoreApplication::quit);
            else if (!core.error().isEmpty()) app.exit(1);
        });
        QTimer::singleShot(8000, &app, [&app] { app.exit(1); });
    }
#ifdef OMASTORE_TESTING
    if(parser.isSet("interaction-test")) QTimer::singleShot(100,&app,[&]{app.exit(checkDiscovery(core,window)?0:1);});
    if(parser.isSet("expect-search") && core.filters().value("text").toString()!=parser.value("expect-search"))return 1;
#endif
    core.start();
    return app.exec();
}
