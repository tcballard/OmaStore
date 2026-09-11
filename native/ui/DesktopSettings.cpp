#include "DesktopSettings.h"
#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QFontDatabase>
#include <QGuiApplication>
#include <QPalette>
#include <QStyleHints>
#include <cmath>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QRegularExpression>
#include <QSettings>
#include <QStandardPaths>

namespace {
double luminance(const QColor &c) {
    auto linear = [](double v) { return v <= 0.04045 ? v / 12.92 : std::pow((v + 0.055) / 1.055, 2.4); };
    return 0.2126 * linear(c.redF()) + 0.7152 * linear(c.greenF()) + 0.0722 * linear(c.blueF());
}
QColor readable(QColor text, const QColor &background) {
    const double a = luminance(text), b = luminance(background);
    if ((qMax(a, b) + 0.05) / (qMin(a, b) + 0.05) >= 4.5) return text;
    return b > 0.179 ? QColor(Qt::black) : QColor(Qt::white);
}
}

DesktopSettings::DesktopSettings(QObject *parent)
    : DesktopSettings(QDir::homePath() + "/.local/state/omarchy/current/theme",
          QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation) + "/fontconfig/fonts.conf", parent) {}

DesktopSettings::DesktopSettings(const QString &themeDirectory, const QString &fontConfig, QObject *parent)
    : QObject(parent), m_themeDirectory(themeDirectory), m_fontConfig(fontConfig) {
    m_follow = QSettings().value("appearance/followOmarchy", true).toBool();
    m_font = QFontDatabase::systemFont(QFontDatabase::FixedFont).family();
    m_debounce.setSingleShot(true);
    m_debounce.setInterval(180);
    connect(&m_debounce, &QTimer::timeout, this, [this] { reloadTheme(); refreshFont(); });
    connect(&m_files, &QFileSystemWatcher::fileChanged, this, [this] { m_debounce.start(); });
    connect(&m_files, &QFileSystemWatcher::directoryChanged, this, [this] { m_debounce.start(); });
    // Polling also covers missing ancestors, symlink retargets and fontconfig includes.
    m_poll.setInterval(5000);
    connect(&m_poll, &QTimer::timeout, this, [this] { reloadTheme(); refreshFont(); });
    m_poll.start();
    m_fontTimeout.setSingleShot(true);
    m_fontTimeout.setInterval(1000);
    connect(&m_fontTimeout, &QTimer::timeout, &m_fontProcess, &QProcess::kill);
    connect(&m_fontProcess, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
        [this](int code, QProcess::ExitStatus status) {
            m_fontTimeout.stop();
            const QByteArray output = m_fontProcess.readAllStandardOutput();
            if (code != 0 || status != QProcess::NormalExit || output.size() > 1024) return;
            const QString family = QString::fromUtf8(output).section('\n', 0, 0).section(',', 0, 0).trimmed();
            if (!family.isEmpty() && family.size() <= 200 && family != m_font) { m_font = family; emit changed(); }
        });
    reloadTheme();
    refreshFont();
    m_dark = QGuiApplication::palette().color(QPalette::Window).lightness() < 128;
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    connect(QGuiApplication::styleHints(), &QStyleHints::colorSchemeChanged, this, [this](Qt::ColorScheme scheme) {
        if (scheme != Qt::ColorScheme::Unknown) { m_dark = scheme == Qt::ColorScheme::Dark; emit changed(); }
    });
#endif
    auto bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) return;
    bus.connect("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings", "SettingChanged",
        this, SLOT(settingChanged(QString,QString,QDBusVariant)));
    read("org.freedesktop.appearance", "color-scheme");
    read("org.gnome.desktop.interface", "text-scaling-factor");
}
QString DesktopSettings::monoFont() const { return m_font; }
DesktopSettings::~DesktopSettings() {
    if (m_fontProcess.state() != QProcess::NotRunning) {
        m_fontProcess.kill();
        m_fontProcess.waitForFinished(1000);
    }
}
void DesktopSettings::setFollowOmarchy(bool value) {
    if (m_follow == value) return;
    m_follow = value;
    QSettings().setValue("appearance/followOmarchy", value);
    emit changed();
}
void DesktopSettings::refreshFont() {
    if (m_fontProcess.state() != QProcess::NotRunning) return;
    // Fixed read-only executable/arguments; no theme-supplied commands or shell.
    m_fontProcess.setProgram("/usr/bin/fc-match");
    m_fontProcess.setArguments({"monospace", "-f", "%{family}\n"});
    m_fontProcess.setStandardErrorFile(QProcess::nullDevice());
    m_fontProcess.start();
    m_fontTimeout.start();
}
void DesktopSettings::watchPaths() {
    QStringList desired;
    for (QString path : {m_themeDirectory + "/colors.toml", m_fontConfig}) {
        // Nearest existing path plus two parents covers replacement without
        // subscribing to every unrelated change above the configuration tree.
        int found = 0;
        for (;;) {
            if (QFileInfo::exists(path)) {
                if (!desired.contains(path)) desired.append(path);
                if (++found == 3) break;
            }
            const QString parent = QFileInfo(path).absolutePath();
            if (parent == path || parent == "/") break;
            path = parent;
        }
    }
    const QStringList watched = m_files.files() + m_files.directories();
    for (const auto &path : watched) if (!desired.contains(path)) m_files.removePath(path);
    for (const auto &path : desired) if (!watched.contains(path)) m_files.addPath(path);
}
void DesktopSettings::reloadTheme() {
    watchPaths();
    QVariantMap next, raw;
    QFile file(m_themeDirectory + "/colors.toml");
    if (QFileInfo(file).isFile() && file.size() <= 32768 && file.open(QIODevice::ReadOnly)) {
        const QByteArray bytes = file.read(32769);
        if (bytes.size() <= 32768) {
            const QRegularExpression colorLine(R"re(^\s*([a-z_]+)\s*=\s*["'](#[0-9a-fA-F]{6})["']\s*(?:#.*)?$)re");
            for (const auto &line : QString::fromUtf8(bytes).split('\n')) {
                const auto match = colorLine.match(line);
                if (match.hasMatch()) raw.insert(match.captured(1), QColor(match.captured(2)));
            }
        }
    }
    if (raw.contains("background") && raw.contains("foreground")) {
        auto color = [&raw](const char *key, QColor fallback) { return raw.value(QString::fromLatin1(key), fallback).value<QColor>(); };
        const QColor page = color("background", Qt::black);
        const bool dark = luminance(page) < 0.179;
        const QColor surface = color("lighter_background", dark ? page.lighter(125) : page.darker(105));
        const QColor sidebar = color("dark_background", page);
        const QColor ink = readable(color("foreground", Qt::white), page);
        const QColor accent = readable(color("accent", ink), page);
        QColor secondary = color("dark_foreground", ink);
        if (readable(secondary, page) != secondary || readable(secondary, surface) != secondary) secondary = ink;
        next = {{"dark", dark}, {"page", page}, {"surface", surface}, {"sidebar", sidebar},
            {"ink", readable(readable(ink, surface), sidebar)},
            {"muted", secondary},
            {"accent", accent}, {"accentInk", readable(page, accent)},
            {"line", color("muted", ink)}, {"wash", color("selection", surface)}};
        // Secondary labels also occur on raised surfaces.
        next["muted"] = readable(next["muted"].value<QColor>(), surface);
    }
    if (next != m_colors) { m_colors = next; emit changed(); }
}
void DesktopSettings::read(const QString &nameSpace, const QString &key) {
    auto call = QDBusMessage::createMethodCall("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings", "Read");
    call << nameSpace << key;
    auto *watcher = new QDBusPendingCallWatcher(QDBusConnection::sessionBus().asyncCall(call, 250), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, nameSpace, key](QDBusPendingCallWatcher *done) {
        QDBusPendingReply<QDBusVariant> reply = *done;
        if (!reply.isError()) apply(nameSpace, key, reply.value().variant());
        done->deleteLater();
    });
}
void DesktopSettings::settingChanged(const QString &nameSpace, const QString &key, const QDBusVariant &value) { apply(nameSpace, key, value.variant()); }
void DesktopSettings::apply(const QString &nameSpace, const QString &key, QVariant value) {
    while (value.metaType() == QMetaType::fromType<QDBusVariant>()) value = value.value<QDBusVariant>().variant();
    if (nameSpace == "org.freedesktop.appearance" && key == "color-scheme") {
        bool ok; const int scheme = value.toInt(&ok);
        if (ok && (scheme == 1 || scheme == 2) && m_dark != (scheme == 1)) { m_dark = scheme == 1; emit changed(); }
    } else if (nameSpace == "org.gnome.desktop.interface" && key == "text-scaling-factor") {
        bool ok; const double scale = value.toDouble(&ok);
        if (ok && std::isfinite(scale) && scale > 0) {
            const double next = qBound(0.5, scale, 3.0);
            if (!qFuzzyCompare(m_scale, next)) { m_scale = next; emit changed(); }
        }
    }
}
