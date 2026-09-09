//! Frozen purchases and durable provider reconciliation. Browser returns grant no authority.
use crate::{
    commerce_license::{Envelope, Grant, Issuer},
    commerce_model::{provider_id, Price},
    commerce_provider::{CheckoutIntent, CheckoutObservation, Provider},
    digest, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Seller {
    pub id: String,
    pub owner: String,
    pub account: String,
    pub name: String,
    pub active: bool,
    pub report_url: String,
    pub report_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Purchase {
    pub price_id: String,
    pub version: u32,
    pub digest: String,
    pub accepted: bool,
}
pub(crate) fn signed_in(c: &Connection, a: &Actor) -> Result<()> {
    if c.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND active=1)",
        [&a.id],
        |r| r.get::<_, bool>(0),
    )? {
        Ok(())
    } else {
        Err(Error::new(403, "account_unavailable"))
    }
}
pub(crate) fn provider_guard(c: &Connection, mode: &str) -> Result<()> {
    let env: String = c.query_row(
        "SELECT value FROM metadata WHERE key='environment'",
        [],
        |r| r.get(0),
    )?;
    let actual: Option<String> = c
        .query_row(
            "SELECT value FROM metadata WHERE key='commerce_provider'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if actual.as_deref() != Some(mode) {
        return Err(Error::new(503, "commerce_database_provider_mismatch"));
    }
    if env != "development"
        || !cfg!(feature = "development-workflow")
        || !["sample", "stripe_test"].contains(&mode)
    {
        return Err(Error::new(503, "managed_checkout_disabled"));
    }
    Ok(())
}
pub(crate) fn event(
    t: &Transaction<'_>,
    order: &str,
    kind: &str,
    detail: Value,
    now: i64,
) -> Result<()> {
    t.execute(
        "INSERT INTO commerce_events(order_id,kind,detail,at) VALUES(?1,?2,?3,?4)",
        params![order, kind, detail.to_string(), now],
    )?;
    Ok(())
}
pub(crate) fn register_seller(
    t: &Transaction<'_>,
    a: &Actor,
    s: Seller,
    now: i64,
) -> Result<Value> {
    recheck(t, a, "operator")?;
    if !omastore_catalogue::token(&s.id)
        || !provider_id(&s.account, "acct_")
        || s.report_digest.len() != 64
        || !s.report_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::new(422, "invalid_seller_record"));
    }
    crate::bounded(&s.name, 1000)?;
    crate::net::public_url(&s.report_url)?;
    let active: bool = t.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id=?1 AND active=1)",
        [&s.owner],
        |r| r.get(0),
    )?;
    if !active {
        return Err(Error::new(422, "seller_owner_unavailable"));
    }
    let old: Option<(String, String)> = t
        .query_row(
            "SELECT owner,account FROM commerce_sellers WHERE id=?1",
            [&s.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if old.is_some_and(|(owner, account)| owner != s.owner || account != s.account) {
        return Err(Error::new(409, "seller_identity_frozen"));
    }
    t.execute("INSERT INTO commerce_sellers VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET owner=excluded.owner,account=excluded.account,name=excluded.name,active=excluded.active,report_url=excluded.report_url,report_digest=excluded.report_digest",params![s.id,s.owner,s.account,s.name,s.active,s.report_url,s.report_digest])?;
    audit(
        t,
        &a.id,
        "commerce_seller_recorded",
        &s.id,
        now,
        &json!({"owner":s.owner,"active":s.active,"reportUrl":s.report_url,"reportDigest":s.report_digest}),
    )?;
    Ok(json!({"recorded":true,"id":s.id}))
}
fn seller_matches(c: &Connection, p: &Price, owner: Option<&str>) -> Result<()> {
    let row: Option<(String, String, String, bool)> = c
        .query_row(
            "SELECT owner,account,name,active FROM commerce_sellers WHERE id=?1",
            [&p.seller_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    if row.is_none_or(|(o, acct, name, active)| {
        !active
            || acct != p.connected_account
            || name != p.seller_name
            || owner.is_some_and(|a| a != o)
    }) {
        return Err(Error::new(409, "approved_seller_required"));
    }
    Ok(())
}
fn available_app(c: &Connection, id: &str) -> Result<()> {
    let body: Option<String> = c
        .query_row(
            "SELECT value FROM metadata WHERE key='delivered_catalogue'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let catalogue: omastore_catalogue::Catalogue =
        serde_json::from_str(&body.ok_or(Error::new(409, "published_app_required"))?)?;
    if !catalogue.apps.iter().any(|a| a.id == id) {
        return Err(Error::new(409, "published_app_required"));
    }
    let held: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM distribution_holds WHERE app_id=?1 AND state='open')",
        [id],
        |r| r.get(0),
    )?;
    if held {
        return Err(Error::new(409, "app_distribution_held"));
    }
    Ok(())
}
pub(crate) fn publish_price(t: &Transaction<'_>, a: &Actor, p: Price, now: i64) -> Result<Value> {
    recheck(t, a, "author")?;
    p.amounts()?;
    seller_matches(t, &p, Some(&a.id))?;
    available_app(t, &p.app_id)?;
    let owner: bool = t.query_row(
        "SELECT EXISTS(SELECT 1 FROM entity_owners WHERE kind='app' AND entity_id=?1 AND owner=?2)",
        params![p.app_id, a.id],
        |r| r.get(0),
    )?;
    if !owner {
        return Err(Error::new(403, "listing_steward_required"));
    }
    let last: Option<(u32, String)> = t
        .query_row(
            "SELECT version,body FROM commerce_prices WHERE id=?1 ORDER BY version DESC LIMIT 1",
            [&p.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((v, b)) = last {
        let old: Price = serde_json::from_str(&b)?;
        if p.version != v + 1 || p.app_id != old.app_id || p.seller_id != old.seller_id {
            return Err(Error::new(409, "immutable_price_identity"));
        }
    } else if p.version != 1 {
        return Err(Error::new(422, "first_price_version_required"));
    }
    let public = p.public()?;
    let sha = digest(serde_json::to_vec(&public)?);
    t.execute(
        "INSERT INTO commerce_prices VALUES(?1,?2,?3,?4,?5,?6)",
        params![p.id, p.version, serde_json::to_string(&p)?, sha, a.id, now],
    )?;
    t.execute("INSERT INTO commerce_offers VALUES(?1,?2,1) ON CONFLICT(id) DO UPDATE SET version=excluded.version,active=1",params![p.id,p.version])?;
    audit(
        t,
        &a.id,
        "commerce_price_published",
        &p.id,
        now,
        &json!({"version":p.version,"digest":sha}),
    )?;
    Ok(json!({"price":public,"digest":sha}))
}
pub(crate) fn withdraw_offer(t: &Transaction<'_>, a: &Actor, id: &str, now: i64) -> Result<Value> {
    recheck(t, a, "author")?;
    let (p, _) = price(t, id)?;
    seller_matches(t, &p, Some(&a.id))?;
    t.execute("UPDATE commerce_offers SET active=0 WHERE id=?1", [id])?;
    audit(
        t,
        &a.id,
        "commerce_offer_withdrawn",
        id,
        now,
        &json!({"receiptsRetained":true}),
    )?;
    Ok(json!({"withdrawn":true}))
}
fn price(c: &Connection, id: &str) -> Result<(Price, String)> {
    let (body,sha):(String,String)=c.query_row("SELECT p.body,p.digest FROM commerce_prices p JOIN commerce_offers o ON o.id=p.id AND o.version=p.version WHERE p.id=?1 AND o.active=1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(Error::new(404,"commerce_offer_unavailable"))?;
    Ok((serde_json::from_str(&body)?, sha))
}
pub(crate) fn intent(c: &Connection, id: &str) -> Result<(CheckoutIntent, String)> {
    let (buyer, body, at, mode): (String, String, i64, String) = c
        .query_row(
            "SELECT buyer_reference,price,created_at,mode FROM commerce_orders WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or(Error::new(404, "order_unavailable"))?;
    Ok((
        CheckoutIntent {
            order_id: id.into(),
            buyer_reference: buyer,
            price: serde_json::from_str(&body)?,
            created_at: at,
        },
        mode,
    ))
}
pub(crate) fn access(c: &Connection, a: &Actor, id: &str) -> Result<()> {
    signed_in(c, a)?;
    let buyer: Option<String> = c
        .query_row("SELECT buyer FROM commerce_orders WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    if buyer.as_deref() != Some(&a.id) && recheck(c, a, "operator").is_err() {
        return Err(Error::new(404, "order_unavailable"));
    }
    Ok(())
}
impl Store {
    pub fn commerce_prices(&self, app_id: Option<&str>) -> Result<Value> {
        let c = self.connection()?;
        let mut q=c.prepare("SELECT p.id FROM commerce_prices p JOIN commerce_offers o ON o.id=p.id AND o.version=p.version WHERE o.active=1 ORDER BY p.id LIMIT 100")?;
        let ids: Vec<String> = q
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let mut out = Vec::new();
        for id in ids {
            let (p, sha) = price(&c, &id)?;
            if app_id.is_none_or(|v| v == p.app_id)
                && seller_matches(&c, &p, None).is_ok()
                && available_app(&c, &p.app_id).is_ok()
            {
                out.push(json!({"price":p.public()?,"digest":sha}));
            }
        }
        Ok(json!({"items":out,"realCheckoutEnabled":false}))
    }
    pub fn commerce_begin(
        &self,
        a: &Actor,
        key: &str,
        p: Purchase,
        mode: &str,
        now: i64,
    ) -> Result<String> {
        if key.len() < 16
            || key.len() > 128
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::new(422, "invalid_idempotency_key"));
        }
        if !p.accepted {
            return Err(Error::new(422, "purchase_consent_required"));
        }
        let sha = digest(serde_json::to_vec(&p)?);
        self.transaction(|t|{signed_in(t,a)?;provider_guard(t,mode)?;
   if let Some((id,old))=t.query_row("SELECT id,request_digest FROM commerce_orders WHERE buyer=?1 AND request_key=?2",params![a.id,key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?{if old!=sha{return Err(Error::new(409,"idempotency_key_conflict"));}return Ok(id);}
   let paused:bool=t.query_row("SELECT value='1' FROM metadata WHERE key='commerce_paused'",[],|r|r.get(0))?;if paused{return Err(Error::new(503,"new_purchases_paused"));}
   let (pr,expected)=price(t,&p.price_id)?;if expected!=p.digest||pr.version!=p.version{return Err(Error::new(409,"price_preview_changed"));}seller_matches(t,&pr,None)?;available_app(t,&pr.app_id)?;
   let recent:i64=t.query_row("SELECT count(*) FROM commerce_orders WHERE buyer=?1 AND created_at>?2",params![a.id,now-86400],|r|r.get(0))?;if recent>=100{return Err(Error::new(429,"purchase_limit_reached"));}
   let id=nonce()?;let buyer=nonce()?;t.execute("INSERT INTO commerce_orders(id,buyer,buyer_reference,request_key,request_digest,mode,price,price_digest,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,a.id,buyer,key,sha,mode,serde_json::to_string(&pr)?,expected,now])?;
   event(t,&id,"created",json!({"priceDigest":expected}),now)?;Ok(id)
  })
    }
    pub async fn commerce_checkout(
        &self,
        a: &Actor,
        id: &str,
        provider: &dyn Provider,
        now: i64,
    ) -> Result<Value> {
        {
            let c = self.connection()?;
            access(&c, a, id)?;
        }
        let claim=self.transaction(|t|{let (i,mode)=intent(t,id)?;provider_guard(t,&mode)?;if provider.mode()!=mode{return Err(Error::new(503,"payment_provider_mode_mismatch"));}
   let (state,lease,at):(String,i64,Option<i64>)=t.query_row("SELECT creation_state,lease_until,attempt_at FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
   if state=="created"{return Ok(None);}let paused:bool=t.query_row("SELECT value='1' FROM metadata WHERE key='commerce_paused'",[],|r|r.get(0))?;if paused&&at.is_none(){return Err(Error::new(503,"new_purchases_paused"));}
if lease>now{return Err(Error::new(409,"checkout_in_progress"));}
   t.execute("UPDATE commerce_orders SET creation_state='running',lease_until=?2,attempt_at=COALESCE(attempt_at,?3) WHERE id=?1",params![id,now+60,now])?;
   event(t,id,"checkout_attempt",json!({"reconciliation":at.is_some()}),now)?;Ok(Some((i,at)))
  })?;
        if let Some((i, at)) = claim {
            let result = async {
                if at.is_some() {
                    if let Some(v) = provider.reconcile_creation(&i).await? {
                        return Ok(v);
                    }
                }
                if at.is_some_and(|n| now - n >= 23 * 3600) {
                    return Err(Error::new(409, "checkout_requires_provider_reconciliation"));
                }
                {
                    let c = self.connection()?;
                    let paused: bool = c.query_row(
                        "SELECT value='1' FROM metadata WHERE key='commerce_paused'",
                        [],
                        |r| r.get(0),
                    )?;
                    if paused {
                        return Err(Error::new(503, "new_purchases_paused"));
                    }
                }
                provider
                    .checkout(&i, &format!("checkout-{}", i.order_id))
                    .await
            }
            .await;
            match result {
                Ok(v) => {
                    if let Err(e) = self.commerce_observe(provider.mode(), &v, now) {
                        self.checkout_failed(id, e.code, now)?;
                        return Err(e);
                    }
                }
                Err(e) => {
                    self.checkout_failed(id, e.code, now)?;
                    return Err(e);
                }
            }
        }
        self.commerce_order(a, id)
    }
    fn checkout_failed(&self, id: &str, code: &str, now: i64) -> Result<()> {
        self.transaction(|t|{t.execute("UPDATE commerce_orders SET creation_state=CASE WHEN creation_state='created' THEN creation_state ELSE 'unknown' END,lease_until=0 WHERE id=?1",[id])?;event(t,id,"checkout_needs_reconciliation",json!({"code":code}),now)})
    }
    pub fn commerce_observe(&self, mode: &str, o: &CheckoutObservation, now: i64) -> Result<()> {
        self.transaction(|t|{provider_guard(t,mode)?;let (i,storedmode)=intent(t,&o.order_id)?;let a=i.price.amounts()?;
   if storedmode!=mode||o.livemode||o.account!=i.price.connected_account||o.buyer_reference!=i.buyer_reference||o.currency!=i.price.currency||json!(o.total)!=a["total"]||o.tax!=i.price.tax||!provider_id(&o.session_id,"cs_test_"){return Err(Error::new(409,"payment_order_mismatch"));}
   if o.paid&&(!o.payment_id.as_ref().is_some_and(|s|provider_id(s,"ch_"))||o.fee_amount!=a["serviceFee"].as_u64()||(a["serviceFee"]!=0&&!o.fee_id.as_ref().is_some_and(|s|provider_id(s,"fee_")))||!o.customer_id.as_ref().is_some_and(|s|provider_id(s,"cus_"))){return Err(Error::new(409,"payment_fee_or_identity_mismatch"));}
   if o.paid&&i.price.delivery_kind=="subscription"&&(!o.subscription_id.as_ref().is_some_and(|s|provider_id(s,"sub_"))||o.paid_until.is_none_or(|n|n<=i.created_at)){return Err(Error::new(409,"subscription_period_unconfirmed"));}
   let (session,payment,state):(Option<String>,Option<String>,String)=t.query_row("SELECT session_id,payment_id,payment_state FROM commerce_orders WHERE id=?1",[&o.order_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
   if session.as_ref().is_some_and(|s|s!=&o.session_id)||(o.paid&&payment.is_some()&&payment!=o.payment_id){return Err(Error::new(409,"payment_identity_changed"));}
   if let Some(url)=&o.url {if mode!="sample"{crate::commerce_stripe::checkout_url(url)?;}}
   if state=="paid" {return Ok(());} // Out-of-order unpaid observations cannot undo payment/delivery.
   t.execute("UPDATE commerce_orders SET session_id=?2,creation_state='created',lease_until=0,checkout_url=?3,provider_observation=?4,payment_state=?5,payment_id=?6,fee_id=?7,subscription_id=?8,customer_id=?9 WHERE id=?1",params![o.order_id,o.session_id,o.url,serde_json::to_string(o)?,if o.paid{"paid"}else{"payment_pending"},o.payment_id,o.fee_id,o.subscription_id,o.customer_id])?;
   event(t,&o.order_id,if o.paid{"paid"}else{"payment_pending"},json!({"provider":mode}),now)?;
   if o.paid{crate::commerce_accounting::sale(t,&o.order_id,now)?;t.execute("UPDATE commerce_orders SET delivery_state='delivery_pending' WHERE id=?1 AND delivery_state='not_started'",[&o.order_id])?;event(t,&o.order_id,"delivery_pending",json!({}),now)?;}Ok(())
  })
    }
    pub async fn commerce_reconcile(
        &self,
        a: &Actor,
        id: &str,
        provider: &dyn Provider,
        now: i64,
    ) -> Result<Value> {
        let (i, mode, session) = {
            let c = self.connection()?;
            access(&c, a, id)?;
            let (i, m) = intent(&c, id)?;
            let s: Option<String> = c.query_row(
                "SELECT session_id FROM commerce_orders WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            (i, m, s)
        };
        if mode != provider.mode() {
            return Err(Error::new(503, "payment_provider_mode_mismatch"));
        }
        if let Some(session) = session {
            let o = provider
                .lookup(&i.price.connected_account, &session)
                .await?;
            if o.order_id != id {
                return Err(Error::new(409, "payment_order_mismatch"));
            }
            self.commerce_observe(&mode, &o, now)?;
            self.commerce_order(a, id)
        } else {
            self.commerce_checkout(a, id, provider, now).await
        }
    }
    pub fn commerce_order(&self, a: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        access(&c, a, id)?;
        order_projection(&c, id, true)
    }
    pub fn commerce_orders(&self, a: &Actor, before: i64) -> Result<Value> {
        let c = self.connection()?;
        signed_in(&c, a)?;
        let mut q=c.prepare("SELECT rowid,id FROM commerce_orders WHERE buyer=?1 AND (?2=0 OR rowid<?2) ORDER BY rowid DESC LIMIT 30")?;
        let rows: Vec<(i64, String)> = q
            .query_map(params![a.id, before], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        let items: Vec<Value> = rows
            .iter()
            .map(|(_, id)| order_projection(&c, id, false))
            .collect::<Result<_>>()?;
        Ok(json!({"items":items,"nextCursor":rows.last().map(|v|v.0)}))
    }
    pub fn commerce_support(&self, a: &Actor, id: &str) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, a, "operator")?;
        let mut out = order_projection(&c, id, false)?;
        let mut q=c.prepare("SELECT seq,kind,detail,at FROM commerce_events WHERE order_id=?1 ORDER BY seq DESC LIMIT 100")?;
        let events:Vec<Value>=q.query_map([id],|r|{let detail:String=r.get(2)?;Ok(json!({"seq":r.get::<_,i64>(0)?,"kind":r.get::<_,String>(1)?,"detail":serde_json::from_str::<Value>(&detail).unwrap_or(Value::Null),"at":r.get::<_,i64>(3)?}))})?.collect::<std::result::Result<_,_>>()?;
        out["events"] = json!(events);
        Ok(out)
    }
    pub fn commerce_deliver_local(
        &self,
        id: &str,
        issuer: Option<&Issuer>,
        now: i64,
    ) -> Result<bool> {
        self.transaction(|t|{let(i,mode)=intent(t,id)?;provider_guard(t,&mode)?;
   let (paid,state,obs):(String,String,Option<String>)=t.query_row("SELECT payment_state,delivery_state,provider_observation FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
   if state=="delivered"{return Ok(false);}
crate::commerce_lifecycle::delivery_allowed(t,id,now)?;
if paid!="paid"{return Err(Error::new(409,"confirmed_payment_required"));}
   if ["service","subscription"].contains(&i.price.delivery_kind.as_str()){return Err(Error::new(409,"service_fulfilment_required"));}
   t.execute("UPDATE commerce_orders SET delivery_attempts=delivery_attempts+1 WHERE id=?1",[id])?;
   let Some(issuer)=issuer else{t.execute("UPDATE commerce_orders SET delivery_state='delivery_failed',delivery_error='licence_issuer_unconfigured' WHERE id=?1",[id])?;event(t,id,"delivery_failed",json!({"code":"licence_issuer_unconfigured"}),now)?;return Ok(false);};
   let _=obs;let grant=Grant{version:1,order_id:id.into(),app_id:i.price.app_id,buyer_reference:i.buyer_reference,seller_id:i.price.seller_id,kind:i.price.delivery_kind,licence:i.price.licence,issued_at:now,expires_at:None,recovery_url:None};let envelope=issuer.sign(&grant)?;
   t.execute("UPDATE commerce_orders SET delivery_state='delivered',delivery_error=NULL,grant=?2 WHERE id=?1",params![id,serde_json::to_string(&envelope)?])?;event(t,id,"delivered",json!({"grantDigest":digest(serde_json::to_vec(&envelope)?)}),now)?;Ok(true)
  })
    }
    pub fn commerce_retry_delivery(&self, a: &Actor, id: &str, now: i64) -> Result<Value> {
        self.transaction(|t|{access(t,a,id)?;crate::commerce_lifecycle::delivery_allowed(t,id,now)?;let state:String=t.query_row("SELECT delivery_state FROM commerce_orders WHERE id=?1",[id],|r|r.get(0))?;if state=="delivery_failed"{t.execute("UPDATE commerce_orders SET delivery_state='delivery_pending',delivery_error=NULL WHERE id=?1",[id])?;event(t,id,"delivery_retry_requested",json!({"actor":a.id}),now)?;}Ok(())})?;
        self.commerce_order(a, id)
    }
    pub fn commerce_pending_delivery(&self) -> Result<Vec<String>> {
        let c = self.connection()?;
        let mut q=c.prepare("SELECT id FROM commerce_orders WHERE payment_state='paid' AND delivery_state='delivery_pending' AND NOT EXISTS(SELECT 1 FROM commerce_cycles WHERE cycle_order=commerce_orders.id AND period_start>unixepoch()) ORDER BY created_at LIMIT 20")?;
        let ids = q
            .query_map([], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(ids)
    }
}
pub(crate) fn order_projection(c: &Connection, id: &str, grant: bool) -> Result<Value> {
    let (i, mode) = intent(c, id)?;
    let (payment,delivery,code,raw,url):(String,String,Option<String>,Option<String>,Option<String>)=c.query_row("SELECT payment_state,delivery_state,delivery_error,grant,checkout_url FROM commerce_orders WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    let mut v = json!({"id":id,"mode":mode,"createdAt":i.created_at,"price":i.price.public()?,"paymentState":payment,"deliveryState":delivery,"deliveryError":code,"checkoutUrl":if payment=="paid"{None}else{url},"receiptAvailable":payment=="paid","openSourceRightsIndependent":true});
    let refunded:i64=c.query_row("SELECT COALESCE(SUM(amount),0) FROM commerce_refunds WHERE order_id=?1 AND state='succeeded'",[id],|r|r.get(0))?;
    let held: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM distribution_holds WHERE app_id=?1 AND state='open')",
        [&i.price.app_id],
        |r| r.get(0),
    )?;
    v["distributionHeld"] = json!(held);
    v["refunded"] = json!(refunded);
    v["fullyRefunded"] = json!(refunded as u64 >= i.price.amounts()?["total"].as_u64().unwrap());
    if i.price.delivery_kind == "subscription" && payment == "paid" {
        v["subscription"] = crate::commerce_subscriptions::summary(c, id, crate::now())?;
    }
    if grant {
        v["grant"] = match raw {
            Some(s) => json!(serde_json::from_str::<Envelope>(&s)?),
            None => Value::Null,
        };
    }
    Ok(v)
}

impl Store {
    /// The worker only observes existing provider effects; it never starts a purchase.
    pub fn commerce_payment_queue(&self, mode: &str, now: i64) -> Result<Vec<String>> {
        self.transaction(|t|{provider_guard(t,mode)?;let mut q=t.prepare("SELECT id FROM commerce_orders WHERE mode=?1 AND payment_state!='paid' AND attempt_at IS NOT NULL AND last_reconciled_at<?2 ORDER BY last_reconciled_at,created_at LIMIT 20")?;let ids:Vec<String>=q.query_map(params![mode,now-300],|r|r.get(0))?.collect::<std::result::Result<_,_>>()?;for id in &ids{t.execute("UPDATE commerce_orders SET last_reconciled_at=?2 WHERE id=?1",params![id,now])?;}Ok(ids)})
    }
    pub async fn commerce_poll_payment(
        &self,
        id: &str,
        provider: &dyn Provider,
        now: i64,
    ) -> Result<()> {
        let (i, mode, session) = {
            let c = self.connection()?;
            let (i, m) = intent(&c, id)?;
            provider_guard(&c, &m)?;
            let s: Option<String> = c.query_row(
                "SELECT session_id FROM commerce_orders WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            (i, m, s)
        };
        if mode != provider.mode() {
            return Err(Error::new(503, "payment_provider_mode_mismatch"));
        }
        let observation = match session {
            Some(s) => Some(provider.lookup(&i.price.connected_account, &s).await?),
            None => provider.reconcile_creation(&i).await?,
        };
        if let Some(o) = observation {
            if o.order_id != id {
                return Err(Error::new(409, "payment_order_mismatch"));
            }
            self.commerce_observe(&mode, &o, now)?;
        }
        Ok(())
    }
}

impl Store {
    pub fn commerce_author(&self, a: &Actor) -> Result<Value> {
        let c = self.connection()?;
        recheck(&c, a, "author")?;
        let mut q=c.prepare("SELECT id,account,name FROM commerce_sellers WHERE owner=?1 AND active=1 ORDER BY id LIMIT 100")?;
        let sellers:Vec<Value>=q.query_map([&a.id],|r|Ok(json!({"id":r.get::<_,String>(0)?,"account":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?})))?.collect::<std::result::Result<_,_>>()?;
        let mut q=c.prepare("SELECT entity_id FROM entity_owners WHERE owner=?1 AND kind='app' ORDER BY entity_id LIMIT 100")?;
        let apps: Vec<String> = q
            .query_map([&a.id], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        let mut q=c.prepare("SELECT p.body,o.active FROM commerce_prices p JOIN commerce_offers o ON o.id=p.id AND o.version=p.version JOIN commerce_sellers s ON s.id=json_extract(p.body,'$.sellerId') WHERE s.owner=?1 ORDER BY p.id LIMIT 100")?;
        let offers:Vec<Value>=q.query_map([&a.id],|r|{let b:String=r.get(0)?;Ok(json!({"price":serde_json::from_str::<Value>(&b).unwrap_or(Value::Null),"active":r.get::<_,bool>(1)?}))})?.collect::<std::result::Result<_,_>>()?;
        Ok(
            json!({"sellers":sellers,"apps":apps,"offers":offers,"notice":"Only a current listing steward with an approved commercial seller can publish a price. Commercial actions cannot approve or rank an app."}),
        )
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod tests {
    use super::*;
    use crate::commerce_sample::Sample;
    use hmac::{Hmac, Mac};
    fn setup() -> (tempfile::TempDir, Store, Actor, Actor) {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("commerce.db")).unwrap();
        let c: omastore_catalogue::Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&c).unwrap();
        s.commerce_seed_sample(1000).unwrap();
        let op = s.development_login("operator", 1000).unwrap();
        let op = s.actor(op["token"].as_str().unwrap(), 1000).unwrap();
        let a = s.development_login("author", 1000).unwrap();
        let a = s.actor(a["token"].as_str().unwrap(), 1000).unwrap();
        s.transaction(|t| crate::commerce::pause(t, &op, false, "Fixture rehearsal", 1000))
            .unwrap();
        (dir, s, a, op)
    }
    fn purchase(s: &Store) -> Purchase {
        let v = s.commerce_prices(None).unwrap();
        let p = v["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["price"]["id"] == "sample-perpetual")
            .unwrap();
        Purchase {
            price_id: p["price"]["id"].as_str().unwrap().into(),
            version: 1,
            digest: p["digest"].as_str().unwrap().into(),
            accepted: true,
        }
    }
    fn signed(secret: &str, body: &[u8], at: i64) -> String {
        let mut m = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        m.update(format!("{at}.").as_bytes());
        m.update(body);
        format!(
            "t={at},v1={}",
            m.finalize()
                .into_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    }
    #[tokio::test]
    async fn lost_checkout_reply_reconciles_after_restart_and_delivers_one_recoverable_grant() {
        let (dir, s, a, op) = setup();
        let key = nonce().unwrap();
        let p = purchase(&s);
        let id = s
            .commerce_begin(&a, &key, p.clone(), "sample", 1001)
            .unwrap();
        let i = intent(&s.connection().unwrap(), &id).unwrap().0;
        let provider = Sample(s.clone());
        // The provider effect happened; process died before storing its reply.
        provider
            .checkout(&i, &format!("checkout-{id}"))
            .await
            .unwrap();
        s.connection()
            .unwrap()
            .execute(
                "UPDATE commerce_orders SET creation_state='unknown',attempt_at=1002 WHERE id=?1",
                [&id],
            )
            .unwrap();
        let restarted = Store::development(&dir.path().join("commerce.db")).unwrap();
        let provider = Sample(restarted.clone());
        let pending = restarted
            .commerce_checkout(&a, &id, &provider, 1003)
            .await
            .unwrap();
        assert_eq!(pending["paymentState"], "payment_pending");
        assert_eq!(
            restarted
                .commerce_begin(&a, &key, p, "sample", 1004)
                .unwrap(),
            id
        );
        provider.capture(&a, &id, 1005).unwrap();
        restarted
            .commerce_reconcile(&a, &id, &provider, 1006)
            .await
            .unwrap();
        assert!(!restarted.commerce_deliver_local(&id, None, 1007).unwrap());
        assert_eq!(
            restarted.commerce_order(&a, &id).unwrap()["deliveryState"],
            "delivery_failed"
        );
        restarted.commerce_retry_delivery(&a, &id, 1008).unwrap();
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        assert!(restarted
            .commerce_deliver_local(&id, Some(&issuer), 1009)
            .unwrap());
        let receipt = restarted.commerce_order(&a, &id).unwrap();
        assert!(!restarted
            .commerce_deliver_local(&id, Some(&issuer), 1010)
            .unwrap());
        assert_eq!(
            restarted.commerce_order(&a, &id).unwrap()["grant"],
            receipt["grant"]
        );
        let e: Envelope = serde_json::from_value(receipt["grant"].clone()).unwrap();
        let public = issuer.public_key();
        assert_eq!(
            crate::commerce_license::verify(&e, &[(issuer.key_id(), &public)], 1000000000)
                .unwrap()
                .order_id,
            id
        );
        restarted
            .transaction(|t| crate::commerce::pause(t, &op, true, "Stop new purchases", 1011))
            .unwrap();
        restarted
            .connection()
            .unwrap()
            .execute("UPDATE commerce_offers SET active=0", [])
            .unwrap();
        assert!(
            restarted.commerce_order(&a, &id).unwrap()["receiptAvailable"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            restarted.commerce_orders(&a, 0).unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(restarted
            .connection()
            .unwrap()
            .execute("DELETE FROM commerce_orders", [])
            .is_err());
        assert!(restarted
            .connection()
            .unwrap()
            .execute("UPDATE commerce_orders SET price='{}'", [])
            .is_err());
        assert!(restarted
            .connection()
            .unwrap()
            .execute("UPDATE commerce_prices SET body='{}'", [])
            .is_err());
        let other = restarted.development_login("reviewer", 1012).unwrap();
        let other = restarted
            .actor(other["token"].as_str().unwrap(), 1012)
            .unwrap();
        assert!(restarted.commerce_order(&other, &id).is_err());
        assert!(restarted.commerce_support(&other, &id).is_err());
        assert!(!restarted.commerce_support(&op, &id).unwrap()["events"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    #[tokio::test]
    async fn verified_duplicate_delayed_and_forged_webhooks_converge_without_double_delivery() {
        let (_, s, a, _) = setup();
        let p = purchase(&s);
        let id = s
            .commerce_begin(&a, &nonce().unwrap(), p, "sample", 1001)
            .unwrap();
        let provider = Sample(s.clone());
        s.commerce_checkout(&a, &id, &provider, 1002).await.unwrap();
        let i = intent(&s.connection().unwrap(), &id).unwrap().0;
        let pending = provider.reconcile_creation(&i).await.unwrap().unwrap();
        provider.capture(&a, &id, 1003).unwrap();
        let paid = provider
            .lookup(&pending.account, &pending.session_id)
            .await
            .unwrap();
        let mut bad = paid.clone();
        bad.total += 1;
        assert!(s.commerce_observe("sample", &bad, 1004).is_err());
        bad = paid.clone();
        bad.fee_amount = Some(0);
        assert!(s.commerce_observe("sample", &bad, 1004).is_err());
        bad = paid.clone();
        bad.buyer_reference = "another-buyer".into();
        assert!(s.commerce_observe("sample", &bad, 1004).is_err());
        let secret = "fictional-webhook-contract-value";
        let body=serde_json::to_vec(&json!({"id":"evt_capture1","api_version":crate::commerce_model::STRIPE_VERSION,"account":paid.account,"livemode":false,"type":"checkout.session.completed","data":{"object":{"id":paid.session_id}}})).unwrap();
        let header = signed(secret, &body, 1004);
        assert!(s
            .commerce_webhook(&provider, secret, &header, b"{}", 1004)
            .await
            .is_err());
        assert_eq!(
            s.commerce_webhook(&provider, secret, &header, &body, 1004)
                .await
                .unwrap()["replayed"],
            false
        );
        assert_eq!(
            s.commerce_webhook(&provider, secret, &header, &body, 1004)
                .await
                .unwrap()["replayed"],
            true
        );
        s.commerce_observe("sample", &pending, 1005).unwrap();
        assert_eq!(s.commerce_order(&a, &id).unwrap()["paymentState"], "paid");
        let issuer = crate::commerce_license::sample_issuer().unwrap();
        s.commerce_deliver_local(&id, Some(&issuer), 1006).unwrap();
        s.commerce_webhook(&provider, secret, &header, &body, 1007)
            .await
            .unwrap();
        assert_eq!(
            s.commerce_order(&a, &id).unwrap()["deliveryState"],
            "delivered"
        );
        let count: i64 = s
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM commerce_events WHERE kind='delivered'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
    #[tokio::test]
    async fn expired_provider_idempotency_never_authorises_a_second_creation() {
        let (_, s, a, op) = setup();
        let p = purchase(&s);
        let key = nonce().unwrap();
        let id = s
            .commerce_begin(&a, &key, p.clone(), "sample", 1001)
            .unwrap();
        s.connection()
            .unwrap()
            .execute(
                "UPDATE commerce_orders SET creation_state='unknown',attempt_at=1001 WHERE id=?1",
                [&id],
            )
            .unwrap();
        let provider = Sample(s.clone());
        assert_eq!(
            s.commerce_checkout(&a, &id, &provider, 1001 + 86400)
                .await
                .unwrap_err()
                .code,
            "checkout_requires_provider_reconciliation"
        );
        let count: i64 = s
            .connection()
            .unwrap()
            .query_row("SELECT count(*) FROM commerce_sample_provider", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        s.transaction(|t| crate::commerce::pause(t, &op, true, "Paused", 1002))
            .unwrap();
        assert_eq!(
            s.commerce_begin(&a, &key, p.clone(), "sample", 1001 + 86400 * 90)
                .unwrap(),
            id
        );
        assert!(s
            .commerce_begin(&a, &nonce().unwrap(), p.clone(), "sample", 1003)
            .is_err());
        let mut bad = serde_json::to_value(&p).unwrap();
        bad["currency"] = json!("usd");
        assert!(serde_json::from_value::<Purchase>(bad).is_err());
        let mut bad = p;
        bad.version = 999;
        assert_eq!(
            s.commerce_begin(&a, &key, bad, "sample", 1003)
                .unwrap_err()
                .code,
            "idempotency_key_conflict"
        );
        assert!(s
            .commerce_begin(&a, &nonce().unwrap(), purchase(&s), "live", 1003)
            .is_err());
    }
    #[test]
    fn commercial_ownership_does_not_grant_listing_or_review_authority() {
        let (_, s, a, op) = setup();
        let (p, _) = price(&s.connection().unwrap(), "sample-perpetual").unwrap();
        let mut changed = p.clone();
        changed.id = "another-price".into();
        assert_eq!(
            s.transaction(|t| publish_price(t, &a, changed, 1001))
                .unwrap_err()
                .code,
            "listing_steward_required"
        );
        s.set_role(&op.id, "operator", false, 1002).unwrap();
        let seller = Seller {
            id: "x".into(),
            owner: a.id,
            account: "acct_another".into(),
            name: "Another".into(),
            active: true,
            report_url: "https://example.com/review".into(),
            report_digest: "a".repeat(64),
        };
        assert!(s
            .transaction(|t| register_seller(t, &op, seller, 1003))
            .is_err());
    }
}
