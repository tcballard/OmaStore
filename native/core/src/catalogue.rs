use omastore_catalogue::{Catalogue, MAX_CATALOGUE_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

// A release-controlled origin. Catalogue records and IPC requests cannot override it.
pub const ORIGIN: &str =
    "https://raw.githubusercontent.com/tcballard/OmaStore/catalogue-live/data/registry.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cache {
    origin: String,
    etag: Option<String>,
    fetched_at: String,
    catalogue: Catalogue,
}

pub struct Client {
    pub catalogue: Catalogue,
    pub source: &'static str,
    pub warning: Option<&'static str>,
    pub fetched_at: Option<String>,
    pub demo: bool,
    path: Option<PathBuf>,
    etag: Option<String>,
}

impl Client {
    pub fn new(demo: bool) -> Self {
        let seed = include_bytes!("../../../data/registry.json").as_slice();
        #[cfg(feature = "development-catalogue")]
        let seed = if demo {
            include_bytes!("../../../tests/fixtures/catalogue.json").as_slice()
        } else {
            seed
        };
        let mut client = Self {
            catalogue: Catalogue::parse(seed, demo).expect("validated bundled catalogue"),
            source: if demo { "development" } else { "bundled" },
            warning: None,
            fetched_at: None,
            demo,
            path: if demo { None } else { cache_path() },
            etag: None,
        };
        client.load_cache();
        #[cfg(feature = "development-catalogue")]
        if demo {
            client.load_sample();
        }
        client
    }

