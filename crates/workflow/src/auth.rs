use crate::{
    bounded, digest, net, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
pub struct GithubOAuth {
    pub client_id: String,
    pub client_secret: String,
    pub callback: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    pub attempt_id: String,
    pub authorization_url: String,
    pub expires_at: i64,
}
pub struct Callback {
    pub attempt_id: String,
    pub verifier: String,
}
struct Identity {
    id: String,
    login: String,
}

impl GithubOAuth {
    pub fn authorization(&self, state: &str, verifier: &str) -> Result<String> {
        bounded(&self.client_id, 200)?;
        net::public_url(&self.callback)?;
        let mut u = Url::parse("https://github.com/login/oauth/authorize").expect("fixed URL");
        u.query_pairs_mut().extend_pairs([
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", self.callback.as_str()),
            ("state", state),
            ("scope", "read:user"),
            ("code_challenge", challenge(verifier).as_str()),
            ("code_challenge_method", "S256"),
        ]);
        Ok(u.into())
    }
    async fn identity(&self, code: &str, verifier: &str) -> Result<Identity> {
        bounded(code, 512)?;
        let u = net::public_url("https://github.com/login/oauth/access_token")?;
        let response = net::client_for(&u)
            .await?
            .post(u)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("redirect_uri", self.callback.as_str()),
                ("code", code),
                ("code_verifier", verifier),
            ])
            .send()
            .await
            .map_err(|_| Error::new(503, "identity_provider_unavailable"))?;
        let body = net::read_response(response, net::TEXT_LIMIT).await?;
        let data: Value = serde_json::from_slice(&body.bytes)?;
        let token = data["access_token"]
            .as_str()
            .ok_or(Error::new(401, "provider_authorization_failed"))?;
        let u = net::public_url("https://api.github.com/user")?;
        let response = net::client_for(&u)
            .await?
            .get(u)
            .bearer_auth(token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|_| Error::new(503, "identity_provider_unavailable"))?;
        let body = net::read_response(response, net::TEXT_LIMIT).await?;
        let data: Value = serde_json::from_slice(&body.bytes)?;
        let id = data["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or(Error::new(401, "provider_identity_invalid"))?;
        let login = data["login"]
            .as_str()
            .ok_or(Error::new(401, "provider_identity_invalid"))?
            .to_owned();
        bounded(&login, 100)?;
        Ok(Identity {
            id: format!("github:{id}"),
            login,
        })
    }
    pub async fn complete(&self, store: &Store, state: &str, code: &str, now: i64) -> Result<()> {
        let pending = store.begin_callback(state, now)?;
        match self.identity(code, &pending.verifier).await {
            Ok(identity) => store.finish_login(&pending.attempt_id, &identity, now),
            Err(error) => {
                store.connection()?.execute(
                    "UPDATE logins SET status='failed',provider_verifier='' WHERE id=?1",
                    [pending.attempt_id],
                )?;
                Err(error)
            }
        }
    }
}

