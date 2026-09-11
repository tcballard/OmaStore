use crate::{
    query::{self, Query},
    *,
};
use serde_json::{json, Value};

pub fn read(snapshot: &Snapshot, path: &str, query_string: &str) -> Result<Value, &'static str> {
    if path.len() + query_string.len() > 4096 {
        return Err("invalid_request");
    }
    if path == "/api/v1/catalogue" {
        return Ok(serde_json::to_value(snapshot).unwrap());
    }
    if path == "/api/v1/apps" {
        let mut q = Query::default();
        let mut seen = HashSet::new();
        for (key, value) in url::form_urlencoded::parse(query_string.as_bytes()) {
            if !seen.insert(key.clone()) {
                return Err("invalid_filter");
            }
            match key.as_ref() {
                "text" => q.text = value.into_owned(),
                "category" => q.category = value.into_owned(),
                "appType" => q.app_type = value.into_owned(),
                "license" => q.license = value.into_owned(),
                "architecture" => q.architecture = value.into_owned(),
                "pricing" => q.pricing = value.into_owned(),
                "offline" => q.offline = value.into_owned(),
                "testResult" => q.test_result = value.into_owned(),
                "cursor" => q.cursor = Some(value.into_owned()),
                "limit" => q.limit = value.parse().map_err(|_| "invalid_filter")?,
                _ => return Err("invalid_filter"),
            }
        }
        return query::query(snapshot, &q).map(|p| serde_json::to_value(p).unwrap());
    }
    if let Some(id) = path.strip_prefix("/api/v1/apps/") {
        return snapshot
            .apps
            .iter()
            .find(|a| a.id == id || a.slug == id)
            .map(|a| serde_json::to_value(a).unwrap())
            .ok_or("not_found");
    }
    if let Some(id) = path.strip_prefix("/api/v1/makers/") {
        return snapshot
            .makers
            .iter()
            .find(|m| m.id == id)
            .map(|m| serde_json::to_value(m).unwrap())
            .ok_or("not_found");
    }
    if let Some(id) = path.strip_prefix("/api/v1/setups/") {
        return snapshot
            .recipes
            .iter()
            .find(|r| r.id == id || r.slug == id)
            .map(|r| serde_json::to_value(r).unwrap())
            .ok_or("not_found");
    }
    Err("not_found")
}
pub fn failure(code: &str) -> Value {
    json!({"error":{"code":code}})
}
