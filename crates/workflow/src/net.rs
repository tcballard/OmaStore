//! Public-origin reads pin checked DNS results and never follow redirects or proxies.
use crate::{Error, Result};
use reqwest::Client;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;

pub const TEXT_LIMIT: usize = 1024 * 1024;
pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..128).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..32).contains(&b))
                || (a == 192 && (b == 168 || (b == 0 && (c == 0 || c == 2))))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public_ip(IpAddr::V4(v4));
            }
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000 && !(s[0] == 0x2001 && (s[1] == 0xdb8 || s[1] < 0x200))
        }
    }
}
pub fn public_url(input: &str) -> Result<Url> {
    if input.len() > 2048 {
        return Err(Error::new(422, "unsafe_url"));
    }
    let u = Url::parse(input).map_err(|_| Error::new(422, "unsafe_url"))?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.fragment().is_some()
        || u.port().is_some_and(|p| p != 443)
    {
        return Err(Error::new(422, "unsafe_url"));
    }
    if let Some(host) = u.host_str() {
        if host.eq_ignore_ascii_case("localhost")
            || host.ends_with(".localhost")
            || host.ends_with(".local")
            || host.parse::<IpAddr>().is_ok_and(|ip| !public_ip(ip))
        {
            return Err(Error::new(422, "unsafe_url"));
        }
    }
    Ok(u)
}
pub async fn client_for(u: &Url) -> Result<Client> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let host = u.host_str().ok_or(Error::new(422, "unsafe_url"))?;
    let addresses: Vec<SocketAddr> =
        tokio::time::timeout(Duration::from_secs(3), tokio::net::lookup_host((host, 443)))
            .await
            .map_err(|_| Error::new(503, "upstream_timeout"))?
            .map_err(|_| Error::new(503, "upstream_unavailable"))?
            .take(17)
            .collect();
    if addresses.is_empty() || addresses.len() > 16 || addresses.iter().any(|a| !public_ip(a.ip()))
    {
        return Err(Error::new(422, "unsafe_address"));
    }
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(3))
        .gzip(true)
        .user_agent(concat!("OmaStore/", env!("CARGO_PKG_VERSION")))
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|_| Error::new(503, "upstream_unavailable"))
}
pub struct Fetched {
    pub bytes: Vec<u8>,
    pub content_type: String,
}
pub async fn read_response(mut response: reqwest::Response, limit: usize) -> Result<Fetched> {
    if !response.status().is_success() {
        return Err(Error::new(503, "upstream_unavailable"));
    }
    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_owned();
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(Error::new(422, "upstream_too_large"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| Error::new(503, "upstream_unavailable"))?
    {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::new(422, "upstream_too_large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Fetched {
        bytes,
        content_type,
    })
}
pub async fn fetch(input: &str, limit: usize) -> Result<Fetched> {
    let u = public_url(input)?;
    tokio::time::timeout(Duration::from_secs(10), async {
        let response = client_for(&u)
            .await?
            .get(u)
            .send()
            .await
            .map_err(|_| Error::new(503, "upstream_unavailable"))?;
        read_response(response, limit).await
    })
    .await
    .map_err(|_| Error::new(503, "upstream_timeout"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_credentials_and_nonpublic_destinations() {
        for value in [
            "http://example.com",
            "https://user:pass@example.com",
            "https://localhost/",
            "https://127.1/",
            "https://169.254.169.254/latest/",
            "https://example.com:8080/",
        ] {
            assert!(public_url(value).is_err(), "{value}");
        }
        for value in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
        ] {
            assert!(!public_ip(value.parse().unwrap()), "{value}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
    }
}
