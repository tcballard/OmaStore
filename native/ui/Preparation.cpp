#include "Preparation.h"
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLockFile>
#include <QSaveFile>

namespace {
constexpr qint64 Limit = 128 * 1024;
QByteArray encode(const QVariantMap &map) { return QJsonDocument(QJsonObject::fromVariantMap(map)).toJson(QJsonDocument::Compact); }
}
Preparation::Preparation(const QString &directory, QObject *parent) : QObject(parent), m_directory(directory) {
    m_fields = {{"category", "productivity"}, {"appType", "desktop"}, {"maturity", "development"},
        {"licence", "unknown"}, {"identity", "source_commit"}, {"architecture", "x86_64"},
        {"offline", "unknown"}, {"account", "unknown"}, {"activation", "unknown"}, {"model", "free"}, {"currency", "USD"}};
    reload();
}
QString Preparation::candidateJson() const { return QJsonDocument(QJsonObject::fromVariantMap(m_result.value("candidate").toMap())).toJson(QJsonDocument::Indented); }
void Preparation::setField(const QString &key, const QString &value) {
    if (key.size() > 64 || value.size() > 12000 || m_fields.value(key).toString() == value) return;
    auto proposed = m_fields; proposed.insert(key, value);
    if (encode(proposed).size() > 96 * 1024) { m_status = "This worksheet is too large. Shorten its descriptions before saving."; emit changed(); return; }
    m_fields = proposed; m_dirty = true; m_result.clear(); m_checkedFields.clear(); m_status = "Unsaved changes on this device."; emit changed();
}
bool Preparation::reload() {
    QFile file(QDir(m_directory).filePath("preparation-v1.json"));
    if (!file.exists()) { m_status = "Local worksheet. Nothing has been submitted."; emit changed(); return true; }
    if (!file.open(QIODevice::ReadOnly) || file.size() > Limit) { m_blocked = true; m_status = "Could not read the saved worksheet. The existing file has been preserved."; emit changed(); return false; }
    const auto bytes = file.read(Limit + 1); QJsonParseError parse;
    const auto document = QJsonDocument::fromJson(bytes, &parse); const auto obj = document.object();
    const auto fields = obj.value("fields").toObject().toVariantMap();
    bool valid = parse.error == QJsonParseError::NoError && obj.value("version").toInt() == 1 && obj.value("revision").toInt() > 0 && obj.value("revision").toInt() < 1000000000 && obj.value("fields").isObject();
    for (const auto &field : fields) valid = valid && field.metaType().id() == QMetaType::QString && field.toString().size() <= 12000;
    if (!valid) { m_blocked = true; m_status = "The saved worksheet is invalid or from an unsupported version. It has not been overwritten."; emit changed(); return false; }
    m_fields = fields; m_revision = obj.value("revision").toInt(); m_base = bytes; m_dirty = false; m_blocked = false; m_result.clear(); m_checkedFields.clear();
    m_status = "Loaded the worksheet saved on this device."; emit changed(); return true;
}
bool Preparation::save() {
    if (m_blocked || !QDir().mkpath(m_directory)) { m_status = "Cannot save here. The existing worksheet has been preserved."; emit changed(); return false; }
    QLockFile lock(QDir(m_directory).filePath("preparation.lock"));
    if (!lock.tryLock(0)) { m_status = "Another window is saving this worksheet. Try again."; emit changed(); return false; }
    QFile current(QDir(m_directory).filePath("preparation-v1.json")); QByteArray existing;
    if (current.exists()) {
        if (!current.open(QIODevice::ReadOnly) || current.size() > Limit) { m_status = "Cannot read the existing worksheet. It has been preserved."; emit changed(); return false; }
        existing = current.read(Limit + 1);
    }
    if (existing != m_base) { m_status = "This worksheet changed in another window. Export your checked candidate, or reload the saved version before editing again."; emit changed(); return false; }
    const QByteArray bytes = QJsonDocument(QJsonObject{{"version", 1}, {"revision", m_revision + 1}, {"fields", QJsonObject::fromVariantMap(m_fields)}}).toJson(QJsonDocument::Compact);
    QSaveFile file(current.fileName());
    if (!file.open(QIODevice::WriteOnly)) { m_status = "Could not save the worksheet."; emit changed(); return false; }
    if (!file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner) || file.write(bytes) != bytes.size() || !file.commit()) { m_status = "Save failed. Your changes are still open in this window."; emit changed(); return false; }
    m_base = bytes; ++m_revision; m_dirty = false; m_status = "Saved on this device. Nothing has been submitted."; emit changed(); return true;
}
void Preparation::check() { m_checkedFields = encode(m_fields); m_result.clear(); m_status = "Checking the listing fields…"; emit changed(); emit checkRequested(m_fields); }
void Preparation::acceptResult(const QVariantMap &result) {
    if (m_checkedFields != encode(m_fields)) return;
    m_result = result;
    m_status = result.value("valid").toBool() ? "Fields are structurally valid. Review the candidate before exporting." : "Some fields need attention. Your worksheet has been kept.";
    emit changed();
}
bool Preparation::exportCandidate(const QUrl &destination) {
    if (!destination.isLocalFile() || !m_result.value("valid").toBool() || m_checkedFields != encode(m_fields)) return false;
    const auto path = destination.toLocalFile();
    const QFileInfo requested(path), owned(QDir(m_directory).filePath("preparation-v1.json"));
    if (requested.absoluteFilePath() == owned.absoluteFilePath() || (owned.exists() && !requested.canonicalFilePath().isEmpty() && requested.canonicalFilePath() == owned.canonicalFilePath())) { m_status = "Choose a different file for the exported candidate."; emit changed(); return false; }
    QSaveFile file(path); const auto bytes = candidateJson().toUtf8();
    const bool ok = file.open(QIODevice::WriteOnly) && file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner) && file.write(bytes) == bytes.size() && file.commit();
    m_status = ok ? "Candidate exported. No submission or publication has occurred." : "Could not export the candidate. Your worksheet is unchanged."; emit changed(); return ok;
}
