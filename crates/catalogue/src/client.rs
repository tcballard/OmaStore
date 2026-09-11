use crate::*;
use reqwest::blocking::Client;
use std::{
    fs,
    io::{Read, Write},
    net::{IpAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    time::Duration,
};

pub const DEFAULT_ORIGIN: &str =
    "https://raw.githubusercontent.com/tcballard/OmaStore/main/data/registry.json";
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub source: String,
    pub stale: bool,
    pub error: Option<String>,
    pub revision: String,
}
pub struct CatalogueClient {
    pub snapshot: Snapshot,
    pub state: State,
    cache: Option<PathBuf>,
    origin: String,
}
fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !v.is_private()
                && !v.is_loopback()
                && !v.is_link_local()
                && !v.is_unspecified()
                && !v.is_multicast()
                && !v.is_broadcast()
                && !v.is_documentation()
                && v.octets()[0] != 0
                && v.octets()[0] < 224
                && !(v.octets()[0] == 100 && (64..128).contains(&v.octets()[1]))
        }
        IpAddr::V6(v) => v.to_ipv4_mapped().map(public_ip_v4).unwrap_or_else(|| {
            let s = v.segments();
            !v.is_loopback()
                && !v.is_unspecified()
                && !v.is_multicast()
                && s[0] & 0xfe00 != 0xfc00
                && s[0] & 0xffc0 != 0xfe80
                && !(s[0] == 0x2001 && s[1] == 0xdb8)
        }),
    }
}
fn public_ip_v4(ip: std::net::Ipv4Addr) -> bool {
    public_ip(IpAddr::V4(ip))
}
fn load(path: &Path) -> Result<Snapshot, &'static str> {
    let f = fs::File::open(path).map_err(|_| "cache_unavailable")?;
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "cache_unavailable")?;
    Snapshot::parse(&bytes, false).map_err(|_| "invalid_catalogue")
}
impl CatalogueClient {
    pub fn new(cache: Option<PathBuf>, origin: String) -> Self {
        let bundled = Snapshot::parse(include_bytes!("../../../data/registry.json"), false)
            .expect("valid bundled catalogue");
        let loaded = cache.as_ref().and_then(|p| load(p).ok());
        let source = if loaded.is_some() { "cache" } else { "bundled" }.to_string();
        let snapshot = loaded.unwrap_or(bundled);
        Self {
            state: State {
                source,
                stale: true,
                error: None,
                revision: snapshot.revision.clone(),
            },
            snapshot,
            cache,
            origin,
        }
    }
    pub fn refresh(&mut self) {
        match self.fetch() {
            Ok(snapshot) => {
                let error = self.save(&snapshot).err().map(str::to_string);
                self.state = State {
                    source: "network".into(),
                    stale: false,
                    error,
                    revision: snapshot.revision.clone(),
                };
                self.snapshot = snapshot;
            }
            Err(code) => {
                self.state.stale = true;
                self.state.error = Some(code.into());
            }
        }
    }
    fn fetch(&self) -> Result<Snapshot, &'static str> {
        // Origin is operator configuration, never taken from a catalogue or URI.
        let url = Url::parse(&self.origin).map_err(|_| "invalid_origin")?;
        if !https(&self.origin) {
            return Err("invalid_origin");
        }
        let host = url.host_str().ok_or("invalid_origin")?;
        let started = std::time::Instant::now();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        let target = (host.to_owned(), url.port_or_known_default().unwrap_or(443));
        std::thread::spawn(move || {
            let result = target.to_socket_addrs().map(|a| a.collect::<Vec<_>>());
            let _ = send.send(result);
        });
        let addresses = receive
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "network_unavailable")?
            .map_err(|_| "network_unavailable")?;
        if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
            return Err("unsafe_origin");
        }
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10).saturating_sub(started.elapsed()))
            .connect_timeout(Duration::from_secs(5))
            .resolve_to_addrs(host, &addresses)
            .build()
            .map_err(|_| "network_unavailable")?;
        let response = client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .map_err(|_| "network_unavailable")?;
        if !response.status().is_success() {
            return Err("catalogue_unavailable");
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_BYTES as u64)
        {
            return Err("catalogue_too_large");
        }
        let mut bytes = Vec::new();
        response
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "network_unavailable")?;
        Snapshot::parse(&bytes, false).map_err(|_| "invalid_catalogue")
    }
    fn save(&self, snapshot: &Snapshot) -> Result<(), &'static str> {
        let Some(path) = &self.cache else {
            return Ok(());
        };
        let parent = path.parent().ok_or("cache_unavailable")?;
        fs::create_dir_all(parent).map_err(|_| "cache_unavailable")?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut f = options.open(&temporary).map_err(|_| "cache_unavailable")?;
            f.write_all(&serde_json::to_vec(snapshot).unwrap())
                .and_then(|_| f.sync_all())
                .map_err(|_| "cache_unavailable")?;
            fs::rename(&temporary, path).map_err(|_| "cache_unavailable")
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_roundtrip_and_corruption_are_recoverable() {
        let dir = std::env::temp_dir().join(format!("omastore-cache-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("catalogue.json");
        let client = CatalogueClient::new(Some(path.clone()), DEFAULT_ORIGIN.into());
        client.save(&client.snapshot).unwrap();
        let cached = CatalogueClient::new(Some(path.clone()), DEFAULT_ORIGIN.into());
        assert_eq!(cached.state.source, "cache");
        assert_eq!(cached.snapshot.etag(), client.snapshot.etag());
        fs::write(&path, b"broken JSON").unwrap();
        let recovered = CatalogueClient::new(Some(path.clone()), DEFAULT_ORIGIN.into());
        assert_eq!(recovered.state.source, "bundled");
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn forbids_internal_networks() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "fd00::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()));
        }
    }
    #[test]
    fn corrupt_cache_and_failed_refresh_keep_bundled_snapshot() {
        let mut c = CatalogueClient::new(None, "http://127.0.0.1".into());
        let before = c.snapshot.etag();
        c.refresh();
        assert_eq!(c.snapshot.etag(), before);
        assert!(c.state.stale);
        assert_eq!(c.state.error.as_deref(), Some("invalid_origin"));
    }
}
