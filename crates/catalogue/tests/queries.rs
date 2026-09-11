use chrono::{TimeZone, Utc};
use omastore_catalogue::{
    http,
    query::{self, Query},
    Catalogue,
};
use serde_json::{json, Value};

fn fixture() -> Catalogue {
    Catalogue::parse(
        include_bytes!("../../../tests/fixtures/catalogue.json"),
        true,
    )
    .unwrap()
}

#[test]
fn native_and_http_queries_share_order_and_pagination_rejects_changed_snapshots() {
    let mut c = fixture();
    let now = Utc.with_ymd_and_hms(2026, 9, 8, 12, 0, 0).unwrap();
    let q = Query {
        limit: 2,
        ..Query::default()
    };
    let page = query::list(&c, &q, now).unwrap();
    let http = http::handle(&c, "GET", "/api/v1/apps?limit=2", None, now);
    assert_eq!(http.status, 200);
    assert_eq!(page, serde_json::from_slice::<Value>(&http.body).unwrap());
    let mut next = q.clone();
    next.cursor = page["nextCursor"].as_str().map(str::to_owned);
    let second = query::list(&c, &next, now).unwrap();
    assert_ne!(page["items"][0]["id"], second["items"][0]["id"]);
    next.q = "changed".into();
    assert_eq!(query::list(&c, &next, now).unwrap_err(), "invalid_cursor");
    next.q.clear();
    c.apps[0].summary = "new description".into();
    assert_eq!(query::list(&c, &next, now).unwrap_err(), "snapshot_changed");
}

#[test]
fn filters_errors_conditionals_and_private_field_projection() {
    let c = fixture();
    let now = Utc::now();
    for route in [
        "/api/v1/apps?limit=0",
        "/api/v1/apps?limit=51",
        "/api/v1/apps?price=cheap",
        "/api/v1/apps?token=secret",
        "/api/v1/apps?q=a&q=b",
        "/api/v1/apps?cursor=broken",
    ] {
        assert_eq!(
            http::handle(&c, "GET", route, None, now).status,
            400,
            "{route}"
        );
    }
    let page = query::list(
        &c,
        &Query {
            q: "fieldnotes".into(),
            ..Query::default()
        },
        now,
    )
    .unwrap();
    assert_eq!(page["items"][0]["id"], "demo-fieldnotes");
    let first = http::handle(&c, "GET", "/api/v1/catalogue", None, now);
    assert_eq!(
        http::handle(&c, "GET", "/api/v1/catalogue", Some(&first.etag), now).status,
        304
    );
    assert_eq!(
        http::handle(&c, "POST", "/api/v1/catalogue", None, now).status,
        405
    );
    let detail = query::app_detail(&c, "branchline", now).unwrap();
    assert!(detail["summary"]["routeLabel"]
        .as_str()
        .unwrap()
        .starts_with("AUR"));
    assert_eq!(detail["acquisition"]["kind"], "external");
    let mut private = json!(c);
    private["reviewerContact"] = json!("secret@example.com");
    assert!(Catalogue::parse(&serde_json::to_vec(&private).unwrap(), true).is_err());
    private.as_object_mut().unwrap().remove("reviewerContact");
    private["apps"][0]["draft"] = json!({"token": "secret"});
    assert!(Catalogue::parse(&serde_json::to_vec(&private).unwrap(), true).is_err());
}

#[test]
#[ignore = "recorded performance sample; run with --release --ignored --nocapture"]
fn thousand_entry_query_sample() {
    let mut c = fixture();
    c.apps.clear();
    c.recipes.clear();
    let template = fixture().apps.remove(0);
    for n in 0..1000 {
        let mut app = template.clone();
        app.id = format!("sample-{n:04}");
        app.slug = app.id.clone();
        app.name = format!("Sample {n:04}");
        c.apps.push(app);
    }
    let q = Query {
        q: "sample".into(),
        ..Query::default()
    };
    let mut search = Vec::new();
    let mut api = Vec::new();
    for _ in 0..100 {
        let start = std::time::Instant::now();
        std::hint::black_box(query::list(&c, &q, Utc::now()).unwrap());
        search.push(start.elapsed().as_micros());
        let start = std::time::Instant::now();
        std::hint::black_box(http::handle(
            &c,
            "GET",
            "/api/v1/apps?q=sample",
            None,
            Utc::now(),
        ));
        api.push(start.elapsed().as_micros());
    }
    search.sort();
    api.sort();
    println!("1000 entries, 100 samples; shared query p95={}us; HTTP handler p95={}us (excludes socket/proxy and disk)", search[94], api[94]);
}
