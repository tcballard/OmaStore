//! Operator-owned GitHub App authentication. No connector credentials enter the product.
use crate::{net, Error, Result};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Config {
    pub repository: String,
    pub branch: String,
    pub delivery_branch: String,
    pub app_id: u64,
    pub installation_id: u64,
    pub private_key: Arc<jsonwebtoken::EncodingKey>,
    pub webhook_secret: Arc<Vec<u8>>,
}
impl Config {
    pub fn from_env() -> Result<Option<Self>> {
        let Some(id) = std::env::var("OMASTORE_GITHUB_APP_ID").ok() else {
            return Ok(None);
        };
        let app_id = id
            .parse::<u64>()
            .map_err(|_| Error::new(503, "github_configuration_invalid"))?;
        let installation_id = std::env::var("OMASTORE_GITHUB_INSTALLATION_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or(Error::new(503, "github_configuration_incomplete"))?;
        let repository = std::env::var("OMASTORE_PUBLISH_REPOSITORY")
            .unwrap_or_else(|_| "tcballard/OmaStore".into());
        if !repository_name(&repository) {
            return Err(Error::new(503, "github_configuration_invalid"));
        }
        let branch = std::env::var("OMASTORE_PUBLISH_BRANCH").unwrap_or_else(|_| "main".into());
        if !omastore_catalogue::token(&branch) {
            return Err(Error::new(503, "github_configuration_invalid"));
        }
        let delivery_branch =
            std::env::var("OMASTORE_DELIVERY_BRANCH").unwrap_or_else(|_| "catalogue-live".into());
        if !omastore_catalogue::token(&delivery_branch) || delivery_branch == branch {
            return Err(Error::new(503, "delivery_branch_must_be_separate"));
        }
        let key_path = std::env::var("OMASTORE_GITHUB_PRIVATE_KEY_FILE")
            .map_err(|_| Error::new(503, "github_configuration_incomplete"))?;
        let secret_path = std::env::var("OMASTORE_GITHUB_WEBHOOK_SECRET_FILE")
            .map_err(|_| Error::new(503, "github_configuration_incomplete"))?;
        let pem = private_file(Path::new(&key_path), 32768)?;
        let private_key = jsonwebtoken::EncodingKey::from_rsa_pem(&pem)
            .map_err(|_| Error::new(503, "github_key_invalid"))?;
        let webhook_secret = private_file(Path::new(&secret_path), 1024)?;
        if webhook_secret.len() < 32 || app_id == 0 || installation_id == 0 {
            return Err(Error::new(503, "github_configuration_invalid"));
        }
        Ok(Some(Self {
            repository,
            branch,
            delivery_branch,
            app_id,
            installation_id,
            private_key: Arc::new(private_key),
            webhook_secret: Arc::new(webhook_secret),
        }))
    }
}
pub fn repository_name(name: &str) -> bool {
    let parts: Vec<_> = name.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|s| {
            !s.is_empty()
                && s.len() <= 100
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                && *s != "."
                && *s != ".."
        })
}
fn private_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::{io::Read, os::unix::fs::PermissionsExt};
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > limit || metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::new(503, "private_secret_file_required"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::new(503, "private_secret_file_required"));
    }
    Ok(bytes)
}
#[async_trait]
pub trait Api: Send + Sync {
    fn development(&self) -> bool {
        false
    }
    async fn read_public(&self, origin: &str, limit: usize) -> Result<Vec<u8>> {
        Ok(net::fetch(&format!("{origin}/api/v1/catalogue"), limit)
            .await?
            .bytes)
    }
    async fn call(&self, method: &str, path: &str, body: Value) -> Result<Value>;
    async fn read_file(
        &self,
        repository: &str,
        reference: &str,
        path: &str,
        limit: usize,
    ) -> Result<Vec<u8>>;
}
#[derive(Clone)]
pub struct Github {
    pub config: Config,
    token: Arc<Mutex<Option<(String, i64)>>>,
}
fn app_jwt(app_id: u64, key: &jsonwebtoken::EncodingKey, now: i64) -> Result<String> {
    #[derive(Serialize)]
    struct Claims {
        iat: i64,
        exp: i64,
        iss: String,
    }
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
        &Claims {
            iat: now - 60,
            exp: now + 540,
            iss: app_id.to_string(),
        },
        key,
    )
    .map_err(|_| Error::new(503, "github_signing_failed"))
}
impl Github {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            token: Arc::new(Mutex::new(None)),
        }
    }
    async fn token(&self) -> Result<String> {
        let mut token = self.token.lock().await;
        if let Some((value, expires)) = &*token {
            if *expires > crate::now() + 120 {
                return Ok(value.clone());
            }
        }
        let now = crate::now();
        let jwt = app_jwt(self.config.app_id, &self.config.private_key, now)?;
        let name = self
            .config
            .repository
            .split('/')
            .nth(1)
            .ok_or(Error::new(503, "github_configuration_invalid"))?;
        let response=send("POST",&format!("/app/installations/{}/access_tokens",self.config.installation_id),json!({"repositories":[name],"permissions":{"contents":"write","issues":"write","pull_requests":"write","checks":"write"}}),&jwt).await?;
        let value = response["token"]
            .as_str()
            .filter(|s| s.len() <= 4096)
            .ok_or(Error::new(503, "github_token_invalid"))?
            .to_owned();
        let expires = response["expires_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp())
            .ok_or(Error::new(503, "github_token_invalid"))?;
        if expires <= now + 120 {
            return Err(Error::new(503, "github_token_expired"));
        }
        *token = Some((value.clone(), expires));
        Ok(value)
    }
}
#[async_trait]
impl Api for Github {
    async fn call(&self, method: &str, path: &str, body: Value) -> Result<Value> {
        let prefix = format!("/repos/{}/", self.config.repository);
        if !path.starts_with(&prefix) {
            return Err(Error::new(403, "github_repository_scope"));
        }
        let result = send(method, path, body, &self.token().await?).await;
        if result.as_ref().is_err_and(|e| e.status == 401) {
            *self.token.lock().await = None;
        }
        result
    }
    async fn read_file(
        &self,
        repository: &str,
        reference: &str,
        path: &str,
        limit: usize,
    ) -> Result<Vec<u8>> {
        if repository != self.config.repository
            || (!crate::publication::commit_sha(reference)
                && reference != self.config.delivery_branch)
            || !allowed_file(path)
        {
            return Err(Error::new(403, "github_file_scope"));
        }
        Ok(net::fetch(
            &format!("https://raw.githubusercontent.com/{repository}/{reference}/{path}"),
            limit,
        )
        .await?
        .bytes)
    }
}
pub fn allowed_file(path: &str) -> bool {
    path == "data/registry.json"
        || path.strip_prefix("media/").is_some_and(|s| {
            s.split_once('.').is_some_and(|(sha, ext)| {
                sha.len() == 64
                    && sha.bytes().all(|b| b.is_ascii_hexdigit())
                    && ["png", "mp4", "webm"].contains(&ext)
            })
        })
        || path
            .strip_prefix("data/approvals/")
            .and_then(|s| s.strip_suffix(".json"))
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
async fn send(method: &str, path: &str, body: Value, token: &str) -> Result<Value> {
    if !path.starts_with('/') || path.contains("..") || path.contains('#') || path.len() > 4096 {
        return Err(Error::new(422, "github_path_invalid"));
    }
    let url = net::public_url(&format!("https://api.github.com{path}"))?;
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| Error::new(422, "github_method_invalid"))?;
    let read = method == reqwest::Method::GET;
    let request = net::client_for(&url)
        .await?
        .request(method, url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10");
    let request = if read { request } else { request.json(&body) };
    let response = request.send().await.map_err(|_| {
        Error::new(
            503,
            if read {
                "github_unavailable"
            } else {
                "github_write_outcome_unknown"
            },
        )
    })?;
    let status = response.status().as_u16();
    if status == 401 {
        return Err(Error::new(401, "github_authentication_failed"));
    }
    if status == 404 {
        return Err(Error::new(404, "github_resource_missing"));
    }
    if status == 409 || status == 422 {
        return Err(Error::new(409, "github_conflict"));
    }
    if !(200..300).contains(&status) {
        return Err(Error::new(503, "github_request_rejected"));
    }
    if status == 204 {
        return Ok(Value::Null);
    }
    let response = net::read_response(response, net::TEXT_LIMIT)
        .await
        .map_err(|_| {
            Error::new(
                503,
                if read {
                    "github_response_invalid"
                } else {
                    "github_write_outcome_unknown"
                },
            )
        })?;
    serde_json::from_slice(&response.bytes).map_err(|_| {
        Error::new(
            503,
            if read {
                "github_response_invalid"
            } else {
                "github_write_outcome_unknown"
            },
        )
    })
}

#[cfg(test)]
mod signing_tests {
    use super::*;
    #[test]
    fn github_app_tokens_use_rs256_and_bounded_claims() {
        use std::process::{Command, Stdio};
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("ephemeral.pem");
        let generated = Command::new("/usr/bin/openssl")
            .args([
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-out",
            ])
            .arg(&file)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(generated.success());
        let public = Command::new("/usr/bin/openssl")
            .args(["pkey", "-pubout", "-in"])
            .arg(&file)
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert!(public.status.success());
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(&std::fs::read(&file).unwrap()).unwrap();
        let now = crate::now();
        let jwt = app_jwt(42, &key, now).unwrap();
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&["42"]);
        let claims = jsonwebtoken::decode::<Value>(
            &jwt,
            &jsonwebtoken::DecodingKey::from_rsa_pem(&public.stdout).unwrap(),
            &validation,
        )
        .unwrap()
        .claims;
        assert_eq!(claims["iat"], now - 60);
        assert_eq!(claims["exp"], now + 540);
        assert_eq!(claims["iss"], "42");
        let mut tampered = jwt.into_bytes();
        tampered[20] = if tampered[20] == b'A' { b'B' } else { b'A' };
        assert!(jsonwebtoken::decode::<Value>(
            &String::from_utf8(tampered).unwrap(),
            &jsonwebtoken::DecodingKey::from_rsa_pem(&public.stdout).unwrap(),
            &validation
        )
        .is_err());
    }
}
