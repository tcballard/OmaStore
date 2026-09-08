#include "MediaPreview.h"
#include <QBuffer>
#include <QCryptographicHash>
#include <QImage>
#include <QTemporaryDir>
#include <QTest>

class NativeContracts : public QObject {
    Q_OBJECT
private slots:
    void mediaRequiresMatchingBytesAndAnInertFormat() {
        QTemporaryDir dir;
        QImage image(16, 12, QImage::Format_RGB32); image.fill(Qt::green);
        QByteArray bytes; QBuffer output(&bytes); output.open(QIODevice::WriteOnly);
        QVERIFY(image.save(&output, "PNG"));
        const auto sha = QCryptographicHash::hash(bytes, QCryptographicHash::Sha256).toHex();
        QVERIFY(MediaPreview::verifyImage(bytes, sha, dir.filePath("preview.png")));
        QVERIFY(!MediaPreview::verifyImage(bytes, QByteArray(64, '0'), dir.filePath("wrong.png")));
        const QByteArray active("<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>");
        QVERIFY(!MediaPreview::verifyImage(active, QCryptographicHash::hash(active, QCryptographicHash::Sha256).toHex(), dir.filePath("active.png")));
        QVERIFY(!MediaPreview::verifyImage(QByteArray(5 * 1024 * 1024 + 1, 'a'), sha, dir.filePath("large.png")));
    }
    void unapprovedOriginsNeverEnterThePreviewFetcher() {
        MediaPreview preview;
        for (const auto &url : {"https://127.0.0.1/private.png", "file:///etc/passwd", "https://example.com/image.png", "https://raw.githubusercontent.com/tcballard/OmaStore/main/media/../secret.png"}) {
            preview.load({{"url", url}, {"sha256", QString(64, 'a')}, {"kind", "screenshot"}});
            QCOMPARE(preview.status(), QString("external"));
            QVERIFY(preview.source().isEmpty());
        }
    }
};
QTEST_GUILESS_MAIN(NativeContracts)
#include "native_contracts.moc"
