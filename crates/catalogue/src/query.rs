use crate::*;
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Query {
    pub q: String,
    pub category: Option<String>,
    pub app_type: Option<AppType>,
    pub licence: Option<LicenceClass>,
    pub price: Option<OfferModel>,
    pub architecture: Option<String>,
    pub offline: Option<Behaviour>,
    pub evidence: Option<String>,
    pub profile: Option<String>,
    pub limit: usize,
    pub cursor: Option<String>,
}
impl Default for Query {
    fn default() -> Self {
        Self {
            q: String::new(),
            category: None,
            app_type: None,
            licence: None,
            price: None,
            architecture: None,
            offline: None,
            evidence: None,
            profile: None,
            limit: 20,
            cursor: None,
        }
    }
}

impl Query {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.q.len() > 200
            || !(1..=50).contains(&self.limit)
            || self.category.as_deref().is_some_and(|c| !token(c))
            || self
                .architecture
                .as_deref()
                .is_some_and(|a| !["x86_64", "aarch64", "any"].contains(&a))
            || self.evidence.as_deref().is_some_and(|s| {
                !["passes", "limitations", "fails", "not_tested", "retest_due"].contains(&s)
            })
            || self.profile.as_ref().is_some_and(|s| s.len() > 120)
        {
            Err("invalid_filter")
        } else {
            Ok(())
        }
    }
    fn fingerprint(&self) -> String {
        let mut q = self.clone();
        q.cursor = None;
        format!("{:x}", Sha256::digest(serde_json::to_vec(&q).unwrap()))
    }
}

pub fn summary(app: &App, now: DateTime<Utc>) -> Value {
    let release = app.current_release();
    let (evidence, evidence_label) = app.evidence(now);
    json!({"id": app.id, "slug": app.slug, "name": app.name, "summary": app.summary,
        "category": app.category, "appType": app.app_type, "maturity": app.maturity,
        "licence": app.licence, "makerIds": app.maker_ids, "priceLabel": price_label(app),
        "evidence": evidence, "evidenceLabel": evidence_label, "offline": release.offline,
        "routeLabel": release.route.label(), "architectures": release.architectures})
}

pub fn price_label(app: &App) -> String {
    let Some(offer) = app.offers.first() else {
        return "Price not supplied".into();
    };
    let label = match offer.model {
        OfferModel::Free => "Free",
        OfferModel::Donation => "Free · support welcome",
        OfferModel::PayWhatYouWant => "Pay what you want",
        OfferModel::Paid => "Paid",
        OfferModel::Upgrade => "Paid upgrade",
        OfferModel::PaidFeatures => "Paid features",
        OfferModel::Subscription => "Subscription",
        OfferModel::Service => "Paid service",
        OfferModel::WorkingPreview => "Working preview",
    };
    if let Some(p) = &offer.price {
        let divisor = 10u64.pow(p.exponent.into());
        let amount = if p.exponent == 0 {
            p.minor_units.to_string()
        } else {
            format!(
                "{}.{:0width$}",
                p.minor_units / divisor,
                p.minor_units % divisor,
                width = p.exponent.into()
            )
        };
        let amount = format!(
            "{} {}{}",
            p.currency,
            amount,
            offer
                .billing_interval
                .as_ref()
                .map(|b| format!(" / {b}"))
                .unwrap_or_default()
        );
        match offer.model {
            OfferModel::Free => "Free".into(),
            OfferModel::Donation => format!("Free · optional {amount}"),
            OfferModel::PayWhatYouWant => format!("Pay what you want · reference {amount}"),
            OfferModel::Upgrade | OfferModel::PaidFeatures | OfferModel::WorkingPreview => {
                format!("{label} · {amount}")
            }
            _ => amount,
        }
    } else {
        label.into()
    }
}

