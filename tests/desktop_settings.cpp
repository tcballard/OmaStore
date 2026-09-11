#include "DesktopSettings.h"
#include <QColor>
#include <QDir>
#include <QFile>
#include <QSaveFile>
#include <QSettings>
#include <QTemporaryDir>
#include <QtTest>

class DesktopSettingsTests : public QObject {
    Q_OBJECT
    QTemporaryDir prefs;
    // Recovery may use the five-second poll, followed by event-loop dispatch.
    // Qt's default five-second QTRY deadline races that legitimate path.
    static constexpr int ReloadTimeoutMs = 8000;
    static void write(const QString &path, const QByteArray &data) {
        QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
        QSaveFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        QCOMPARE(file.write(data), data.size());
        QVERIFY(file.commit());
    }
private slots:
    void initTestCase() {
        QCoreApplication::setOrganizationName("OmaStoreTests");
        QCoreApplication::setApplicationName("DesktopSettings");
        QSettings::setDefaultFormat(QSettings::IniFormat);
        QSettings::setPath(QSettings::IniFormat, QSettings::UserScope, prefs.path());
    }
    void init() { QSettings().clear(); }
    void preferenceAndFallback() {
        QTemporaryDir root;
        DesktopSettings settings(root.path() + "/theme", root.path() + "/fonts.conf");
        QVERIFY(settings.followOmarchy());
        QVERIFY(settings.themeColors().isEmpty());
        QVERIFY(!settings.monoFont().isEmpty());
        settings.setFollowOmarchy(false);
        DesktopSettings reopened(root.path() + "/theme", root.path() + "/fonts.conf");
        QVERIFY(!reopened.followOmarchy());
        reopened.setFollowOmarchy(true);
        QCOMPARE(QSettings().value("appearance/followOmarchy").toBool(), true);
    }
    void atomicReplacementAndLateCreation() {
        QTemporaryDir root;
        const QString theme = root.path() + "/current/theme";
        DesktopSettings settings(theme, root.path() + "/fonts.conf");
        write(theme + "/colors.toml", "background = '#1a1b26'\nforeground = '#c0caf5'\naccent = '#7aa2f7'\nmuted = '#414868'\n");
        QTRY_COMPARE_WITH_TIMEOUT(settings.themeColors().value("page").value<QColor>(), QColor("#1a1b26"), ReloadTimeoutMs);
        QCOMPARE(settings.themeColors().value("dark").toBool(), true);
        QVERIFY(settings.themeColors().value("muted").value<QColor>() != QColor("#414868"));
        QVERIFY(QDir().rename(theme, root.path() + "/previous"));
        write(theme + "/colors.toml", "background = \"#fafafa\"\nforeground = \"#202020\"\naccent = \"#315a3d\"\n");
        QTRY_COMPARE_WITH_TIMEOUT(settings.themeColors().value("page").value<QColor>(), QColor("#fafafa"), ReloadTimeoutMs);
        QCOMPARE(settings.themeColors().value("dark").toBool(), false);
        write(theme + "/colors.toml", "background = '#eeeeee'\nforeground = '#111111'\n");
        QTRY_COMPARE_WITH_TIMEOUT(settings.themeColors().value("page").value<QColor>(), QColor("#eeeeee"), ReloadTimeoutMs);
        QVERIFY(QFile::remove(theme + "/colors.toml"));
        QTRY_VERIFY_WITH_TIMEOUT(settings.themeColors().isEmpty(), ReloadTimeoutMs);
    }
    void malformedAndBounded() {
        QTemporaryDir root;
        const QString theme = root.path() + "/theme";
        write(theme + "/colors.toml", "background = '$(touch nope)'\nforeground = '#ffffff'\n");
        DesktopSettings settings(theme, root.path() + "/fonts.conf");
        QVERIFY(settings.themeColors().isEmpty());
        write(theme + "/colors.toml", "background = '#ffffff'\nforeground = '#ffffff'\naccent = '#eeeeee'\n");
        QTRY_VERIFY_WITH_TIMEOUT(!settings.themeColors().isEmpty(), ReloadTimeoutMs);
        QCOMPARE(settings.themeColors().value("ink").value<QColor>(), QColor(Qt::black));
        QCOMPARE(settings.themeColors().value("accent").value<QColor>(), QColor(Qt::black));
        write(theme + "/colors.toml", QByteArray(33000, 'x'));
        QTRY_VERIFY_WITH_TIMEOUT(settings.themeColors().isEmpty(), ReloadTimeoutMs);
    }
    void fontconfigLiveReload() {
        if (!QFile::exists("/usr/bin/fc-match")) QSKIP("fontconfig executable unavailable");
        QTemporaryDir root;
        const QString config = root.path() + "/fonts.conf";
        const QByteArray previous = qgetenv("FONTCONFIG_FILE");
        struct Restore { QByteArray value; ~Restore() { if (value.isNull()) qunsetenv("FONTCONFIG_FILE"); else qputenv("FONTCONFIG_FILE", value); } } restore{previous};
        auto font = [&](const QByteArray &family) {
            write(config, "<fontconfig><include ignore_missing=\"yes\">/etc/fonts/fonts.conf</include><match target=\"pattern\"><test name=\"family\"><string>monospace</string></test><edit name=\"family\" mode=\"assign\" binding=\"strong\"><string>" + family + "</string></edit></match></fontconfig>");
        };
        font("DejaVu Serif");
        qputenv("FONTCONFIG_FILE", config.toUtf8());
        DesktopSettings settings(root.path() + "/theme", config);
        QTRY_COMPARE_WITH_TIMEOUT(settings.monoFont(), QString("DejaVu Serif"), ReloadTimeoutMs);
        font("DejaVu Sans Mono");
        QTRY_COMPARE_WITH_TIMEOUT(settings.monoFont(), QString("DejaVu Sans Mono"), ReloadTimeoutMs);
    }
};
QTEST_MAIN(DesktopSettingsTests)
#include "desktop_settings.moc"
