use crate::platform::Result;
use omastore_catalogue::Catalogue;
use serde_json::{json, Value};

pub fn parse(raw: &str) -> Result<Value> {
    if raw.len() > 400 || raw.contains('%') || raw.bytes().any(|c| c.is_ascii_control()) {
        return Err("invalid_handoff");
    }
    let uri = url::Url::parse(raw).map_err(|_| "invalid_handoff")?;
    if uri.scheme() != "omastore"
        || uri.port().is_some()
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.fragment().is_some()
    {
        return Err("invalid_handoff");
    }
    let id = uri
        .path()
        .strip_prefix('/')
        .filter(|s| omastore_catalogue::token(s))
        .ok_or("invalid_handoff")?;
    match uri.host_str() {
        Some("app") if uri.query().is_none() && raw == format!("omastore://app/{id}") => {
            Ok(json!({"kind":"app","id":id}))
        }
        Some("setup") => {
            let pairs: Vec<_> = uri.query_pairs().collect();
            if pairs.len() != 1
                || pairs[0].0 != "revision"
                || !omastore_catalogue::token(&pairs[0].1)
                || raw != format!("omastore://setup/{id}?revision={}", pairs[0].1)
            {
                return Err("invalid_handoff");
            }
            Ok(json!({"kind":"setup","id":id,"revision":pairs[0].1}))
        }
        _ => Err("invalid_handoff"),
    }
}
pub fn open(c: &Catalogue, raw: &str) -> Result<Value> {
    let value = parse(raw)?;
    let id = value["id"].as_str().ok_or("invalid_handoff")?;
    let exists = if value["kind"] == "app" {
        c.apps.iter().any(|a| a.id == id)
    } else {
        c.recipes
            .iter()
            .any(|r| r.id == id && value["revision"] == r.revision)
    };
    if !exists {
        return Err("handoff_revision_unavailable");
    }
    Ok(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identities_never_carry_authority() {
        assert_eq!(
            parse("omastore://app/fieldnotes").unwrap()["id"],
            "fieldnotes"
        );
        assert_eq!(
            parse("omastore://setup/desk?revision=2").unwrap()["revision"],
            "2"
        );
        for raw in [
            "omastore://app/x?command=install",
            "omastore://app/x?origin=https://example.com",
            "omastore://app/x#install",
            "omastore://user@app/x",
            "omastore://app:80/x",
            "omastore://app/../x",
            "omastore://app/%2Dx",
            "omastore://setup/x?revision=1&revision=2",
            "omastore://setup/x",
            "file:///tmp/x",
        ] {
            assert!(parse(raw).is_err(), "{raw}");
        }
    }
}
