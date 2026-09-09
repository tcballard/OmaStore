import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
ColumnLayout {
 id:panel
 required property var core
 required property var theme
 property var report:({})
 property string notice:""
 readonly property bool operator:!!core.workspace.actor && (core.workspace.actor.roles || []).indexOf("operator")>=0
 spacing:14
 function refresh(){core.workspaceAction("commerce.status",{});}
 Connections {target:panel.core;function onWorkspaceChanged(){
  if(!panel.operator)modelText.text="";
  const r=panel.core.workspaceReply;
  if(r.action==="commerce.status"&&!r.error){panel.report=r;if(r.model)modelText.text=JSON.stringify(r.model,null,2);}
  if(r.action==="commerce.status"&&r.error)panel.notice=r.error.replace(/_/g," ");
  if(r.action==="command"&&(r.command==="commerce_model"||r.command==="commerce_pause")){panel.notice=r.error?r.error.replace(/_/g," "):"Commercial record saved. Real checkout remains unavailable.";ack.checked=false;panel.refresh();}
 }}
 Label {text:"Author income";font.bold:true;font.pixelSize:26*theme.scale;Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {text:"List for free. Keep your own checkout and support links with no OmaStore commission. Managed checkout proposes a 5% fee on the discounted subtotal before tax, with provider charges shown separately.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {objectName:"commerceGate";text:"Managed checkout is unavailable until its commercial launch requirements are verified.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Button {objectName:"loadCommerce";text:"View commerce readiness";enabled:!core.loading;onClicked:panel.refresh()}
 Label {text:panel.report.environment?"Record environment: "+panel.report.environment:"No operating record loaded";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Repeater {model:panel.report.missing || [];Label {required property string modelData;text:"Needed: "+modelData.replace(/([A-Z])/g," $1").toLowerCase();Layout.fillWidth:true;wrapMode:Text.Wrap}}
 Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
 Label {visible:panel.operator;text:"Operator record · Record actual responsibilities and link reviewed reports. Saving an assertion cannot open live payments.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 TextArea {id:modelText;visible:panel.operator;Layout.fillWidth:true;Layout.preferredHeight:240;wrapMode:TextEdit.Wrap;selectByMouse:true;Accessible.name:"Commercial operating facts JSON";textFormat:TextEdit.PlainText}
 TextField {id:reportDigest;visible:panel.operator;placeholderText:"Reviewed report SHA-256";maximumLength:64;Layout.fillWidth:true;Accessible.name:placeholderText}
 CheckBox {id:ack;visible:panel.operator;text:"I reviewed these actual operating facts";Layout.fillWidth:true}
 Button {visible:panel.operator;text:"Save operating record";enabled:ack.checked&&reportDigest.text.length===64&&!core.loading;onClicked:{try{core.workspaceAction("command",{command:"commerce_model",version:panel.report.version || 0,model:JSON.parse(modelText.text),report_digest:reportDigest.text});}catch(e){panel.notice="Enter a valid operating record.";}}}
 TextField {id:pauseReason;visible:panel.operator;placeholderText:"Reason for changing purchase availability";maximumLength:1000;Layout.fillWidth:true;Accessible.name:placeholderText}
 Button {visible:panel.operator;text:panel.report.newPurchasesPaused?"Allow test purchases":"Pause new purchases";enabled:pauseReason.text.length>0&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_pause",paused:!panel.report.newPurchasesPaused,reason:pauseReason.text})}
 Label {text:"Pausing purchases preserves receipt recovery, delivery retries, refunds and reconciliation. Free software and open-source rights remain independent.";Layout.fillWidth:true;wrapMode:Text.Wrap}
}
