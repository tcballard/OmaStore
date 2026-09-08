//! Public metadata adapters only. No application, package or build hook is executed.
use crate::{monitor::Observation, net, Error, Result};
use omastore_catalogue::{App, ReleaseIdentity};
use serde_json::Value;

async fn github(path: &str) -> Result<Option<Value>> {
    let url = net::public_url(&format!("https://api.github.com{path}"))?;
    let response = net::client_for(&url)
        .await?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await
        .map_err(|_| Error::new(503, "upstream_unavailable"))?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    let bytes = net::read_response(response, net::TEXT_LIMIT).await?.bytes;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
fn repo(url: &str) -> Option<String> {
    crate::auth::claim_target(url)
        .ok()?
        .0
        .strip_prefix("https://github.com/")
        .filter(|s| crate::github::repository_name(s))
        .map(str::to_owned)
}
fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
async fn owner(repository: &str) -> Result<String> {
    let v = github(&format!("/repos/{repository}"))
        .await?
        .ok_or(Error::new(503, "source_unavailable"))?;
    let id = v["id"]
        .as_u64()
        .ok_or(Error::new(503, "upstream_response_invalid"))?;
    let owner = v["owner"]["id"]
        .as_u64()
        .ok_or(Error::new(503, "upstream_response_invalid"))?;
    Ok(format!("github:repository:{id}:owner:{owner}"))
}
pub async fn observe(app: &App) -> Result<Observation> {
    let release = app.current_release();
    let mut result = Observation {
        declared_identity_available: false,
        owner_identity: None,
        latest_version: None,
        artifact_sha256: None,
        source: "unsupported".into(),
    };
    match &release.identity {
        ReleaseIdentity::SourceCommit { repository, commit } => {
            let repository =
                repo(repository).ok_or(Error::new(503, "source_adapter_unavailable"))?;
            result.owner_identity = Some(owner(&repository).await?);
            let current = github(&format!("/repos/{repository}/commits/{commit}")).await?;
            result.declared_identity_available =
                current.as_ref().is_some_and(|v| v["sha"] == *commit);
            if let Some(latest) = github(&format!("/repos/{repository}/releases/latest")).await? {
                result.latest_version = text(&latest["tag_name"]);
            }
            result.source = "github_commit_and_release_metadata".into();
        }
        ReleaseIdentity::RepositoryPackage {
            repository,
            package,
            version,
            ..
        } => {
            if !["core", "extra", "multilib"].contains(&repository.as_str())
                || !release.architectures.iter().any(|a| a == "x86_64")
            {
                return Err(Error::new(503, "repository_adapter_unavailable"));
            }
            let bytes = net::fetch(
                &format!("https://archlinux.org/packages/{repository}/x86_64/{package}/json/"),
                net::TEXT_LIMIT,
            )
            .await?
            .bytes;
            let v: Value = serde_json::from_slice(&bytes)?;
            if v["pkgname"] != *package
                || v["repo"]
                    .as_str()
                    .is_none_or(|r| !r.eq_ignore_ascii_case(repository))
            {
                return Err(Error::new(503, "upstream_response_invalid"));
            }
            let epoch = v["epoch"].as_u64().unwrap_or(0);
            let observed = format!(
                "{}{}-{}",
                if epoch == 0 {
                    String::new()
                } else {
                    format!("{epoch}:")
                },
                text(&v["pkgver"]).ok_or(Error::new(503, "upstream_response_invalid"))?,
                text(&v["pkgrel"]).ok_or(Error::new(503, "upstream_response_invalid"))?
            );
            result.declared_identity_available = &observed == version;
            result.latest_version = Some(observed);
            result.owner_identity = Some(format!("archlinux:{repository}:{package}"));
            result.source = "arch_package_metadata_only".into();
        }
        ReleaseIdentity::BinaryArtifact { .. } => {
            // A website URL is not an artifact URL. Only an exact GitHub release asset route is supported here.
            let url = net::public_url(release.route.url())?;
            if url.host_str() != Some("github.com") {
                return Err(Error::new(503, "artifact_adapter_unavailable"));
            }
            let parts = url
                .path_segments()
                .ok_or(Error::new(503, "artifact_adapter_unavailable"))?
                .collect::<Vec<_>>();
            if parts.len() != 6 || parts[2] != "releases" || parts[3] != "download" {
                return Err(Error::new(503, "artifact_adapter_unavailable"));
            }
            let repository = format!("{}/{}", parts[0], parts[1]);
            if !crate::github::repository_name(&repository) {
                return Err(Error::new(503, "artifact_adapter_unavailable"));
            }
            result.owner_identity = Some(owner(&repository).await?);
            let release_record = github(&format!("/repos/{repository}/releases/tags/{}", parts[4]))
                .await?
                .ok_or(Error::new(503, "source_unavailable"))?;
            let asset = release_record["assets"]
                .as_array()
                .and_then(|a| {
                    a.iter()
                        .find(|a| a["browser_download_url"] == release.route.url())
                })
                .ok_or(Error::new(503, "artifact_unavailable"))?;
            let hash = asset["digest"]
                .as_str()
                .and_then(|s| s.strip_prefix("sha256:"))
                .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or(Error::new(503, "provider_artifact_digest_unavailable"))?;
            result.declared_identity_available = true;
            result.artifact_sha256 = Some(hash.into());
            result.source = "github_release_asset_digest".into();
            if let Some(latest) = github(&format!("/repos/{repository}/releases/latest")).await? {
                result.latest_version = text(&latest["tag_name"]);
            }
        }
    }
    if !matches!(release.identity, ReleaseIdentity::BinaryArtifact { .. }) {
        let route = net::public_url(release.route.url())?;
        let response = net::client_for(&route)
            .await?
            .head(route)
            .send()
            .await
            .map_err(|_| Error::new(503, "route_observation_unavailable"))?;
        result.declared_identity_available &= response.status().is_success();
    }
    Ok(result)
}
