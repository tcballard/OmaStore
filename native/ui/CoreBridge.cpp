#include "CoreBridge.h"
#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSettings>
#include <QUrl>

namespace { constexpr qsizetype MaxLineBytes = 256 * 1024; }
CoreBridge::CoreBridge(bool demo, QObject *parent) : QObject(parent), m_demo(demo) {
    m_clock.start();
    QSettings settings;
    m_query = QJsonDocument::fromJson(settings.value("browse/query").toByteArray()).object().toVariantMap();
    m_saved = QJsonDocument::fromJson(settings.value("browse/saved").toByteArray()).toVariant().toList();
    if (m_saved.size() > 1000) m_saved.clear();
    m_query.remove("cursor"); m_query.remove("limit");
    m_searchDebounce.setSingleShot(true); m_searchDebounce.setInterval(80);
    connect(&m_searchDebounce, &QTimer::timeout, this, &CoreBridge::search);
    m_timeout.setInterval(1000);
    connect(&m_timeout, &QTimer::timeout, this, [this] {
        for (const auto &pending : m_pending) {
            if (m_clock.elapsed() - pending.since > 15000) { fail("The local service stopped responding. Reconnect to try again."); break; }
        }
    });
    connect(&m_process, &QProcess::started, this, [this] { request("core.info"); });
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &CoreBridge::readOutput);
    connect(&m_process, &QProcess::readyReadStandardError, this, [this] { m_process.readAllStandardError(); });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError) { fail("Could not run the local service. Check that omastore-core is installed alongside OmaStore."); });
    connect(&m_process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this, [this](int, QProcess::ExitStatus) {
        if (m_ready || !m_pending.isEmpty()) fail("The local service stopped. Reconnect to continue browsing.");
    });
}
CoreBridge::~CoreBridge() {
    m_timeout.stop(); m_process.disconnect(this); m_process.closeWriteChannel();
    if (!m_process.waitForFinished(250)) {
        // Only catalogue reads/atomic cache writes exist. Package lifecycles require B15.
        m_process.terminate();
        if (!m_process.waitForFinished(250)) { m_process.kill(); m_process.waitForFinished(250); }
    }
}
void CoreBridge::start() {
    if (m_process.state() != QProcess::NotRunning) return;
    m_output.clear(); m_pending.clear(); m_ready = false; m_error.clear();
    m_process.setProgram(QDir(QCoreApplication::applicationDirPath()).filePath("omastore-core"));
    QStringList args{"--stdio"}; if (m_demo) args << "--demo";
    m_process.setArguments(args); m_process.setProcessChannelMode(QProcess::SeparateChannels);
    m_timeout.start(); m_process.start(); emit stateChanged();
}
void CoreBridge::request(const QString &method, const QVariantMap &params) {
    if (m_process.state() != QProcess::Running || m_pending.size() >= 16) return;
    const QString id = QString::number(++m_sequence);
    if (method == "candidate.prepare") m_candidateRequest = id;
    m_pending.insert(id, {method, m_clock.elapsed(), m_generation, params.contains("cursor")});
    const QJsonObject envelope{{"protocol_version", 1}, {"id", id}, {"method", method}, {"params", QJsonObject::fromVariantMap(params)}};
    m_process.write(QJsonDocument(envelope).toJson(QJsonDocument::Compact) + '\n'); emit stateChanged();
}
void CoreBridge::readOutput() {
    while (m_process.bytesAvailable() > 0) {
        m_output += m_process.read(4096);
        qsizetype newline;
        while ((newline = m_output.indexOf('\n')) >= 0) {
            if (newline + 1 > MaxLineBytes) { fail("The local service sent an oversized reply."); return; }
            const auto line = m_output.left(newline); m_output.remove(0, newline + 1); acceptReply(line);
            if (m_process.state() != QProcess::Running) return;
        }
        if (m_output.size() > MaxLineBytes) { fail("The local service sent an oversized reply."); return; }
    }
}
void CoreBridge::acceptReply(const QByteArray &line) {
    QJsonParseError parse;
    const auto doc = QJsonDocument::fromJson(line, &parse); const auto reply = doc.object();
    const auto id = reply.value("id").toString();
    if (parse.error != QJsonParseError::NoError || !doc.isObject() || reply.value("protocol_version").toInt(-1) != 1
        || !reply.value("ok").isBool() || !m_pending.contains(id)) { fail("The local service sent an unsupported message."); return; }
    const auto pending = m_pending.take(id);
    if (!reply.value("ok").toBool()) {
        const auto code = reply.value("error").toObject().value("code").toString();
        if (pending.method.startsWith("workspace.")) {
            m_workspaceReply = {{"action",pending.method.mid(10)},{"error",code}};
            if (pending.method == "workspace.auth.poll") m_workspace.insert("signingIn",false);
            emit workspaceChanged(); emit stateChanged(); return;
        }
        if (pending.method == "candidate.prepare" && id == m_candidateRequest) {
            emit candidatePrepared({{"valid", false}, {"errors", QVariantList{QVariantMap{{"path", "fields"}, {"code", "invalid_or_unsupported_fields"}}}}});
            emit stateChanged(); return;
        }
        if (code == "snapshot_changed" || code == "cursor_expired") { m_cursor.clear(); search(); }
        else if (code == "not_found") m_error = "This listing is no longer in the current catalogue.";
        else if (code == "invalid_filter" || code == "invalid_cursor") m_error = "These filters are no longer supported. Clear filters to continue.";
        else m_error = "This request could not be completed. Try again.";
        emit stateChanged(); return;
    }
    if (!reply.value("result").isObject()) { fail("The local service sent an invalid result."); return; }
    const auto result = reply.value("result").toObject().toVariantMap();
    if (pending.method.startsWith("workspace.")) {
        m_workspace = result.value("workspace").toMap();
        m_workspaceReply = result.value("value").toMap();
        m_workspaceReply.insert("action",pending.method.mid(10));
        if (pending.method == "workspace.auth.start") {
            const QUrl url(m_workspaceReply.value("authorizationUrl").toString(),QUrl::StrictMode);
            if (url.scheme()=="https" && url.host()=="github.com" && url.path()=="/login/oauth/authorize" && url.userInfo().isEmpty()) {
                if (!QDesktopServices::openUrl(url)) m_workspaceReply.insert("error","browser_unavailable");
            }
        }
        emit workspaceChanged();
    } else if (pending.method == "candidate.prepare" && id == m_candidateRequest) {
        emit candidatePrepared(result);
    } else if (pending.method == "core.info") {
        if (result.value("service").toString() != "omastore-core" || result.value("version").toString().isEmpty()) { fail("Unsupported local service."); return; }
        m_ready = true; m_version = result.value("version").toString(); request("catalogue.info");
    } else if (pending.method == "catalogue.info" || pending.method == "catalogue.refresh") {
        m_catalogue = result; search(); emit dataChanged();
        if (!m_detailRequested.isEmpty()) showApp(m_detailRequested);
    } else if (pending.method == "apps.list" && pending.generation == m_generation) {
        if (pending.append) m_apps.append(result.value("items").toList()); else m_apps = result.value("items").toList();
        m_total = result.value("total").toInt(); m_cursor = result.value("nextCursor").toString(); m_loaded = true; emit dataChanged();
    } else if (pending.method == "apps.get" && result.value("app").toMap().value("id").toString() == m_detailRequested) {
        m_detail = result; emit detailChanged();
    }
    emit stateChanged();
}
void CoreBridge::fail(const QString &message) {
    m_timeout.stop(); m_ready = false; m_error = message; m_output.clear(); m_pending.clear();
    if (m_process.state() != QProcess::NotRunning) m_process.terminate();
    emit stateChanged();
}
void CoreBridge::refresh() { if (m_ready && !loading()) { m_error.clear(); request("catalogue.refresh"); } }
void CoreBridge::workspaceAction(const QString &action,const QVariantMap &params) {
    static const QStringList actions{"state","auth.start","auth.poll","auth.logout","auth.sandbox","claims.start","claims.verify","claims.revoke"};
    if (!m_ready || !actions.contains(action)) return;
    request("workspace."+action,params);
}
void CoreBridge::persistQuery() {
    QSettings settings; settings.setValue("browse/query", QJsonDocument(QJsonObject::fromVariantMap(m_query)).toJson(QJsonDocument::Compact)); settings.sync();
    if (settings.status() != QSettings::NoError) { m_error = "Could not save your browsing preferences."; emit stateChanged(); }
}
void CoreBridge::setFilter(const QString &key, const QString &value) {
    static const QStringList keys{"q", "category", "appType", "licence", "price", "architecture", "offline", "evidence"};
    if (!keys.contains(key) || value.size() > 200) return;
    if (value.isEmpty()) m_query.remove(key); else m_query.insert(key, value);
    ++m_generation; persistQuery(); emit queryChanged(); m_searchDebounce.start();
}
void CoreBridge::clearFilters() { m_query.clear(); ++m_generation; persistQuery(); emit queryChanged(); search(); }
void CoreBridge::search() { if (!m_ready) return; m_error.clear(); m_cursor.clear(); ++m_generation; request("apps.list", m_query); }
void CoreBridge::nextPage() { if (!m_ready || loading() || m_cursor.isEmpty()) return; auto params = m_query; params.insert("cursor", m_cursor); request("apps.list", params); }
void CoreBridge::showApp(const QString &id) { if (!m_ready) return; m_error.clear(); m_detailRequested = id; request("apps.get", {{"id", id}}); }
void CoreBridge::closeDetail() { m_detailRequested.clear(); m_detail.clear(); emit detailChanged(); }
bool CoreBridge::isSaved(const QString &id) const { for (const auto &entry : m_saved) if (entry.toMap().value("id").toString() == id) return true; return false; }
void CoreBridge::removeSaved(const QString &id) {
    for (qsizetype i = m_saved.size() - 1; i >= 0; --i) if (m_saved[i].toMap().value("id").toString() == id) m_saved.removeAt(i);
    QSettings settings; settings.setValue("browse/saved", QJsonDocument::fromVariant(m_saved).toJson(QJsonDocument::Compact)); settings.sync();
    if (settings.status() != QSettings::NoError) { m_error = "Could not update your saved list on this device."; emit stateChanged(); }
    emit savedChanged();
}
void CoreBridge::toggleSaved() {
    const auto summary = m_detail.value("summary").toMap(); const auto id = summary.value("id").toString(); if (id.isEmpty()) return;
    if (isSaved(id)) { for (qsizetype i = m_saved.size() - 1; i >= 0; --i) if (m_saved[i].toMap().value("id").toString() == id) m_saved.removeAt(i); }
    else if (m_saved.size() < 1000) m_saved.append(summary);
    QSettings settings; settings.setValue("browse/saved", QJsonDocument::fromVariant(m_saved).toJson(QJsonDocument::Compact)); settings.sync();
    if (settings.status() != QSettings::NoError) { m_error = "Could not save this item on this device."; emit stateChanged(); }
    emit savedChanged();
}
bool CoreBridge::openLink(const QString &kind, int index) {
    // QML selects a known field; it cannot pass a command, URL or executable.
    const auto app = m_detail.value("app").toMap(); QString destination;
    if (kind == "source" || kind == "support" || kind == "homepage") destination = app.value(kind).toString();
    else if (kind == "acquisition") destination = m_detail.value("acquisition").toMap().value("url").toString();
    else if (kind == "offer" || kind == "refund" || kind == "terms" || kind == "cancellation") {
        const auto offers = app.value("offers").toList(); if (index >= 0 && index < offers.size()) destination = offers[index].toMap().value(kind == "offer" ? "url" : kind).toString();
    } else if (kind == "media") {
        const auto media = app.value("media").toList(); if (index >= 0 && index < media.size()) destination = media[index].toMap().value("url").toString();
    } else if (kind == "evidence") {
        const auto tests = app.value("tests").toList(); if (index >= 0 && index < tests.size()) destination = tests[index].toMap().value("evidence").toString();
    }
    const QUrl url(destination, QUrl::StrictMode);
    if (!url.isValid() || url.scheme() != "https" || url.host().isEmpty() || !url.userInfo().isEmpty()) return false;
    const bool opened = QDesktopServices::openUrl(url);
    if (!opened) { m_error = "The system browser could not open this link."; emit stateChanged(); }
    return opened;
}
