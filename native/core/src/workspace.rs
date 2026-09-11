//! The token never crosses the Qt pipe. Origins come only from operator configuration.
use base64::{engine::general_purpose::STANDARD, Engine};
use omastore_workflow::{auth::challenge, digest, nonce, Error, Result};
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
    cached: Value,
    demo: bool,
    previews: Vec<tempfile::NamedTempFile>,
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
            private_directory(true)
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
            cached: json!({"actor":null}),
            demo,
            previews: Vec::new(),
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
        self.http_key(method, path, params, "")
    }
    fn http_key(&self, method: &str, path: &str, params: &Value, key: &str) -> Result<Value> {
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
                .header("Idempotency-Key", key)
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
                "candidate_invalid" => "candidate_invalid",
                "candidate_scope_invalid" => "candidate_scope_invalid",
                "candidate_cannot_self_verify" => "candidate_cannot_self_verify",
                "media_count_exceeded" => "media_count_exceeded",
                "media_too_large" | "normalized_media_too_large" => "media_too_large",
                "media_quota_exceeded" => "media_quota_exceeded",
                "unsupported_media_format" => "unsupported_media_format",
                "draft_unavailable" => "draft_unavailable",
                "revision_unavailable" => "revision_unavailable",
                "transition_unavailable" => "transition_unavailable",
                "public_preview_required" => "public_preview_required",
                "independent_reviewer_required" => "independent_reviewer_required",
                "independent_tester_required" => "independent_tester_required",
                "required_checks_missing" => "required_checks_missing",
                "runtime_evidence_missing" => "runtime_evidence_missing",
                "evidence_invalid" => "evidence_invalid",
                "evidence_identity_mismatch" => "evidence_identity_mismatch",
                "evidence_candidate_mismatch" => "evidence_candidate_mismatch",
                "stale_monitor_status" => "stale_monitor_status",
                "fresh_upstream_observation_required" => "fresh_upstream_observation_required",
                "artifact_mismatch_unresolved" => "artifact_mismatch_unresolved",
                "listing_steward_required" => "listing_steward_required",
                "delivery_in_progress" => "delivery_in_progress",
                "publication_busy" => "publication_busy",
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
            let mut value = json!({"configured":true,"sandbox":true,"publicationBridge":"local_rehearsal","storage":"sample_session","actor":null,"claims":[],"drafts":[],"revisions":[]});
            if let Some(token) = &self.token {
                match store.actor(token, omastore_workflow::now()) {
                    Ok(actor) => {
                        let workspace = store.workspace(&actor, omastore_workflow::now())?;
                        for (key, item) in workspace.as_object().unwrap() {
                            value[key] = item.clone();
                        }
                    }
                    Err(_) => self.token = None,
                }
            }
            self.cached = value.clone();
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
            match self.http("GET", "/api/v1/workspace", &json!({})) {
                Ok(value) => value,
                Err(error) if error.status == 401 => {
                    self.token = None;
                    self.storage = "no_session";
                    self.login = None;
                    self.cached = json!({"actor":null});
                    self.previews.clear();
                    json!({"actor":null,"signInConfigured":true,"sessionExpired":true})
                }
                Err(error) => return Err(error),
            }
        } else {
            let info = self.http("GET", "/api/v1/auth/info", &json!({}))?;
            json!({"actor":null,"signInConfigured":info["signInConfigured"]})
        };
        value["configured"] = json!(true);
        value["sandbox"] = json!(false);
        value["storage"] = json!(self.storage);
        value["signingIn"] = json!(self.login.is_some());
        self.cached = value.clone();
        Ok(value)
    }
    pub fn dispatch(&mut self, method: &str, params: Value) -> Result<Value> {
        if method == "status.get" {
            let ids: Vec<String> = serde_json::from_value(params["ids"].clone())?;
            if ids.len() > 100 || ids.iter().any(|id| !omastore_catalogue::token(id)) {
                return Err(Error::new(422, "invalid_status_ids"));
            }
            #[cfg(feature = "development-catalogue")]
            let sample = if let Some(store) = &self.sandbox {
                let catalogue = crate::catalogue::Client::new(true).catalogue;
                store.sync_monitor_catalogue(&catalogue, omastore_workflow::now())?;
                Some(store.public_status(&catalogue, &ids, omastore_workflow::now())?)
            } else {
                None
            };
            #[cfg(not(feature = "development-catalogue"))]
            let sample: Option<Value> = None;
            let value = match sample {
                Some(v) => v,
                None => self.http(
                    "GET",
                    &format!("/api/v1/status?ids={}", ids.join(",")),
                    &json!({}),
                )?,
            };
            return Ok(json!({"value":value,"workspace":self.cached}));
        }

        let result = match method {
            "state" if params == json!({}) => json!({}),
            "command" => self.command(params)?,
            #[cfg(feature = "development-catalogue")]
            "drafts.sample" => {
                if self.sandbox.is_none() {
                    return Err(Error::new(403, "sample_mode_required"));
                }
                let candidate: Value =
                    serde_json::from_str(include_str!("../../../docs/examples/submission.json"))?;
                self.command(json!({"command":"create_draft","kind":"app","candidate":candidate,"base_revision":null}))?
            }
            "drafts.new" => {
                let raw = include_str!("../../../crates/workflow/templates/app.json")
                    .replace("APP_ID", &format!("app-{}", &nonce()?[..12]))
                    .replace("MAKER_ID", &format!("maker-{}", &nonce()?[..12]));
                let mut candidate: Value = serde_json::from_str(&raw)?;
                candidate["generatedAt"] =
                    json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                self.command(json!({"command":"create_draft","kind":"app","candidate":candidate,"base_revision":null}))?
            }
            "drafts.preview" => {
                let candidate = params["candidate"].clone();
                let errors = match serde_json::from_value::<omastore_catalogue::Catalogue>(
                    candidate.clone(),
                ) {
                    Ok(c) => json!(c.validate(true)),
                    Err(_) => json!([{"path":"candidate","code":"invalid_schema"}]),
                };
                return Ok(
                    json!({"value":{"candidate":candidate,"errors":errors},"workspace":self.cached}),
                );
            }
            #[cfg(feature = "development-catalogue")]
            "review.sample_evidence" => {
                let store = self
                    .sandbox
                    .as_ref()
                    .ok_or(Error::new(403, "sample_mode_required"))?;
                let actor = store.actor(
                    self.token.as_deref().unwrap_or(""),
                    omastore_workflow::now(),
                )?;
                store.sample_runtime(&actor, record_id(&params)?, omastore_workflow::now())?
            }
            "review.queue" | "review.get" => {
                let id = if method == "review.get" {
                    record_id(&params)?
                } else {
                    ""
                };
                #[cfg(feature = "development-catalogue")]
                let local = if let Some(store) = &self.sandbox {
                    let actor = store.actor(
                        self.token.as_deref().unwrap_or(""),
                        omastore_workflow::now(),
                    )?;
                    Some(if method == "review.queue" {
                        store.review_queue(&actor, omastore_workflow::now())?
                    } else {
                        store.review_detail(&actor, id, omastore_workflow::now())?
                    })
                } else {
                    None
                };
                #[cfg(not(feature = "development-catalogue"))]
                let local: Option<Value> = None;
                match local {
                    Some(value) => value,
                    None => self.http(
                        "GET",
                        &if method == "review.queue" {
                            "/api/v1/review".into()
                        } else {
                            format!("/api/v1/review/{id}")
                        },
                        &json!({}),
                    )?,
                }
            }
            "drafts.get" | "revisions.get" => {
                let id = record_id(&params)?;
                let mut value = self.record(method, id)?;
                if method == "drafts.get" {
                    if let Ok(bytes) = std::fs::read(self.local_file("draft", id)?) {
                        if bytes.len() <= 220 * 1024 {
                            value["localRecovery"] = serde_json::from_slice(&bytes)?;
                        }
                    }
                }
                value
            }
            #[cfg(feature = "development-catalogue")]
            "publication.sample" => {
                let id = record_id(&params)?;
                let store = self
                    .sandbox
                    .as_ref()
                    .ok_or(Error::new(403, "sample_mode_required"))?;
                let actor = store.actor(
                    self.token.as_deref().unwrap_or(""),
                    omastore_workflow::now(),
                )?;
                let dir = private_directory(true)?;
                let objects = omastore_workflow::media::LocalObjects::new(&dir.join("objects"))?;
                omastore_workflow::sample_publication::rehearse(
                    store,
                    &actor,
                    id,
                    &objects,
                    &sample_catalogue_path()?,
                )?
            }
            "publication.export" => self.export_publication(&params)?,
            "monitor.queue" => {
                #[cfg(feature = "development-catalogue")]
                let sample = if let Some(store) = &self.sandbox {
                    let actor = store.actor(
                        self.token.as_deref().unwrap_or(""),
                        omastore_workflow::now(),
                    )?;
                    store.sync_monitor_catalogue(
                        &crate::catalogue::Client::new(true).catalogue,
                        omastore_workflow::now(),
                    )?;
                    Some(store.monitoring_queue(&actor)?)
                } else {
                    None
                };
                #[cfg(not(feature = "development-catalogue"))]
                let sample: Option<Value> = None;
                match sample {
                    Some(v) => v,
                    None => self.http("GET", "/api/v1/monitoring", &json!({}))?,
                }
            }
            #[cfg(feature = "development-catalogue")]
            "monitor.sample" => {
                let store = self
                    .sandbox
                    .as_ref()
                    .ok_or(Error::new(403, "sample_mode_required"))?;
                let actor = store.actor(
                    self.token.as_deref().unwrap_or(""),
                    omastore_workflow::now(),
                )?;
                actor.require("operator")?;
                store.sync_monitor_catalogue(
                    &crate::catalogue::Client::new(true).catalogue,
                    omastore_workflow::now(),
                )?;
                for _ in 0..100 {
                    let Some(lease) = store.lease_monitor(omastore_workflow::now())? else {
                        break;
                    };
                    let sha = if let omastore_catalogue::ReleaseIdentity::BinaryArtifact {
                        sha256,
                        ..
                    } = &lease.app.current_release().identity
                    {
                        Some(sha256.clone())
                    } else {
                        None
                    };
                    let observation = omastore_workflow::monitor::Observation {
                        declared_identity_available: true,
                        owner_identity: Some("fictional-owner".into()),
                        latest_version: Some(lease.app.current_release().version.clone()),
                        artifact_sha256: sha,
                        source: "local_simulation".into(),
                    };
                    store.finish_monitor(&lease, Ok(&observation), omastore_workflow::now())?;
                }
                json!({"simulated":true,"notice":"Fictional observations recorded locally. No upstream repository was checked."})
            }

            "drafts.cache" => {
                let id = record_id(&params)?;
                let bytes = serde_json::to_vec(&params)?;
                if bytes.len() > 220 * 1024 {
                    return Err(Error::new(413, "candidate_too_large"));
                }
                save_private(&self.local_file("draft", id)?, &bytes)?;
                return Ok(json!({"value":{"id":id,"localSaved":true},"workspace":self.cached}));
            }
            "media.upload" => self.upload(params)?,
            "media.preview" => self.preview_media(params)?,
            #[cfg(feature = "development-catalogue")]
            "checks.run_sample" => {
                let store = self
                    .sandbox
                    .as_ref()
                    .ok_or(Error::new(403, "sample_mode_required"))?;
                store.actor(
                    self.token.as_deref().unwrap_or(""),
                    omastore_workflow::now(),
                )?;
                store.sample_checks(omastore_workflow::now())?
            }
            "evidence.import" => {
                let path = params["file"]
                    .as_str()
                    .and_then(|p| Url::parse(p).ok())
                    .and_then(|u| u.to_file_path().ok())
                    .ok_or(Error::new(422, "local_file_required"))?;
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(80 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > 80 * 1024 {
                    return Err(Error::new(413, "evidence_too_large"));
                }
                let evidence: omastore_workflow::checks::RuntimeEvidence =
                    serde_json::from_slice(&bytes)?;
                self.command(json!({"command":"record_runtime","evidence":evidence}))?
            }
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
                let value = match self.http(
                    "POST",
                    "/api/v1/auth/poll",
                    &json!({"attemptId":attempt,"verifier":verifier}),
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        self.login = None;
                        return Err(error);
                    }
                };
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
                self.cached = json!({"actor":null});
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
        // A refresh failure must not turn a committed mutation into an ambiguous failure.
        let workspace = match self.state() {
            Ok(v) => v,
            Err(error) => {
                let mut v = self.cached.clone();
                v["refreshWarning"] = json!(error.code);
                v
            }
        };
        Ok(json!({"value":result,"workspace":workspace}))
    }
    fn local_file(&self, kind: &str, id: &str) -> Result<std::path::PathBuf> {
        let owner = self.cached["actor"]["id"]
            .as_str()
            .ok_or(Error::new(401, "sign_in_required"))?;
        let dir = private_directory(self.demo)?.join("recovery");
        let key = digest(format!(
            "{}:{owner}:{kind}:{id}",
            self.origin.as_deref().unwrap_or("sample")
        ));
        Ok(dir.join(format!("{key}.json")))
    }
    fn record(&self, method: &str, id: &str) -> Result<Value> {
        #[cfg(feature = "development-catalogue")]
        if let Some(store) = &self.sandbox {
            let actor = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            return if method == "drafts.get" {
                store.draft(&actor, id)
            } else {
                let mut value = store.revision(&actor, id)?;
                value["publication"] = store.publication_internal(id)?;
                value["publication"]["bridge"] = json!("local_rehearsal");
                Ok(value)
            };
        }
        let collection = if method == "drafts.get" {
            "drafts"
        } else {
            "revisions"
        };
        let mut value = self.http("GET", &format!("/api/v1/{collection}/{id}"), &json!({}))?;
        if collection == "revisions" {
            value["publication"] = self
                .http("GET", &format!("/api/v1/publication/{id}"), &json!({}))
                .unwrap_or(json!({"bridge":"unavailable"}));
        }
        Ok(value)
    }
    fn export_publication(&self, params: &Value) -> Result<Value> {
        let id = record_id(params)?;
        let path = params["file"]
            .as_str()
            .and_then(|s| Url::parse(s).ok())
            .and_then(|u| u.to_file_path().ok())
            .filter(|p| p.is_absolute())
            .ok_or(Error::new(422, "local_file_required"))?;
        #[cfg(feature = "development-catalogue")]
        let sample = if let Some(store) = &self.sandbox {
            let actor = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            Some(store.publication_manifest(&actor, id)?)
        } else {
            None
        };
        #[cfg(not(feature = "development-catalogue"))]
        let sample: Option<Value> = None;
        let manifest = if let Some(value) = sample {
            value
        } else {
            let origin = self
                .origin
                .as_ref()
                .ok_or(Error::new(503, "workspace_unconfigured"))?;
            let mut response = self
                .agent
                .get(format!("{origin}/api/v1/publication/{id}/manifest"))
                .header("X-OmaStore-Client", "native-v1")
                .header(
                    "Authorization",
                    &format!("Bearer {}", self.token.as_deref().unwrap_or("")),
                )
                .call()
                .map_err(|_| Error::new(503, "workspace_unavailable"))?;
            if response.status().as_u16() != 200 {
                return Err(Error::new(409, "publication_manifest_unavailable"));
            }
            let bytes = response
                .body_mut()
                .with_config()
                .limit(10 * 1024 * 1024)
                .read_to_vec()
                .map_err(|_| Error::new(503, "publication_manifest_invalid"))?;
            serde_json::from_slice(&bytes)?
        };
        let registry: omastore_catalogue::Catalogue =
            serde_json::from_value(manifest["registry"].clone())?;
        let registry_bytes = registry.canonical_bytes();
        let receipt_bytes = serde_json::to_vec_pretty(&manifest["receipt"])?;
        let receipt_path = format!("data/approvals/{id}.json");
        if manifest["files"]["data/registry.json"] != digest(&registry_bytes)
            || manifest["files"][&receipt_path] != digest(&receipt_bytes)
        {
            return Err(Error::new(409, "publication_manifest_invalid"));
        }
        let output = json!({"files":[{"path":"data/registry.json","sha256":digest(&registry_bytes),"utf8":String::from_utf8(registry_bytes).map_err(|_|Error::new(500,"publication_manifest_invalid"))?},{"path":receipt_path,"sha256":digest(&receipt_bytes),"utf8":String::from_utf8(receipt_bytes).map_err(|_|Error::new(500,"publication_manifest_invalid"))?}],"mediaHashes":manifest["files"],"notice":"Write each utf8 value verbatim to its stated repository path. Supply the reviewed media bytes; attach the PR for independent verification."});
        if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
            return Err(Error::new(422, "regular_file_required"));
        }
        let parent = path
            .parent()
            .ok_or(Error::new(422, "local_file_required"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&serde_json::to_vec_pretty(&output)?)?;
        file.as_file().sync_all()?;
        file.persist(&path)
            .map_err(|_| Error::new(500, "local_export_failed"))?;
        Ok(json!({"id":id,"exported":true}))
    }
    fn request_key(&self, params: &Value) -> Result<(String, std::path::PathBuf)> {
        let path = self.local_file("outbox", "pending")?;
        let hash = digest(serde_json::to_vec(params)?);
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() < 230 * 1024 {
                if let Ok(old) = serde_json::from_slice::<Value>(&bytes) {
                    if old["digest"] == hash {
                        if let Some(key) = old["key"].as_str() {
                            return Ok((key.to_owned(), path));
                        }
                    }
                }
            }
        }
        let key = nonce()?;
        save_private(
            &path,
            &serde_json::to_vec(&json!({"key":key,"digest":hash,"request":params}))?,
        )?;
        Ok((key, path))
    }
    fn command(&self, params: Value) -> Result<Value> {
        let command: omastore_workflow::drafts::Command = serde_json::from_value(params.clone())?;
        let (key, path) = self.request_key(&params)?;
        #[cfg(feature = "development-catalogue")]
        let local = if let Some(store) = &self.sandbox {
            let actor = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            Some(store.command(&actor, &key, command, omastore_workflow::now())?)
        } else {
            None
        };
        #[cfg(not(feature = "development-catalogue"))]
        let local: Option<Value> = {
            let _ = command;
            None
        };
        let mut value = match local {
            Some(v) => v,
            None => self.http_key("POST", "/api/v1/commands", &params, &key)?,
        };
        let _ = std::fs::remove_file(path);
        if params["command"] == "save_draft" {
            if let Some(id) = params["id"].as_str() {
                let _ = std::fs::remove_file(self.local_file("draft", id)?);
            }
        }
        value["command"] = params["command"].clone();
        Ok(value)
    }
    fn upload(&self, params: Value) -> Result<Value> {
        let draft = params["draftId"]
            .as_str()
            .ok_or(Error::new(422, "invalid_fields"))?;
        if draft.len() != 64 || !draft.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::new(422, "invalid_fields"));
        }
        let path = params["file"]
            .as_str()
            .and_then(|p| Url::parse(p).ok())
            .filter(|u| u.scheme() == "file")
            .and_then(|u| u.to_file_path().ok())
            .ok_or(Error::new(422, "local_file_required"))?;
        let meta = std::fs::symlink_metadata(&path)?;
        if !meta.is_file() || meta.len() > omastore_workflow::media::VIDEO_LIMIT as u64 {
            return Err(Error::new(413, "media_too_large"));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take((omastore_workflow::media::VIDEO_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > omastore_workflow::media::VIDEO_LIMIT {
            return Err(Error::new(413, "media_too_large"));
        }
        let mut identity = params.clone();
        identity["file"] = json!(digest(&bytes));
        let (key, pending) = self.request_key(&identity)?;
        #[cfg(feature = "development-catalogue")]
        if let Some(store) = &self.sandbox {
            let actor = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            store.draft(&actor, draft)?;
            let objects = omastore_workflow::media::LocalObjects::new(
                &private_directory(true)?.join("objects"),
            )?;
            let asset = omastore_workflow::media::normalize(
                params["kind"].as_str().unwrap_or(""),
                &bytes,
                &objects,
            )?;
            let value = store.attach_media(
                &actor,
                omastore_workflow::media::Upload {
                    draft_id: draft,
                    version: params["version"].as_i64().unwrap_or(0),
                    kind: params["kind"].as_str().unwrap_or(""),
                    alt: params["alt"].as_str().unwrap_or(""),
                    rights: params["rights"].as_str().unwrap_or(""),
                    key: &key,
                },
                &asset,
                &objects,
                omastore_workflow::now(),
            )?;
            let _ = std::fs::remove_file(pending);
            return Ok(value);
        }
        let mut body = params;
        body.as_object_mut().unwrap().remove("file");
        body["bytes"] = json!(STANDARD.encode(bytes));
        let value = self.http_key("POST", "/api/v1/media", &body, &key)?;
        let _ = std::fs::remove_file(pending);
        Ok(value)
    }
    fn preview_media(&mut self, params: Value) -> Result<Value> {
        #[cfg(feature = "development-catalogue")]
        use omastore_workflow::media::ObjectStorage;
        use omastore_workflow::media::VIDEO_LIMIT;
        let id = record_id(&params)?;
        let expected = params["sha256"]
            .as_str()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or(Error::new(422, "invalid_fields"))?;
        #[cfg(feature = "development-catalogue")]
        let local = if let Some(store) = &self.sandbox {
            let actor = store.actor(
                self.token.as_deref().unwrap_or(""),
                omastore_workflow::now(),
            )?;
            let (key, mime) = store.private_media_key(&actor, id)?;
            let objects = omastore_workflow::media::LocalObjects::new(
                &private_directory(true)?.join("objects"),
            )?;
            Some((mime, objects.read_private(&key, VIDEO_LIMIT)?))
        } else {
            None
        };
        #[cfg(not(feature = "development-catalogue"))]
        let local: Option<(String, Vec<u8>)> = None;
        let (mime, bytes) = match local {
            Some(value) => value,
            None => {
                let origin = self
                    .origin
                    .as_deref()
                    .ok_or(Error::new(503, "workspace_unconfigured"))?;
                let token = self
                    .token
                    .as_deref()
                    .ok_or(Error::new(401, "sign_in_required"))?;
                let mut response = self
                    .agent
                    .get(&format!("{origin}/api/v1/media/{id}"))
                    .header("X-OmaStore-Client", "native-v1")
                    .header("Authorization", &format!("Bearer {token}"))
                    .call()
                    .map_err(|_| Error::new(503, "workspace_unavailable"))?;
                if response.status().as_u16() != 200 {
                    return Err(Error::new(404, "media_unavailable"));
                }
                let mime = response
                    .headers()
                    .get("Content-Type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_owned();
                let bytes = response
                    .body_mut()
                    .with_config()
                    .limit(VIDEO_LIMIT as u64)
                    .read_to_vec()
                    .map_err(|_| Error::new(413, "media_too_large"))?;
                (mime, bytes)
            }
        };
        if digest(&bytes) != expected {
            return Err(Error::new(422, "media_digest_mismatch"));
        }
        let ext = match mime.as_str() {
            "image/png" => ".png",
            "video/mp4" => ".mp4",
            "video/webm" => ".webm",
            _ => return Err(Error::new(422, "unsupported_media_format")),
        };
        let mut file = tempfile::Builder::new()
            .prefix("review-media-")
            .suffix(ext)
            .tempfile_in(private_directory(self.demo)?)?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        let url = Url::from_file_path(file.path())
            .map_err(|_| Error::new(500, "local_storage_unavailable"))?
            .to_string();
        if self.previews.len() >= 2 {
            self.previews.remove(0);
        }
        self.previews.push(file);
        Ok(json!({"id":id,"url":url,"contentType":mime}))
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
fn private_directory(demo: bool) -> Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"))
        })
        .ok_or(Error::new(500, "local_storage_unavailable"))?;
    let path = base.join(if demo { "omastore-sample" } else { "omastore" });
    if !path.exists() {
        std::fs::create_dir_all(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

fn record_id(params: &Value) -> Result<&str> {
    params["id"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(Error::new(422, "invalid_fields"))
}
fn save_private(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let parent = path
        .parent()
        .ok_or(Error::new(500, "local_storage_unavailable"))?;
    std::fs::create_dir_all(parent)?;
    if std::fs::symlink_metadata(parent)?.file_type().is_symlink() {
        return Err(Error::new(500, "unsafe_local_storage"));
    }
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|_| Error::new(500, "local_storage_unavailable"))?;
    Ok(())
}

#[cfg(feature = "development-catalogue")]
pub(crate) fn sample_catalogue_path() -> Result<std::path::PathBuf> {
    Ok(private_directory(true)?.join("sample-published.json"))
}
