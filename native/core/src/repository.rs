//! Published package observations for discovery; never authorisation to install.
use omastore_catalogue::{Catalogue, LicenceClass, ReleaseIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

pub const ORIGIN: &str = "https://pkgs.omarchy.org/stable/x86_64/omarchy.db";
const MAX_WIRE: u64 = 4 * 1024 * 1024;
const MAX_TAR: u64 = 32 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Saved {
    origin: String,
    etag: Option<String>,
    checked_at: String,
    catalogue: Catalogue,
}

pub struct Repository {
    pub catalogue: Catalogue,
    pub checked_at: Option<String>,
    pub warning: Option<&'static str>,
    etag: Option<String>,
    path: Option<PathBuf>,
}
impl Repository {
    pub fn new(path: Option<PathBuf>) -> Self {
        let mut result = Self {
            catalogue: seed(),
            checked_at: None,
            warning: None,
            etag: None,
            path,
        };
        if let Some(path) = &result.path {
            if path.exists() {
                let read = || -> Option<Saved> {
                    let mut bytes = Vec::new();
                    File::open(path)
                        .ok()?
                        .take(1024 * 1024 + 1)
                        .read_to_end(&mut bytes)
                        .ok()?;
                    if bytes.len() > 1024 * 1024 {
                        return None;
                    }
                    let saved: Saved = serde_json::from_slice(&bytes).ok()?;
                    let allowed = seed();
                    if saved.origin != ORIGIN
                        || !saved.catalogue.validate(false).is_empty()
                        || chrono::DateTime::parse_from_rfc3339(&saved.checked_at).is_err()
                        || saved.etag.as_ref().is_some_and(|e| !valid_etag(e))
                        || saved
                            .catalogue
                            .apps
                            .iter()
                            .any(|a| !allowed.apps.iter().any(|b| b.id == a.id))
                    {
                        return None;
                    }
                    Some(saved)
                };
                if let Some(saved) = read() {
                    result.catalogue = saved.catalogue;
                    result.checked_at = Some(saved.checked_at);
                    result.etag = saved.etag;
                } else {
                    result.warning = Some("repository_cache_invalid");
                }
            }
        }
        result
    }
    pub fn refresh(&mut self) -> Result<(), &'static str> {
        let result = self.fetch();
        if let Err(code) = result {
            self.warning = Some(code);
        }
        result
    }
    fn fetch(&mut self) -> Result<(), &'static str> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(10)))
            .max_response_header_size(16384)
            .http_status_as_error(false)
            .build()
            .into();
        let mut request = agent.get(ORIGIN).header("Accept-Encoding", "identity");
        if let Some(etag) = &self.etag {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request.call().map_err(|e| match e {
            ureq::Error::Io(_) => "repository_io_error",
            ureq::Error::HostNotFound => "repository_dns_error",
            ureq::Error::ConnectProxyFailed(_) | ureq::Error::InvalidProxyUrl => {
                "repository_proxy_error"
            }
            ureq::Error::Tls(_) | ureq::Error::Rustls(_) => "repository_tls_error",
            ureq::Error::Timeout(_) => "repository_timeout",
            _ => "repository_unreachable",
        })?;
        let status = response.status().as_u16();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .filter(|e| valid_etag(e))
            .map(str::to_owned);
        let bytes = if status == 200 {
            response
                .body_mut()
                .with_config()
                .limit(MAX_WIRE)
                .read_to_vec()
                .map_err(|_| "repository_read_failed")?
        } else {
            Vec::new()
        };
        self.accept(status, &bytes, etag)
    }
    fn accept(
        &mut self,
        status: u16,
        bytes: &[u8],
        etag: Option<String>,
    ) -> Result<(), &'static str> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let catalogue = match status {
            200 => from_index(bytes, &now)?,
            304 if self.etag.is_some() && self.checked_at.is_some() => self.catalogue.clone(),
            _ => return Err("repository_http_error"),
        };
        self.catalogue = catalogue;
        if status == 200 {
            self.etag = etag;
        }
        self.checked_at = Some(now);
        self.warning = None;
        if self.save().is_err() {
            self.warning = Some("cache_write_failed");
        }
        Ok(())
    }
    fn save(&self) -> std::io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| std::io::Error::other("no cache path"))?;
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(
            &mut file,
            &Saved {
                origin: ORIGIN.into(),
                etag: self.etag.clone(),
                checked_at: self.checked_at.clone().unwrap(),
                catalogue: self.catalogue.clone(),
            },
        )?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        File::open(parent)?.sync_all()
    }
}
fn valid_etag(e: &str) -> bool {
    e.len() <= 256 && !e.contains(['\r', '\n'])
}
fn seed() -> Catalogue {
    Catalogue::parse(
        include_bytes!("../../../data/repository/catalogue.json"),
        false,
    )
    .expect("bundled repository catalogue")
}