impl Store {
    #[cfg(feature = "development-workflow")]
    pub fn development_login(&self, name: &str, now: i64) -> Result<Value> {
        let role = match name {
            "author" => "author",
            "reviewer" | "second-reviewer" => "reviewer",
            "maintainer" => "maintainer",
            "operator" => "operator",
            "editor" => "editor",
            _ => return Err(Error::new(422, "invalid_development_actor")),
        };
        self.transaction(|t| {
            let environment: String = t.query_row(
                "SELECT value FROM metadata WHERE key='environment'",
                [],
                |r| r.get(0),
            )?;
            if environment != "development" {
                return Err(Error::new(403, "development_database_required"));
            }
            let id = format!("development:{name}");
            let token = nonce()?;
            t.execute(
                "INSERT OR IGNORE INTO users VALUES(?1,?2,1,?3)",
                params![id, format!("Sample {name}"), now],
            )?;
            for r in ["author", role] {
                t.execute("INSERT OR IGNORE INTO roles VALUES(?1,?2)", params![id, r])?;
            }
            t.execute(
                "INSERT INTO sessions VALUES(?1,?2,?3,NULL)",
                params![digest(&token), id, now + 3600],
            )?;
            Ok(json!({"token":token,"status":"signed_in","development":true,"expiresAt":now+3600}))
        })
    }
    pub fn start_login(
        &self,
        desktop_challenge: &str,
        oauth: &GithubOAuth,
        now: i64,
    ) -> Result<LoginStart> {
        if desktop_challenge.len() != 43
            || !desktop_challenge
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(Error::new(422, "invalid_desktop_challenge"));
        }
        let id = nonce()?;
        let state = nonce()?;
        let verifier = nonce()?;
        let expires = now + 600;
        let authorization_url = oauth.authorization(&state, &verifier)?;
        self.connection()?.execute(
            "INSERT INTO logins VALUES(?1,?2,?3,?4,'pending',?5,NULL)",
            params![id, digest(&state), desktop_challenge, verifier, expires],
        )?;
        Ok(LoginStart {
            attempt_id: id,
            authorization_url,
            expires_at: expires,
        })
    }
    pub fn begin_callback(&self, state: &str, now: i64) -> Result<Callback> {
        if state.len() != 64 {
            return Err(Error::new(401, "invalid_oauth_state"));
        }
        self.transaction(|t| {
            let value=t.query_row("SELECT id,provider_verifier FROM logins WHERE state_hash=?1 AND status='pending' AND expires_at>?2",params![digest(state),now],|r|Ok(Callback{attempt_id:r.get(0)?,verifier:r.get(1)?})).optional()?.ok_or(Error::new(401,"expired_or_used_oauth_state"))?;
            t.execute("UPDATE logins SET status='exchanging' WHERE id=?1",[&value.attempt_id])?;Ok(value)
        })
    }
    fn finish_login(&self, attempt: &str, identity: &Identity, now: i64) -> Result<()> {
        self.transaction(|t| {
            let ready:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM logins WHERE id=?1 AND status='exchanging' AND expires_at>?2)",params![attempt,now],|r|r.get(0))?;
            if !ready { return Err(Error::new(401,"login_expired")); }
            let created=t.execute("INSERT INTO users VALUES(?1,?2,1,?3) ON CONFLICT(id) DO NOTHING",params![identity.id,identity.login,now])?;
            t.execute("UPDATE users SET login=?2 WHERE id=?1",params![identity.id,identity.login])?;
            if created==1 { t.execute("INSERT INTO roles VALUES(?1,'author')",[&identity.id])?; }
            t.execute("UPDATE logins SET user_id=?2,status='ready',provider_verifier='' WHERE id=?1",params![attempt,identity.id])?;
            audit(t,&identity.id,"signed_in",attempt,now,&json!({}))
        })
    }
    pub fn poll_login(&self, attempt: &str, verifier: &str, now: i64) -> Result<Value> {
        if verifier.len() < 43 || verifier.len() > 128 {
            return Err(Error::new(401, "invalid_desktop_proof"));
        }
        self.transaction(|t| {
            let row=t.query_row("SELECT status,user_id,desktop_challenge FROM logins WHERE id=?1 AND expires_at>?2",params![attempt,now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,String>(2)?))).optional()?.ok_or(Error::new(401,"login_expired"))?;
            if challenge(verifier)!=row.2 { return Err(Error::new(401,"invalid_desktop_proof")); }
            match row.0.as_str() {
                "pending"|"exchanging"=>Ok(json!({"status":"pending"})),
                "ready"=>{
                    let token=nonce()?;let expiry=now+7*86400;
                    t.execute("INSERT INTO sessions VALUES(?1,?2,?3,NULL)",params![digest(&token),row.1,expiry])?;
                    t.execute("UPDATE logins SET status='consumed',desktop_challenge='' WHERE id=?1",[attempt])?;
                    Ok(json!({"status":"signed_in","token":token,"expiresAt":expiry}))
                },
                _=>Err(Error::new(401,"login_expired_or_used"))
            }
        })
    }
    pub fn begin_claim(
        &self,
        actor: &Actor,
        target: &str,
        issuer: &str,
        now: i64,
    ) -> Result<Value> {
        let (target, proof_url) = claim_target(target)?;
        net::public_url(issuer)?;
        let id = nonce()?;
        let proof_nonce = nonce()?;
        self.transaction(|t| {
            recheck(t,actor,"author")?;
            t.execute("INSERT INTO challenges VALUES(?1,?2,?3,?4,?5,?6,?7,NULL)",params![id,actor.id,target,proof_url,issuer,proof_nonce,now+1800])?;
            Ok(json!({"id":id,"target":target,"proofUrl":proof_url,"expiresAt":now+1800,"proof":{"claim":id,"account":actor.id,"target":target,"issuer":issuer,"nonce":proof_nonce}}))
        })
    }
    pub fn claim_proof_url(&self, actor: &Actor, id: &str, now: i64) -> Result<String> {
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        c.query_row("SELECT proof_url FROM challenges WHERE id=?1 AND user_id=?2 AND expires_at>?3 AND consumed_at IS NULL",params![id,actor.id,now],|r|r.get(0)).optional()?.ok_or(Error::new(404,"claim_challenge_unavailable"))
    }
    pub fn verify_claim(&self, actor: &Actor, id: &str, proof: &[u8], now: i64) -> Result<Value> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Proof {
            claim: String,
            account: String,
            target: String,
            issuer: String,
            nonce: String,
        }
        if proof.len() > 4096 {
            return Err(Error::new(422, "claim_proof_invalid"));
        }
        let proof: Proof = serde_json::from_slice(proof)?;
        self.transaction(|t| {
            recheck(t,actor,"author")?;
            let row=t.query_row("SELECT target,proof_url,issuer,nonce FROM challenges WHERE id=?1 AND user_id=?2 AND expires_at>?3 AND consumed_at IS NULL",params![id,actor.id,now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).optional()?.ok_or(Error::new(404,"claim_challenge_unavailable"))?;
            if proof.claim!=id||proof.account!=actor.id||proof.target!=row.0||proof.issuer!=row.2||proof.nonce!=row.3 { return Err(Error::new(422,"claim_proof_mismatch")); }
            let other:bool=t.query_row("SELECT EXISTS(SELECT 1 FROM claims WHERE target=?1 AND user_id!=?2 AND revoked_at IS NULL AND expires_at>?3)",params![row.0,actor.id,now],|r|r.get(0))?;
            if other { return Err(Error::new(409,"ownership_review_required")); }
            t.execute("INSERT INTO claims VALUES(?1,?2,?3,?4,?5,NULL) ON CONFLICT(target) DO UPDATE SET user_id=excluded.user_id,proof_url=excluded.proof_url,verified_at=excluded.verified_at,expires_at=excluded.expires_at,revoked_at=NULL",params![row.0,actor.id,row.1,now,now+30*86400])?;
            t.execute("UPDATE challenges SET consumed_at=?2 WHERE id=?1",params![id,now])?;
            audit(t,&actor.id,"claim_verified",&row.0,now,&json!({"proofUrl":row.1}))?;
            Ok(json!({"target":row.0,"status":"verified","expiresAt":now+30*86400}))
        })
    }
    pub fn revoke_claim(&self, actor: &Actor, target: &str, now: i64) -> Result<()> {
        self.transaction(|t| { recheck(t,actor,"author")?;
            let n=t.execute("UPDATE claims SET revoked_at=?3 WHERE target=?1 AND user_id=?2 AND revoked_at IS NULL",params![target,actor.id,now])?;
            if n==0 { return Err(Error::new(404,"claim_unavailable")); }
            audit(t,&actor.id,"claim_revoked",target,now,&json!({}))
        })
    }
}
pub fn claim_target(input: &str) -> Result<(String, String)> {
    let mut u = net::public_url(input)?;
    if u.query().is_some() {
        return Err(Error::new(422, "invalid_claim_target"));
    }
    let host = u.host_str().unwrap_or("").to_ascii_lowercase();
    if host == "github.com" {
        let path = u
            .path()
            .trim_matches('/')
            .trim_end_matches(".git")
            .to_ascii_lowercase();
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != 2 || parts.iter().any(|v| !omastore_catalogue::token(v)) {
            return Err(Error::new(422, "invalid_claim_target"));
        }
        Ok((
            format!("https://github.com/{path}"),
            format!("https://raw.githubusercontent.com/{path}/HEAD/.omastore-claim.json"),
        ))
    } else {
        if u.path() != "/" {
            return Err(Error::new(422, "invalid_claim_target"));
        }
        u.set_path("/.well-known/omastore-claim.json");
        Ok((format!("https://{host}"), u.into()))
    }
}

