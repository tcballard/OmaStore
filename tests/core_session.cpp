#include "CoreBridge.h"
#include <QFile>
#include <QTemporaryDir>
#include <QTest>

class CoreSession : public QObject {
    Q_OBJECT
private slots:
    void refreshPublishesWholeSnapshotsAndSurvivesProcessLoss() {
        QTemporaryDir dir;
        const auto path=dir.filePath("state");
        qputenv("OMASTORE_BRIDGE_TEST_STATE", path.toUtf8());
        auto mode=[&](const QByteArray &value) { QFile f(path); QVERIFY(f.open(QIODevice::WriteOnly)); f.write(value); f.close(); };
        mode("initial");
        CoreBridge core(true); core.start();
        QTRY_COMPARE_WITH_TIMEOUT(core.appStates().size(), 2, 5000);
        QCOMPARE(core.appStates().value("first").toMap().value("state").toString(), QString("not_installed"));
        mode("installed"); core.refreshDevice();
        QTest::qWait(150);
        QVERIFY(!core.loading()); // background probes do not disable browsing controls
        QCOMPARE(core.appStates().value("first").toMap().value("state").toString(), QString("not_installed"));
        QTRY_COMPARE_WITH_TIMEOUT(core.appStates().value("second").toMap().value("state").toString(), QString("installed"), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(core.activity().size(), 1, 5000);
        QCOMPARE(core.activity().first().toMap().value("state").toString(), QString("failed"));
        QCOMPARE(core.actionForApp("first").value("kind").toString(), QString("review_operation"));
        QTRY_COMPARE_WITH_TIMEOUT(core.actionForApp("second").value("kind").toString(), QString("open"), 5000);
        mode("recovered"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.activity().size(), 2, 5000);
        QTRY_COMPARE_WITH_TIMEOUT(core.actionForApp("first").value("kind").toString(), QString("open"), 5000);
        mode("updates"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.actionForApp("second").value("secondaryKind").toString(), QString("system_update"), 5000);
        QCOMPARE(core.actionForApp("second").value("kind").toString(), QString("open"));
        mode("no_launcher"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.actionForApp("second").value("kind").toString(), QString("no_launcher"), 5000);
        QVERIFY(!core.actionForApp("second").value("enabled").toBool());
        mode("installed"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.actionForApp("second").value("kind").toString(), QString("open"), 5000);
        mode("changed"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.device().value("observationState").toString(), QString("stale"), 5000);
        QCOMPARE(core.appStates().size(), 2);
        QCOMPARE(core.device().value("installedCount").toInt(), 2);
        QCOMPARE(core.device().value("observedAt").toInt(), 123);
        QVERIFY(core.appStates().value("second").toMap().value("stale").toBool());
        QCOMPARE(core.actionForApp("second").value("kind").toString(), QString("refresh"));
        // A stale button requests observation; it cannot launch or install.
        core.activateAppAction("second");
        QVERIFY(core.ready());
        mode("installed"); core.refreshDevice();
        QTRY_COMPARE_WITH_TIMEOUT(core.appStates().size(), 2, 5000);
        mode("crash"); core.refreshDevice();
        QTRY_VERIFY_WITH_TIMEOUT(!core.ready(), 5000);
        QCOMPARE(core.appStates().size(), 2);
        QCOMPARE(core.actionForApp("second").value("kind").toString(), QString("reconnect"));
        QVERIFY(!core.activityCurrent());
        QCOMPARE(core.activity().size(), 1); // recorded recovery remains visible, marked unavailable
        mode("installed"); core.start();
        QTRY_VERIFY_WITH_TIMEOUT(core.ready() && core.device().value("observationState").toString()=="available" && core.appStates().size()==2 && core.activityCurrent(), 5000);
        QFile requests(path+".requests"); QVERIFY(requests.open(QIODevice::ReadOnly));
        const auto sent=requests.readAll();
        QVERIFY(!sent.contains("operations.confirm"));
        QVERIFY(!sent.contains("operations.replan"));
        QVERIFY(!sent.contains("system.handoff"));
    }
};
QTEST_MAIN(CoreSession)
#include "core_session.moc"
