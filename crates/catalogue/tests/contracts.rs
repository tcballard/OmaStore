use chrono::{TimeZone, Utc};
use omastore_catalogue::*;
use serde_json::{json, Value};

fn fixture() -> Catalogue {
    Catalogue::parse(
        include_bytes!("../../../tests/fixtures/catalogue.json"),
        true,
    )
    .unwrap()
}

#[test]
fn empty_public_and_complete_example_are_valid_but_demo_is_not_public() {
    assert!(
        Catalogue::parse(include_bytes!("../../../data/registry.json"), false)
            .unwrap()
            .apps
            .is_empty()
    );
    assert!(Catalogue::parse(
        include_bytes!("../../../docs/examples/submission.json"),
        true
    )
    .is_ok());
    assert!(Catalogue::parse(
        include_bytes!("../../../tests/fixtures/catalogue.json"),
        false
    )
    .unwrap_err()
    .iter()
    .any(|e| e.code == "development_data_forbidden"));
    let c = fixture();
    assert!(c.apps.iter().any(|a| matches!(
        a.current_release().identity,
        ReleaseIdentity::SourceCommit { .. }
    )));
    assert!(c.apps.iter().any(|a| matches!(
        a.current_release().identity,
        ReleaseIdentity::BinaryArtifact { .. }
    )));
    assert!(c.apps.iter().any(|a| matches!(
        a.current_release().identity,
        ReleaseIdentity::RepositoryPackage { .. }
    )));
}

#[test]
fn unsafe_or_ambiguous_records_fail_with_field_codes() {
    let base = serde_json::to_value(fixture()).unwrap();
    let cases: Vec<(&str, Value)> = vec![
        ("/schemaVersion", json!(99)),
        ("/apps/1/id", json!("demo-fieldnotes")),
        ("/apps/1/slug", json!("fieldnotes")),
        ("/apps/0/currentReleaseId", json!("missing")),
        ("/apps/0/makerIds/0", json!("missing")),
        ("/apps/0/releases/0/route/package", json!("--overwrite=*")),
        ("/apps/0/releases/0/identity/version", json!("")),
        ("/apps/0/homepage", json!("file:///etc/passwd")),
        ("/apps/0/homepage", json!("https://127.0.0.1/")),
        ("/apps/1/offers/0/price/minorUnits", json!(24.5)),
        ("/apps/1/offers/0/price/minorUnits", json!(-1)),
        ("/apps/1/offers/0/price/exponent", json!(0)),
        (
            "/apps/0/offers/0/price",
            json!({"currency": "USD", "minorUnits": 100, "exponent": 2}),
        ),
        ("/recipes/0/components/0/releaseId", json!("missing")),
        ("/makers/0/claim", json!("verified")),
    ];
    for (path, value) in cases {
        let mut c = base.clone();
        *c.pointer_mut(path).unwrap() = value;
        assert!(
            Catalogue::parse(&serde_json::to_vec(&c).unwrap(), true).is_err(),
            "accepted {path}"
        );
    }
    let mut c = base;
    c["apps"][0]["privateToken"] = json!("must-never-leak");
    assert!(Catalogue::parse(&serde_json::to_vec(&c).unwrap(), true).is_err());
}

#[test]
fn current_evidence_requires_same_candidate_bytes_and_recent_environment_record() {
    let now = Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
    let mut app = fixture().apps.remove(0);
    assert_eq!(app.evidence(now).0, "not_tested");
    app.tests.push(TestRecord {
        release_id: app.current_release_id.clone(),
        candidate_digest: app.candidate_digest(),
        executed_identity: app.current_release().identity.clone(),
        result: TestResult::Passes,
        freshness: Freshness::Current,
        tested_at: "2026-09-07T00:00:00Z".into(),
        environment: "Synthetic test environment".into(),
        tool_version: "fixture-1".into(),
        actor: "fixture".into(),
        evidence: "https://example.com/evidence".into(),
        limitations: "Fixture only".into(),
    });
    assert_eq!(app.evidence(now).0, "passes");
    app.tests[0].result = TestResult::Fails;
    app.tests.push(TestRecord {
        environment: "Another fixture environment".into(),
        result: TestResult::Passes,
        tested_at: "2026-09-08T00:00:00Z".into(),
        ..app.tests[0].clone()
    });
    assert_eq!(app.evidence(now).0, "passes");
    assert_eq!(
        app.evidence_for_profile(now, Some("Synthetic test environment"))
            .0,
        "fails"
    );
    app.tests.pop();
    app.tests[0].result = TestResult::Passes;
    app.tests[0].tested_at = "2025-09-07T00:00:00Z".into();
    assert_eq!(app.evidence(now).0, "retest_due");
    app.tests[0].tested_at = "2026-09-07T00:00:00Z".into();
    app.releases[0].services.push("new-service".into());
    assert_eq!(app.evidence(now).0, "not_tested");
    app.tests[0].candidate_digest = app.candidate_digest();
    app.tests[0].executed_identity = fixture().apps[1].current_release().identity.clone();
    assert_eq!(app.evidence(now).0, "not_tested");
}

#[test]
fn canonical_snapshot_is_stable_and_recipe_cycles_fail() {
    let c = fixture();
    let mut reversed = c.clone();
    reversed.apps.reverse();
    assert_eq!(c.snapshot_id(), reversed.snapshot_id());
    assert_eq!(
        c.canonical_bytes(),
        Catalogue::parse(&c.canonical_bytes(), true)
            .unwrap()
            .canonical_bytes()
    );
    let mut cycle = c;
    cycle.recipes[0].components[0]
        .depends_on
        .push("demo-papertrail".into());
    cycle.recipes[0].components[1]
        .depends_on
        .push("demo-fieldnotes".into());
    assert!(cycle
        .validate(true)
        .iter()
        .any(|e| e.code == "cyclic_components"));
}
