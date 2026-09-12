#include <QDateTime>
#include "CoreBridge.h"
#include <QGuiApplication>
#include <QClipboard>
#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSettings>
#include <QUrl>

namespace { constexpr qsizetype MaxLineBytes = 256 * 1024; }
CoreBridge::CoreBridge(bool demo, QObject *parent) : QObject(parent), m_demo(demo) {
    m_clock.start();
    const auto actions = [this] { ++m_actionRevision; emit actionsChanged(); };
    connect(this, &CoreBridge::deviceChanged, this, actions);
    connect(this, &CoreBridge::activityChanged, this, actions);
    connect(this, &CoreBridge::stateChanged, this, actions);
    m_deviceTimer.setInterval(1000);
    connect(&m_deviceTimer, &QTimer::timeout, this, &CoreBridge::pollDevice);
    m_deviceTimer.start();
    connect(qGuiApp, &QGuiApplication::applicationStateChanged, this, [this](Qt::ApplicationState state) {
        if (state == Qt::ApplicationActive) refreshDevice();
    });
    m_repositorySync.setInterval(60 * 1000);
    connect(&m_repositorySync, &QTimer::timeout, this, [this] { if (m_clock.elapsed() >= m_nextRepositorySync) refresh(); });
    const auto arguments = QCoreApplication::arguments();
    const bool qa = arguments.contains("--smoke-test") || arguments.contains("--ui-test") || arguments.contains("--storefront-test") || arguments.contains("--screenshot");
    if (!demo && !qa) {
        m_repositorySync.start();
        QTimer::singleShot(2000, this, &CoreBridge::refresh);
    }
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
            if (m_clock.elapsed() - pending.since > (pending.method == "workspace.media.upload" || pending.method.startsWith("remixes.") || pending.method.startsWith("settings.") || pending.method.startsWith("system.") || pending.method.startsWith("library.") || pending.method.startsWith("operations.") ? 55000 : 35000)) { fail("The local service stopped responding. Reconnect to try again."); break; }
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
        // Package mutations belong to independent journalled workers, never this pipe child.
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
bool CoreBridge::request(const QString &method, const QVariantMap &params, const QString &scope) {
    if (m_process.state() != QProcess::Running || m_pending.size() >= 16) return false;
    const QString id = QString::number(++m_sequence);
    if (scope.isEmpty() && (method.startsWith("remixes.") || method.startsWith("settings.") || method.startsWith("makers.") || method.startsWith("editorial.") || method.startsWith("setups.") || method=="apps.pick" || method.startsWith("system.") || method.startsWith("library.") || method.startsWith("operations.") || method=="handoff.open")) m_communityRequests[method]=id;
    if (method == "candidate.prepare") m_candidateRequest = id;
    if (method == "apps.get") m_detailRequestId=id;
    m_pending.insert(id, {method, m_clock.elapsed(), m_generation, params.contains("cursor"), scope});
    const QJsonObject envelope{{"protocol_version", 1}, {"id", id}, {"method", method}, {"params", QJsonObject::fromVariantMap(params)}};
    m_process.write(QJsonDocument(envelope).toJson(QJsonDocument::Compact) + '\n'); emit stateChanged(); return true;
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
    if (!pending.scope.isEmpty()) {
        if (reply.value("ok").toBool() && !reply.value("result").isObject()) { fail("The local service sent an invalid result."); return; }
        acceptDevice(pending.scope, reply.value("result").toObject().toVariantMap(),
                     reply.value("ok").toBool() ? QString{} : reply.value("error").toObject().value("code").toString("invalid_result"));
        emit stateChanged(); return;
    }
    if(pending.method=="apps.get" && id!=m_detailRequestId){emit stateChanged();return;}
    if (!reply.value("ok").toBool()) {
        const auto code = reply.value("error").toObject().value("code").toString();
        if(pending.method=="apps.get"){m_detail.clear();emit detailChanged();}
        if (pending.method.startsWith("workspace.")) {
            m_workspaceReply = {{"action",pending.method.mid(10)},{"error",code}};
            if (pending.method == "workspace.auth.poll") m_workspace.insert("signingIn",false);
            emit workspaceChanged(); emit stateChanged(); return;
        }
        if (pending.method == "candidate.prepare" && id == m_candidateRequest) {
            emit candidatePrepared({{"valid", false}, {"errors", QVariantList{QVariantMap{{"path", "fields"}, {"code", "invalid_or_unsupported_fields"}}}}});
            emit stateChanged(); return;
        }
        if(m_communityRequests.value(pending.method)==id) {
            m_community.insert(pending.method,QVariantMap{{"error",code}});emit communityChanged();
            m_error=code=="snapshot_changed"?"The catalogue changed. Reload the selection before continuing.":"This selection is unavailable or no longer matches the catalogue.";
            if(pending.method.startsWith("remixes.") || pending.method.startsWith("settings.") || pending.method.startsWith("operations.") || pending.method.startsWith("library.")) m_error="Local operation unavailable: "+QString(code).replace('_',' ')+".";
            emit stateChanged();return;
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
        if (pending.method == "workspace.status.get") {
            const auto items=m_workspaceReply.value("items").toList();
            for(const auto &value:items) {
                const auto item=value.toMap();
                if(item.value("appId").toString()==m_detailRequested) {
                    m_distribution=item;
                    const auto now=QDateTime::currentSecsSinceEpoch();
                    const auto generated=m_workspaceReply.value("generatedAt").toLongLong();
                    m_distributionUntil=m_workspaceReply.value("validUntil").toLongLong();
                    if(generated>now+30 || generated<now-300 || m_distributionUntil>generated+300 || m_workspaceReply.value("catalogueSnapshot").toString()!=m_catalogue.value("snapshot").toString()) m_distributionUntil=0;
                    m_distributionExpiry.setSingleShot(true);
                    m_distributionExpiry.disconnect(this);
                    connect(&m_distributionExpiry,&QTimer::timeout,this,[this](){emit distributionChanged();});
                    m_distributionExpiry.start(static_cast<int>(qMax<qint64>(1,m_distributionUntil-now)*1000));
                    emit distributionChanged();
                }
            }
        }
        if (pending.method == "workspace.auth.start") {
            const QUrl url(m_workspaceReply.value("authorizationUrl").toString(),QUrl::StrictMode);
            if (url.scheme()=="https" && url.host()=="github.com" && url.path()=="/login/oauth/authorize" && url.userInfo().isEmpty()) {
                if (!QDesktopServices::openUrl(url)) m_workspaceReply.insert("error","browser_unavailable");
            }
        }
        if (pending.method == "workspace.media.preview" && m_workspaceReply.value("contentType").toString().startsWith("video/")) {
            const QUrl url(m_workspaceReply.value("url").toString(),QUrl::StrictMode);
            const QFileInfo file(url.toLocalFile());
            const QDir owned(QDir(QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation)).filePath(m_demo ? "omastore-sample" : "omastore"));
            if (!url.isLocalFile() || file.absolutePath()!=owned.absolutePath() || !file.fileName().startsWith("review-media-") || !QDesktopServices::openUrl(url)) m_workspaceReply.insert("error","media_player_unavailable");
        }
        emit workspaceChanged();
    } else if (m_communityRequests.value(pending.method)==id) {
        m_community.insert(pending.method=="setups.import"?"setups.select":pending.method=="library.refresh"?"library.list":pending.method,result);
        if(pending.method=="operations.confirm" || pending.method=="operations.cancel" || pending.method=="operations.reconcile") m_community.insert("operations.status",result);
        if(pending.method=="operations.replan") m_community.insert("system.plan",result);
        if(pending.method=="operations.get") m_community.insert("system.plan",result.value("plan").toMap());
        emit communityChanged();
        if(pending.method=="operations.confirm" || pending.method=="operations.cancel" || pending.method=="operations.reconcile" || pending.method=="library.refresh" || pending.method=="system.handoff") refreshDevice();
        if(pending.method=="handoff.open") {
            if(result.value("kind").toString()=="app")showApp(result.value("id").toString());
            else communityAction("setups.select",{{"id",result.value("id")},{"revision",result.value("revision")}});
            emit handoffReady(result);
        }
    } else if (pending.method == "candidate.prepare" && id == m_candidateRequest) {
        emit candidatePrepared(result);
    } else if (pending.method == "core.info") {
        if (result.value("service").toString() != "omastore-core" || result.value("version").toString().isEmpty()) { fail("Unsupported local service."); return; }
        m_ready = true; m_version = result.value("version").toString(); request("catalogue.info");
        if(!m_pendingHandoff.isEmpty()) { const auto uri=m_pendingHandoff;m_pendingHandoff.clear();communityAction("handoff.open",{{"uri",uri}}); }
    } else if (pending.method == "catalogue.info" || pending.method == "catalogue.refresh") {
        if (m_catalogue.value("snapshot") != result.value("snapshot")) invalidateDevice("Catalogue changed. Rechecking device status…");
        m_catalogue = result; refreshDevice(); search(); communityAction("makers.list"); communityAction("editorial.list"); communityAction("setups.list"); emit dataChanged();
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
    m_inventoryRunning = false; m_deviceRequested = false;
    invalidateDevice("Device status unavailable. Showing the last observation; reconnect to check again.");
    m_activityCurrent = false; emit deviceChanged(); emit activityChanged();
    m_timeout.stop(); m_ready = false; m_error = message; m_output.clear(); m_pending.clear();
    if (m_process.state() != QProcess::NotRunning) m_process.terminate();
    emit stateChanged();
}
void CoreBridge::refresh() { if (m_ready && !loading()) { m_error.clear(); m_nextRepositorySync = m_clock.elapsed() + 60 * 60 * 1000; request("catalogue.refresh"); } }
void CoreBridge::workspaceAction(const QString &action,const QVariantMap &params) {
    static const QStringList actions{"state","auth.start","auth.poll","auth.logout","auth.sandbox","claims.start","claims.verify","claims.revoke","command","drafts.get","drafts.cache","drafts.new","drafts.remix","drafts.sample","drafts.preview","revisions.get","media.upload","media.preview","checks.run_sample","evidence.import","review.queue","review.get","review.sample_evidence","publication.sample","publication.export","status.get","monitor.queue","monitor.sample","feed.export","feed.info","operations.dashboard","commerce.status","commerce.author","commerce.prices","commerce.prepare","commerce.purchase","commerce.orders","commerce.order","commerce.reconcile","commerce.retry","commerce.recover","commerce.support","commerce.lifecycle","commerce.packet.export","commerce.sample.capture","commerce.pending","commerce.resume","commerce.licence.export","commerce.licence.verify"};
    if (!m_ready || !actions.contains(action)) return;
    request("workspace."+action,params);
}
void CoreBridge::persistQuery() {
    QSettings settings; settings.setValue("browse/query", QJsonDocument(QJsonObject::fromVariantMap(m_query)).toJson(QJsonDocument::Compact)); settings.sync();
    if (settings.status() != QSettings::NoError) { m_error = "Could not save your browsing preferences."; emit stateChanged(); }
}
void CoreBridge::setFilter(const QString &key, const QString &value) {
    static const QStringList keys{"q", "makerId", "category", "appType", "licence", "price", "architecture", "offline", "evidence"};
    if (!keys.contains(key) || value.size() > 200) return;
    if (value.isEmpty()) m_query.remove(key); else m_query.insert(key, value);
    ++m_generation; persistQuery(); emit queryChanged(); m_searchDebounce.start();
}
void CoreBridge::clearFilters() { m_query.clear(); ++m_generation; persistQuery(); emit queryChanged(); search(); }
void CoreBridge::search() { if (!m_ready) return; m_error.clear(); m_cursor.clear(); ++m_generation; request("apps.list", m_query); }
void CoreBridge::nextPage() { if (!m_ready || loading() || m_cursor.isEmpty()) return; auto params = m_query; params.insert("cursor", m_cursor); request("apps.list", params); }
void CoreBridge::showApp(const QString &id) { if (!m_ready) return; m_error.clear(); m_detailRequested = id; m_distribution.clear();m_distributionUntil=0;emit distributionChanged();request("apps.get", {{"id", id}});workspaceAction("status.get",{{"ids",QVariantList{id}}}); }
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
    else if (kind == "acquisition") {if(!distributionCurrent() || m_distribution.value("distribution").toString()=="suspended") return false;destination = m_detail.value("acquisition").toMap().value("url").toString();}
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

bool CoreBridge::distributionCurrent() const {return !m_distribution.isEmpty() && m_distributionUntil>QDateTime::currentSecsSinceEpoch();}

void CoreBridge::communityAction(const QString &method,const QVariantMap &params) {
    static const QStringList methods{"remixes.create","remixes.list","remixes.get","remixes.rename","remixes.export","remixes.import","settings.apply","settings.history","settings.restore_preview","settings.restore","settings.reconcile","settings.adapters","settings.preview","settings.get","makers.list","makers.get","editorial.list","editorial.get","setups.list","setups.select","setups.export","setups.import","apps.pick","system.probe","system.plan","library.app_status","library.inventory","library.list","library.refresh","library.launchers","library.launch","operations.get","operations.events","operations.status","operations.confirm","operations.cancel","operations.reconcile","operations.replan","operations.diagnostics","operations.diagnostics.export","library.setups","library.detach_setup","library.remember_setup","system.handoff","handoff.open"};
    if(m_ready && methods.contains(method)) { if(method=="library.app_status" || method=="library.inventory" || method=="library.launchers" || method.startsWith("remixes.") || method.startsWith("settings.") || method=="library.detach_setup" || method=="operations.diagnostics") m_community.remove(method); if(method=="system.plan" || method=="operations.get" || method=="operations.replan") { m_community.remove("system.plan"); emit communityChanged(); } request(method,params); }
}
bool CoreBridge::openMakerLink(const QString &kind) {
    if(kind!="homepage" && kind!="support") return false;
    const QUrl url(m_community.value("makers.get").toMap().value(kind).toString(),QUrl::StrictMode);
    return url.isValid() && url.scheme()=="https" && !url.host().isEmpty() && url.userInfo().isEmpty() && QDesktopServices::openUrl(url);
}

bool CoreBridge::openSetupLink(const QString &kind,int index) {
    const auto setup=m_community.value("setups.select").toMap();QString raw;
    if(kind=="support" || kind=="homepage") raw=setup.value("maker").toMap().value(kind).toString();
    else if(kind=="offer") {const auto rows=setup.value("components").toList();if(index<0 || index>=rows.size())return false;raw=rows[index].toMap().value("offer").toMap().value("url").toString();}
    else if(kind=="media") {const auto rows=setup.value("recipe").toMap().value("media").toList();if(index<0 || index>=rows.size())return false;raw=rows[index].toMap().value("url").toString();}
    else return false;
    const QUrl url(raw,QUrl::StrictMode);return url.isValid() && url.scheme()=="https" && !url.host().isEmpty() && url.userInfo().isEmpty() && QDesktopServices::openUrl(url);
}
void CoreBridge::copySetupLink() {
    const auto uri=m_community.value("setups.select").toMap().value("shareUri").toString();
    if(uri.startsWith("omastore://setup/") && uri.size()<300) QGuiApplication::clipboard()->setText(uri);
}

void CoreBridge::openHandoff(const QString &uri) { if(uri.isEmpty() || uri.size()>400)return;if(m_ready)communityAction("handoff.open",{{"uri",uri}});else m_pendingHandoff=uri; }

void CoreBridge::copyFeedLink(){const QUrl url(m_workspaceReply.value("feedUrl").toString(),QUrl::StrictMode);if(url.isValid()&&url.scheme()=="https"&&!url.host().isEmpty()&&url.userInfo().isEmpty())QGuiApplication::clipboard()->setText(url.toString());}


bool CoreBridge::loading() const {
    for (const auto &pending : m_pending) if (pending.scope.isEmpty()) return true;
    return false;
}
void CoreBridge::refreshDevice() {
    m_deviceRequested = true; m_nextActivity = 0;
    emit deviceChanged();
    QTimer::singleShot(0, this, &CoreBridge::pollDevice);
}
void CoreBridge::pollDevice() {
    if (!m_ready || m_catalogue.isEmpty() || loading()) return;
    for (const auto &pending : m_pending) if (!pending.scope.isEmpty()) return;
    const auto now = m_clock.elapsed();
    if (m_inventoryRunning) {
        request("library.inventory", {{"offset",m_inventoryOffset},{"snapshot",m_inventoryMetadata.value("snapshot")}}, "inventory");
    } else if (m_deviceRequested || now >= m_nextDevice) {
        m_inventoryStage.clear(); m_inventoryItems.clear(); m_inventoryMetadata.clear(); m_inventoryOffset = 0;
        m_inventoryRunning = true; m_deviceRequested = false;
        request("library.inventory", {}, "inventory"); emit deviceChanged();
    } else if (now >= m_nextActivity) {
        request("library.activity", {}, "activity");
    } else if (m_device.value("observationState").toString() == "available") {
        // Resolve launchability once per device snapshot, without per-card probes.
        for (auto it=m_appStates.cbegin(); it!=m_appStates.cend(); ++it) {
            const auto state=it.value().toMap().value("state").toString();
            if ((state=="installed" || state=="update_available") && !m_launchability.contains(it.key())) {
                request("library.launchers", {{"id",it.key()}}, "launcher:"+it.key()); break;
            }
        }
    }
}
void CoreBridge::acceptDevice(const QString &scope, const QVariantMap &result, const QString &error) {
    if (scope == "inventory") {
        if (!error.isEmpty()) {
            m_inventoryRunning = false; m_inventoryStage.clear(); m_inventoryItems.clear();
            invalidateDevice("Could not check this device. Showing the last observation; actions need a fresh check.");
            m_nextDevice = m_clock.elapsed() + 3000;
        } else {
            if (m_inventoryOffset == 0) m_inventoryMetadata = result;
            for (const auto &entry : result.value("items").toList()) {
                const auto item = entry.toMap(); m_inventoryStage.insert(item.value("id").toString(), item);
                m_inventoryItems.append(item);
            }
            const auto next = result.value("nextOffset");
            if (!next.isNull() && next.isValid() && next.toInt() > m_inventoryOffset && next.toInt() <= 20000) {
                m_inventoryOffset = next.toInt();
            } else {
                if (m_inventoryMetadata.value("observationState").toString()=="available") {
                    m_appStates = m_inventoryStage; m_device = m_inventoryMetadata;
                    m_device.insert("items", m_inventoryItems); m_device.insert("nextOffset", QVariant{});
                    m_device.insert("stale",false);
                    if (m_launcherSnapshot!=m_device.value("snapshot").toString()) {
                        m_launchability.clear(); m_launcherSnapshot=m_device.value("snapshot").toString();
                    }
                } else {
                    if (!m_device.value("observedAt").isValid() || m_device.value("observedAt").isNull()) { m_device=m_inventoryMetadata; m_appStates=m_inventoryStage; }
                    invalidateDevice("Device status unavailable. Any retained versions are from the last successful check.");
                    m_device.insert("host",m_inventoryMetadata.value("host"));
                }
                m_inventoryRunning = false; m_nextDevice = m_clock.elapsed() + 15000;
            }
        }
        emit deviceChanged();
    } else if (scope == "activity") {
        m_activityCurrent = error.isEmpty();
        if (m_activityCurrent) {
            const auto items = result.value("items").toList();
            if (items != m_activity) { m_activity = items; m_deviceRequested = true; }
            // Fetch complete results for an open review; summaries cannot replace
            // per-component outcomes or execution eligibility.
            const auto id = m_community.value("system.plan").toMap().value("digest").toString();
            for (const auto &entry : m_activity) {
                const auto item = entry.toMap();
                const auto status = m_community.value("operations.status").toMap();
                if (item.value("id").toString() == id &&
                    (status.value("id").toString()!=id || status.value("version")!=item.value("version") || status.value("cancelRequested")!=item.value("cancelRequested")))
                    request("operations.status", {{"id",id}}, "review");
            }
        }
        bool active = false;
        for (const auto &entry : m_activity) {
            const auto state = entry.toMap().value("state").toString();
            active |= state == "running" || state == "awaiting_user";
        }
        m_nextActivity = m_clock.elapsed() + (active ? 1000 : 15000);
        emit activityChanged();
    } else if (scope.startsWith("launcher:")) {
        // A catalogue/device refresh may have invalidated this lookup in flight.
        if (m_device.value("observationState").toString()=="available") {
            m_launchability.insert(scope.mid(9), error.isEmpty() ? result : QVariantMap{{"error",error}});
            ++m_actionRevision; emit actionsChanged();
        }
    } else if (scope == "review" && error.isEmpty() && result.value("id") == m_community.value("system.plan").toMap().value("digest")) {
        m_community.insert("operations.status", result); emit communityChanged();
    }
    QTimer::singleShot(0, this, &CoreBridge::pollDevice);
}


void CoreBridge::invalidateDevice(const QString &notice) {
    bool observed=m_device.value("observedAt").isValid() && !m_device.value("observedAt").isNull();
    QVariantList items;
    for (auto it=m_appStates.begin(); it!=m_appStates.end(); ++it) {
        auto item=it.value().toMap();
        const bool wasObserved=item.value("observedAt").isValid() && !item.value("observedAt").isNull();
        observed |= wasObserved;
        item.insert("stale",wasObserved); item.insert("primaryAction","details");
        it.value()=item;
    }
    // Preserve the original stable ordering, versions, counts and timestamp.
    for (const auto &entry:m_device.value("items").toList())
        items.append(m_appStates.value(entry.toMap().value("id").toString()));
    m_device.insert("items",items); m_device.insert("stale",observed);
    m_device.insert("observationState",observed ? "stale" : "unavailable");
    m_device.insert("notice",notice); m_launchability.clear();
    emit deviceChanged();
}
QVariantMap CoreBridge::actionForApp(const QString &id) const {
    const auto make=[this](const QString &kind,const QString &label,const QString &operation=QString{}) {
        return QVariantMap{{"kind",kind},{"label",label},{"enabled",!loading()},{"operationId",operation}};
    };
    if (!m_ready) return make("reconnect","Reconnect");
    for (const auto &entry:m_activity) {
        const auto op=entry.toMap(); const auto phase=op.value("state").toString();
        if (phase!="running" && phase!="awaiting_user" && phase!="unknown" && phase!="failed") continue;
        for (const auto &app:op.value("apps").toList()) {
            const auto a=app.toMap();
            if (a.value("appId").toString()==id && (a.value("action")=="install" || a.value("action")=="remove"))
                return make("review_operation", "Review operation", op.value("id").toString());
        }
    }
    const auto device=m_appStates.value(id).toMap();
    if (device.value("stale").toBool() || m_device.value("stale").toBool()) return make("refresh","Recheck status");
    const auto state=device.value("state").toString();
    if (state=="external") return make("details","View options");
    if (m_device.value("observationState").toString()!="available" || state=="unknown" || state.isEmpty())
        return make("inspect_availability","Check availability…");
    if (state=="not_installed") return make("review_install","Review installation");
    const auto launch=m_launchability.value(id).toMap();
    const auto choices=launch.value("items").toList();
    if (!choices.isEmpty()) {
        auto action=make("open",choices.size()>1?"Open…":"Open");
        if (state=="update_available") { action.insert("secondaryKind","system_update"); action.insert("secondaryLabel","Update with Omarchy…"); }
        return action;
    }
    if (state=="update_available") return make("system_update","Update with Omarchy…");
    if (launch.contains("items")) { auto action=make("no_launcher","No desktop launcher"); action.insert("enabled",false); return action; }
    for (const auto &pending:m_pending) if (pending.scope=="launcher:"+id) {
        auto action=make("check_launch","Checking launcher…"); action.insert("enabled",false); return action;
    }
    return make("check_launch",launch.contains("error")?"Retry launcher check":"Check launch options…");
}
void CoreBridge::activateAppAction(const QString &id, bool secondary) {
    if (id.isEmpty()) return;
    const auto action=actionForApp(id);
    if (!action.value("enabled").toBool()) return;
    const auto kind=action.value(secondary?"secondaryKind":"kind").toString();
    if (kind=="reconnect") start();
    else if (kind=="refresh") refreshDevice();
    else if (kind=="review_operation") communityAction("operations.get",{{"id",action.value("operationId")}});
    else if (kind=="review_install" || kind=="inspect_availability") communityAction("system.plan",{{"kind","app"},{"id",id}});
    else if (kind=="details") showApp(id);
    else if (kind=="check_launch") {
        m_launchability.remove(id); request("library.launchers",{{"id",id}},"launcher:"+id);
    } else if (kind=="open") {
        const auto choices=m_launchability.value(id).toMap().value("items").toList();
        if (choices.size()==1) communityAction("library.launch",{{"id",id},{"desktop",choices.first()}});
        else emit appActionRequested("choose_launcher",id,choices);
    } else if (kind=="system_update") emit appActionRequested(kind,id,{});
}
