//! Selective, versioned setup recipes. Produces intent and costs; never performs host I/O.
use crate::{query, Catalogue, FieldError, OfferModel, Recipe};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingReference {
    pub adapter: String,
    pub value_id: String,
    pub revision: String,
}
pub fn validate(r: &Recipe) -> Vec<FieldError> {
    let mut errors = Vec::new();
    let mut check = |ok: bool, path: &str, code: &str| {
        if !ok {
            errors.push(FieldError {
                path: path.into(),
                code: code.into(),
            });
        }
    };
    check(
        !r.name.is_empty()
            && r.name.len() <= 160
            && r.summary.len() <= 280
            && r.description.len() <= 6000,
        "name",
        "invalid_setup_text",
    );
    check(
        !r.rights.trim().is_empty() && r.rights.len() <= 1000,
        "rights",
        "sharing_rights_required",
    );
    check(
        !r.components.is_empty() && r.components.len() <= 100,
        "components",
        "invalid_component_count",
    );
    check(
        r.parent
            .as_ref()
            .is_none_or(|p| crate::token(p) && p != &r.id)
            && r.parent_revision
                .as_ref()
                .is_none_or(|v| r.parent.is_some() && crate::token(v)),
        "parent",
        "invalid_parent_reference",
    );
    for component in &r.components {
        let unique = component.conflicts.iter().collect::<BTreeSet<_>>();
        check(
            unique.len() == component.conflicts.len()
                && component.conflicts.iter().all(|id| {
                    id != &component.app_id && r.components.iter().any(|c| &c.app_id == id)
                }),
            "components",
            "invalid_component_conflict",
        );
    }
    let keys = r
        .settings
        .iter()
        .map(|s| &s.adapter)
        .collect::<BTreeSet<_>>();
    check(
        r.settings.len() <= 20
            && keys.len() == r.settings.len()
            && r.settings.iter().all(|s| {
                crate::token(&s.adapter) && crate::token(&s.value_id) && crate::token(&s.revision)
            }),
        "settings",
        "invalid_setting_reference",
    );
    check(
        r.media.len() <= 7
            && r.media.iter().all(|m| {
                crate::public_url(&m.url)
                    && crate::digest(&m.sha256, 64)
                    && !m.alt.is_empty()
                    && m.alt.len() <= 1000
                    && !m.rights.is_empty()
                    && m.rights.len() <= 1000
            }),
        "media",
        "invalid_setup_media",
    );
    errors
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub id: String,
    pub revision: String,
    pub chosen: Option<Vec<String>>,
}
pub fn list(c: &Catalogue, q: &crate::editorial::Browse) -> Result<Value, &'static str> {
    if q.q.len() > 200 || q.offset > 10000 {
        return Err("invalid_filter");
    }
    let snapshot = c.snapshot_id();
    if q.offset > 0 && q.snapshot.as_deref() != Some(&snapshot) {
        return Err("snapshot_changed");
    }
    let needle = q.q.to_lowercase();
    let mut recipes = c
        .recipes
        .iter()
        .filter(|r| {
            format!("{} {}", r.name, r.summary)
                .to_lowercase()
                .contains(&needle)
        })
        .collect::<Vec<_>>();
    recipes.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    let total = recipes.len();
    let items=recipes.into_iter().skip(q.offset).take(30).map(|r|json!({"id":r.id,"revision":r.revision,"name":r.name,"summary":r.summary,"makerId":r.maker_id,"components":r.components.len(),"rights":r.rights})).collect::<Vec<_>>();
    Ok(
        json!({"items":items,"total":total,"offset":q.offset,"nextOffset":if q.offset+30<total{Some(q.offset+30)}else{None},"snapshot":snapshot}),
    )
}
pub fn select(c: &Catalogue, input: &Selection, now: DateTime<Utc>) -> Result<Value, &'static str> {
    if !crate::token(&input.id) || !crate::token(&input.revision) {
        return Err("invalid_id");
    }
    let recipe = c
        .recipes
        .iter()
        .find(|r| r.id == input.id || r.slug == input.id)
        .ok_or("not_found")?;
    if recipe.revision != input.revision {
        return Err("setup_revision_unavailable");
    }
    let chosen = input.chosen.clone().unwrap_or_default();
    let requested = chosen.iter().cloned().collect::<BTreeSet<_>>();
    if chosen.len() > 100
        || requested.len() != chosen.len()
        || chosen
            .iter()
            .any(|id| !recipe.components.iter().any(|p| &p.app_id == id))
    {
        return Err("invalid_setup_selection");
    }
    let mut selected = requested.clone();
    selected.extend(
        recipe
            .components
            .iter()
            .filter(|p| !p.optional)
            .map(|p| p.app_id.clone()),
    );
    // Bounded fixed point also remains safe when called with a malformed, unvalidated graph.
    for _ in 0..100 {
        let before = selected.len();
        for p in &recipe.components {
            if selected.contains(&p.app_id) {
                selected.extend(p.depends_on.iter().cloned());
            }
        }
        if selected.len() == before {
            break;
        }
    }
    let mut rows = Vec::new();
    let mut conflicts = BTreeSet::new();
    let mut unavailable = Vec::new();
    let mut unknown = 0;
    let mut totals: BTreeMap<(String, u8, Option<String>, String), u64> = BTreeMap::new();
    for p in &recipe.components {
        let app = c
            .apps
            .iter()
            .find(|a| a.id == p.app_id)
            .ok_or("missing_component")?;
        let release = app
            .releases
            .iter()
            .find(|r| r.id == p.release_id)
            .ok_or("missing_component_release")?;
        let on = selected.contains(&p.app_id);
        let required_by = recipe
            .components
            .iter()
            .filter(|other| {
                selected.contains(&other.app_id) && other.depends_on.contains(&p.app_id)
            })
            .map(|other| other.app_id.clone())
            .collect::<Vec<_>>();
        if on {
            for conflict in &p.conflicts {
                if selected.contains(conflict) {
                    let mut pair = [p.app_id.clone(), conflict.clone()];
                    pair.sort();
                    conflicts.insert(pair);
                }
            }
        }
        let current = app.current_release_id == p.release_id;
        if on && !current {
            unavailable.push(p.app_id.clone());
        }
        let offer = app.offers.first();
        let free =
            offer.is_some_and(|o| matches!(o.model, OfferModel::Free | OfferModel::Donation));
        if on && !free {
            if let Some(o) = offer.filter(|o| {
                matches!(
                    o.model,
                    OfferModel::Paid | OfferModel::Subscription | OfferModel::Service
                ) && o.checked_at.as_ref().is_some_and(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .is_ok_and(|d| d <= now && now.signed_duration_since(d).num_hours() < 24)
                })
            }) {
                if let Some(price) = &o.price {
                    let key = (
                        price.currency.clone(),
                        price.exponent,
                        o.billing_interval.clone(),
                        serde_json::to_value(&o.tax)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_owned(),
                    );
                    let entry = totals.entry(key).or_default();
                    *entry = entry
                        .checked_add(price.minor_units)
                        .ok_or("setup_cost_overflow")?;
                } else {
                    unknown += 1;
                }
            } else {
                unknown += 1;
            }
        }
        rows.push(json!({"appId":app.id,"name":app.name,"summary":app.summary,"releaseId":release.id,"version":release.version,"identity":release.identity,"selected":on,"optional":p.optional,"required":!p.optional||!required_by.is_empty(),"requiredBy":required_by,"dependsOn":p.depends_on,"conflicts":p.conflicts,"currentRelease":current,"priceLabel":query::price_label(app),"offer":offer,"externalPurchase":!free,"routeLabel":release.route.label(),"evidenceLabel":if current{app.evidence(now).1}else{"Selected release is not current; current-release evidence does not apply"}}));
    }
    let totals=totals.into_iter().map(|((currency,exponent,interval,tax),minor)|json!({"currency":currency,"minorUnits":minor,"exponent":exponent,"billingInterval":interval,"tax":tax})).collect::<Vec<_>>();
    let export = json!({"schemaVersion":1,"kind":"omastore_setup_selection","setupId":recipe.id,"revision":recipe.revision,"creatorMakerId":recipe.maker_id,"parentId":recipe.parent,"parentRevision":recipe.parent_revision,"rights":recipe.rights,"components":recipe.components.iter().filter(|p|selected.contains(&p.app_id)).map(|p|json!({"appId":p.app_id,"releaseId":p.release_id})).collect::<Vec<_>>(),"settings":recipe.settings});
    Ok(
        json!({"recipe":recipe,"maker":c.makers.iter().find(|m|m.id==recipe.maker_id).map(|m|crate::editorial::maker_summary(m,now)),"requested":requested,"selected":selected,"components":rows,"conflicts":conflicts,"changedReleases":unavailable,"valid":conflicts.is_empty()&&unavailable.is_empty(),"costs":{"groups":totals,"unknownItems":unknown,"known":unknown==0,"notice":"Listed prices checked within 24 hours. Currency, billing interval and tax states stay separate; external checkout confirms the final amount."},"shareUri":format!("omastore://setup/{}?revision={}",recipe.id,recipe.revision),"export":export,"snapshot":c.snapshot_id(),"notice":"Selection is a preview. Installing applications and applying settings each require a separate reviewed plan and explicit confirmation."}),
    )
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Export {
    schema_version: u32,
    kind: String,
    setup_id: String,
    revision: String,
    creator_maker_id: String,
    parent_id: Option<String>,
    parent_revision: Option<String>,
    rights: String,
    components: Vec<ExportComponent>,
    settings: Vec<SettingReference>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportComponent {
    app_id: String,
    release_id: String,
}
pub fn restore(c: &Catalogue, bytes: &[u8], now: DateTime<Utc>) -> Result<Value, &'static str> {
    if bytes.len() > 128 * 1024 {
        return Err("selection_too_large");
    }
    let export: Export = serde_json::from_slice(bytes).map_err(|_| "invalid_selection_export")?;
    if export.schema_version != 1
        || export.kind != "omastore_setup_selection"
        || export.components.len() > 100
    {
        return Err("invalid_selection_export");
    }
    let result = select(
        c,
        &Selection {
            id: export.setup_id.clone(),
            revision: export.revision.clone(),
            chosen: Some(export.components.iter().map(|p| p.app_id.clone()).collect()),
        },
        now,
    )?;
    if result["export"] != serde_json::to_value(export).map_err(|_| "invalid_selection_export")? {
        return Err("selection_context_changed");
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Catalogue {
        serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap()
    }
    fn input(c: &Catalogue, chosen: Vec<String>) -> Selection {
        Selection {
            id: c.recipes[0].id.clone(),
            revision: c.recipes[0].revision.clone(),
            chosen: Some(chosen),
        }
    }
    #[test]
    fn dependencies_are_explicit_optional_choices_are_selective_and_conflicts_block_handoff() {
        let mut c = fixture();
        let first = c.recipes[0].components[0].app_id.clone();
        let second = c.recipes[0].components[1].app_id.clone();
        c.recipes[0].components[0].optional = true;
        c.recipes[0].components[1].depends_on = vec![first.clone()];
        let chosen = select(&c, &input(&c, vec![second.clone()]), Utc::now()).unwrap();
        assert_eq!(chosen["components"][0]["requiredBy"], json!([second]));
        assert_eq!(chosen["components"][0]["required"], true);
        assert_eq!(chosen["selected"].as_array().unwrap().len(), 2);
        let none = select(&c, &input(&c, vec![]), Utc::now()).unwrap();
        assert_eq!(none["selected"], json!([]));
        c.recipes[0].components[0].conflicts = vec![second.clone()];
        let conflicts = select(&c, &input(&c, vec![second]), Utc::now()).unwrap();
        assert_eq!(conflicts["valid"], false);
        assert_eq!(conflicts["conflicts"].as_array().unwrap().len(), 1);
        c.recipes[0].components[0].depends_on = vec![c.recipes[0].components[1].app_id.clone()];
        assert!(c
            .validate(true)
            .iter()
            .any(|e| e.code == "cyclic_components"));
    }
    #[test]
    fn sharing_round_trip_pins_revision_and_rejects_private_fields_or_changed_attribution() {
        let mut c = fixture();
        let now = Utc::now();
        let v = select(&c, &input(&c, vec![]), now).unwrap();
        let bytes = serde_json::to_vec(&v["export"]).unwrap();
        assert_eq!(restore(&c, &bytes, now).unwrap()["export"], v["export"]);
        let mut bad = v["export"].clone();
        bad["localPath"] = json!("/home/person/private");
        assert_eq!(
            restore(&c, &serde_json::to_vec(&bad).unwrap(), now).unwrap_err(),
            "invalid_selection_export"
        );
        c.recipes[0].rights = "Changed rights after export".into();
        assert_eq!(
            restore(&c, &bytes, now).unwrap_err(),
            "selection_context_changed"
        );
        c.recipes[0].revision = "2".into();
        assert_eq!(
            restore(&c, &bytes, now).unwrap_err(),
            "setup_revision_unavailable"
        );
    }
    #[test]
    fn costs_keep_unknown_quotes_and_currencies_separate() {
        let mut c = fixture();
        let now = Utc::now();
        let ids = c.recipes[0]
            .components
            .iter()
            .map(|p| p.app_id.clone())
            .collect::<Vec<_>>();
        for (index, id) in ids.iter().enumerate() {
            let a = c.apps.iter_mut().find(|a| &a.id == id).unwrap();
            let o = &mut a.offers[0];
            o.model = OfferModel::Paid;
            o.price = Some(crate::Money {
                currency: if index == 0 { "GBP" } else { "USD" }.into(),
                minor_units: 1234,
                exponent: 2,
            });
            o.checked_at = Some(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        }
        let v = select(&c, &input(&c, ids.clone()), now).unwrap();
        assert_eq!(v["costs"]["groups"].as_array().unwrap().len(), 2);
        assert_eq!(v["costs"]["unknownItems"], 0);
        assert!(v["costs"].get("total").is_none());
        c.apps.iter_mut().find(|a| a.id == ids[0]).unwrap().offers[0].checked_at = None;
        let v = select(&c, &input(&c, ids.clone()), now).unwrap();
        assert_eq!(v["costs"]["known"], false);
        assert_eq!(v["costs"]["unknownItems"], 1);
        let v = select(&c, &input(&c, ids), now + chrono::Duration::days(2)).unwrap();
        assert_eq!(v["costs"]["unknownItems"], 2);
    }
}
