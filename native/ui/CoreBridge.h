#pragma once

#include <QObject>
#include <QProcess>
#include <QTimer>

class CoreBridge final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(QString version READ version NOTIFY stateChanged)
    Q_PROPERTY(QString error READ error NOTIFY stateChanged)

public:
    explicit CoreBridge(QObject *parent = nullptr);
    ~CoreBridge() override;
    void start();
    bool ready() const { return m_ready; }
    QString version() const { return m_version; }
    QString error() const { return m_error; }

signals:
    void stateChanged();

private:
    void readOutput();
    void acceptReply(const QByteArray &line);
    void fail(const QString &message);

    QProcess m_process;
    QTimer m_timeout;
    QByteArray m_output;
    bool m_ready = false;
    QString m_version;
    QString m_error;
};
