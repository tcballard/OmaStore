use crate::*;
use serde_json::{json, Value};

record!(Query { text: String, category: String, app_type: String, license: String,
    architecture: String, pricing: String, offline: String, test_result: String,
    cursor: Option<String>, limit: usize });
impl Default for Query {
    fn default() -> Self {
        Self {
            text: String::new(),
            category: String::new(),
            app_type: String::new(),
            license: String::new(),
            architecture: String::new(),
            pricing: String::new(),
            offline: String::new(),
            test_result: String::new(),
            cursor: None,
            limit: 20,
        }
    }
}
record!(Page { revision: String, apps: Vec<App>, total: usize, next_cursor: Option<String> });
fn name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
pub fn query(snapshot: &Snapshot, q: &Query) -> Result<Page, &'static str> {
    if q.text.len() > 256
        || q.category.len() > 64
        || q.limit == 0
        || q.limit > 50
        || !["", "desktop", "terminal", "shell_plugin", "web", "service"]
            .contains(&q.app_type.as_str())
        || ![
            "",
            "open_source",
            "source_available",
            "proprietary",
            "unknown",
        ]
        .contains(&q.license.as_str())
        || !["", "x86_64", "aarch64", "any", "unknown"].contains(&q.architecture.as_str())
        || !["", "yes", "no", "unknown"].contains(&q.offline.as_str())
        || !["", "passes", "limitations", "fails", "not_tested"].contains(&q.test_result.as_str())
        || ![
            "",
            "free",
            "voluntary_support",
            "pay_what_you_want",
            "paid_app",
            "paid_upgrade",
            "paid_features",
            "subscription",
            "professional_services",
            "paid_preview",
            "unknown",
        ]
        .contains(&q.pricing.as_str())
    {
        return Err("invalid_filter");
    }
    let mut base = q.clone();
    base.cursor = None;
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&base).unwrap()));
    let etag = snapshot.etag();
    let revision = etag.trim_matches('"');
    let offset = if let Some(cursor) = &q.cursor {
        if cursor.len() > 180 {
            return Err("invalid_cursor");
        }
        let parts: Vec<_> = cursor.split(':').collect();
        if parts.len() != 3 {
            return Err("invalid_cursor");
        }
        if parts[0] != revision {
            return Err("snapshot_changed");
        }
        if parts[1] != fingerprint {
            return Err("cursor_query_changed");
        }
        parts[2].parse::<usize>().map_err(|_| "invalid_cursor")?
    } else {
        0
    };
    let text = q.text.trim().to_lowercase();
    let mut matches: Vec<_> = snapshot
        .apps
        .iter()
        .filter_map(|app| {
            if !q.category.is_empty() && !app.category.eq_ignore_ascii_case(&q.category) {
                return None;
            }
            if !q.app_type.is_empty() && name(&app.app_type) != q.app_type {
                return None;
            }
            if !q.license.is_empty() && name(&app.license.class) != q.license {
                return None;
            }
            let release = app.release();
            if !q.architecture.is_empty()
                && !release.is_some_and(|r| {
                    r.architectures.contains(&q.architecture)
                        || r.architectures.contains(&"any".into())
                })
            {
                return None;
            }
            if !q.offline.is_empty()
                && release
                    .map(|r| name(&r.offline))
                    .unwrap_or("unknown".into())
                    != q.offline
            {
                return None;
            }
            if !q.pricing.is_empty()
                && !(app.offers.is_empty() && q.pricing == "unknown"
                    || app.offers.iter().any(|o| name(&o.model) == q.pricing))
            {
                return None;
            }
            if !q.test_result.is_empty()
                && name(&app.evidence_status(Utc::now()).0) != q.test_result
            {
                return None;
            }
            let n = app.name.to_lowercase();
            let rank = if n == text || app.id.to_lowercase() == text {
                0
            } else if n.starts_with(&text) {
                1
            } else if n.contains(&text) {
                2
            } else if format!("{} {} {}", app.summary, app.description, app.tags.join(" "))
                .to_lowercase()
                .contains(&text)
            {
                3
            } else {
                return None;
            };
            Some((rank, n, app))
        })
        .collect();
    matches.sort_by(|a, b| (&a.0, &a.1, &a.2.id).cmp(&(&b.0, &b.1, &b.2.id)));
    if offset > matches.len() {
        return Err("invalid_cursor");
    }
    let end = (offset + q.limit).min(matches.len());
    Ok(Page {
        revision: snapshot.revision.clone(),
        apps: matches[offset..end]
            .iter()
            .map(|(_, _, a)| (*a).clone())
            .collect(),
        total: matches.len(),
        next_cursor: if end < matches.len() {
            Some(format!("{revision}:{fingerprint}:{end}"))
        } else {
            None
        },
    })
}
pub fn detail(app: &App) -> Value {
    let mut v = serde_json::to_value(app).unwrap();
    let (result, freshness) = app.evidence_status(Utc::now());
    v["testResult"] = json!(result);
    v["testFreshness"] = json!(freshness);
    let route = app.release().and_then(|r| r.route.as_ref());
    v["actionLabel"] = json!(route.map(|r| r.label()).unwrap_or("View source"));
    v["actionUrl"] = json!(route.map(|r| r.url()).unwrap_or(&app.source));
    v["managedInstall"] = json!(false);
    v
}