    fn load_cache(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        if !path.exists() {
            return;
        }
        let read = || -> Option<Cache> {
            let mut bytes = Vec::new();
            File::open(path)
                .ok()?
                .take((MAX_CATALOGUE_BYTES + 4097) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() > MAX_CATALOGUE_BYTES + 4096 {
                return None;
            }
            let cache: Cache = serde_json::from_slice(&bytes).ok()?;
            if cache.origin != ORIGIN
                || !cache.catalogue.validate(false).is_empty()
                || chrono::DateTime::parse_from_rfc3339(&cache.fetched_at).is_err()
                || cache
                    .etag
                    .as_ref()
                    .is_some_and(|s| s.len() > 256 || s.contains(['\r', '\n']))
            {
                return None;
            }
            Some(cache)
        };
        if let Some(cache) = read() {
            self.catalogue = cache.catalogue;
            self.etag = cache.etag;
            self.fetched_at = Some(cache.fetched_at);
            self.source = "cached";
        } else {
            self.warning = Some("cache_invalid");
        }
    }

    pub fn info(&self) -> Value {
        let mut categories: Vec<_> = self
            .catalogue
            .apps
            .iter()
            .map(|a| a.category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        json!({"snapshot": self.catalogue.snapshot_id(), "revision": self.catalogue.revision,
            "schemaVersion": self.catalogue.schema_version, "source": self.source,
            "warning": self.warning, "fetchedAt": self.fetched_at, "demo": self.demo,
            "appCount": self.catalogue.apps.len(), "categories": categories,
            "origin": if self.demo { None } else { Some(ORIGIN) }})
    }

    pub fn refresh(&mut self) -> Value {
        if self.demo {
            #[cfg(feature = "development-catalogue")]
            self.load_sample();
            return self.info();
        }
        let result = self.fetch();
        if let Err(code) = result {
            self.source = "stale";
            self.warning = Some(code);
        }
        self.info()
    }

    #[cfg(feature = "development-catalogue")]
    fn load_sample(&mut self) {
        let read = || -> Option<Catalogue> {
            let path = crate::workspace::sample_catalogue_path().ok()?;
            let mut bytes = Vec::new();
            File::open(path)
                .ok()?
                .take((MAX_CATALOGUE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            Catalogue::parse(&bytes, true).ok()
        };
        if let Some(c) = read() {
            for app in c.apps {
                self.catalogue.apps.retain(|a| a.id != app.id);
                self.catalogue.apps.push(app);
            }
            for maker in c.makers {
                self.catalogue.makers.retain(|m| m.id != maker.id);
                self.catalogue.makers.push(maker);
            }
            for recipe in c.recipes {
                self.catalogue.recipes.retain(|r| r.id != recipe.id);
                self.catalogue.recipes.push(recipe);
            }
            self.catalogue.revision = c.revision;
        }
    }
    fn fetch(&mut self) -> Result<(), &'static str> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(10)))
            .max_response_header_size(16 * 1024)
            .http_status_as_error(false)
            .build()
            .into();
        let mut request = agent.get(ORIGIN).header("Accept", "application/json");
        if let Some(etag) = &self.etag {
            request = request.header("If-None-Match", etag);
        }
        let mut response = request.call().map_err(|_| "catalogue_unreachable")?;
        let status = response.status().as_u16();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .filter(|s| s.len() <= 256)
            .map(str::to_owned);
        let bytes = if status == 200 {
            Some(
                response
                    .body_mut()
                    .with_config()
                    .limit(MAX_CATALOGUE_BYTES as u64)
                    .read_to_vec()
                    .map_err(|_| "catalogue_read_failed")?,
            )
        } else {
            None
        };
        self.accept(status, bytes.as_deref(), etag)
    }

    fn accept(
        &mut self,
        status: u16,
        bytes: Option<&[u8]>,
        etag: Option<String>,
    ) -> Result<(), &'static str> {
        match status {
            200 => {
                let catalogue = Catalogue::parse(bytes.ok_or("catalogue_read_failed")?, false)
                    .map_err(|_| "catalogue_invalid")?;
                self.catalogue = catalogue;
                self.etag = etag;
            }
            304 if self.etag.is_some() && self.fetched_at.is_some() => {}
            _ => return Err("catalogue_http_error"),
        }
        self.source = "live";
        self.warning = None;
        self.fetched_at = Some(chrono::Utc::now().to_rfc3339());
        if self.save().is_err() {
            self.warning = Some("cache_write_failed");
        }
        Ok(())
    }

    fn save(&self) -> std::io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| std::io::Error::other("cache directory unavailable"))?;
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)?;
        let cache = Cache {
            origin: ORIGIN.into(),
            etag: self.etag.clone(),
            fetched_at: self.fetched_at.clone().unwrap(),
            catalogue: self.catalogue.clone(),
        };
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(&mut file, &cache)?;
        file.flush()?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

fn cache_path() -> Option<PathBuf> {
    let xdg = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    let root = xdg.or_else(|| {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".cache"))
            .filter(|p| p.is_absolute())
    })?;
    Some(root.join("omastore/catalogue-v1.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn isolated() -> (Client, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mut client = Client::new(false);
        client.path = Some(dir.path().join("catalogue.json"));
        client.etag = None;
        client.fetched_at = None;
        (client, dir)
    }
    #[test]
    fn validation_precedes_replacement_and_atomic_cache_survives_reload() {
        let (mut client, _dir) = isolated();
        client
            .accept(
                200,
                Some(include_bytes!("../../../data/registry.json")),
                Some("\"one\"".into()),
            )
            .unwrap();
        let digest = client.catalogue.snapshot_id();
        assert!(client.accept(200, Some(b"{invalid"), None).is_err());
        assert_eq!(client.catalogue.snapshot_id(), digest);
        client.source = "bundled";
        client.load_cache();
        assert_eq!(client.source, "cached");
        client.accept(304, None, None).unwrap();
        assert_eq!(client.source, "live");
        fs::write(client.path.as_ref().unwrap(), b"unfinished write").unwrap();
        client.load_cache();
        assert_eq!(client.warning, Some("cache_invalid"));
        assert_eq!(client.catalogue.snapshot_id(), digest);
    }
    #[test]
    fn conditional_response_requires_a_cache_and_development_never_enters_it() {
        let (mut client, _dir) = isolated();
        assert!(client.accept(304, None, None).is_err());
        assert!(client
            .accept(
                200,
                Some(include_bytes!("../../../tests/fixtures/catalogue.json")),
                None
            )
            .is_err());
        assert!(!client.path.unwrap().exists());
    }
}
