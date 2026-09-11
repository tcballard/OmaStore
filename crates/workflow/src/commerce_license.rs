//! Issued grants are verified offline against independently pinned issuer keys.
use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Grant {
    pub version: u8,
    pub order_id: String,
    pub app_id: String,
    pub buyer_reference: String,
    pub seller_id: String,
    pub kind: String,
    pub licence: String,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
    pub recovery_url: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Envelope {
    pub key_id: String,
    pub payload: String,
    pub signature: String,
}
pub struct Issuer {
    id: String,
    pair: Ed25519KeyPair,
}
impl Issuer {
    pub fn from_seed(id: &str, seed: &[u8]) -> Result<Self> {
        if !omastore_catalogue::token(id) {
            return Err(Error::new(422, "invalid_issuer_identity"));
        }
        Ok(Self {
            id: id.into(),
            pair: Ed25519KeyPair::from_seed_unchecked(seed)
                .map_err(|_| Error::new(503, "invalid_signing_key"))?,
        })
    }
    pub fn public_key(&self) -> Vec<u8> {
        self.pair.public_key().as_ref().to_vec()
    }
    pub fn key_id(&self) -> &str {
        &self.id
    }
    pub fn sign(&self, grant: &Grant) -> Result<Envelope> {
        validate(grant)?;
        let body = serde_json::to_vec(grant)?;
        Ok(Envelope {
            key_id: self.id.clone(),
            payload: STANDARD.encode(&body),
            signature: STANDARD.encode(self.pair.sign(&body).as_ref()),
        })
    }
}
pub fn verify(envelope: &Envelope, anchors: &[(&str, &[u8])], now: i64) -> Result<Grant> {
    if envelope.payload.len() > 24 * 1024 || envelope.signature.len() > 128 {
        return Err(Error::new(422, "invalid_licence"));
    }
    let key = anchors
        .iter()
        .find(|(id, _)| *id == envelope.key_id)
        .map(|(_, k)| *k)
        .ok_or(Error::new(422, "untrusted_licence_issuer"))?;
    let body = STANDARD
        .decode(&envelope.payload)
        .map_err(|_| Error::new(422, "invalid_licence"))?;
    let sig = STANDARD
        .decode(&envelope.signature)
        .map_err(|_| Error::new(422, "invalid_licence"))?;
    UnparsedPublicKey::new(&ED25519, key)
        .verify(&body, &sig)
        .map_err(|_| Error::new(422, "invalid_licence_signature"))?;
    let grant: Grant = serde_json::from_slice(&body)?;
    validate(&grant)?;
    if grant.issued_at > now + 300 || grant.expires_at.is_some_and(|n| n <= now) {
        return Err(Error::new(422, "licence_expired_or_future"));
    }
    Ok(grant)
}
fn validate(g: &Grant) -> Result<()> {
    if g.version != 1
        || g.issued_at <= 0
        || !omastore_catalogue::token(&g.app_id)
        || !omastore_catalogue::token(&g.seller_id)
        || g.expires_at.is_some_and(|n| n <= g.issued_at)
    {
        return Err(Error::new(422, "invalid_licence"));
    }
    for v in [&g.order_id, &g.buyer_reference] {
        crate::bounded(v, 128)?;
    }
    crate::bounded(&g.licence, 1000)?;
    if ![
        "perpetual",
        "paid_upgrade",
        "paid_feature",
        "service",
        "subscription",
        "support",
    ]
    .contains(&g.kind.as_str())
        || (["perpetual", "paid_upgrade", "paid_feature"].contains(&g.kind.as_str())
            && g.expires_at.is_some())
    {
        return Err(Error::new(422, "invalid_licence"));
    }
    if let Some(url) = &g.recovery_url {
        crate::net::public_url(url)?;
    }
    Ok(())
}
#[cfg(feature = "development-workflow")]
pub fn sample_issuer() -> Result<Issuer> {
    Issuer::from_seed("development-fixture", &[42; 32])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perpetual_recovery_uses_pinned_key_and_rejects_forgery_expiry_and_extra_fields() {
        let issuer = Issuer::from_seed("test-pinned", &[19; 32]).unwrap();
        let key = issuer.public_key();
        let anchors = [("test-pinned", key.as_slice())];
        let g = Grant {
            version: 1,
            order_id: "order-one".into(),
            app_id: "app-one".into(),
            buyer_reference: "opaque-buyer".into(),
            seller_id: "seller-one".into(),
            kind: "perpetual".into(),
            licence: "Personal perpetual licence; OSS rights remain independent".into(),
            issued_at: 1000,
            expires_at: None,
            recovery_url: None,
        };
        let e = issuer.sign(&g).unwrap();
        assert_eq!(verify(&e, &anchors, 1000 + 86400 * 365 * 50).unwrap(), g);
        assert!(verify(&e, &[], 2000).is_err());
        let mut fake = e.clone();
        fake.payload = STANDARD.encode(b"{}");
        assert!(verify(&fake, &anchors, 2000).is_err());
        let attacker = Issuer::from_seed("test-pinned", &[20; 32]).unwrap();
        assert!(verify(&attacker.sign(&g).unwrap(), &anchors, 2000).is_err());
        let mut v = serde_json::to_value(&e).unwrap();
        v["publicKey"] = serde_json::json!(STANDARD.encode(attacker.public_key()));
        assert!(serde_json::from_value::<Envelope>(v).is_err());
        let mut service = g;
        service.kind = "subscription".into();
        service.expires_at = Some(2000);
        assert!(verify(&issuer.sign(&service).unwrap(), &anchors, 2000).is_err());
    }
}
