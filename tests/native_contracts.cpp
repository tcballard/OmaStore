#include "MediaPreview.h"
#include "Preparation.h"
#include <QBuffer>
#include <QCryptographicHash>
#include <QImage>
#include <QTemporaryDir>
#include <QTest>
#include <QFile>

class NativeContracts : public QObject {
    Q_OBJECT
private slots:
    void worksheetRecoversAndRejectsConcurrentOverwrites() {
        QTemporaryDir dir;
        Preparation first(dir.path()), second(dir.path());
        first.setField("name", "First editor"); QVERIFY(first.save());
        second.setField("name", "Second editor"); QVERIFY(!second.save());
        Preparation recovered(dir.path()); QCOMPARE(recovered.fields().value("name").toString(), QString("First editor"));
        QVERIFY(second.reload()); second.setField("name", "Second editor"); QVERIFY(second.save());
        first.setField("name", "Stale edit"); QVERIFY(!first.save());
        QFile file(dir.filePath("preparation-v1.json")); QVERIFY(file.open(QIODevice::WriteOnly)); file.write("broken file"); file.close();
        QVERIFY(!first.reload()); QVERIFY(!first.save());
        QVERIFY(file.open(QIODevice::ReadOnly)); QCOMPARE(file.readAll(), QByteArray("broken file"));
    }
    void exportRequiresCheckedCurrentFieldsAndNeverUploads() {
        QTemporaryDir dir; Preparation worksheet(dir.path());
        const auto target = QUrl::fromLocalFile(dir.filePath("candidate.json"));
        QVERIFY(!worksheet.exportCandidate(target));
        worksheet.check();
        worksheet.acceptResult({{"valid", true}, {"candidate", QVariantMap{{"channel", "development"}}}});
        QVERIFY(!worksheet.exportCandidate(QUrl("https://example.com/upload")));
        QVERIFY(worksheet.exportCandidate(target));
        worksheet.setField("name", "Changed after check"); QVERIFY(!worksheet.exportCandidate(target));
    }
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
        for (const auto &url : {"https://127.0.0.1/private.png", "file:///etc/passwd", "https://example.com/image.png", "https://raw.githubusercontent.com/tcballard/OmaStore/catalogue-live/media/../secret.png"}) {
            preview.load({{"url", url}, {"sha256", QString(64, 'a')}, {"kind", "screenshot"}});
            QCOMPARE(preview.status(), QString("external"));
            QVERIFY(preview.source().isEmpty());
        }
    }
};
QTEST_GUILESS_MAIN(NativeContracts)
#include "native_contracts.moc"
