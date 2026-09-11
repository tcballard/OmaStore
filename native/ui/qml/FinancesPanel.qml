import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
ColumnLayout {
 id:panel
 objectName:"financesPanel"
 required property var core
 required property var theme
 property var sellers:[]
 property var report:({})
 property var support:({})
 property var selectedOrder:({})
 property var packet:({})
 property var invoiceIssues:[]
 property var reserve:({sellerId:"",currency:"gbp",target:0,expected:0,reason:"",reportUrl:"",reportDigest:""})
 property var evidence:({disputeId:"",reportUrl:"",reportDigest:"",note:""})
 property string notice:""
 property string previousActor:""
 property var seen:({})
 readonly property bool operator:!!core.workspace.actor && (core.workspace.actor.roles || []).indexOf("operator")>=0
 function money(n,currency){const e=currency==="jpy"?0:2;return currency.toUpperCase()+" "+(n/Math.pow(10,e)).toFixed(e);}
 function run(action,args){core.workspaceAction("commerce.lifecycle",Object.assign({action:action},args || {}));}
 function sellerId(){return sellers[sellerChoice.currentIndex]?sellers[sellerChoice.currentIndex].id:"";}
 function refresh(){if(sellerId())run("finances",{seller_id:sellerId(),refresh:true,cursor:null});}
 function edit(target,path,value){const copy=JSON.parse(JSON.stringify(target));let node=copy;for(let n=0;n<path.length-1;n++)node=node[path[n]];node[path[path.length-1]]=value;return copy;}
 spacing:12
 Connections {target:panel.core;function onWorkspaceChanged(){
  const actor=JSON.stringify([(panel.core.workspace.actor || {}).id || "",(panel.core.workspace.actor || {}).roles || []]);
  if(actor!==panel.previousActor){panel.previousActor=actor;panel.sellers=[];panel.report=({});panel.support=({});panel.selectedOrder=({});panel.packet=({});panel.invoiceIssues=[];panel.notice="";panel.seen=({});reserveConsent.checked=false;evidenceConsent.checked=false;}
  const r=panel.core.workspaceReply;
  if(r.action==="commerce.order"&&!r.error&&panel.operator)panel.selectedOrder=r;
  if(r.action==="commerce.packet.export"){panel.notice=r.error?r.error.replace(/_/g," "):"Private dispute packet saved.";return;}
  if(r.action==="command"&&(r.command==="commerce_reserve"||r.command==="commerce_evidence")){panel.notice=r.error?r.error.replace(/_/g," "):"Auditable record saved.";reserveConsent.checked=false;evidenceConsent.checked=false;panel.refresh();return;}
  if(r.action!=="commerce.lifecycle")return;
  if(panel.seen[r.operation || "error"]===JSON.stringify(r))return;panel.seen[r.operation || "error"]=JSON.stringify(r);
  if(r.error){panel.notice=r.error.replace(/_/g," ");return;}
  if(r.operation==="sellers")panel.sellers=r.items || [];
  if(r.operation==="finances"||r.operation==="dispute")panel.report=r;
  if(r.operation==="support"&&panel.operator)panel.support=r;
  if(r.operation==="dispute_packet"&&panel.operator)panel.packet=r;
  if(r.operation==="invoice_issues"&&panel.operator)panel.invoiceIssues=r.items || [];
  if(r.operation==="sample_scenario"){panel.notice="Fictional provider event created. Refresh its records to reconcile.";if(r.disputeId){disputeId.text=r.disputeId;panel.run("dispute",{seller_id:panel.sellerId(),id:r.disputeId});}if(r.invoiceId)panel.notice="Fictional renewal recorded at the provider. Refresh billing on the original subscription to recover its receipt.";panel.refresh();}
  if(r.operation==="retry_invoice")panel.notice="Verified invoice recovered. Delivery is queued under its recorded period.";
 }}
 FileDialog {id:savePacket;title:"Save reviewed private dispute evidence";fileMode:FileDialog.SaveFile;nameFilters:["JSON document (*.json)"];defaultSuffix:"json";onAccepted:core.workspaceAction("commerce.packet.export",{id:panel.packet.packet.dispute.id,digest:panel.packet.digest,file:selectedFile.toString()})}
 Label {text:"Sales and provider records";font.bold:true;font.pixelSize:22*theme.scale;Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {text:"Each currency stays separate. Recorded store proceeds include seller-owned tax, less observed fees and refunds. The provider account can contain other income, and its payouts are account-level records.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.role:Accessible.AlertMessage}
 Button {objectName:"loadFinanceSellers";text:"Load my commercial sellers";enabled:!!core.workspace.actor&&!core.loading;onClicked:panel.run("sellers",{})}
 ComboBox {id:sellerChoice;objectName:"financeSeller";model:panel.sellers;textRole:"name";Layout.fillWidth:true;Accessible.name:"Commercial seller";onActivated:{panel.report=({});panel.refresh();}}
 Button {objectName:"reconcileFinances";text:"Reconcile provider balances and payouts";enabled:!!panel.sellerId()&&!core.loading;onClicked:panel.refresh()}
 Label {visible:!!panel.report.sellerId;objectName:"financeObservation";text:"Provider observed: "+(panel.report.providerObservedAt?new Date(panel.report.providerObservedAt*1000).toLocaleString():"not yet")+" · Orders awaiting processing fees: "+(panel.report.providerFeesPending || 0);Layout.fillWidth:true;wrapMode:Text.Wrap}
 Repeater {model:panel.report.currencies || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
  Label {objectName:"financeBalance-"+modelData.currency;text:"Recorded proceeds: "+panel.money(modelData.recordedProceeds,modelData.currency)+"\nAccounting reserve: "+panel.money(modelData.accountingReserve,modelData.currency)+"\nAfter reserve: "+panel.money(modelData.afterReserve,modelData.currency)+(modelData.negative?" · Negative balance":"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  Button {visible:panel.operator;text:"Prepare reserve record for this currency";onClicked:{const r=JSON.parse(JSON.stringify(panel.reserve));r.sellerId=panel.sellerId();r.currency=modelData.currency;r.expected=modelData.accountingReserve;r.target=modelData.accountingReserve;panel.reserve=r;}}
 }}}
 Repeater {model:(panel.report.providerReport || {}).balances || [];Label {required property var modelData;text:"Provider available: "+panel.money(modelData.available,modelData.currency)+" · Pending: "+panel.money(modelData.pending,modelData.currency);Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
 Label {visible:(panel.report.reconciliationIssues || []).length>0;text:"Reconciliation needs attention. Outside-store refunds or missing provider observations can change the amount owed.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Repeater {model:panel.report.reconciliationIssues || [];Label {required property var modelData;text:modelData.orderId+" · "+(modelData.error || "Provider and store refund totals differ").replace(/_/g," ")+" · Provider refunded minor units: "+modelData.providerRefunded+" · Recorded: "+modelData.recordedRefunded;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
 Label {text:"Payout observations";font.bold:true}
 Repeater {model:panel.report.payouts || [];Label {required property var modelData;objectName:"payout-"+modelData.id;text:panel.money(modelData.amount,modelData.currency)+" · "+modelData.state.replace(/_/g," ")+(modelData.failureCode?" · "+modelData.failureCode.replace(/_/g," "):"")+"\n"+modelData.id+(modelData.arrivalAt?" · Expected arrival: "+new Date(modelData.arrivalAt*1000).toLocaleDateString():"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
 Button {visible:!!(panel.report.providerReport || {}).hasMorePayouts;text:"Reconcile older provider payouts";enabled:!core.loading;onClicked:panel.run("finances",{seller_id:panel.sellerId(),refresh:true,cursor:panel.report.providerReport.nextPayoutCursor})}
 Label {text:"Recent order ledger";font.bold:true}
 Repeater {model:panel.report.ledger || [];Label {required property var modelData;text:panel.money(modelData.amount,modelData.currency)+" · "+modelData.kind.replace(/_/g," ")+(modelData.orderId?" · "+modelData.orderId:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
 Label {text:"Disputes";font.bold:true}
 Repeater {model:panel.report.disputes || [];Label {required property var modelData;text:panel.money(modelData.amount,modelData.currency)+" · "+modelData.state.replace(/_/g," ")+" · "+modelData.id+(modelData.evidenceDue?" · Evidence due: "+new Date(modelData.evidenceDue*1000).toLocaleString():"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
 TextField {id:disputeId;placeholderText:"Provider dispute reference";Accessible.name:placeholderText;maximumLength:128;Layout.fillWidth:true}
 Button {text:"Retrieve current dispute";enabled:!!panel.sellerId()&&disputeId.text.length>0&&!core.loading;onClicked:panel.run("dispute",{seller_id:panel.sellerId(),id:disputeId.text})}
 ColumnLayout {visible:panel.operator;Layout.fillWidth:true
  Label {text:"Operator support";font.bold:true;font.pixelSize:20*theme.scale}
  TextField {id:orderId;objectName:"financeOrderReference";placeholderText:"Order reference for support or refund handling";Accessible.name:placeholderText;maximumLength:128;Layout.fillWidth:true}
  Flow {Layout.fillWidth:true;spacing:8
   Button {objectName:"loadSupportOrder";text:"Open order and refund controls";enabled:orderId.text.length>0&&!core.loading;onClicked:core.workspaceAction("commerce.order",{id:orderId.text})}
   Button {text:"Read support history";enabled:orderId.text.length>0&&!core.loading;onClicked:panel.run("support",{id:orderId.text,note:null})}
  }
  Repeater {model:panel.report.orders || [];Button {required property var modelData;text:"Order "+modelData.id+" · "+modelData.paymentState+" / "+modelData.deliveryState.replace(/_/g," ");Layout.fillWidth:true;enabled:!core.loading;onClicked:{orderId.text=modelData.id;core.workspaceAction("commerce.order",{id:modelData.id});}}}
  Label {visible:!!panel.selectedOrder.id;text:((panel.selectedOrder.price || {}).title || "")+" · "+(panel.selectedOrder.id || "");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  RefundsPanel {core:panel.core;theme:panel.theme;order:panel.selectedOrder;visible:panel.selectedOrder.paymentState==="paid";Layout.fillWidth:true}
  TextField {id:supportNote;placeholderText:"Private support outcome or next step";Accessible.name:placeholderText;maximumLength:1000;Layout.fillWidth:true}
  Button {text:"Record support note";enabled:orderId.text.length>0&&supportNote.text.trim().length>0&&!core.loading;onClicked:panel.run("support",{id:orderId.text,note:supportNote.text})}
  Repeater {model:panel.support.events || [];Label {required property var modelData;text:new Date(modelData.at*1000).toLocaleString()+" · "+modelData.kind.replace(/_/g," ")+"\n"+JSON.stringify(modelData.detail);Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}}
  Button {text:"Load unresolved renewal invoices";enabled:!!panel.sellerId()&&!core.loading;onClicked:panel.run("invoice_issues",{seller_id:panel.sellerId()})}
  Repeater {model:panel.invoiceIssues;RowLayout {required property var modelData;Layout.fillWidth:true;Label {text:modelData.invoiceId+" · "+modelData.error.replace(/_/g," ");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText} Button {text:"Reconcile invoice";enabled:!core.loading;onClicked:panel.run("retry_invoice",{seller_id:panel.sellerId(),id:modelData.invoiceId})}}}
  Label {text:"Accounting reserve";font.bold:true}
  Label {text:"This is an accounting record. Confirm the actual provider reserve arrangement separately; this form does not move or lock provider funds.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  ValueEditor {value:panel.reserve;field:"reserve";theme:panel.theme;onEdited:(path,value)=>panel.reserve=panel.edit(panel.reserve,path,value)}
  CheckBox {id:reserveConsent;text:"I reviewed this reserve and its supporting report";Layout.fillWidth:true}
  Button {text:"Record reserve";enabled:reserveConsent.checked&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_reserve",reserve:panel.reserve})}
  Label {text:"Dispute evidence";font.bold:true}
  Button {text:"Prepare evidence for selected dispute";enabled:disputeId.text.length>0;onClicked:{const e=JSON.parse(JSON.stringify(panel.evidence));e.disputeId=disputeId.text;panel.evidence=e;}}
  ValueEditor {value:panel.evidence;field:"evidence";theme:panel.theme;onEdited:(path,value)=>panel.evidence=panel.edit(panel.evidence,path,value)}
  CheckBox {id:evidenceConsent;text:"I reviewed this private evidence record";Layout.fillWidth:true}
  Button {text:"Record evidence";enabled:evidenceConsent.checked&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_evidence",evidence:panel.evidence})}
  Button {objectName:"previewFinancePacket";text:"Preview private evidence packet";enabled:disputeId.text.length>0&&!core.loading;onClicked:panel.run("dispute_packet",{id:disputeId.text})}
  TextArea {visible:!!panel.packet.digest;text:panel.packet.packet?JSON.stringify(panel.packet.packet,null,2):"";readOnly:true;selectByMouse:true;wrapMode:TextEdit.Wrap;Layout.fillWidth:true;Layout.maximumHeight:220;textFormat:TextEdit.PlainText;Accessible.name:"Exact private dispute packet for export"}
  Button {visible:!!panel.packet.digest;text:"Save reviewed evidence packet";enabled:!core.loading;onClicked:savePacket.open()}
  Label {text:"The responsible operator submits evidence through the provider’s dispute process. A local packet is not a submission confirmation.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  ColumnLayout {visible:!!core.workspace.sandbox;Layout.fillWidth:true
   Label {text:"Fictional provider scenarios";font.bold:true}
   ComboBox {id:scenario;objectName:"financeScenario";model:["payout_pending","payout_failed","payout_paid","dispute_open","dispute_won","dispute_lost","next_refund_pending","settle_refunds","renewal"];Layout.fillWidth:true;Accessible.name:"Fictional provider event"}
   Button {objectName:"simulateFinance";text:"Create fictional provider event for order";enabled:orderId.text.length>0&&!core.loading;onClicked:panel.run("sample_scenario",{id:orderId.text,scenario:scenario.currentText})}
  }
 }
}
