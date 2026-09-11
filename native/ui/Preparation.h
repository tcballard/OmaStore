#pragma once
#include <QObject>
#include <QUrl>
#include <QVariantMap>

class Preparation final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap fields READ fields NOTIFY changed)
    Q_PROPERTY(bool dirty READ dirty NOTIFY changed)
    Q_PROPERTY(QString status READ status NOTIFY changed)
    Q_PROPERTY(QVariantMap result READ result NOTIFY changed)
    Q_PROPERTY(QString candidateJson READ candidateJson NOTIFY changed)
public:
    explicit Preparation(const QString &directory, QObject *parent = nullptr);
    QVariantMap fields() const { return m_fields; }
    bool dirty() const { return m_dirty; }
    QString status() const { return m_status; }
    QVariantMap result() const { return m_result; }
    QString candidateJson() const;
    Q_INVOKABLE void setField(const QString &key, const QString &value);
    Q_INVOKABLE bool save();
    Q_INVOKABLE bool reload();
    Q_INVOKABLE void check();
    Q_INVOKABLE bool exportCandidate(const QUrl &destination);
    void acceptResult(const QVariantMap &result);
signals:
    void changed();
    void checkRequested(const QVariantMap &fields);
private:
    QString m_directory, m_status;
    QByteArray m_base, m_checkedFields;
    QVariantMap m_fields, m_result;
    int m_revision = 0;
    bool m_dirty = false, m_blocked = false;
};
