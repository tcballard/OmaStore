import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
Dialog {
 id:dialog
 objectName:"purchasesDialog"
 required property var core
 required property var theme
 property string appId:""
 property var offers:[]
 property var orders:[]
 property var attempts:[]
 property var preview:({})
 property var order:({})
 property var recovery:({})
 property var readiness:({})
 property var seen:({})
 property string notice:""
 property string previousBuyer:""
 property int cursor:0
 readonly property bool signedIn:!!core.workspace.actor
 readonly property bool sample:!!core.workspace.sandbox
 function money(amounts,key){const a=amounts || {};return (a.currency || "").toUpperCase()+" "+((a[key] || 0)/Math.pow(10,a.exponent || 0)).toFixed(a.exponent || 0);}
 function openFor(id){appId=id || "";preview=({});order=({});recovery=({});notice="";consent.checked=false;refresh();open();}
 function refresh(){core.workspaceAction("commerce.prices",appId?{appId:appId}:{});core.workspaceAction("commerce.status",{});if(signedIn){core.workspaceAction("commerce.orders",{before:0});core.workspaceAction("commerce.pending",{});}}
 parent:Overlay.overlay;anchors.centerIn:parent;modal:true;title:"Purchases and licences"
 width:Math.min(800,parent?parent.width-32:800);height:Math.min(780,parent?parent.height-32:780);standardButtons:Dialog.Close
 Connections {target:dialog.core;function onWorkspaceChanged(){
  const buyer=(dialog.core.workspace.actor || {}).id || "";
  if(!dialog.signedIn || buyer!==dialog.previousBuyer){dialog.previousBuyer=buyer;dialog.seen=({});dialog.orders=[];dialog.attempts=[];dialog.preview=({});dialog.order=({});dialog.recovery=({});}
  const r=dialog.core.workspaceReply;if(!r.action||r.action.indexOf("commerce.")!==0)return;
  if(dialog.seen[r.action]===JSON.stringify(r))return;dialog.seen[r.action]=JSON.stringify(r);
  if(r.error){dialog.notice=r.error.replace(/_/g," ");return;}
  if(r.action==="commerce.status")dialog.readiness=r;
  if(r.action==="commerce.prices")dialog.offers=r.items || [];
  if(r.action==="commerce.orders"){dialog.orders=r.items || [];dialog.cursor=r.nextCursor || 0;}
  if(r.action==="commerce.pending")dialog.attempts=r.items || [];
  if(r.action==="commerce.prepare"||r.action==="commerce.resume"){dialog.preview=r;dialog.order=({});consent.checked=false;}
  if(["commerce.purchase","commerce.order","commerce.reconcile","commerce.retry","commerce.sample.capture"].indexOf(r.action)>=0){dialog.order=r;dialog.preview=({});dialog.recovery=({});consent.checked=false;dialog.core.workspaceAction("commerce.orders",{before:0});dialog.core.workspaceAction("commerce.pending",{});}
  if(r.action==="commerce.recover")dialog.recovery=r;
  if(r.action==="commerce.licence.export")dialog.notice="Licence saved. Keep a copy with your receipt.";
  if(r.action==="commerce.licence.verify")dialog.notice=(r.sample?"Fictional test licence":"Licence")+" verified offline for "+r.grant.appId+". "+r.notice;
 }}
 FileDialog {id:saveLicence;title:"Save your signed licence";fileMode:FileDialog.SaveFile;nameFilters:["OmaStore licence (*.json)"];defaultSuffix:"json";onAccepted:core.workspaceAction("commerce.licence.export",{id:dialog.order.id,digest:dialog.order.licenceDigest,file:selectedFile.toString()})}
 FileDialog {id:verifyLicence;title:"Verify a saved licence offline";fileMode:FileDialog.OpenFile;nameFilters:["OmaStore licence (*.json)"];onAccepted:core.workspaceAction("commerce.licence.verify",{file:selectedFile.toString()})}
 contentItem:ScrollView {id:purchaseScroll;clip:true;contentWidth:availableWidth;ColumnLayout {width:purchaseScroll.availableWidth;spacing:14
  Label {text:dialog.sample?"Fictional commerce rehearsal · No money changes hands.":"Receipts and licences stay recoverable from your account. Free software installs do not require an account.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Label {visible:!dialog.signedIn;text:"Sign in from the Author workspace to buy or recover a purchase.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Label {visible:!!dialog.notice;text:dialog.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText;Accessible.role:Accessible.AlertMessage}
  RowLayout {Layout.fillWidth:true;Button {text:"Refresh purchases";enabled:!core.loading;onClicked:dialog.refresh()} Button {objectName:"verifyLicence";text:"Verify saved licence offline";enabled:!core.loading;onClicked:verifyLicence.open()}}
  Label {text:dialog.readiness.newPurchasesPaused?"New purchases are paused. Existing receipts and recovery remain available.":dialog.sample?"Sample purchase controls are available.":"Real checkout remains unavailable pending commercial verification.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Label {text:"Managed offers";font.bold:true}
  Label {visible:dialog.offers.length===0;text:"No managed offer is available here. Author checkout and support links remain on each app’s page.";Layout.fillWidth:true;wrapMode:Text.Wrap}
  Repeater {model:dialog.offers;Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
   Label {text:modelData.price.title+" · "+dialog.money(modelData.price.amounts,"total")+(modelData.price.billingInterval?" / "+modelData.price.billingInterval:"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   Label {text:"Sold by "+modelData.price.sellerName;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   Button {objectName:"reviewPurchase-"+modelData.price.id;text:"Review purchase";enabled:dialog.signedIn&&!core.loading;onClicked:core.workspaceAction("commerce.prepare",{priceId:modelData.price.id})}
  }}}
  Frame {visible:!!dialog.preview.id;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
   Label {text:(dialog.preview.price || {}).title || "";font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   Label {text:"Seller: "+((dialog.preview.price || {}).sellerName || "")+"\nSubtotal: "+dialog.money((dialog.preview.price || {}).amounts,"subtotal")+"\nDiscount: "+dialog.money((dialog.preview.price || {}).amounts,"discount")+"\nTax: "+dialog.money((dialog.preview.price || {}).amounts,"tax")+"\nTotal: "+dialog.money((dialog.preview.price || {}).amounts,"total")+"\nOmaStore fee within seller proceeds: "+dialog.money((dialog.preview.price || {}).amounts,"serviceFee");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   Label {text:((dialog.preview.price || {}).licence || "")+"\n"+((dialog.preview.price || {}).onlineRequirement || "")+((dialog.preview.price || {}).billingInterval?"\nRenews every "+dialog.preview.price.billingInterval+" until cancelled under the seller's terms.":"");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   RowLayout {Button {text:"Seller terms ↗";onClicked:Qt.openUrlExternally(dialog.preview.price.termsUrl)} Button {text:"Seller support ↗";onClicked:Qt.openUrlExternally(dialog.preview.price.supportUrl)}}
   CheckBox {id:consent;objectName:"purchaseConsent";text:"I accept this price, seller, licence and billing terms";Layout.fillWidth:true}
   Button {objectName:"confirmPurchase";text:dialog.sample?"Create sample checkout":"Continue to checkout";enabled:consent.checked&&!dialog.readiness.newPurchasesPaused&&!core.loading&&(dialog.sample||dialog.readiness.environment==="development");onClicked:core.workspaceAction("commerce.purchase",{id:dialog.preview.id,accepted:true})}
  }}
  Frame {visible:!!dialog.order.id;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
   Label {objectName:"purchaseState";text:"Payment: "+(dialog.order.paymentState || "unknown").replace(/_/g," ")+" · Delivery: "+(dialog.order.deliveryState || "unknown").replace(/_/g," ");font.bold:true;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   TextField {text:dialog.order.id || "";readOnly:true;selectByMouse:true;Layout.fillWidth:true;Accessible.name:"Order reference"}
   Label {text:(dialog.order.price || {}).title || "";Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
   Label {visible:!!dialog.order.deliveryError;text:(dialog.order.deliveryError || "").replace(/_/g," ")+". Your paid order remains recorded; retry delivery or contact support.";Layout.fillWidth:true;wrapMode:Text.Wrap}
   Flow {Layout.fillWidth:true;spacing:8
    Button {visible:!!dialog.order.checkoutUrl;text:"Open secure provider checkout ↗";onClicked:Qt.openUrlExternally(dialog.order.checkoutUrl)}
    Button {text:"Check payment";enabled:!core.loading;onClicked:core.workspaceAction("commerce.reconcile",{id:dialog.order.id})}
    Button {objectName:"retryPurchaseDelivery";visible:dialog.order.paymentState==="paid"&&dialog.order.deliveryState!=="delivered";text:"Retry delivery";enabled:!core.loading;onClicked:core.workspaceAction("commerce.retry",{id:dialog.order.id})}
    Button {visible:!!dialog.order.grant;text:"Recover hosted access/download";enabled:!core.loading;onClicked:core.workspaceAction("commerce.recover",{id:dialog.order.id})}
    Button {text:"Contact seller support ↗";onClicked:Qt.openUrlExternally(dialog.order.price.supportUrl)}
   }
   CheckBox {id:failDelivery;objectName:"sampleDeliveryFailure";visible:dialog.sample&&dialog.order.paymentState!=="paid";text:"Rehearse a paid order with failed delivery";Layout.fillWidth:true}
   Button {objectName:"sampleCapture";visible:dialog.sample&&dialog.order.paymentState!=="paid";text:"Simulate successful payment";enabled:!core.loading;onClicked:core.workspaceAction("commerce.sample.capture",{id:dialog.order.id,failDelivery:failDelivery.checked})}
   TextArea {objectName:"licenceDocument";visible:!!dialog.order.licenceDocument;text:dialog.order.licenceDocument || "";readOnly:true;selectByMouse:true;wrapMode:TextEdit.Wrap;Layout.fillWidth:true;Layout.maximumHeight:220;textFormat:TextEdit.PlainText;Accessible.name:"Exact signed licence for export"}
   Button {objectName:"saveLicence";visible:!!dialog.order.licenceDigest;text:"Save signed licence";enabled:!core.loading;onClicked:saveLicence.open()}
   Label {visible:!!dialog.order.grant;text:"A perpetual licence can be verified offline with a trusted issuer key. Open-source rights remain independent. A signature records issuance; later refunds and service status have their own records.";Layout.fillWidth:true;wrapMode:Text.Wrap}
   Button {visible:!!dialog.recovery.recoveryUrl;text:"Open recovered access ↗";onClicked:Qt.openUrlExternally(dialog.recovery.recoveryUrl)}
   Button {visible:!!dialog.recovery.download;text:"Open recovered download ↗";onClicked:Qt.openUrlExternally(dialog.recovery.download.url)}
   TextField {visible:!!dialog.recovery.download;text:(dialog.recovery.download || {}).sha256 || "";readOnly:true;selectByMouse:true;Layout.fillWidth:true;Accessible.name:"Recovered download SHA-256"}
  }}
  Label {text:"Your receipts";font.bold:true}
  Repeater {model:dialog.orders;Button {required property var modelData;text:modelData.price.title+" · "+modelData.paymentState+" · "+modelData.deliveryState.replace(/_/g," ");Layout.fillWidth:true;enabled:!core.loading;onClicked:core.workspaceAction("commerce.order",{id:modelData.id})}}
  Button {visible:dialog.orders.length===30;text:"Older receipts";enabled:!core.loading;onClicked:core.workspaceAction("commerce.orders",{before:dialog.cursor})}
  Label {text:"Saved checkout attempts on this device";font.bold:true}
  Repeater {model:dialog.attempts;Button {required property var modelData;text:modelData.price.title+" · "+(modelData.orderId?"Open recorded order":"Resume exact checkout request");Layout.fillWidth:true;enabled:!core.loading;onClicked:core.workspaceAction(modelData.orderId?"commerce.order":"commerce.resume",{id:modelData.orderId || modelData.id})}}
  Button {objectName:"closePurchases";text:"Done";Layout.alignment:Qt.AlignRight;onClicked:dialog.close()}
 }}
}
