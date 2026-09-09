import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
ColumnLayout {
 id:panel
 required property var core
 required property var theme
 property var author:({})
 property var price:({})
 property var seller:({id:"",owner:"",account:"",name:"",active:false,reportUrl:"",reportDigest:""})
 property string notice:""
 property string previousActor:""
 readonly property bool signedIn:!!core.workspace.actor
 readonly property bool operator:signedIn&&(core.workspace.actor.roles || []).indexOf("operator")>=0
 function patch(source,path,value){const copy=JSON.parse(JSON.stringify(source));let target=copy;for(let i=0;i<path.length-1;i++)target=target[path[i]];target[path[path.length-1]]=value;return copy;}
 function newPrice(){const s=(author.sellers || [])[sellerChoice.currentIndex];const app=(author.apps || [])[appChoice.currentIndex];if(!s||!app)return;price={id:"",version:1,appId:app,sellerId:s.id,sellerName:s.name,connectedAccount:s.account,title:"",currency:"gbp",subtotal:0,discount:0,tax:0,taxRateId:null,deliveryKind:"perpetual",billingInterval:null,licence:"",onlineRequirement:"",supportUrl:"",termsUrl:""};}
 Connections {target:panel.core;function onWorkspaceChanged(){const r=panel.core.workspaceReply;const actor=(panel.core.workspace.actor || {}).id || "";if(!panel.signedIn||actor!==panel.previousActor){panel.previousActor=actor;panel.author=({});panel.price=({});}if(r.action==="commerce.author"){if(r.error)panel.notice=r.error.replace(/_/g," ");else panel.author=r;}if(r.action==="command"&&["commerce_seller","commerce_price","commerce_withdraw"].indexOf(r.command)>=0){panel.notice=r.error?r.error.replace(/_/g," "):"Commercial record saved.";priceConsent.checked=false;sellerConsent.checked=false;panel.core.workspaceAction("commerce.author",{});}}}
 Label {text:"Your managed offers";font.bold:true;font.pixelSize:22*theme.scale;Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {text:"Amounts use the currency’s minor unit: 100 means £1.00 for GBP, or ¥100 for JPY. Publish a new version to change a price; existing orders keep their agreed terms. These controls do not enable live sales.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Button {objectName:"loadCommerceAuthor";text:"Load your approved sellers and offers";enabled:panel.signedIn&&!core.loading;onClicked:core.workspaceAction("commerce.author",{})}
 Label {text:panel.author.notice || "";Layout.fillWidth:true;wrapMode:Text.Wrap}
 Label {visible:!!panel.notice;text:panel.notice;Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
 ComboBox {id:sellerChoice;model:panel.author.sellers || [];textRole:"name";Layout.fillWidth:true;Accessible.name:"Approved commercial seller"}
 ComboBox {id:appChoice;model:panel.author.apps || [];Layout.fillWidth:true;Accessible.name:"Your published app"}
 Button {text:"Prepare new managed offer";enabled:sellerChoice.currentIndex>=0&&appChoice.currentIndex>=0;onClicked:panel.newPrice()}
 Repeater {model:panel.author.offers || [];Frame {required property var modelData;Layout.fillWidth:true;ColumnLayout {anchors.fill:parent
  Label {text:modelData.price.title+" · version "+modelData.price.version+" · "+(modelData.active?"Available":"Withdrawn");Layout.fillWidth:true;wrapMode:Text.Wrap;textFormat:Text.PlainText}
  Button {text:"Prepare next price version";onClicked:{const next=JSON.parse(JSON.stringify(modelData.price));next.version++;panel.price=next;priceConsent.checked=false;}}
  Button {text:"Withdraw new purchases";enabled:modelData.active&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_withdraw",id:modelData.price.id})}
 }}}
 ValueEditor {visible:Object.keys(panel.price).length>0;value:panel.price;field:"Managed offer";theme:panel.theme;onEdited:(path,value)=>panel.price=panel.patch(panel.price,path,value)}
 CheckBox {id:priceConsent;visible:Object.keys(panel.price).length>0;text:"I reviewed this price, licence, tax and billing terms";Layout.fillWidth:true}
 Button {visible:Object.keys(panel.price).length>0;text:"Publish price version";enabled:priceConsent.checked&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_price",price:panel.price})}
 Label {visible:panel.operator;text:"Approve a commercial seller";font.bold:true}
 Label {visible:panel.operator;text:"Match the account to the reviewed operator/provider agreement. A source-control claim alone does not establish a billing identity. Existing orders keep their original seller record.";Layout.fillWidth:true;wrapMode:Text.Wrap}
 ValueEditor {visible:panel.operator;value:panel.seller;field:"Seller agreement";theme:panel.theme;onEdited:(path,value)=>panel.seller=panel.patch(panel.seller,path,value)}
 CheckBox {id:sellerConsent;visible:panel.operator;text:"I checked the seller, account and agreement report";Layout.fillWidth:true}
 Button {visible:panel.operator;text:"Record approved seller";enabled:sellerConsent.checked&&!core.loading;onClicked:core.workspaceAction("command",{command:"commerce_seller",seller:panel.seller})}
}
