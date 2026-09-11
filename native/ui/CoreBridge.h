#pragma once
#include <QObject>
#include <QProcess>
#include <QTimer>
#include <QVariantMap>
#include <QHash>

class CoreBridge final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(QString version READ version NOTIFY stateChanged)
    Q_PROPERTY(QString error READ error NOTIFY stateChanged)
    Q_PROPERTY(bool busy READ busy NOTIFY catalogueChanged)
    Q_PROPERTY(QVariantList apps READ apps NOTIFY catalogueChanged)
    Q_PROPERTY(QVariantMap detail READ detail NOTIFY detailChanged)
    Q_PROPERTY(QVariantMap catalogueState READ catalogueState NOTIFY catalogueChanged)
    Q_PROPERTY(QVariantMap filters READ filters NOTIFY catalogueChanged)
    Q_PROPERTY(QString catalogueError READ catalogueError NOTIFY catalogueChanged)
    Q_PROPERTY(int total READ total NOTIFY catalogueChanged)
    Q_PROPERTY(bool hasMore READ hasMore NOTIFY catalogueChanged)
public:
    explicit CoreBridge(QObject *parent=nullptr);
    ~CoreBridge() override;
    void start();
    bool ready() const {return m_ready;}
    QString version() const {return m_version;}
    QString error() const {return m_error;}
    bool busy() const {return !m_pending.isEmpty();}
    QVariantList apps() const {return m_apps;}
    QVariantMap detail() const {return m_detail;}
    QVariantMap catalogueState() const {return m_catalogueState;}
    QVariantMap filters() const {return m_filters;}
    QString catalogueError() const {return m_catalogueError;}
    int total() const {return m_total;}
    bool hasMore() const {return !m_cursor.isEmpty();}
    Q_INVOKABLE void search(const QVariantMap &filters);
    Q_INVOKABLE void nextPage();
    Q_INVOKABLE void showApp(const QString &id);
    Q_INVOKABLE void clearDetail();
    Q_INVOKABLE void refresh();
    Q_INVOKABLE void openExternal(const QString &url);
signals:
    void stateChanged();
    void catalogueChanged();
    void detailChanged();
private:
    QString send(const QString &method,const QVariantMap &params={});
    void readOutput();
    void acceptReply(const QByteArray &line);
    void fail(const QString &message);
    QProcess m_process;
    QTimer m_timeout;
    QByteArray m_output;
    bool m_ready=false;
    QString m_version,m_error,m_catalogueError,m_cursor,m_latestQuery,m_latestDetail;
    quint64 m_sequence=0;
    QHash<QString,QString> m_pending;
    QVariantList m_apps;
    QVariantMap m_detail,m_catalogueState,m_filters;
    int m_total=0;
};
