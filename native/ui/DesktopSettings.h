#pragma once
#include <QObject>
#include <QDBusVariant>
#include <QFileSystemWatcher>
#include <QTimer>
#include <QVariantMap>
#include <QProcess>

class DesktopSettings final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
    Q_PROPERTY(double textScale READ textScale NOTIFY changed)
    Q_PROPERTY(QString monoFont READ monoFont NOTIFY changed)
    Q_PROPERTY(bool followOmarchy READ followOmarchy WRITE setFollowOmarchy NOTIFY changed)
    Q_PROPERTY(QVariantMap themeColors READ themeColors NOTIFY changed)
public:
    explicit DesktopSettings(QObject *parent = nullptr);
    ~DesktopSettings() override;
    DesktopSettings(const QString &themeDirectory, const QString &fontConfig, QObject *parent = nullptr);
    bool dark() const { return m_dark; }
    double textScale() const { return m_scale; }
    QString monoFont() const;
    bool followOmarchy() const { return m_follow; }
    void setFollowOmarchy(bool value);
    QVariantMap themeColors() const { return m_colors; }
signals:
    void changed();
private slots:
    void settingChanged(const QString &nameSpace, const QString &key, const QDBusVariant &value);
private:
    void read(const QString &nameSpace, const QString &key);
    void apply(const QString &nameSpace, const QString &key, QVariant value);
    void reloadTheme();
    void watchPaths();
    void refreshFont();
    QString m_themeDirectory, m_fontConfig, m_font;
    QVariantMap m_colors;
    QFileSystemWatcher m_files;
    QTimer m_debounce, m_poll, m_fontTimeout;
    QProcess m_fontProcess;
    bool m_follow = true;
    bool m_dark = false;
    double m_scale = 1.0;
};
