//! Local preparation is neither a submission nor a publisher claim.
use crate::{Catalogue, FieldError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Fields {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub category: String,
    pub app_type: String,
    pub maturity: String,
    pub maker_name: String,
    pub maker_homepage: String,
    pub homepage: String,
    pub source: String,
    pub support: String,
    pub licence: String,
    pub licence_id: String,
    pub licence_url: String,
    pub version: String,
    pub identity: String,
    pub commit: String,
    pub sha256: String,
    pub repository: String,
    pub package: String,
    pub architecture: String,
    pub offline: String,
    pub account: String,
    pub activation: String,
    pub service_costs: String,
    pub removal: String,
    pub model: String,
    pub price: String,
    pub currency: String,
    pub offer_url: String,
    pub terms: String,
    pub refund: String,
    pub cancellation: String,
    pub billing_interval: String,
}

pub fn prepare(fields: Fields) -> Value {
    let mut errors = Vec::new();
    for (path, value) in [
        ("id", &fields.id),
        ("name", &fields.name),
        ("summary", &fields.summary),
        ("description", &fields.description),
        ("makerName", &fields.maker_name),
        ("version", &fields.version),
        ("serviceCosts", &fields.service_costs),
        ("removal", &fields.removal),
    ] {
        if value.trim().is_empty() {
            errors.push(FieldError {
                path: path.into(),
                code: "required".into(),
            });
        }
    }
    for (path, value) in [
        ("homepage", &fields.homepage),
        ("makerHomepage", &fields.maker_homepage),
        ("support", &fields.support),
        ("offerUrl", &fields.offer_url),
        ("terms", &fields.terms),
    ] {
        if !crate::public_url(value) {
            errors.push(FieldError {
                path: path.into(),
                code: "https_url_required".into(),
            });
        }
    }
    if !errors.is_empty() {
        return json!({"valid": false, "errors": errors});
    }
    let optional = |s: &str| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_owned())
        }
    };
    let identity = match fields.identity.as_str() {
        "source_commit" => {
            json!({"kind": "source_commit", "repository": fields.source, "commit": fields.commit})
        }
        "binary_artifact" => {
            json!({"kind": "binary_artifact", "publisher": "proposed-maker", "version": fields.version, "sha256": fields.sha256})
        }
        "repository_package" => {
            json!({"kind": "repository_package", "repository": fields.repository, "package": fields.package, "version": fields.version, "signature": null})
        }
        _ => {
            return json!({"valid": false, "errors": [{"path": "identity", "code": "release_identity_required"}]})
        }
    };
    let price = if fields.price.is_empty() {
        None
    } else {
        let exponent = match fields.currency.as_str() {
            "JPY" => 0,
            "KWD" => 3,
            _ => 2,
        };
        let Some(amount) = parse_price(&fields.price, exponent) else {
            return json!({"valid": false, "errors": [{"path": "price", "code": "invalid_decimal_price"}]});
        };
        Some(json!({"currency": fields.currency, "minorUnits": amount, "exponent": exponent}))
    };
    let release = json!({"id": "proposed-release", "version": fields.version, "identity": identity, "architectures": [fields.architecture],
        "route": {"kind": "upstream_external", "url": fields.homepage}, "notes": "Locally prepared candidate. Awaiting author review and publication workflow.",
        "offline": fields.offline, "account": fields.account, "activation": fields.activation, "serviceCosts": fields.service_costs,
        "privileges": [], "services": [], "removal": fields.removal});
    let offer = json!({"sellerId": "proposed-maker", "model": fields.model, "url": fields.offer_url, "price": price,
        "billingInterval": optional(&fields.billing_interval), "tax": "unknown", "checkedAt": null,
        "entitlement": "Check the author's terms. This local record is not an entitlement.",
        "terms": fields.terms, "refund": optional(&fields.refund), "cancellation": optional(&fields.cancellation)});
    let app = json!({"id": fields.id, "slug": fields.id, "name": fields.name, "summary": fields.summary, "description": fields.description,
        "tags": [], "category": fields.category, "appType": fields.app_type, "maturity": fields.maturity,
        "licence": {"class": fields.licence, "identifier": optional(&fields.licence_id), "url": optional(&fields.licence_url)},
        "makerIds": ["proposed-maker"], "homepage": fields.homepage, "source": optional(&fields.source), "support": fields.support,
        "media": [], "capabilities": [], "currentReleaseId": "proposed-release", "tests": [], "releases": [release], "offers": [offer]});
    let proposed = json!({"schemaVersion": 1, "channel": "development", "revision": "local-proposal", "buildRevision": "preparation-v1",
        "generatedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "makers": [{"id": "proposed-maker", "slug": "proposed-maker", "name": fields.maker_name, "bio": "Project control has not been verified.",
            "homepage": fields.maker_homepage, "claim": "unclaimed", "claimEvidence": null}], "recipes": [], "editorial": [], "apps": [app]});
    match Catalogue::parse(&serde_json::to_vec(&proposed).unwrap(), true) {
        Ok(c) => {
            json!({"valid": true, "candidateDigest": c.apps[0].candidate_digest(), "candidate": c,
            "warnings": ["Saved locally only; nothing has been submitted.", "Project control is unclaimed.", "Real media, capabilities, release notes and Omarchy evidence still need review.", "This development candidate cannot enter the public catalogue."]})
        }
        Err(errors) => json!({"valid": false, "errors": errors}),
    }
}

