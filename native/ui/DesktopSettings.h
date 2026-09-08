#pragma once
#include <QObject>
#include <QDBusVariant>

class DesktopSettings final : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
    Q_PROPERTY(double textScale READ textScale NOTIFY changed)
    Q_PROPERTY(QString monoFont READ monoFont CONSTANT)
public:
    explicit DesktopSettings(QObject *parent = nullptr);
    bool dark() const { return m_dark; }
    double textScale() const { return m_scale; }
    QString monoFont() const;
signals:
    void changed();
private slots:
    void settingChanged(const QString &nameSpace, const QString &key, const QDBusVariant &value);
private:
    void read(const QString &nameSpace, const QString &key);
    void apply(const QString &nameSpace, const QString &key, QVariant value);
    bool m_dark = false;
    double m_scale = 1.0;
};
