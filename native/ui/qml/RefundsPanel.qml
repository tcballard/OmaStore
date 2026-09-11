import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
ColumnLayout {
 id:panel
 objectName:"refundsPanel"
 required property var core
 required property var theme
 required property var order
 property var report:({})
 property var subscription:({})
 property string notice:""
 property string previousOrder:""
 readonly property bool operator:!!core.workspace.actor && (core.workspace.actor.roles || []).indexOf("operator")>=0
 readonly property var amounts:(order.price || {}).amounts || {}
 readonly property int exponent:amounts.exponent || 0
 function money(n){return (amounts.currency || "").toUpperCase()+" "+(n/Math.pow(10,exponent)).toFixed(exponent);}
 function minor(){const parts=amount.text.trim().split(".");if(parts.length>2 || !/^[0-9]+$/.test(parts[0]) || (parts.length===2 && (!/^[0-9]+$/.test(parts[1]) || parts[1].length>exponent)))return 0;const n=Number(parts[0])*Math.pow(10,exponent)+Number(((parts[1] || "")+"0".repeat(exponent)).slice(0,exponent));return Number.isSafeInteger(n)?n:0;}
 function run(action,args){core.workspaceAction("commerce.lifecycle",Object.assign({action:action},args || {}));}
 function refresh(){if(order.id && order.paymentState==="paid")run("refunds",{id:order.id});}
 onOrderChanged:{if(previousOrder!==order.id){previousOrder=order.id || "";report=({});notice="";amount.text="";reason.text="";refundConsent.checked=false;cancelConsent.checked=false;operatorConsent.checked=false;}subscription=order.subscription || {};refresh();}
 spacing:10
 Connections {target:panel.core;function onWorkspaceChanged(){
  if(!panel.visible)return;
  const r=panel.core.workspaceReply;
  if(r.action!=="commerce.lifecycle")return;
  if(r.error){panel.notice=r.error.replace(/_/g," ");return;}
  if((r.operation==="refunds" || r.operation==="poll_refund") && r.orderId===panel.order.id)panel.report=r;
  if(["request_refund","execute_refund","reject_refund"].indexOf(r.operation)>=0 && r.orderId===panel.order.id){refundConsent.checked=false;operatorConsent.checked=false;panel.notice=r.operation==="request_refund"?"Refund request recorded. Approval and provider settlement are separate steps.":"Refund record updated.";panel.refresh();panel.core.workspaceAction("commerce.order",{id:panel.order.id});}
  if(["subscription","cancel_subscription"].indexOf(r.operation)>=0 && r.rootOrderId===(panel.order.subscription || {}).rootOrderId){panel.subscription=r;cancelConsent.checked=false;panel.notice=r.cancellation && r.cancellation.state==="confirmed"?"Future billing cancellation confirmed. Your records remain available.":r.notice;}
 }}
 Label {text:"Refunds";font.bold:true;Layout.fillWidth:true}
 Label {objectName:"refundSummary";text:"Completed: "+panel.money(panel.report.refunded || 0)+" · Remaining refundable: "+panel.money(panel.report.remaining || 0);Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.role:Accessible.AlertMessage}
 Button {text:"Refresh refund status";enabled:!core.loading;onClicked:panel.refresh()}
 RowLayout {Layout.fillWidth:true
  TextField {id:amount;objectName:"refundAmount";placeholderText:"Refund amount in "+(panel.amounts.currency || "").toUpperCase();Accessible.name:placeholderText;maximumLength:12;Layout.fillWidth:true;inputMethodHints:Qt.ImhFormattedNumbersOnly}
  Button {text:"Full remaining amount";onClicked:amount.text=((panel.report.remaining || 0)/Math.pow(10,panel.exponent)).toFixed(panel.exponent)}
 }
 TextField {id:reason;objectName:"refundReason";placeholderText:"Reason for this refund request";Accessible.name:placeholderText;maximumLength:1000;Layout.fillWidth:true}
 CheckBox {id:refundConsent;objectName:"refundConsent";text:"I request this amount under the seller’s refund terms";Layout.fillWidth:true}
 Button {objectName:"requestRefund";text:"Request refund";enabled:refundConsent.checked&&reason.text.trim().length>0&&panel.minor()>0&&panel.minor()<=(panel.report.remaining || 0)&&!core.loading;onClicked:panel.run("request_refund",{request:{orderId:panel.order.id,amount:panel.minor(),expectedRefunded:panel.report.refunded || 0,reason:reason.text,accepted:true}})}
 Label {text:"Refunds return money through the original provider. Provider charges and the platform-fee reversal are recorded separately. Existing receipts and issued licences stay in your history.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 CheckBox {id:operatorConsent;objectName:"approveRefundConsent";visible:panel.operator;text:"I reviewed this refund and authorise its provider execution";Layout.fillWidth:true}
 TextField {id:rejection;visible:panel.operator;placeholderText:"Reason if rejecting a request";Accessible.name:placeholderText;maximumLength:1000;Layout.fillWidth:true}
 Repeater {model:panel.report.items || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
  Label {text:panel.money(modelData.amount)+" · "+modelData.state+" · Fee reversal: "+panel.money(modelData.feeAmount)+" / "+modelData.feeState;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  Label {text:modelData.reason+(modelData.error?"\n"+modelData.error.replace(/_/g," "):"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  Flow {Layout.fillWidth:true;spacing:8
   Button {objectName:"executeRefund-"+modelData.id;visible:panel.operator && ["requested","pending"].indexOf(modelData.state)>=0;text:modelData.state==="requested"?"Approve refund":"Reconcile approved refund";enabled:operatorConsent.checked&&!core.loading;onClicked:panel.run("execute_refund",{id:modelData.id})}
   Button {visible:panel.operator && modelData.state==="requested";text:"Reject request";enabled:rejection.text.trim().length>0&&!core.loading;onClicked:panel.run("reject_refund",{id:modelData.id,reason:rejection.text})}
   Button {visible:modelData.state==="pending" || modelData.state==="succeeded" && modelData.feeState!=="succeeded";text:"Check provider status";enabled:!core.loading;onClicked:panel.run("poll_refund",{id:modelData.id})}
  }
 }}}
 ColumnLayout {visible:!!panel.subscription.rootOrderId;Layout.fillWidth:true
  Label {text:"Subscription";font.bold:true}
  Label {text:"Paid through "+(panel.subscription.paidUntil?new Date(panel.subscription.paidUntil*1000).toLocaleString():"unconfirmed")+" · Cancellation: "+((panel.subscription.cancellation || {}).state || "not requested")+((panel.subscription.cancellation || {}).error?" · "+panel.subscription.cancellation.error.replace(/_/g," "):"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  Label {visible:!!panel.subscription.providerStatus;text:"Provider billing: "+((panel.subscription.providerStatus || {}).state || "unknown")+" · Cancel at period end: "+((panel.subscription.providerStatus || {}).cancelAtPeriodEnd?"yes":"no");Layout.fillWidth:true;wrapMode:Text.Wrap}
  Label {visible:!!panel.subscription.paidPeriodElapsed;text:"This recorded paid period has ended. Refresh billing to look for a verified renewal.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Button {text:"Refresh billing and renewal receipts";enabled:!core.loading;onClicked:panel.run("subscription",{id:panel.order.id,refresh:true})}
  CheckBox {id:cancelConsent;objectName:"cancelSubscriptionConsent";text:"Stop future billing at the end of the current period";Layout.fillWidth:true}
  Button {objectName:"cancelSubscription";text:"Cancel future billing";enabled:cancelConsent.checked&&!core.loading;onClicked:panel.run("cancel_subscription",{id:panel.order.id,accepted:true})}
  Label {text:"Cancellation keeps paid receipts and local documents. Hosted access follows the seller’s agreed terms and recorded paid periods.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Repeater {model:panel.subscription.cycles || [];Button {required property var modelData;text:"Open renewal receipt · "+new Date(modelData.periodEnd*1000).toLocaleDateString();Layout.fillWidth:true;onClicked:core.workspaceAction("commerce.order",{id:modelData.orderId})}}
 }
}