fn parse_price(s: &str, exponent: usize) -> Option<u64> {
    if s.is_empty() || s.len() > 16 {
        return None;
    }
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() > 2 || parts[0].is_empty() || !parts[0].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fraction = parts.get(1).copied().unwrap_or("");
    if fraction.len() > exponent
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || (parts.len() == 2 && fraction.is_empty())
    {
        return None;
    }
    let whole = parts[0]
        .parse::<u64>()
        .ok()?
        .checked_mul(10u64.pow(exponent as u32))?;
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u64>()
            .ok()?
            .checked_mul(10u64.pow((exponent - fraction.len()) as u32))?
    };
    whole.checked_add(fraction)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_is_exact_and_never_float_rounded() {
        assert_eq!(parse_price("19.99", 2), Some(1999));
        assert_eq!(parse_price("19.9", 2), Some(1990));
        assert_eq!(parse_price("1500", 0), Some(1500));
        assert_eq!(parse_price("1.234", 3), Some(1234));
        for value in ["NaN", "-3", "1.234", "1e2", "1.", ".5", "1,99"] {
            assert_eq!(parse_price(value, 2), None);
        }
    }
    #[test]
    fn local_preparation_cannot_turn_into_claimed_or_public_data() {
        let fields: Fields = serde_json::from_value(json!({"id": "sample-app", "name": "Sample", "summary": "Sample purpose", "description": "Example description", "category": "writing", "appType": "desktop", "maturity": "preview", "makerName": "Example", "makerHomepage": "https://example.com", "homepage": "https://example.com", "source": "https://example.com/source", "support": "https://example.com/support", "licence": "open_source", "licenceId": "MIT", "version": "one", "identity": "source_commit", "commit": "a".repeat(40), "architecture": "x86_64", "offline": "yes", "account": "no", "activation": "no", "serviceCosts": "None", "removal": "Follow upstream instructions", "model": "free", "offerUrl": "https://example.com", "terms": "https://example.com/terms"})).unwrap();
        let result = prepare(fields);
        assert_eq!(result["valid"], true);
        let bytes = serde_json::to_vec(&result["candidate"]).unwrap();
        let c = Catalogue::parse(&bytes, true).unwrap();
        assert_eq!(c.makers[0].claim, crate::Claim::Unclaimed);
        assert_eq!(c.apps[0].evidence(chrono::Utc::now()).0, "not_tested");
        assert!(Catalogue::parse(&bytes, false).is_err());
        assert_eq!(prepare(Fields::default())["valid"], false);
    }
}
