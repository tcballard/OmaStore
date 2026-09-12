//! One read-only device-state projection for every storefront surface.
//! Repository catalogue versions never establish an installed version or update.
use crate::platform::{Host, Result};
use omastore_catalogue::{Catalogue, InstallRoute};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    #[default]
    All,
    Installed,
    Updates,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Query {
    #[serde(default)]
    pub filter: Filter,
    #[serde(default)]
    pub offset: usize,
    pub id: Option<String>,
    pub snapshot: Option<String>,
}

pub fn view(c: &Catalogue, host: &Host, query: Query, now: i64) -> Result<Value> {
    if query.offset > 20000 {
        return Err("invalid_offset");
    }
    if query
        .id
        .as_ref()
        .is_some_and(|id| !c.apps.iter().any(|a| &a.id == id))
    {
        return Err("not_found");
    }
    let snapshot = crate::platform::hash(&(c, host));
    if (query.offset > 0 && query.snapshot.is_none())
        || query.snapshot.as_ref().is_some_and(|s| s != &snapshot)
    {
        return Err("snapshot_changed");
    }
    let filter_name = match query.filter {
        Filter::All => "all",
        Filter::Installed => "installed",
        Filter::Updates => "updates",
    };
    let known = host.state == "supported" && !host.locked;
    let mut installed_count = 0;
    let mut update_count = 0;
    let mut items = Vec::new();
    for app in &c.apps {
        let package = match &app.current_release().route {
            InstallRoute::ArchPackage { package, .. } => Some(package),
            _ => None,
        };
        let installed = package
            .and_then(|p| host.installed.get(p))
            .filter(|_| known);
        let update = package
            .and_then(|p| host.updates.get(p))
            .filter(|_| installed.is_some());
        installed_count += usize::from(installed.is_some());
        update_count += usize::from(update.is_some());
        if query.id.as_ref().is_some_and(|id| &app.id != id)
            || matches!(query.filter, Filter::Installed) && installed.is_none()
            || matches!(query.filter, Filter::Updates) && update.is_none()
        {
            continue;
        }
        let state = if package.is_none() {
            "external"
        } else if !known {
            "unknown"
        } else if update.is_some() {
            "update_available"
        } else if installed.is_some() {
            "installed"
        } else {
            "not_installed"
        };
        // These are intents, never execution grants. Opening resolves a desktop
        // launcher; installation still needs an eligible, confirmed typed plan.
        let action = match state {
            "update_available" => "system_update",
            "installed" => "open",
            "not_installed" => "review_install",
            _ => "details",
        };
        items.push(json!({"id":app.id,"name":app.name,"package":package,
            "state":state,"primaryAction":action,"installedVersion":installed,
            "availableVersion":update.map(|u| &u.available_version),
            "updateIgnored":update.is_some_and(|u| u.ignored),
            "observedAt":if known {Some(now)} else {None}}));
    }
    items.sort_by_key(|item| {
        (
            item["name"].as_str().unwrap_or_default().to_lowercase(),
            item["id"].as_str().unwrap_or_default().to_owned(),
        )
    });
    let total = items.len();
    let page: Vec<_> = items.into_iter().skip(query.offset).take(30).collect();
    Ok(
        json!({"schemaVersion":1,"snapshot":snapshot,"filter":filter_name,"items":page,"total":total,"offset":query.offset,
        "nextOffset":if query.offset+30<total {Some(query.offset+30)} else {None},
        "installedCount":if known {Some(installed_count)} else {None},
        "updateCount":if known {Some(update_count)} else {None},
        "observationState":if known {"available"} else {"unavailable"},
        "observedAt":if known {Some(now)} else {None},"host":host.summary(),
        "simulated":host.simulated,"databasesStale":host.databases_stale,
        "updateSource":"local_package_databases","scope":"catalogue_apps",
        "notice":if !known {"Device status is unavailable. Refresh when the package manager is ready."}
        else if host.databases_stale {"Update information may be out of date. Use Omarchy’s updater to check and update the system."}
        else {"Updates reflect this device’s package databases. Omarchy’s updater checks and updates the whole system."}}),
    )
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[test]
    fn device_state_is_independent_of_catalogue_versions_and_saved_items() {
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let selection = crate::planner::Selection::App {
            id: "demo-fieldnotes".into(),
        };
        let (mut host, _, packages) =
            crate::planner::sample(&c, &selection, BTreeMap::new(), 1).unwrap();
        let p = &packages[0];
        let query = || Query {
            id: Some("demo-fieldnotes".into()),
            ..Default::default()
        };
        assert_eq!(
            view(&c, &host, query(), 1).unwrap()["items"][0]["state"],
            "not_installed"
        );
        host.installed.insert(p.name.clone(), "99:1.0-1".into());
        assert_eq!(
            view(&c, &host, query(), 2).unwrap()["items"][0]["state"],
            "installed"
        );
        host.updates =
            crate::platform::parse_updates(&format!("{} 99:1.0-1 -> 99:2.0-1 [ignored]\n", p.name))
                .unwrap();
        let item = view(&c, &host, query(), 3).unwrap()["items"][0].clone();
        assert_eq!(item["primaryAction"], "system_update");
        assert_eq!(item["availableVersion"], "99:2.0-1");
        assert_eq!(item["updateIgnored"], true);
        host.locked = true;
        let value = view(&c, &host, query(), 4).unwrap();
        assert_eq!(value["items"][0]["state"], "unknown");
        assert!(value["installedCount"].is_null());
        host.locked = false;
        host.installed.clear();
        host.updates.clear();
        assert_eq!(
            view(
                &c,
                &host,
                Query {
                    filter: Filter::Installed,
                    ..Default::default()
                },
                5
            )
            .unwrap()["total"],
            0
        );
        host.state = "unknown".into();
        assert_eq!(
            view(&c, &host, query(), 6).unwrap()["observationState"],
            "unavailable"
        );
        assert!(serde_json::from_value::<Query>(json!({"command":"pacman -S foo"})).is_err());
        assert!(view(
            &c,
            &host,
            Query {
                offset: 20001,
                ..Default::default()
            },
            6
        )
        .is_err());
    }
}
