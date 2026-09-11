#include "MediaPreview.h"
#include <QBuffer>
#include <QCryptographicHash>
#include <QImageReader>
#include <QNetworkRequest>
#include <QRegularExpression>

MediaPreview::MediaPreview(QObject *parent) : QObject(parent) {
    m_timeout.setSingleShot(true); m_timeout.setInterval(10000);
    connect(&m_timeout, &QTimer::timeout, this, [this] { if (m_reply) m_reply->abort(); });
}
bool MediaPreview::verifyImage(const QByteArray &bytes, const QByteArray &sha256, const QString &destination) {
    if (bytes.isEmpty() || bytes.size() > 5 * 1024 * 1024
        || QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex() != sha256) return false;
    QBuffer buffer; buffer.setData(bytes); buffer.open(QIODevice::ReadOnly);
    QImageReader reader(&buffer); reader.setDecideFormatFromContent(true);
    const auto format = reader.format(); const auto size = reader.size();
    if ((format != "png" && format != "jpeg" && format != "webp") || !size.isValid()
        || size.width() > 8192 || size.height() > 8192 || qint64(size.width()) * size.height() > 16000000) return false;
    reader.setScaledSize(size.scaled(1600, 1200, Qt::KeepAspectRatio));
    const auto image = reader.read();
    return !image.isNull() && image.save(destination, "PNG");
}
void MediaPreview::load(const QVariantMap &media) {
    if (m_reply) { auto *old = m_reply; m_reply = nullptr; old->abort(); old->deleteLater(); }
    m_timeout.stop(); m_source = QUrl(); m_bytes.clear();
    if (media.isEmpty()) { m_status = "idle"; emit changed(); return; }
    const QUrl url(media.value("url").toString(), QUrl::StrictMode);
    const auto sha = media.value("sha256").toByteArray();
    const auto kind = media.value("kind").toString();
    // Public preview assets are inert, digest-addressed files from our release-controlled origin.
    // Other publisher media remains a labelled external link until its hosting adapter is approved.
    const QString prefix = "/tcballard/OmaStore/main/media/";
    const QRegularExpression filePattern("^" + QString::fromLatin1(sha) + "\\.(png|jpg|jpeg|webp)$");
    if (kind == "demo" || sha.size() != 64 || !QRegularExpression("^[a-f0-9]{64}$").match(QString::fromLatin1(sha)).hasMatch()
        || url.scheme() != "https" || url.host() != "raw.githubusercontent.com" || !url.userInfo().isEmpty() || url.port(-1) != -1
        || !url.query().isEmpty() || !url.fragment().isEmpty() || !url.path().startsWith(prefix)
        || !filePattern.match(url.path().mid(prefix.size())).hasMatch()) { m_status = "external"; emit changed(); return; }
    QNetworkRequest request(url);
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute, QNetworkRequest::ManualRedirectPolicy);
    request.setTransferTimeout(10000);
    m_status = "loading"; emit changed();
    auto *reply = m_network.get(request); m_reply = reply;
    reply->setReadBufferSize(64 * 1024);
    const qsizetype limit = kind == "icon" ? 1024 * 1024 : 5 * 1024 * 1024;
    connect(reply, &QNetworkReply::readyRead, this, [this, reply, limit] {
        if (reply != m_reply) return;
        m_bytes += reply->readAll(); if (m_bytes.size() > limit) reply->abort();
    });
    connect(reply, &QNetworkReply::finished, this, [this, reply, sha, limit] {
        if (reply != m_reply) { reply->deleteLater(); return; }
        m_timeout.stop(); m_bytes += reply->readAll();
        const auto path = m_directory.filePath(QString::fromLatin1(sha) + ".png");
        const bool ok = reply->error() == QNetworkReply::NoError && reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt() == 200
            && m_bytes.size() <= limit && verifyImage(m_bytes, sha, path);
        if (ok) m_source = QUrl::fromLocalFile(path);
        m_status = ok ? "ready" : "failed"; m_reply = nullptr; reply->deleteLater(); m_bytes.clear(); emit changed();
    });
    m_timeout.start();
}
