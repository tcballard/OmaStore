//! Commercial readiness records do not enable real charges.
use crate::{
    commerce_model::OperatingModel,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
impl Store {
    pub fn commerce_status(&self, actor: Option<&Actor>) -> Result<Value> {
        let c = self.connection()?;
        let private = if let Some(a) = actor {
            recheck(&c, a, "operator").is_ok()
        } else {
            false
        };
        let row:Option<(i64,String,String)>=c.query_row("SELECT version,body,report_digest FROM commerce_model ORDER BY version DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (version, model, sha) = match row {
            Some((v, b, s)) => (v, serde_json::from_str::<OperatingModel>(&b)?, s),
            None => (0, OperatingModel::default(), String::new()),
        };
        let paused: bool = c.query_row(
            "SELECT value='1' FROM metadata WHERE key='commerce_paused'",
            [],
            |r| r.get(0),
        )?;
        let environment: String = c.query_row(
            "SELECT value FROM metadata WHERE key='environment'",
            [],
            |r| r.get(0),
        )?;
        let mut v = model.readiness();
        v["newPurchasesPaused"] = json!(paused);
        v["environment"] = json!(environment);
        v["version"] = json!(version);
        if private {
            v["model"] = json!(model);
            v["reportDigest"] = json!(sha);
        }
        Ok(v)
    }
}
pub(crate) fn record_model(
    t: &Transaction<'_>,
    actor: &Actor,
    version: i64,
    model: OperatingModel,
    sha: &str,
    now: i64,
) -> Result<Value> {
    recheck(t, actor, "operator")?;
    model.validate()?;
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::new(422, "invalid_commerce_report_digest"));
    }
    let current: i64 = t.query_row(
        "SELECT COALESCE(MAX(version),0) FROM commerce_model",
        [],
        |r| r.get(0),
    )?;
    if current != version {
        return Err(Error::new(409, "stale_revision"));
    }
    t.execute(
        "INSERT INTO commerce_model VALUES(?1,?2,?3,?4,?5)",
        params![
            version + 1,
            serde_json::to_string(&model)?,
            sha,
            actor.id,
            now
        ],
    )?;
    audit(
        t,
        &actor.id,
        "commerce_model_recorded",
        "commercial-operation",
        now,
        &json!({"version":version+1,"reportDigest":sha}),
    )?;
    Ok(json!({"recorded":true,"version":version+1,"realCheckoutEnabled":false}))
}
pub(crate) fn pause(
    t: &Transaction<'_>,
    actor: &Actor,
    paused: bool,
    reason: &str,
    now: i64,
) -> Result<Value> {
    recheck(t, actor, "operator")?;
    crate::bounded(reason, 1000)?;
    t.execute(
        "UPDATE metadata SET value=?1 WHERE key='commerce_paused'",
        [if paused { "1" } else { "0" }],
    )?;
    audit(
        t,
        &actor.id,
        "commerce_purchase_pause",
        "commercial-operation",
        now,
        &json!({"paused":paused,"reason":reason}),
    )?;
    Ok(
        json!({"newPurchasesPaused":paused,"realCheckoutEnabled":false,"notice":"Receipt recovery and existing buyer obligations remain available. The commercial release gate remains closed."}),
    )
}

impl Store {
    pub fn commerce_bind_provider(&self, mode: &str) -> Result<()> {
        if !cfg!(feature = "development-workflow") || !["sample", "stripe_test"].contains(&mode) {
            return Err(Error::new(503, "managed_checkout_disabled"));
        }
        self.transaction(|t| {
            let env: String = t.query_row(
                "SELECT value FROM metadata WHERE key='environment'",
                [],
                |r| r.get(0),
            )?;
            if env != "development" {
                return Err(Error::new(503, "commerce_test_database_required"));
            }
            let current: Option<String> = t
                .query_row(
                    "SELECT value FROM metadata WHERE key='commerce_provider'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if current.as_deref().is_some_and(|m| m != mode) {
                return Err(Error::new(503, "commerce_database_provider_mismatch"));
            }
            let other: bool = t.query_row(
                "SELECT EXISTS(SELECT 1 FROM commerce_orders WHERE mode!=?1)",
                [mode],
                |r| r.get(0),
            )?;
            if other {
                return Err(Error::new(503, "commerce_database_provider_mismatch"));
            }
            t.execute(
                "INSERT OR IGNORE INTO metadata VALUES('commerce_provider',?1)",
                [mode],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operator_records_are_versioned_private_revocable_and_never_open_live_gate() {
        let s = Store::memory().unwrap();
        s.connection()
            .unwrap()
            .execute("INSERT INTO users VALUES('op','operator',1,1)", [])
            .unwrap();
        s.set_role("op", "operator", true, 1).unwrap();
        let a = Actor {
            id: "op".into(),
            login: "operator".into(),
            roles: vec!["operator".into()],
        };
        let m = OperatingModel {
            operator: Some("Private legal entity".into()),
            ..Default::default()
        };
        s.transaction(|t| record_model(t, &a, 0, m.clone(), &"a".repeat(64), 2))
            .unwrap();
        assert_eq!(
            s.commerce_status(Some(&a)).unwrap()["model"]["operator"],
            "Private legal entity"
        );
        assert!(s.commerce_status(None).unwrap().get("model").is_none());
        assert!(s
            .transaction(|t| record_model(t, &a, 0, m.clone(), &"a".repeat(64), 3))
            .is_err());
        s.transaction(|t| pause(t, &a, false, "Test mode preparation", 4))
            .unwrap();
        assert_eq!(
            s.commerce_status(Some(&a)).unwrap()["realCheckoutEnabled"],
            false
        );
        assert!(s
            .connection()
            .unwrap()
            .execute("DELETE FROM commerce_model", [])
            .is_err());
        s.set_role("op", "operator", false, 5).unwrap();
        assert!(s
            .transaction(|t| pause(t, &a, true, "Revoked role", 6))
            .is_err());
    }
}
