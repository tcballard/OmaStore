#include "CoreBridge.h"
#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSettings>
#include <QUrl>
namespace { constexpr qsizetype MaxLineBytes=256*1024; }
CoreBridge::CoreBridge(QObject *parent):QObject(parent) {
    QSettings settings;
    m_filters=settings.value("discovery/filters").toMap();
    m_filters.remove("cursor");m_filters["limit"]=12;
    m_timeout.setSingleShot(true);m_timeout.setInterval(12000);
    connect(&m_timeout,&QTimer::timeout,this,[this]{fail("The local core did not respond. Reopen OmaStore to retry.");});
    connect(&m_process,&QProcess::started,this,[this]{send("core.info");});
    connect(&m_process,&QProcess::readyReadStandardOutput,this,&CoreBridge::readOutput);
    connect(&m_process,&QProcess::readyReadStandardError,this,[this]{while(!m_process.readAllStandardError().isEmpty()) {}});
    connect(&m_process,&QProcess::errorOccurred,this,[this](QProcess::ProcessError){fail("Could not run the local core.");});
    connect(&m_process,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),this,[this](int,QProcess::ExitStatus){if(m_error.isEmpty())fail("The local core stopped. Reopen OmaStore to retry.");});
}
CoreBridge::~CoreBridge(){
    m_timeout.stop();m_process.disconnect(this);m_process.closeWriteChannel();
    if(!m_process.waitForFinished(250)){m_process.terminate();if(!m_process.waitForFinished(250)){m_process.kill();m_process.waitForFinished(250);}}
}
void CoreBridge::start(){
    if(m_process.state()!=QProcess::NotRunning)return;
    m_output.clear();m_pending.clear();m_ready=false;m_error.clear();emit stateChanged();
    m_process.setProgram(QDir(QCoreApplication::applicationDirPath()).filePath("omastore-core"));
    m_process.setArguments({"--stdio"});m_process.setProcessChannelMode(QProcess::SeparateChannels);m_timeout.start();m_process.start();
}
QString CoreBridge::send(const QString &method,const QVariantMap &params){
    const auto id=QString::number(++m_sequence);
    QJsonObject request{{"protocol_version",1},{"id",id},{"method",method},{"params",QJsonObject::fromVariantMap(params)}};
    const auto bytes=QJsonDocument(request).toJson(QJsonDocument::Compact)+'\n';
    if(bytes.size()>MaxLineBytes){fail("The local request was too large.");return {};}
    m_pending.insert(id,method);m_process.write(bytes);m_timeout.start();emit catalogueChanged();return id;
}
void CoreBridge::search(const QVariantMap &filters){
    m_filters=filters;m_filters.remove("cursor");m_filters["limit"]=12;
    QSettings().setValue("discovery/filters",m_filters);
    if(!m_ready)return;
    m_catalogueError.clear();m_cursor.clear();
    m_latestQuery=send("catalogue.query",m_filters);
}
void CoreBridge::nextPage(){
    if(!m_ready || m_cursor.isEmpty() || busy())return;
    auto params=m_filters;params["cursor"]=m_cursor;
    m_latestQuery=send("catalogue.query",params);m_pending[m_latestQuery]="catalogue.next";
}
void CoreBridge::showApp(const QString &id){
    if(!m_ready)return;
    m_detail.clear();emit detailChanged();m_latestDetail=send("catalogue.detail",{{"id",id}});
}
void CoreBridge::clearDetail(){m_latestDetail.clear();m_detail.clear();emit detailChanged();}
void CoreBridge::refresh(){if(m_ready && !busy()){m_catalogueError.clear();send("catalogue.refresh");}}
void CoreBridge::openExternal(const QString &text){
    const QUrl url(text,QUrl::StrictMode);
    if(url.isValid() && url.scheme()=="https" && !url.host().isEmpty() && url.userInfo().isEmpty())QDesktopServices::openUrl(url);
}
void CoreBridge::readOutput(){
    while(m_process.bytesAvailable()>0){
        m_output+=m_process.read(4096);qsizetype newline;
        while((newline=m_output.indexOf('\n'))>=0){
            if(newline+1>MaxLineBytes){fail("The local core sent an oversized reply.");return;}
            auto line=m_output.left(newline);m_output.remove(0,newline+1);acceptReply(line);if(!m_error.isEmpty())return;
        }
        if(m_output.size()>MaxLineBytes){fail("The local core sent an oversized reply.");return;}
    }
}
void CoreBridge::acceptReply(const QByteArray &line){
    QJsonParseError parseError;const auto document=QJsonDocument::fromJson(line,&parseError);const auto reply=document.object();const auto id=reply.value("id").toString();
    if(parseError.error!=QJsonParseError::NoError || !document.isObject() || reply.value("protocol_version").toInt(-1)!=1 || !m_pending.contains(id)){fail("The local core replied with an unsupported message.");return;}
    const auto method=m_pending.take(id);if(m_pending.isEmpty())m_timeout.stop();
    if(!reply.value("ok").toBool()){
        const auto code=reply.value("error").toObject().value("code").toString();
        if(method=="core.info"){fail("The local core handshake failed.");return;}
        // A delayed obsolete search error must not replace the current results.
        if((method=="catalogue.query" || method=="catalogue.next") && id!=m_latestQuery)return;
        if(code=="snapshot_changed"){search(m_filters);return;}
        m_catalogueError=code=="invalid_filter" ? "A saved filter is invalid. Clear filters to continue." : "Could not load this catalogue view. Retry or refresh.";
        emit catalogueChanged();return;
    }
    const auto result=reply.value("result").toObject();
    if(method=="core.info"){
        if(result.value("service").toString()!="omastore-core" || result.value("version").toString().isEmpty()){fail("The local core handshake failed.");return;}
        m_version=result.value("version").toString();m_ready=true;emit stateChanged();send("catalogue.info");search(m_filters);
    }else if(method=="catalogue.info" || method=="catalogue.refresh"){
        m_catalogueState=result.value("state").toObject().toVariantMap();
        if(method=="catalogue.refresh"){
            search(m_filters);
            if(!m_detail.isEmpty())showApp(m_detail.value("id").toString());
        }
    }else if((method=="catalogue.query" || method=="catalogue.next") && id==m_latestQuery){
        if(method=="catalogue.next")m_apps+=result.value("apps").toVariant().toList();else m_apps=result.value("apps").toVariant().toList();
        m_total=result.value("total").toInt();m_cursor=result.value("nextCursor").toString();
    }else if(method=="catalogue.detail" && id==m_latestDetail){m_detail=result.toVariantMap();emit detailChanged();}
    emit catalogueChanged();
}
void CoreBridge::fail(const QString &message){
    if(!m_error.isEmpty())return;m_timeout.stop();m_ready=false;m_error=message;m_output.clear();m_pending.clear();
    m_process.closeReadChannel(QProcess::StandardOutput);if(m_process.state()!=QProcess::NotRunning)m_process.terminate();
    emit stateChanged();emit catalogueChanged();
}
