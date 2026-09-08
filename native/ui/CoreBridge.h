#pragma once
#include <QObject>
#include <QProcess>
#include <QTimer>
#include <QElapsedTimer>
#include <QMap>
#include <QVariant>

class CoreBridge final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(bool loading READ loading NOTIFY stateChanged)
    Q_PROPERTY(bool loaded READ loaded NOTIFY dataChanged)
    Q_PROPERTY(QString version READ version NOTIFY stateChanged)
    Q_PROPERTY(QString error READ error NOTIFY stateChanged)
    Q_PROPERTY(QVariantMap catalogue READ catalogue NOTIFY dataChanged)
    Q_PROPERTY(QVariantList apps READ apps NOTIFY dataChanged)
    Q_PROPERTY(QVariantMap detail READ detail NOTIFY detailChanged)
    Q_PROPERTY(QVariantMap query READ query NOTIFY queryChanged)
    Q_PROPERTY(QVariantList saved READ saved NOTIFY savedChanged)
    Q_PROPERTY(int total READ total NOTIFY dataChanged)
    Q_PROPERTY(bool hasMore READ hasMore NOTIFY dataChanged)
public:
    explicit CoreBridge(bool demo = false, QObject *parent = nullptr);
    ~CoreBridge() override;
    Q_INVOKABLE void start();
    Q_INVOKABLE void refresh();
    Q_INVOKABLE void setFilter(const QString &key, const QString &value);
    Q_INVOKABLE void clearFilters();
    Q_INVOKABLE void nextPage();
    Q_INVOKABLE void showApp(const QString &id);
    Q_INVOKABLE void closeDetail();
    Q_INVOKABLE void toggleSaved();
    Q_INVOKABLE bool isSaved(const QString &id) const;
    Q_INVOKABLE bool openLink(const QString &kind, int index = 0);
    bool ready() const { return m_ready; }
    bool loading() const { return !m_pending.isEmpty(); }
    bool loaded() const { return m_loaded; }
    QString version() const { return m_version; }
    QString error() const { return m_error; }
    QVariantMap catalogue() const { return m_catalogue; }
    QVariantList apps() const { return m_apps; }
    QVariantMap detail() const { return m_detail; }
    QVariantMap query() const { return m_query; }
    QVariantList saved() const { return m_saved; }
    int total() const { return m_total; }
    bool hasMore() const { return !m_cursor.isEmpty(); }
signals:
    void stateChanged();
    void dataChanged();
    void detailChanged();
    void queryChanged();
    void savedChanged();
private:
    struct Pending { QString method; qint64 since; int generation; bool append; };
    void request(const QString &method, const QVariantMap &params = {});
    void readOutput();
    void acceptReply(const QByteArray &line);
    void fail(const QString &message);
    void search();
    void persistQuery();
    QProcess m_process;
    QTimer m_timeout, m_searchDebounce;
    QElapsedTimer m_clock;
    QMap<QString, Pending> m_pending;
    QByteArray m_output;
    bool m_ready = false, m_loaded = false, m_demo;
    quint64 m_sequence = 0;
    int m_generation = 0, m_total = 0;
    QString m_version, m_error, m_cursor, m_detailRequested;
    QVariantMap m_catalogue, m_detail, m_query;
    QVariantList m_apps, m_saved;
};
