#include "Instance.h"
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDir>
#include <QStandardPaths>

Instance::Instance(bool demo,QObject *parent):QObject(parent),m_service(demo?"io.github.tcballard.OmaStore.Sample":"io.github.tcballard.OmaStore") {}
Instance::Result Instance::acquire(const QString &uri) {
    const auto runtime=QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    if(runtime.isEmpty())return Unavailable;
    m_lock=std::make_unique<QLockFile>(QDir(runtime).filePath(m_service+".lock"));
    m_lock->setStaleLockTime(0);
    if(!m_lock->tryLock(0)) {
        QDBusInterface existing(m_service,"/Window","io.github.tcballard.OmaStore.Window",QDBusConnection::sessionBus());
        existing.setTimeout(1500);
        const QDBusReply<bool> reply=existing.call("Open",uri);
        return reply.isValid()&&reply.value()?Forwarded:Unavailable;
    }
    auto bus=QDBusConnection::sessionBus();
    if(bus.isConnected() && (!bus.registerService(m_service) || !bus.registerObject("/Window",this,QDBusConnection::ExportAllSlots)))return Unavailable;
    m_pending=uri;return Primary; // Without D-Bus the lock still prevents a second write session.
}
bool Instance::Open(const QString &uri) {
    if(uri.size()>400 || (!uri.isEmpty() && !uri.startsWith("omastore://")))return false;
    if(m_ready)emit requested(uri);else m_pending=uri;return true; // Rust checks the complete identity before navigation.
}

void Instance::ready(){m_ready=true;if(!m_pending.isNull()){emit requested(m_pending);m_pending=QString();}}