pub fn from_index(bytes: &[u8], observed_at: &str) -> Result<Catalogue, &'static str> {
    if bytes.len() as u64 > MAX_WIRE {
        return Err("repository_too_large");
    }
    let mut decoder = zstd::stream::read::Decoder::new(bytes).map_err(|_| "repository_invalid")?;
    decoder
        .window_log_max(23)
        .map_err(|_| "repository_invalid")?;
    let mut decoded = Vec::new();
    decoder
        .take(MAX_TAR + 1)
        .read_to_end(&mut decoded)
        .map_err(|_| "repository_invalid")?;
    if decoded.len() as u64 > MAX_TAR {
        return Err("repository_too_large");
    }
    let mut archive = tar::Archive::new(decoded.as_slice());
    let mut packages = BTreeMap::new();
    for (count, entry) in archive
        .entries()
        .map_err(|_| "repository_invalid")?
        .enumerate()
    {
        if count >= 20000 {
            return Err("repository_too_large");
        }
        let mut entry = entry.map_err(|_| "repository_invalid")?;
        let path = entry.path().map_err(|_| "repository_invalid")?;
        if !entry.header().entry_type().is_file() || path.file_name().is_none_or(|p| p != "desc") {
            continue;
        }
        if entry.size() > 65536 {
            return Err("repository_too_large");
        }
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|_| "repository_invalid")?;
        let mut fields = BTreeMap::new();
        for section in text.trim().split("\n\n") {
            let (key, value) = section.split_once('\n').ok_or("repository_invalid")?;
            if fields.insert(key, value.trim()).is_some() {
                return Err("repository_invalid");
            }
        }
        let get = |key| {
            fields
                .get(key)
                .copied()
                .filter(|s| !s.is_empty())
                .ok_or("repository_invalid")
        };
        let name = get("%NAME%")?.to_owned();
        let package = (
            get("%VERSION%")?.to_owned(),
            get("%ARCH%")?.to_owned(),
            fields
                .get("%LICENSE%")
                .copied()
                .unwrap_or("")
                .replace('\n', " AND "),
        );
        if packages.insert(name, package).is_some() {
            return Err("repository_invalid");
        }
    }
    if packages.is_empty() {
        return Err("repository_empty");
    }
    let mut catalogue = seed();
    catalogue.apps.retain_mut(|app| {
        let release = &mut app.releases[0];
        let ReleaseIdentity::RepositoryPackage { package, version, signature, .. } = &mut release.identity else { return false; };
        let Some((new_version, architecture, licence)) = packages.get(package) else { return false; };
        if architecture != "x86_64" && architecture != "any" { return false; }
        *version = new_version.clone(); *signature = None;
        release.version = new_version.clone(); release.architectures = vec!["x86_64".into()];
        release.notes = format!("Observed in Omarchy stable x86_64 at {observed_at}. Package metadata is not a runtime test or signature verification. Installation requires fresh package-manager checks.");
        app.licence.identifier = if licence.is_empty() { None } else { Some(licence.clone()) };
        // An index licence change cannot silently inherit our earlier classification.
        if app.licence.identifier != seed().apps.iter().find(|a| a.id == app.id).unwrap().licence.identifier { app.licence.class = LicenceClass::Unknown; }
        app.tests.clear();
        true
    });
    catalogue.generated_at = observed_at.into();
    catalogue.revision = format!("repository-sync-{:x}", Sha256::digest(bytes));
    catalogue.build_revision = "repository-sync-v1".into();
    if !catalogue.validate(false).is_empty() {
        return Err("repository_invalid");
    }
    Ok(catalogue)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn index(packages: &[(&str, &str)]) -> Vec<u8> {
        let mut tar = tar::Builder::new(Vec::new());
        for (n, (name, version)) in packages.iter().enumerate() {
            let text = format!(
                "%NAME%\n{name}\n\n%VERSION%\n{version}\n\n%ARCH%\nx86_64\n\n%LICENSE%\nMIT\n\n"
            );
            let mut header = tar::Header::new_gnu();
            header.set_size(text.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, format!("entry-{n}/desc"), text.as_bytes())
                .unwrap();
        }
        zstd::encode_all(tar.into_inner().unwrap().as_slice(), 1).unwrap()
    }
    #[test]
    fn updates_removals_duplicates_and_bad_archives() {
        let now = "2026-09-11T12:00:00Z";
        let cat = from_index(&index(&[("omacalc", "9.2-1"), ("system-only", "1-1")]), now).unwrap();
        assert_eq!(cat.apps.len(), 1);
        assert_eq!(cat.apps[0].releases[0].version, "9.2-1");
        assert!(cat.apps[0].tests.is_empty());
        assert!(from_index(&index(&[("omacalc", "1-1"), ("omacalc", "2-1")]), now).is_err());
        assert!(from_index(b"not zstd", now).is_err());
        assert!(from_index(&index(&[]), now).is_err());
        // A valid index can remove every selected app without being malformed.
        assert!(from_index(&index(&[("other", "1-1")]), now)
            .unwrap()
            .apps
            .is_empty());
    }
    #[test]
    fn refresh_is_conditional_atomic_and_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("repository.json");
        let mut repo = Repository::new(Some(path.clone()));
        assert!(repo.accept(304, &[], None).is_err());
        repo.accept(200, &index(&[("omacalc", "9-1")]), Some("\"v1\"".into()))
            .unwrap();
        let previous = std::fs::read(&path).unwrap();
        assert!(repo.accept(200, b"broken", None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        let mut restored = Repository::new(Some(path));
        assert_eq!(restored.catalogue.apps[0].releases[0].version, "9-1");
        restored.accept(304, &[], None).unwrap();
        assert_eq!(restored.etag.as_deref(), Some("\"v1\""));
    }
    #[test]
    fn archived_upstream_database_parses_with_bounded_reader() {
        let cat = from_index(
            include_bytes!("../../../data/repository/omarchy.db.zst"),
            "2026-09-11T12:00:00Z",
        )
        .unwrap();
        assert_eq!(cat.apps.len(), 6);
        assert!(cat.validate(false).is_empty());
    }
    #[test]
    #[ignore = "explicit live HTTPS integration; no network in ordinary tests"]
    fn live_index_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = Repository::new(Some(dir.path().join("live.json")));
        repo.refresh().unwrap();
        assert!(repo.checked_at.is_some());
        assert!(!repo.catalogue.apps.is_empty());
        repo.refresh().unwrap();
        assert!(repo.warning.is_none());
    }
}
