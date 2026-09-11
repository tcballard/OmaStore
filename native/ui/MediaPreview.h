#pragma once
#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QTemporaryDir>
#include <QTimer>
#include <QUrl>
#include <QVariantMap>

class MediaPreview final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QUrl source READ source NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
public:
    explicit MediaPreview(QObject *parent = nullptr);
    Q_INVOKABLE void load(const QVariantMap &media);
    QUrl source() const { return m_source; }
    QString status() const { return m_status; }
    static bool verifyImage(const QByteArray &bytes, const QByteArray &sha256, const QString &destination);
signals:
    void changed();
private:
    QNetworkAccessManager m_network;
    QNetworkReply *m_reply = nullptr;
    QTemporaryDir m_directory;
    QTimer m_timeout;
    QByteArray m_bytes;
    QUrl m_source;
    QString m_status = "idle";
};
