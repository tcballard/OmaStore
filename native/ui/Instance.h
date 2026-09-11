#pragma once
#include <QObject>
#include <QLockFile>
#include <memory>

class Instance final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "io.github.tcballard.OmaStore.Window")
public:
    explicit Instance(bool demo, QObject *parent=nullptr);
    enum Result { Primary, Forwarded, Unavailable };
    Result acquire(const QString &uri);
    void ready();
public slots:
    bool Open(const QString &uri);
signals:
    void requested(const QString &uri);
private:
    QString m_service,m_pending;
    bool m_ready=false;
    std::unique_ptr<QLockFile> m_lock;
};
