use crate::platform::{self, Host, Package, Result};
use omastore_catalogue::{Catalogue, InstallRoute, ReleaseIdentity};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(feature = "development-catalogue")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppReference {
    pub app_id: String,
    pub release_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    Apps {
        components: Vec<AppReference>,
    },
    App {
        id: String,
    },
    Remove {
        id: String,
    },
    Setup {
        id: String,
        revision: String,
        chosen: Vec<String>,
    },
}
impl Selection {
    pub fn ids(&self, c: &Catalogue, now: i64) -> Result<Vec<String>> {
        match self {
            Self::Apps { components } => {
                if components.is_empty()
                    || components.len() > 100
                    || components
                        .iter()
                        .map(|p| &p.app_id)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != components.len()
                {
                    return Err("invalid_selection");
                }
                for part in components {
                    if !omastore_catalogue::token(&part.app_id)
                        || !omastore_catalogue::token(&part.release_id)
                        || !c
                            .apps
                            .iter()
                            .any(|a| a.id == part.app_id && a.current_release_id == part.release_id)
                    {
                        return Err("selected_release_changed");
                    }
                }
                Ok(components.iter().map(|p| p.app_id.clone()).collect())
            }
            Self::App { id } if omastore_catalogue::token(id) => {
                if !c.apps.iter().any(|a| a.id == *id) {
                    return Err("not_found");
                }
                Ok(vec![id.clone()])
            }
            Self::Setup {
                id,
                revision,
                chosen,
            } => {
                let selected = omastore_catalogue::setups::select(
                    c,
                    &omastore_catalogue::setups::Selection {
                        id: id.clone(),
                        revision: revision.clone(),
                        chosen: Some(chosen.clone()),
                    },
                    chrono::DateTime::from_timestamp(now, 0).ok_or("invalid_time")?,
                )?;
                if selected["valid"] != true {
                    return Err("setup_selection_conflict");
                }
                serde_json::from_value(selected["selected"].clone())
                    .map_err(|_| "invalid_selection")
            }
            _ => Err("invalid_selection"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub app_id: String,
    pub release_id: String,
    pub name: String,
    pub action: String,
    pub reason: String,
    pub package: Option<String>,
    pub repository: Option<String>,
    pub version: String,
    pub installed_version: Option<String>,
    pub identity_digest: String,
    pub privileges: Vec<String>,
    pub services: Vec<String>,
    pub external_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclosure: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub selection: Selection,
    pub created_at: i64,
    pub expires_at: i64,
    pub catalogue_snapshot: String,
    pub status_until: i64,
    pub host: Value,
    pub operations: Vec<Operation>,
    pub packages: Vec<Package>,
    pub blockers: Vec<String>,
    pub material_digest: String,
    pub digest: String,
    pub simulated: bool,
    pub notice: String,
}
pub fn status_current(c: &Catalogue, status: &Value, now: i64) -> bool {
    let Some(generated) = status["generatedAt"].as_i64() else {
        return false;
    };
    let Some(until) = status["validUntil"].as_i64() else {
        return false;
    };
    status["schemaVersion"] == 1
        && status["catalogueSnapshot"] == c.snapshot_id()
        && generated <= now + 30
        && generated >= now - 300
        && until > now
        && until <= generated + 300
}
fn eligible(c: &Catalogue, status: &Value, id: &str, now: i64) -> bool {
    if !status_current(c, status, now) {
        return false;
    }
    let Some(app) = c.apps.iter().find(|a| a.id == id) else {
        return false;
    };
    let rows = status["items"].as_array();
    let matching = rows
        .into_iter()
        .flatten()
        .filter(|r| r["appId"] == id)
        .collect::<Vec<_>>();
    matching.len() == 1
        && matching[0]["releaseId"] == app.current_release_id
        && matching[0]["identityDigest"] == platform::hash(&app.current_release().identity)
        && matching[0]["distribution"] == "active"
        && matching[0]["distributionEligible"] == true
        && matching[0]["sourceCurrent"] == true
        && matching[0]["sourceAvailable"] == true
}
pub fn targets(c: &Catalogue, ids: &[String], h: &Host) -> Vec<String> {
    let mut result = BTreeSet::new();
    for app in c.apps.iter().filter(|a| ids.contains(&a.id)) {
        if let InstallRoute::ArchPackage {
            repository,
            package,
            ..
        } = &app.current_release().route
        {
            if platform::package_token(repository)
                && platform::package_token(package)
                && !h.installed.contains_key(package)
            {
                result.insert(format!("{repository}/{package}"));
            }
        }
    }
    result.into_iter().collect()
}
pub fn build(
    c: &Catalogue,
    selection: Selection,
    h: &Host,
    status: &Value,
    resolved: Result<Vec<Package>>,
    now: i64,
) -> Result<Plan> {
    let mut ids = selection.ids(c, now)?;
    ids.sort();
    ids.dedup();
    if ids.is_empty() || ids.len() > 100 {
        return Err("invalid_selection");
    }
    let mut blockers = BTreeSet::new();
    let packages = match resolved {
        Ok(p) => p,
        Err(code) => {
            blockers.insert(code.to_owned());
            Vec::new()
        }
    };
    if packages.len() > 256 {
        return Err("package_transaction_too_large");
    }
    let mut operations = Vec::new();
    let mut status_material = Vec::new();
    for id in &ids {
        let app = c.apps.iter().find(|a| a.id == *id).ok_or("not_found")?;
        let release = app.current_release();
        let expected_version = match &release.identity {
            ReleaseIdentity::RepositoryPackage { version, .. } => version,
            _ => &release.version,
        };
        let mut op = Operation {
            app_id: id.clone(),
            release_id: release.id.clone(),
            name: app.name.clone(),
            action: "external".into(),
            reason: "This route is handled by its publisher or the system's normal tools".into(),
            package: None,
            repository: None,
            version: expected_version.clone(),
            installed_version: None,
            identity_digest: platform::hash(&release.identity),
            privileges: release.privileges.clone(),
            services: release.services.clone(),
            external_url: Some(release.route.url().into()),
            disclosure: Some(
                json!({"evidence":app.evidence(chrono::DateTime::from_timestamp(now,0).ok_or("invalid_time")?).1,"account":release.account,"activation":release.activation,"serviceCosts":release.service_costs,"removal":release.removal}),
            ),
        };
        let current = eligible(c, status, id, now);
        status_material.push(json!({"id":id,"eligible":current,"identity":op.identity_digest}));
        if let InstallRoute::ArchPackage {
            repository,
            package,
            ..
        } = &release.route
        {
            if !platform::package_token(repository) || !platform::package_token(package) {
                return Err("invalid_package_target");
            }
            op.package = Some(package.clone());
            op.repository = Some(repository.clone());
            op.installed_version = h.installed.get(package).cloned();
            op.action = "blocked".into();
            let identity_matches = matches!(&release.identity,ReleaseIdentity::RepositoryPackage{repository:r,package:p,version:v,..} if r==repository && p==package && platform::version(v));
            if let Some(v) = &op.installed_version {
                op.action = if v == expected_version {
                    "noop"
                } else {
                    "updater"
                }
                .into();
                op.reason=if v==&release.version{"Already installed; no package write is needed"}else{"Installed version differs. Inspect it in the library; use Omarchy's updater for upgrades"}.into();
            } else if !identity_matches {
                op.reason = "The reviewed package identity no longer matches this route".into();
            } else if h.state != "supported" {
                op.reason = h.reason.clone();
            } else if !release
                .architectures
                .iter()
                .any(|a| a == &h.architecture || a == "any")
            {
                op.reason = "This release does not support the observed architecture".into();
            } else if !h.repositories.get(repository).copied().unwrap_or(false) {
                op.reason =
                    "This configured repository does not require trusted package signatures".into();
            } else if h.update_required {
                op.action = "updater".into();
                op.reason = "Run Omarchy's updater, then make a new plan".into();
            } else if h.locked {
                op.reason = "Another package operation holds the system lock".into();
            } else if !current {
                op.reason="Current distribution status is missing, stale, suspended or refers to different content".into();
            } else if !packages.iter().any(|p| {
                p.name == *package && p.repository == *repository && p.version == *expected_version
            }) {
                op.reason =
                    "The configured repositories do not resolve the exact reviewed package version"
                        .into();
            } else {
                op.action = "install".into();
                op.reason =
                    "Install this reviewed repository package and the dependency effects below"
                        .into();
            }
            if op.action == "blocked" || op.action == "updater" {
                blockers.insert(format!("{id}: {}", op.reason));
            }
        }
        operations.push(op);
    }
    for package in &packages {
        if !h
            .repositories
            .get(&package.repository)
            .copied()
            .unwrap_or(false)
        {
            blockers
                .insert("A dependency is outside repositories requiring trusted signatures".into());
        }
        if h.installed
            .get(&package.name)
            .is_some_and(|v| v != &package.version)
        {
            blockers.insert("A dependency needs a system upgrade; run Omarchy's updater".into());
        }
        if !package.conflicts.is_empty() || !package.replaces.is_empty() {
            blockers.insert("A package declares conflicts or replacements; use the system's interactive package tools".into());
        }
    }
    if !blockers.is_empty() {
        for op in &mut operations {
            if op.action == "install" {
                op.action = "blocked".into();
                op.reason = "Resolve the plan's blockers before making a new proposal".into();
            }
        }
    }
    let host = h.summary();
    let catalogue_snapshot = c.snapshot_id();
    let material_digest = platform::hash(
        &json!({"schemaVersion":1,"selection":selection,"catalogueSnapshot":catalogue_snapshot,"host":h.fingerprint(),"operations":operations,"packages":packages,"status":status_material,"blockers":blockers}),
    );
    let mut plan=Plan{schema_version:1,selection,created_at:now,expires_at:now+600,catalogue_snapshot,status_until:status["validUntil"].as_i64().unwrap_or(0),host,operations,packages,blockers:blockers.into_iter().collect(),material_digest,digest:String::new(),simulated:h.simulated,notice:if h.simulated {"Fictional package rehearsal. No system packages will be installed."} else {"Read-only proposal. Administrator privileges and package hooks are part of installation. Confirmation applies only to this exact plan; no package state has changed."}.into()};
    plan.digest = platform::hash(&plan);
    if serde_json::to_vec(&plan).map_err(|_| "plan_invalid")?.len() > 160 * 1024 {
        return Err("plan_too_large");
    }
    Ok(plan)
}

#[cfg(feature = "development-catalogue")]
pub fn sample(
    c: &Catalogue,
    selection: &Selection,
    installed: BTreeMap<String, String>,
    now: i64,
) -> Result<(Host, Value, Vec<Package>)> {
    let ids = selection.ids(c, now)?;
    let mut h = Host {
        state: "supported".into(),
        reason: "Fictional Omarchy host; no real package operations".into(),
        architecture: "x86_64".into(),
        omarchy_version: Some("fictional".into()),
        configuration: "fictional".into(),
        sync_databases: "fictional".into(),
        repositories: BTreeMap::new(),
        installed,
        update_required: false,
        locked: false,
        simulated: true,
    };
    let mut items = Vec::new();
    let mut packages = Vec::new();
    for app in c.apps.iter().filter(|a| ids.contains(&a.id)) {
        items.push(json!({"appId":app.id,"releaseId":app.current_release_id,"identityDigest":platform::hash(&app.current_release().identity),"distribution":"active","distributionEligible":true,"sourceCurrent":true,"sourceAvailable":true}));
        if let InstallRoute::ArchPackage {
            repository,
            package,
            ..
        } = &app.current_release().route
        {
            h.repositories.insert(repository.clone(), true);
            if !h.installed.contains_key(package) {
                packages.push(Package {
                    repository: repository.clone(),
                    name: package.clone(),
                    version: match &app.current_release().identity {
                        ReleaseIdentity::RepositoryPackage { version, .. } => version.clone(),
                        _ => app.current_release().version.clone(),
                    },
                    sha256: "0".repeat(64),
                    architecture: "x86_64".into(),
                    download_bytes: 0,
                    conflicts: String::new(),
                    replaces: String::new(),
                });
            }
        }
    }
    Ok((
        h,
        json!({"schemaVersion":1,"catalogueSnapshot":c.snapshot_id(),"generatedAt":now,"validUntil":now+300,"items":items}),
        packages,
    ))
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    fn fixture() -> Catalogue {
        serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap()
    }
    #[test]
    fn local_recipe_selection_keeps_exact_releases_and_rejects_duplicates() {
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let reference = AppReference {
            app_id: c.apps[0].id.clone(),
            release_id: c.apps[0].current_release_id.clone(),
        };
        assert_eq!(
            Selection::Apps {
                components: vec![reference.clone()]
            }
            .ids(&c, 1000)
            .unwrap(),
            vec![reference.app_id.clone()]
        );
        assert!(Selection::Apps {
            components: vec![reference.clone(), reference.clone()]
        }
        .ids(&c, 1000)
        .is_err());
        let mut changed = reference;
        changed.release_id = "unavailable-release".into();
        assert!(Selection::Apps {
            components: vec![changed]
        }
        .ids(&c, 1000)
        .is_err());
    }
    #[test]
    fn exact_plans_are_deterministic_and_invalidate_changed_trust() {
        let c = fixture();
        let s = Selection::App {
            id: "demo-fieldnotes".into(),
        };
        let now = 1_800_000_000;
        let (mut h, mut status, p) = sample(&c, &s, BTreeMap::new(), now).unwrap();
        let a = build(&c, s.clone(), &h, &status, Ok(p.clone()), now).unwrap();
        assert_eq!(a.operations[0].action, "install", "{:?}", a.blockers);
        assert_eq!(
            a.digest,
            build(&c, s.clone(), &h, &status, Ok(p.clone()), now)
                .unwrap()
                .digest
        );
        status["items"][0]["identityDigest"] = json!("bad");
        assert_eq!(
            build(&c, s.clone(), &h, &status, Ok(p.clone()), now)
                .unwrap()
                .operations[0]
                .action,
            "blocked"
        );
        status["items"][0]["identityDigest"] =
            json!(platform::hash(&c.apps[0].current_release().identity));
        h.installed.insert(p[0].name.clone(), p[0].version.clone());
        assert_eq!(
            build(&c, s.clone(), &h, &Value::Null, Ok(Vec::new()), now)
                .unwrap()
                .operations[0]
                .action,
            "noop"
        );
        h.installed.clear();
        h.update_required = true;
        assert_eq!(
            build(&c, s.clone(), &h, &status, Ok(p.clone()), now)
                .unwrap()
                .operations[0]
                .action,
            "updater"
        );
        h.update_required = false;
        status["validUntil"] = json!(now);
        assert_eq!(
            build(&c, s.clone(), &h, &status, Ok(p.clone()), now)
                .unwrap()
                .operations[0]
                .action,
            "blocked"
        );
        h.state = "unknown".into();
        assert!(!build(&c, s, &h, &status, Ok(p), now)
            .unwrap()
            .blockers
            .is_empty());
        assert!(Selection::App {
            id: "--root".into()
        }
        .ids(&c, now)
        .is_err());
    }
}
