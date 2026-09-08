//! The token never crosses the Qt pipe. Origins come only from operator configuration.
use omastore_workflow::{auth::challenge, nonce, Error, Result};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use url::Url;

pub struct Client {
    origin: Option<String>,
    token: Option<String>,
    login: Option<(String, String)>,
    storage: &'static str,
    agent: ureq::Agent,
    #[cfg(feature = "development-catalogue")]
    sandbox: Option<omastore_workflow::Store>,
}
impl Client {
    pub fn new(demo: bool) -> Self {
        let origin = std::env::var("OMASTORE_SERVICE_ORIGIN")
            .ok()
            .filter(|s| {
                Url::parse(s).is_ok_and(|u| {
                    u.scheme() == "https"
                        && u.host_str().is_some()
                        && u.username().is_empty()
                        && u.password().is_none()
                        && u.query().is_none()
                        && u.fragment().is_none()
                        && u.path() == "/"
                        && u.port().is_none_or(|p| p == 443)
                })
            })
            .map(|s| s.trim_end_matches('/').to_owned());
        #[cfg(feature = "development-catalogue")]
        let sandbox = if demo {
            private_directory()
                .and_then(|p| omastore_workflow::Store::development(&p.join("workflow.db")))
                .ok()
        } else {
            None
        };
        let _ = demo;
        Self {
            origin,
            token: None,
            login: None,
            storage: "signed_out",
            agent: ureq::Agent::new_with_config(
                ureq::Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(10)))
                    .max_redirects(0)
                    .http_status_as_error(false)
                    .build(),
            ),
            #[cfg(feature = "development-catalogue")]
            sandbox,
        }
    }
    fn http(&self, method: &str, path: &str, params: &Value) -> Result<Value> {
        let origin = self
            .origin
            .as_ref()
            .ok_or(Error::new(503, "workspace_unconfigured"))?;
        let url = format!("{origin}{path}");
        let token = self.token.as_deref().unwrap_or("");
        let request = if method == "GET" {
            self.agent
                .get(&url)
                .header("X-OmaStore-Client", "native-v1")
                .header("Authorization", &format!("Bearer {token}"))
                .call()
        } else {
            self.agent
                .post(&url)
                .header("X-OmaStore-Client", "native-v1")
                .header("Authorization", &format!("Bearer {token}"))
                .send_json(params)
        };
        let mut response = request.map_err(|_| Error::new(503, "workspace_unavailable"))?;
        let status = response.status().as_u16();
        let bytes = response
            .body_mut()
            .with_config()
            .limit(240 * 1024)
            .read_to_vec()
            .map_err(|_| Error::new(503, "workspace_response_invalid"))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| Error::new(503, "workspace_response_invalid"))?;
        if status >= 400 {
            // Return only known error codes; remote free text never becomes a diagnostic.
            let code = match value["error"]["code"].as_str().unwrap_or("") {
                "sign_in_unconfigured" => "sign_in_unconfigured",
                "workspace_unconfigured" => "workspace_unconfigured",
                "session_expired" | "sign_in_required" => "sign_in_required",
                "claim_proof_mismatch" => "claim_proof_mismatch",
                "claim_challenge_unavailable" => "claim_challenge_unavailable",
                "role_revoked" | "role_required" => "role_required",
                "rate_limited" => "rate_limited",
                "stale_revision" => "stale_revision",
                "invalid_fields" => "invalid_fields",
                _ => "workspace_request_failed",
            };
            return Err(Error::new(status, code));
        }
        Ok(value)
    }
    pub fn state(&mut self) -> Result<Value> {
        #[cfg(feature = "development-catalogue")]
        if let Some(store) = &self.sandbox {
            let mut value = json!({"configured":true,"sandbox":true,"storage":"sample_session","actor":null,"claims":[],"drafts":[],"revisions":[]});
            if let Some(token) = &self.token {
                match store.actor(token, omastore_workflow::now()) {
                    Ok(actor) => {
                        value["actor"] = json!(actor);
                        value["claims"] = store.claims(&actor, omastore_workflow::now())?;
                    }
                    Err(_) => self.token = None,
                }
            }
            return Ok(value);
        }
        if self.origin.is_none() {
            return Ok(json!({"configured":false,"sandbox":false,"actor":null}));
        }
        if self.token.is_none() && self.storage == "signed_out" {
            if let Ok(token) = secret("lookup", self.origin.as_deref().unwrap_or(""), None) {
                if token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()) {
                    self.token = Some(token);
                    self.storage = "keyring";
                }
            }
            if self.token.is_none() {
                self.storage = "no_session";
            }
        }
        let mut value = if self.token.is_some() {
            self.http("GET", "/api/v1/workspace", &json!({}))?
        } else {
            let info = self.http("GET", "/api/v1/auth/info", &json!({}))?;
            json!({"actor":null,"signInConfigured":info["signInConfigured"]})
        };
        value["configured"] = json!(true);
        value["sandbox"] = json!(false);
        value["storage"] = json!(self.storage);
        value["signingIn"] = json!(self.login.is_some());
        Ok(value)
    }
    pub fn dispatch(&mut self, method: &str, params: Value) -> Result<Value> {
        let result = match method {
            "state" if params == json!({}) => json!({}),
            "auth.start" if params == json!({}) => {
                let verifier = nonce()?;
                let value = self.http(
                    "POST",
                    "/api/v1/auth/start",
                    &json!({"desktopChallenge":challenge(&verifier)}),
                )?;
                let attempt = value["attemptId"]
                    .as_str()
                    .filter(|id| id.len() == 64)
                    .ok_or(Error::new(503, "workspace_response_invalid"))?;
                let url = value["authorizationUrl"]
                    .as_str()
                    .ok_or(Error::new(503, "workspace_response_invalid"))?;
                if !Url::parse(url).is_ok_and(|u| {
                    u.scheme() == "https"
                        && u.host_str() == Some("github.com")
                        && u.path() == "/login/oauth/authorize"
                        && u.username().is_empty()
                        && u.password().is_none()
                        && u.port().is_none()
                }) {
                    return Err(Error::new(503, "workspace_response_invalid"));
                }
                self.login = Some((attempt.to_owned(), verifier));
                json!({"authorizationUrl":url,"status":"pending"})
            }
            "auth.poll" if params == json!({}) => {
                let (attempt, verifier) = self
                    .login
                    .as_ref()
                    .ok_or(Error::new(401, "login_expired"))?;
                let value = self.http(
                    "POST",
                    "/api/v1/auth/poll",
                    &json!({"attemptId":attempt,"verifier":verifier}),
                )?;
                if value["status"] == "signed_in" {
                    self.accept_token(&value)?;
                    self.login = None;
                    json!({"status":"signed_in"})
                } else {
                    json!({"status":"pending"})
                }
            }
            "auth.logout" if params == json!({}) => {
                #[cfg(feature = "development-catalogue")]
                let sandbox = self.sandbox.is_some();
                #[cfg(not(feature = "development-catalogue"))]
                let sandbox = false;
                if !sandbox {
                    let _ = self.http("POST", "/api/v1/auth/logout", &json!({}));
                    if let Some(origin) = &self.origin {
                        let _ = secret("clear", origin, None);
                    }
                }
                self.token = None;
                self.login = None;
                self.storage = "no_session";
                json!({"signedOut":true})
            }
            #[cfg(feature = "development-catalogue")]
            "auth.sandbox" => {
                let store = self
                    .sandbox
                    .as_ref()
                    .ok_or(Error::new(403, "sample_mode_required"))?;
                let name = params["name"]
                    .as_str()
                    .ok_or(Error::new(422, "invalid_fields"))?;
                let value = store.development_login(name, omastore_workflow::now())?;
                self.token = Some(
                    value["token"]
                        .as_str()
                        .ok_or(Error::new(500, "session_failed"))?
                        .to_owned(),
                );
                json!({"status":"signed_in"})
            }
            "claims.start" | "claims.verify" | "claims.revoke" => {
                #[cfg(feature = "development-catalogue")]
                if let Some(store) = &self.sandbox {
                    let actor = store.actor(
                        self.token.as_deref().unwrap_or(""),
                        omastore_workflow::now(),
                    )?;
                    let value = match method {
                        "claims.start" => store.begin_claim(
                            &actor,
                            params["target"].as_str().unwrap_or(""),
                            "https://sandbox.omastore.invalid",
                            omastore_workflow::now(),
                        )?,
                        "claims.revoke" => {
                            store.revoke_claim(
                                &actor,
                                params["target"].as_str().unwrap_or(""),
                                omastore_workflow::now(),
                            )?;
                            json!({"revoked":true})
                        }
                        _ => return Err(Error::new(422, "sample_claim_has_no_public_proof")),
                    };
                    return Ok(json!({"value":value,"workspace":self.state()?}));
                }
                let path = match method {
                    "claims.start" => "/api/v1/claims/start",
                    "claims.verify" => "/api/v1/claims/verify",
                    _ => "/api/v1/claims/revoke",
                };
                self.http("POST", path, &params)?
            }
            _ => return Err(Error::new(422, "unknown_workspace_method")),
        };
        Ok(json!({"value":result,"workspace":self.state()?}))
    }
    fn accept_token(&mut self, value: &Value) -> Result<()> {
        let token = value["token"]
            .as_str()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(Error::new(503, "workspace_response_invalid"))?;
        self.storage = if secret("store", self.origin.as_deref().unwrap_or(""), Some(token)).is_ok()
        {
            "keyring"
        } else {
            "session_only"
        };
        self.token = Some(token.to_owned());
        Ok(())
    }
}
fn secret(operation: &str, origin: &str, token: Option<&str>) -> Result<String> {
    let mut command = Command::new("/usr/bin/secret-tool");
    command.arg(operation);
    if operation == "store" {
        command.arg("--label=OmaStore author session");
    }
    command
        .args([
            "application",
            "io.github.tcballard.OmaStore",
            "origin",
            origin,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|_| Error::new(503, "keyring_unavailable"))?;
    if let Some(token) = token {
        child
            .stdin
            .take()
            .ok_or(Error::new(503, "keyring_unavailable"))?
            .write_all(token.as_bytes())?;
    } else {
        drop(child.stdin.take());
    }
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait()? {
            Some(status) => {
                if !status.success() {
                    return Err(Error::new(503, "keyring_unavailable"));
                }
                break;
            }
            None if Instant::now() < until => std::thread::sleep(Duration::from_millis(10)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::new(503, "keyring_unavailable"));
            }
        }
    }
    let mut value = String::new();
    if let Some(out) = child.stdout.take() {
        out.take(129).read_to_string(&mut value)?;
    }
    if value.len() > 128 {
        return Err(Error::new(503, "keyring_response_invalid"));
    }
    Ok(value.trim_end_matches('\n').to_owned())
}
#[cfg(feature = "development-catalogue")]
fn private_directory() -> Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
        .ok_or(Error::new(500, "local_storage_unavailable"))?;
    let path = base.join("omastore-sample");
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}