#[cfg(test)]
pub(crate) fn test_actor(store: &Store, name: &str, now: i64) -> (Actor, String) {
    let token = nonce().unwrap();
    let c = store.connection().unwrap();
    c.execute("INSERT INTO users VALUES(?1,?1,1,?2)", params![name, now])
        .unwrap();
    c.execute("INSERT INTO roles VALUES(?1,'author')", [name])
        .unwrap();
    c.execute(
        "INSERT INTO sessions VALUES(?1,?2,?3,NULL)",
        params![digest(&token), name, now + 86400],
    )
    .unwrap();
    drop(c);
    (store.actor(&token, now).unwrap(), token)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_is_bound_to_desktop_and_callback_cannot_replay() {
        let s = Store::memory().unwrap();
        let verifier = nonce().unwrap();
        let config = GithubOAuth {
            client_id: "test-client".into(),
            client_secret: "test-only".into(),
            callback: "https://store.example/api/v1/auth/callback".into(),
        };
        let start = s.start_login(&challenge(&verifier), &config, 1000).unwrap();
        let u = Url::parse(&start.authorization_url).unwrap();
        let state = u
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned();
        assert!(s
            .poll_login(&start.attempt_id, &nonce().unwrap(), 1001)
            .is_err());
        let cb = s.begin_callback(&state, 1001).unwrap();
        assert!(s.begin_callback(&state, 1001).is_err());
        s.finish_login(
            &cb.attempt_id,
            &Identity {
                id: "github:42".into(),
                login: "fixture".into(),
            },
            1002,
        )
        .unwrap();
        let result = s.poll_login(&start.attempt_id, &verifier, 1003).unwrap();
        let token = result["token"].as_str().unwrap();
        assert_eq!(s.actor(token, 1003).unwrap().id, "github:42");
        assert!(s.poll_login(&start.attempt_id, &verifier, 1004).is_err());
        s.set_role("github:42", "author", false, 1004).unwrap();
        assert!(s.actor(token, 1004).unwrap().roles.is_empty());
        s.revoke_session(token, 1005).unwrap();
        assert!(s.actor(token, 1005).is_err());
    }
    #[test]
    fn claims_require_scoped_control_and_fail_on_replay_or_wrong_owner() {
        let s = Store::memory().unwrap();
        let (a, _) = test_actor(&s, "author", 1000);
        let (b, _) = test_actor(&s, "other", 1000);
        assert_eq!(s.claims(&a, 1000).unwrap(), json!([]));
        let claim = s
            .begin_claim(
                &a,
                "https://publisher.example",
                "https://store.example",
                1000,
            )
            .unwrap();
        let id = claim["id"].as_str().unwrap();
        assert!(s.claim_proof_url(&b, id, 1001).is_err());
        let mut wrong = claim["proof"].clone();
        wrong["target"] = json!("https://other.example");
        assert!(s
            .verify_claim(&a, id, &serde_json::to_vec(&wrong).unwrap(), 1001)
            .is_err());
        let proof = serde_json::to_vec(&claim["proof"]).unwrap();
        s.verify_claim(&a, id, &proof, 1001).unwrap();
        assert!(s.verify_claim(&a, id, &proof, 1002).is_err());
        assert_eq!(s.claims(&a, 1002).unwrap()[0]["active"], true);
        s.revoke_claim(&a, "https://publisher.example", 1003)
            .unwrap();
        assert_eq!(s.claims(&a, 1003).unwrap()[0]["active"], false);
    }
    #[test]
    fn migration_reopens_populated_private_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workflow.db");
        let s = Store::open(&path).unwrap();
        let (_, token) = test_actor(&s, "existing", 1000);
        drop(s);
        let s = Store::open(&path).unwrap();
        assert_eq!(s.actor(&token, 1001).unwrap().login, "existing");
    }
}
