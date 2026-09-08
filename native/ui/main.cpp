#include "CoreBridge.h"

#include <QCommandLineParser>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickStyle>
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
    parser.process(app);

    CoreBridge core;
    QQmlApplicationEngine engine;
    engine.setInitialProperties({{"core", QVariant::fromValue(&core)}});
    engine.load(QUrl(QStringLiteral("qrc:/qt/qml/OmaStore/Main.qml")));
    if (engine.rootObjects().isEmpty()) return 1;

    if (parser.isSet(smokeTest)) {
        QObject::connect(&core, &CoreBridge::stateChanged, &app, [&] {
            if (core.ready()) QTimer::singleShot(250, &app, &QCoreApplication::quit);
            else if (!core.error().isEmpty()) app.exit(1);
        });
        QTimer::singleShot(8000, &app, [&app] { app.exit(1); });
    }
    core.start();
    return app.exec();
}
