//! Pure HTTP routing shared by the actual service entry and contract tests.
use crate::{query, Catalogue};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub struct Response {
    pub status: u16,
    pub etag: String,
    pub body: Vec<u8>,
}

pub fn handle(
    catalogue: &Catalogue,
    method: &str,
    target: &str,
    if_none_match: Option<&str>,
    now: DateTime<Utc>,
) -> Response {
    let route = || -> Result<Value, &'static str> {
        if method != "GET" {
            return Err("method_not_allowed");
        }
        if target.len() > 4096 || !target.starts_with('/') || target.starts_with("//") {
            return Err("invalid_request");
        }
        let (path, raw) = target.split_once('?').unwrap_or((target, ""));
        if path == "/api/v1/apps" {
            let mut params = serde_json::Map::new();
            let mut seen = HashSet::new();
            for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
                if !seen.insert(key.to_string()) {
                    return Err("invalid_filter");
                }
                let v = if key == "limit" {
                    json!(value.parse::<usize>().map_err(|_| "invalid_filter")?)
                } else {
                    json!(value)
                };
                params.insert(key.into_owned(), v);
            }
            let q: query::Query =
                serde_json::from_value(Value::Object(params)).map_err(|_| "invalid_filter")?;
            return query::list(catalogue, &q, now);
        }
        if path == "/api/v1/makers" || path == "/api/v1/setups" {
            let mut params = serde_json::Map::new();
            for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
                let val = if key == "offset" {
                    json!(value.parse::<usize>().map_err(|_| "invalid_filter")?)
                } else {
                    json!(value)
                };
                if params.insert(key.into_owned(), val).is_some() {
                    return Err("invalid_filter");
                }
            }
            let q = serde_json::from_value(Value::Object(params)).map_err(|_| "invalid_filter")?;
            return if path.ends_with("makers") {
                crate::editorial::makers(catalogue, &q, now)
            } else {
                crate::setups::list(catalogue, &q)
            };
        }
        if !raw.is_empty() {
            return Err("invalid_filter");
        }
        if path == "/api/v1/editorial" {
            return Ok(crate::editorial::stories(catalogue, now));
        }
        if path == "/api/v1/catalogue" {
            return Ok(serde_json::to_value(catalogue).unwrap());
        }
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != 5 || parts[1] != "api" || parts[2] != "v1" || !crate::token(parts[4]) {
            return Err("not_found");
        }
        match parts[3] {
            "apps" => query::app_detail(catalogue, parts[4], now),
            "editorial" => crate::editorial::story(catalogue, parts[4], now),
            "makers" => crate::editorial::maker_detail(catalogue, parts[4], now),
            "setups" => catalogue
                .recipes
                .iter()
                .find(|m| m.id == parts[4] || m.slug == parts[4])
                .map(|m| json!(m))
                .ok_or("not_found"),
            _ => Err("not_found"),
        }
    };
    match route() {
        Ok(value) => {
            let body = serde_json::to_vec(&value).unwrap();
            let etag = format!("\"{:x}\"", Sha256::digest(&body));
            let unchanged = if_none_match.is_some_and(|s| {
                s.split(',')
                    .any(|t| t.trim().trim_start_matches("W/") == etag || t.trim() == "*")
            });
            Response {
                status: if unchanged { 304 } else { 200 },
                etag,
                body: if unchanged { Vec::new() } else { body },
            }
        }
        Err(code) => Response {
            status: match code {
                "not_found" => 404,
                "method_not_allowed" => 405,
                "snapshot_changed" | "cursor_expired" => 409,
                _ => 400,
            },
            etag: String::new(),
            body: serde_json::to_vec(&json!({"error": {"code": code}})).unwrap(),
        },
    }
}
