use omastore_catalogue::{
    api,
    query::{query, Query},
    Snapshot,
};
use serde_json::{json, Value};
fn fixture() -> Snapshot {
    Snapshot::parse(
        include_bytes!("../../../tests/fixtures/catalogue.json"),
        true,
    )
    .unwrap()
}
#[test]
fn api_and_native_share_order_and_cursor_identity() {
    let mut s = fixture();
    let mut b = s.apps[0].clone();
    b.id = "second".into();
    b.slug = "second".into();
    b.name = "A second app".into();
    s.apps.push(b);
    let q = Query {
        limit: 1,
        ..Query::default()
    };
    let page = query(&s, &q).unwrap();
    assert_eq!(
        serde_json::to_value(&page).unwrap(),
        api::read(&s, "/api/v1/apps", "limit=1").unwrap()
    );
    let next = Query {
        cursor: page.next_cursor,
        ..q
    };
    assert_eq!(query(&s, &next).unwrap().apps[0].id, "fixture-notes");
    s.apps[0].summary = "changed without revision bump".into();
    assert_eq!(query(&s, &next).unwrap_err(), "snapshot_changed");
    for qs in [
        "limit=0",
        "limit=51",
        "limit=-1",
        "offline=sometimes",
        "token=secret",
        "text=a&text=b",
        "cursor=invalid",
    ] {
        assert!(api::read(&s, "/api/v1/apps", qs).is_err());
    }
}
#[test]
fn public_read_fields_and_unknowns() {
    let s = fixture();
    assert_eq!(
        api::read(&s, "/api/v1/apps", "text=productivity").unwrap()["total"],
        1
    );
    let v = api::read(&s, "/api/v1/apps/fixture-notes", "").unwrap();
    assert_eq!(v["evidence"], json!([]));
    assert!(v.get("token").is_none());
    assert_eq!(
        api::read(&s, "/api/v1/apps", "testResult=not_tested").unwrap()["total"],
        1
    );
}
#[test]
fn query_thousand_entries_latency() {
    let mut s = fixture();
    let template = s.apps.pop().unwrap();
    for i in 0..1000 {
        let mut app = template.clone();
        app.id = format!("app-{i}");
        app.slug = app.id.clone();
        app.name = format!("Notes {i:04}");
        s.apps.push(app);
    }
    let mut samples = Vec::new();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        let page = query(
            &s,
            &Query {
                text: "notes".into(),
                ..Query::default()
            },
        )
        .unwrap();
        assert_eq!(page.total, 1000);
        samples.push(start.elapsed().as_micros());
    }
    samples.sort();
    eprintln!("1000 records, 100 samples, p95={}us", samples[94]);
}
#[test]
fn rejects_unknown_nested_workflow_fields() {
    let mut v: Value = serde_json::to_value(fixture()).unwrap();
    v["makers"][0]["token"] = json!("PRIVATE");
    assert!(Snapshot::parse(&serde_json::to_vec(&v).unwrap(), true).is_err());
}
