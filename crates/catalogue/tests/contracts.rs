use omastore_catalogue::*;
use serde_json::{json, Value};
fn fixture() -> Value {
    serde_json::from_slice(include_bytes!("../../../tests/fixtures/catalogue.json")).unwrap()
}
fn parse(v: &Value) -> Result<Snapshot, Vec<FieldError>> {
    Snapshot::parse(&serde_json::to_vec(v).unwrap(), true)
}
#[test]
fn fixtures_cannot_enter_public_loader() {
    assert!(Snapshot::parse(include_bytes!("../../../data/registry.json"), false).is_ok());
    assert!(Snapshot::parse(&serde_json::to_vec(&fixture()).unwrap(), false).is_err());
    let s = parse(&fixture()).unwrap();
    assert_eq!(s.apps[0].releases.len(), 3);
    assert_eq!(
        s.etag(),
        parse(&serde_json::to_value(&s).unwrap()).unwrap().etag()
    );
}
#[test]
fn rejects_duplicates_references_money_tokens_and_private_fields() {
    let mut v = fixture();
    let copy = v["apps"][0].clone();
    v["apps"].as_array_mut().unwrap().push(copy);
    assert!(parse(&v).is_err());
    for (field, value) in [
        ("currentRelease", json!("missing")),
        ("makerIds", json!(["missing"])),
    ] {
        let mut v = fixture();
        v["apps"][0][field] = value;
        assert!(parse(&v).is_err());
    }
    for money in [json!(1.25), json!(-1), json!("100")] {
        let mut v = fixture();
        v["apps"][0]["offers"][0]["amountMinor"] = money;
        assert!(parse(&v).is_err());
    }
    for bad in ["--root", "app;touch /tmp/pwned", "$(id)", "../app", "app\n"] {
        let mut v = fixture();
        v["apps"][0]["releases"][0]["route"]["package"] = json!(bad);
        assert!(parse(&v).is_err());
    }
    for field in ["token", "reviewerContact", "draft", "command"] {
        let mut v = fixture();
        v["apps"][0][field] = json!("private");
        assert!(parse(&v).is_err());
    }
    let mut v = fixture();
    v["schemaVersion"] = json!(2);
    assert!(parse(&v).is_err());
}
#[test]
fn evidence_is_bound_to_bytes_and_release_not_dates() {
    let mut v = fixture();
    let now = chrono::DateTime::parse_from_rfc3339("2026-09-11T00:00:00Z")
        .unwrap()
        .to_utc();
    assert_eq!(
        parse(&v).unwrap().apps[0].evidence_status(now).0,
        TestResult::NotTested
    );
    v["apps"][0]["evidence"] = json!([{"id":"test-1","releaseId":"release-1","executedIdentity":v["apps"][0]["releases"][0]["identity"],"candidateSha256":"c".repeat(64),"tool":"manual-v1","actor":"reviewer","environment":"Omarchy fixture VM","result":"passes","testedAt":"2026-09-10T00:00:00Z","limitations":[],"reference":"https://example.com/test"}]);
    assert_eq!(
        parse(&v).unwrap().apps[0].evidence_status(now),
        (TestResult::Passes, Freshness::Current)
    );
    v["apps"][0]["currentRelease"] = json!("release-2");
    assert_eq!(
        parse(&v).unwrap().apps[0].evidence_status(now),
        (TestResult::NotTested, Freshness::Superseded)
    );
    v["apps"][0]["currentRelease"] = json!("release-1");
    v["apps"][0]["evidence"][0]["testedAt"] = json!("2020-01-01T00:00:00Z");
    assert_eq!(
        parse(&v).unwrap().apps[0].evidence_status(now).1,
        Freshness::RetestDue
    );
    v["apps"][0]["evidence"][0]["executedIdentity"]["package"] = json!("other");
    assert!(parse(&v).is_err());
}
