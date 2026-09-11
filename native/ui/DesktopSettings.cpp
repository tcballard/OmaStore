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

DesktopSettings::DesktopSettings(QObject *parent) : QObject(parent) {
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
QString DesktopSettings::monoFont() const { return QFontDatabase::systemFont(QFontDatabase::FixedFont).family(); }
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
