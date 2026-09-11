#pragma once
#include "CoreBridge.h"
#include <QQuickItem>
#include <QQuickWindow>
#include <QTest>
#include <QDebug>
inline bool checkDiscovery(CoreBridge &core,QQuickWindow *window){
    const auto wait=[&]{return QTest::qWaitFor([&]{return core.ready() && !core.busy();},5000);};
    const auto verify=[](bool condition,const char *name){if(!condition)qWarning()<<"FAILED:"<<name;return condition;};
    if(!verify(wait(),"catalogue startup"))return false;
    auto *input=window->findChild<QQuickItem*>("searchInput");
    auto *list=window->findChild<QQuickItem*>("appList");
    if(!verify(input && list,"native controls"))return false;
    QMetaObject::invokeMethod(window,"clearFilters");
    if(!verify(wait() && core.total()==4,"real informational catalogue"))return false;
    input->forceActiveFocus();for(char c:QByteArray("localsend"))QTest::keyClick(window,c);
    QTest::qWait(150);
    if(!verify(wait() && core.total()==1 && core.apps().first().toMap().value("id")=="localsend","keyboard search"))return false;
    list->setProperty("currentIndex",0);list->forceActiveFocus();QTest::keyClick(window,Qt::Key_Return);
    if(!verify(wait() && core.detail().value("id")=="localsend" && window->property("detailOpen").toBool(),"keyboard detail"))return false;
    auto *evidence=window->findChild<QObject*>("evidenceText");
    if(!verify(evidence && evidence->property("text").toString().contains("has not been tested"),"unknown evidence rendering"))return false;
    QTest::keyClick(window,Qt::Key_Left,Qt::AltModifier);QTest::qWait(100);
    if(!verify(!window->property("detailOpen").toBool() && input->property("text")=="localsend","back keeps query"))return false;
    input->forceActiveFocus();QTest::keyClick(window,Qt::Key_A,Qt::ControlModifier);
    for(char c:QByteArray("no-such-application"))QTest::keyClick(window,c);
    QTest::qWait(150);
    if(!verify(wait() && core.total()==0,"empty search"))return false;
    window->setProperty("filtersOpen",true);window->resize(800,600);QTest::qWait(100);
    if(!verify(input->width()>100 && input->isVisible(),"compact search layout"))return false;
    QMetaObject::invokeMethod(window,"clearFilters");if(!wait())return false;
    core.search({{"text","localsend"}});if(!wait())return false;
    core.refresh();if(!wait())return false;
    if(!verify(core.total()==1 && core.catalogueState().value("stale").toBool(),"failed refresh keeps results"))return false;
    qInfo()<<"Native keyboard/search/detail/back/empty/compact/offline checks passed";return true;
}