pub fn app_detail(
    catalogue: &Catalogue,
    id: &str,
    now: DateTime<Utc>,
) -> Result<Value, &'static str> {
    let app = catalogue
        .apps
        .iter()
        .find(|a| a.id == id || a.slug == id)
        .ok_or("not_found")?;
    let paid = app.offers.first().is_some_and(|o| {
        matches!(
            o.model,
            OfferModel::Paid
                | OfferModel::Subscription
                | OfferModel::Service
                | OfferModel::Upgrade
                | OfferModel::PaidFeatures
        )
    });
    let label = if paid {
        "View seller offer"
    } else {
        match app.current_release().route {
            InstallRoute::AurExternal { .. } => "View AUR instructions",
            InstallRoute::ArchPackage { .. } => "View package instructions",
            InstallRoute::PluginExternal { .. } => "View plugin instructions",
            _ => "Get from developer",
        }
    };
    Ok(json!({"snapshot": catalogue.snapshot_id(), "app": app,
        "release": app.current_release(), "summary": summary(app, now),
        "makers": catalogue.makers.iter().filter(|m| app.maker_ids.contains(&m.id)).map(|m|crate::editorial::maker_summary(m,now)).collect::<Vec<_>>(),
        "acquisition": {"kind": "external", "label": label, "url": if paid { app.offers[0].url.as_str() } else { app.current_release().route.url() },
            "reason": "Managed installation is not available in this preview."}}))
}

pub fn list(
    catalogue: &Catalogue,
    query: &Query,
    now: DateTime<Utc>,
) -> Result<Value, &'static str> {
    query.validate()?;
    let snapshot = catalogue.snapshot_id();
    let fingerprint = query.fingerprint();
    let mut offset = 0;
    let mut at = now;
    if let Some(cursor) = &query.cursor {
        if cursor.len() > 200 {
            return Err("invalid_cursor");
        }
        let parts: Vec<_> = cursor.split(':').collect();
        if parts.len() != 4 || parts[1] != fingerprint {
            return Err("invalid_cursor");
        }
        if parts[0] != snapshot {
            return Err("snapshot_changed");
        }
        offset = parts[2].parse::<usize>().map_err(|_| "invalid_cursor")?;
        let epoch = parts[3].parse::<i64>().map_err(|_| "invalid_cursor")?;
        at = DateTime::from_timestamp(epoch, 0).ok_or("invalid_cursor")?;
        if at > now || now.signed_duration_since(at).num_hours() >= 24 {
            return Err("cursor_expired");
        }
    }
    let text = query.q.trim().to_lowercase();
    let terms: Vec<_> = text.split_whitespace().collect();
    let mut scored = Vec::new();
    for app in &catalogue.apps {
        let r = app.current_release();
        if query.category.as_ref().is_some_and(|v| v != &app.category)
            || query.app_type.as_ref().is_some_and(|v| v != &app.app_type)
            || query
                .licence
                .as_ref()
                .is_some_and(|v| v != &app.licence.class)
            || query
                .price
                .as_ref()
                .is_some_and(|v| !app.offers.iter().any(|o| &o.model == v))
            || query.architecture.as_ref().is_some_and(|v| {
                !r.architectures.contains(v) && !r.architectures.iter().any(|a| a == "any")
            })
            || query.offline.as_ref().is_some_and(|v| v != &r.offline)
            || query
                .evidence
                .as_ref()
                .is_some_and(|v| v != app.evidence(at).0)
        {
            continue;
        }
        let name = app.name.to_lowercase();
        let haystack = format!(
            "{} {} {} {}",
            name,
            app.id,
            app.summary.to_lowercase(),
            app.tags.join(" ")
        );
        if !terms.iter().all(|t| haystack.contains(t)) {
            continue;
        }
        let rank = if text.is_empty() {
            3
        } else if name == text || app.id == text {
            0
        } else if name.starts_with(&text) {
            1
        } else if terms.iter().any(|t| name.contains(t)) {
            2
        } else {
            3
        };
        let compatible = query
            .profile
            .as_ref()
            .is_some_and(|p| app.evidence_for_profile(at, Some(p)).0 == "passes");
        scored.push((rank, !compatible, name, app));
    }
    scored.sort_by(|a, b| (&a.0, &a.1, &a.2, &a.3.id).cmp(&(&b.0, &b.1, &b.2, &b.3.id)));
    if offset > scored.len() {
        return Err("invalid_cursor");
    }
    let end = offset.saturating_add(query.limit).min(scored.len());
    let next =
        (end < scored.len()).then(|| format!("{snapshot}:{fingerprint}:{end}:{}", at.timestamp()));
    Ok(
        json!({"snapshot": snapshot, "asOf": at.to_rfc3339(), "total": scored.len(),
        "items": scored[offset..end].iter().map(|(_, _, _, a)| summary(a, at)).collect::<Vec<_>>(), "nextCursor": next}),
    )
}
