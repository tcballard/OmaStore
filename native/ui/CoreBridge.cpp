#include "CoreBridge.h"

#include <QCoreApplication>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>

namespace {
constexpr qsizetype MaxLineBytes = 256 * 1024;
}

CoreBridge::CoreBridge(QObject *parent) : QObject(parent) {
    m_timeout.setSingleShot(true);
    m_timeout.setInterval(5000);
    connect(&m_timeout, &QTimer::timeout, this, [this] {
        fail(QStringLiteral("The local core did not answer within five seconds."));
    });
    connect(&m_process, &QProcess::started, this, [this] {
        const QJsonObject request{{"protocol_version", 1}, {"id", "startup"}, {"method", "core.info"}};
        m_process.write(QJsonDocument(request).toJson(QJsonDocument::Compact) + '\n');
    });
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &CoreBridge::readOutput);
    connect(&m_process, &QProcess::readyReadStandardError, this, [this] {
        // The current core has no write operations; consume diagnostics without exposing input.
        while (!m_process.readAllStandardError().isEmpty()) {}
    });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError) {
        fail(QStringLiteral("Could not run the local core: %1").arg(m_process.errorString()));
    });
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
        [this](int, QProcess::ExitStatus) {
            if (m_error.isEmpty()) fail(QStringLiteral("The local core stopped."));
        });
}

CoreBridge::~CoreBridge() {
    m_timeout.stop();
    m_process.disconnect(this);
    m_process.closeWriteChannel();
    if (!m_process.waitForFinished(250)) {
        // B00 core is read-only. Future package operations need B15's durable lifecycle.
        m_process.terminate();
        if (!m_process.waitForFinished(250)) {
            m_process.kill();
            m_process.waitForFinished(250);
        }
    }
}

void CoreBridge::start() {
    if (m_process.state() != QProcess::NotRunning) return;
    m_output.clear();
    m_ready = false;
    m_error.clear();
    m_version.clear();
    emit stateChanged();
    // Resolve a packaged sibling, never a command or executable supplied by catalogue data.
    m_process.setProgram(QDir(QCoreApplication::applicationDirPath()).filePath("omastore-core"));
    m_process.setArguments({"--stdio"});
    m_process.setProcessChannelMode(QProcess::SeparateChannels);
    m_timeout.start();
    m_process.start();
}

void CoreBridge::readOutput() {
    while (m_process.bytesAvailable() > 0) {
        m_output += m_process.read(4096);
        qsizetype newline;
        while ((newline = m_output.indexOf('\n')) >= 0) {
            if (newline + 1 > MaxLineBytes) {
                fail(QStringLiteral("The local core sent an oversized reply."));
                return;
            }
            const QByteArray line = m_output.left(newline);
            m_output.remove(0, newline + 1);
            acceptReply(line);
            if (!m_error.isEmpty()) return;
        }
        if (m_output.size() > MaxLineBytes) {
            fail(QStringLiteral("The local core sent an oversized reply."));
            return;
        }
    }
}

void CoreBridge::acceptReply(const QByteArray &line) {
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(line, &parseError);
    const QJsonObject reply = document.object();
    const QJsonObject result = reply.value("result").toObject();
    if (parseError.error != QJsonParseError::NoError || !document.isObject()
        || reply.value("protocol_version").toInt(-1) != 1
        || reply.value("id").toString() != "startup"
        || !reply.value("ok").toBool()
        || result.value("service").toString() != "omastore-core"
        || result.value("version").toString().isEmpty()) {
        fail(QStringLiteral("The local core replied with an unsupported message."));
        return;
    }
    m_timeout.stop();
    m_version = result.value("version").toString();
    m_ready = true;
    emit stateChanged();
}

void CoreBridge::fail(const QString &message) {
    if (!m_error.isEmpty()) return;
    m_timeout.stop();
    m_ready = false;
    m_error = message;
    m_output.clear();
    m_process.closeReadChannel(QProcess::StandardOutput);
    // The scaffold core is read-only; terminate a failed protocol session.
    if (m_process.state() != QProcess::NotRunning) m_process.terminate();
    emit stateChanged();
}
