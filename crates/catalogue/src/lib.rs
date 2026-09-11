//! The public catalogue contract. No private workflow records or executable commands.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

pub const MAX_BYTES: usize = 1024 * 1024;
macro_rules! record {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name { $(pub $field: $ty),* }
    };
}
macro_rules! choices {
    ($name:ident { $($variant:ident),* }) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),* }
    };
}
choices!(Channel {
    Public,
    Development
});
choices!(AppType {
    Desktop,
    Terminal,
    ShellPlugin,
    Web,
    Service
});
choices!(Maturity {
    Development,
    Preview,
    Stable
});
choices!(LicenseClass {
    OpenSource,
    SourceAvailable,
    Proprietary,
    Unknown
});
choices!(Knowledge { Yes, No, Unknown });
choices!(TestResult {
    Passes,
    Limitations,
    Fails,
    NotTested
});
choices!(Freshness {
    Current,
    RetestDue,
    Superseded
});
choices!(PriceModel {
    Free,
    VoluntarySupport,
    PayWhatYouWant,
    PaidApp,
    PaidUpgrade,
    PaidFeatures,
    Subscription,
    ProfessionalServices,
    PaidPreview,
    Unknown
});
record!(Snapshot { schema_version: u32, channel: Channel, revision: String, build_revision: String,
    generated_at: String, apps: Vec<App>, makers: Vec<Maker>, recipes: Vec<Recipe>, editorial: Vec<String> });
record!(App { id: String, slug: String, name: String, summary: String, description: String,
    tags: Vec<String>, category: String, app_type: AppType, maturity: Maturity,
    license: License, maker_ids: Vec<String>, source: String, support: String,
    media: Vec<Media>, current_release: Option<String>, releases: Vec<Release>,
    evidence: Vec<Evidence>, offers: Vec<Offer>, limitations: Vec<String>,
    provenance: String, informational: bool });
record!(License { class: LicenseClass, identifier: Option<String> });
record!(Maker {
    id: String,
    name: String,
    project_url: String,
    claimed: bool
});
record!(Media {
    kind: String,
    url: String,
    alt: String,
    rights: String
});
record!(Release { id: String, version: String, identity: ReleaseIdentity, architectures: Vec<String>,
    route: Option<InstallRoute>, notes: String, offline: Knowledge, account_required: Knowledge,
    service_required: Knowledge, activation_required: Knowledge });
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReleaseIdentity {
    SourceCommit {
        repository: String,
        commit: String,
    },
    BinaryArtifact {
        publisher: String,
        source: String,
        version: String,
        sha256: String,
    },
    RepositoryPackage {
        repository: String,
        package: String,
        version: String,
        signature: Option<String>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallRoute {
    ArchPackage {
        repository: String,
        package: String,
        url: String,
        privileges: Vec<String>,
        services: Vec<String>,
    },
    AurExternal {
        package: String,
        url: String,
    },
    UpstreamExternal {
        url: String,
    },
    PluginExternal {
        plugin_id: String,
        url: String,
    },
}
impl InstallRoute {
    pub fn url(&self) -> &str {
        match self {
            Self::ArchPackage { url, .. }
            | Self::AurExternal { url, .. }
            | Self::UpstreamExternal { url }
            | Self::PluginExternal { url, .. } => url,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::ArchPackage { .. } => "Repository package · external instructions",
            Self::AurExternal { .. } => "AUR · external build/install",
            Self::UpstreamExternal { .. } => "Get from developer",
            Self::PluginExternal { .. } => "Open plugin marketplace",
        }
    }
}
record!(Evidence { id: String, release_id: String, executed_identity: ReleaseIdentity,
    candidate_sha256: String, tool: String, actor: String, environment: String,
    result: TestResult, tested_at: String, limitations: Vec<String>, reference: String });
record!(Offer { seller_id: String, model: PriceModel, url: String, currency: Option<String>,
    amount_minor: Option<u64>, interval: Option<String>, tax_included: Knowledge,
    checked_at: Option<String>, entitlement: String, support: String, refund: Option<String>, cancellation: Option<String> });
record!(Recipe { id: String, slug: String, revision: String, name: String, maker_id: String,
    components: Vec<String>, rights: String, parent: Option<String> });
record!(FieldError {
    field: String,
    code: String
});
pub type Validation = Result<(), Vec<FieldError>>;

pub fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}
pub fn https(s: &str) -> bool {
    Url::parse(s).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
    })
}
fn hex(s: &str, length: usize) -> bool {
    s.len() == length && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn timestamp(s: &str) -> bool {
    s.ends_with('Z') && DateTime::parse_from_rfc3339(s).is_ok()
}
fn identity_valid(i: &ReleaseIdentity) -> bool {
    match i {
        ReleaseIdentity::SourceCommit { repository, commit } => {
            https(repository) && hex(commit, 40)
        }
        ReleaseIdentity::BinaryArtifact {
            publisher,
            source,
            version,
            sha256,
        } => token(publisher) && https(source) && !version.is_empty() && hex(sha256, 64),
        ReleaseIdentity::RepositoryPackage {
            repository,
            package,
            version,
            signature,
        } => {
            token(repository)
                && token(package)
                && !version.is_empty()
                && signature.as_ref().is_none_or(|s| https(s))
        }
    }
}
impl Snapshot {
    pub fn parse(bytes: &[u8], allow_development: bool) -> Result<Self, Vec<FieldError>> {
        if bytes.len() > MAX_BYTES {
            return Err(vec![FieldError {
                field: "$".into(),
                code: "too_large".into(),
            }]);
        }
        let snapshot: Self = serde_json::from_slice(bytes).map_err(|_| {
            vec![FieldError {
                field: "$".into(),
                code: "invalid_schema".into(),
            }]
        })?;
        snapshot.validate(allow_development)?;
        Ok(snapshot)
    }
    pub fn validate(&self, allow_development: bool) -> Validation {
        let mut errors = Vec::new();
        let mut check = |ok: bool, field: String, code: &str| {
            if !ok && errors.len() < 100 {
                errors.push(FieldError {
                    field,
                    code: code.into(),
                });
            }
        };
        check(
            self.schema_version == 1,
            "schemaVersion".into(),
            "unsupported_schema",
        );
        check(
            self.channel == Channel::Public || allow_development,
            "channel".into(),
            "development_content",
        );
        check(
            token(&self.revision) && token(&self.build_revision),
            "revision".into(),
            "invalid_revision",
        );
        check(
            timestamp(&self.generated_at),
            "generatedAt".into(),
            "invalid_timestamp",
        );
        check(
            self.apps.len() <= 5000 && self.makers.len() <= 5000 && self.recipes.len() <= 1000,
            "$".into(),
            "too_many_records",
        );
        let mut ids = HashSet::new();
        let mut slugs = HashSet::new();
        let makers: HashSet<_> = self.makers.iter().map(|m| m.id.as_str()).collect();
        let apps: HashSet<_> = self.apps.iter().map(|a| a.id.as_str()).collect();
        for maker in &self.makers {
            check(
                token(&maker.id) && ids.insert(maker.id.clone()),
                format!("makers.{}", maker.id),
                "duplicate_or_invalid_id",
            );
            check(
                !maker.name.is_empty() && https(&maker.project_url),
                format!("makers.{}", maker.id),
                "invalid_maker",
            );
        }
        for app in &self.apps {
            let p = format!("apps.{}", app.id);
            check(
                token(&app.id) && ids.insert(app.id.clone()),
                p.clone(),
                "duplicate_or_invalid_id",
            );
            check(
                token(&app.slug) && slugs.insert(app.slug.clone()),
                format!("{p}.slug"),
                "duplicate_or_invalid_slug",
            );
            check(
                !app.name.trim().is_empty()
                    && app.name.len() <= 128
                    && app.summary.len() <= 512
                    && app.description.len() <= 8192,
                p.clone(),
                "invalid_text",
            );
            check(
                https(&app.source) && https(&app.support) && https(&app.provenance),
                p.clone(),
                "invalid_url",
            );
            check(
                !app.maker_ids.is_empty()
                    && app.maker_ids.iter().all(|id| makers.contains(id.as_str())),
                format!("{p}.makerIds"),
                "missing_maker",
            );
            let mut releases = HashSet::new();
            for release in &app.releases {
                check(
                    token(&release.id) && releases.insert(release.id.clone()),
                    format!("{p}.releases"),
                    "duplicate_or_invalid_release",
                );
                check(
                    !release.version.is_empty()
                        && release.version.len() <= 128
                        && identity_valid(&release.identity),
                    format!("{p}.releases.{}", release.id),
                    "invalid_identity",
                );
                check(
                    !release.architectures.is_empty()
                        && release
                            .architectures
                            .iter()
                            .all(|a| ["x86_64", "aarch64", "any", "unknown"].contains(&a.as_str())),
                    p.clone(),
                    "invalid_architecture",
                );
                if let Some(route) = &release.route {
                    let safe = match route {
                        InstallRoute::ArchPackage {
                            repository,
                            package,
                            ..
                        } => token(repository) && token(package),
                        InstallRoute::AurExternal { package, .. } => token(package),
                        InstallRoute::PluginExternal { plugin_id, .. } => token(plugin_id),
                        InstallRoute::UpstreamExternal { .. } => true,
                    };
                    check(
                        safe && https(route.url()),
                        format!("{p}.route"),
                        "invalid_route",
                    );
                    if let (
                        InstallRoute::ArchPackage {
                            repository,
                            package,
                            ..
                        },
                        ReleaseIdentity::RepositoryPackage {
                            repository: r,
                            package: pkg,
                            ..
                        },
                    ) = (route, &release.identity)
                    {
                        check(
                            repository == r && package == pkg,
                            format!("{p}.route"),
                            "identity_mismatch",
                        );
                    }
                }
            }
            check(
                app.current_release
                    .as_ref()
                    .is_none_or(|r| releases.contains(r)),
                format!("{p}.currentRelease"),
                "missing_release",
            );
            let mut evidence_ids = HashSet::new();
            for e in &app.evidence {
                check(
                    token(&e.id) && evidence_ids.insert(&e.id) && releases.contains(&e.release_id),
                    format!("{p}.evidence"),
                    "missing_or_duplicate_reference",
                );
                check(
                    identity_valid(&e.executed_identity)
                        && app
                            .releases
                            .iter()
                            .any(|r| r.id == e.release_id && r.identity == e.executed_identity),
                    format!("{p}.evidence"),
                    "executed_identity_mismatch",
                );
                check(
                    hex(&e.candidate_sha256, 64)
                        && timestamp(&e.tested_at)
                        && !e.environment.is_empty()
                        && !e.actor.is_empty()
                        && !e.tool.is_empty()
                        && https(&e.reference),
                    format!("{p}.evidence"),
                    "incomplete_evidence",
                );
            }
            check(app.media.len() <= 7, format!("{p}.media"), "too_many_media");
            for media in &app.media {
                check(
                    ["icon", "screenshot", "demo"].contains(&media.kind.as_str())
                        && https(&media.url)
                        && !media.alt.is_empty()
                        && !media.rights.is_empty(),
                    format!("{p}.media"),
                    "invalid_media",
                );
            }
            for offer in &app.offers {
                check(
                    makers.contains(offer.seller_id.as_str())
                        && https(&offer.url)
                        && https(&offer.support),
                    format!("{p}.offers"),
                    "invalid_seller_or_url",
                );
                check(
                    offer.amount_minor.is_some() == offer.currency.is_some()
                        && offer.currency.as_ref().is_none_or(|s| {
                            s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase())
                        }),
                    format!("{p}.offers"),
                    "invalid_money",
                );
                check(
                    offer.checked_at.as_ref().is_none_or(|s| timestamp(s))
                        && offer.refund.as_ref().is_none_or(|s| https(s))
                        && offer.cancellation.as_ref().is_none_or(|s| https(s)),
                    format!("{p}.offers"),
                    "invalid_offer",
                );
            }
        }
        for recipe in &self.recipes {
            check(
                token(&recipe.id)
                    && ids.insert(recipe.id.clone())
                    && token(&recipe.slug)
                    && slugs.insert(recipe.slug.clone()),
                "recipes".into(),
                "duplicate_or_invalid_id",
            );
            check(
                makers.contains(recipe.maker_id.as_str())
                    && recipe
                        .components
                        .iter()
                        .all(|id| apps.contains(id.as_str()))
                    && !recipe.rights.is_empty(),
                "recipes".into(),
                "missing_reference_or_rights",
            );
        }
        check(
            self.editorial.iter().all(|id| apps.contains(id.as_str())),
            "editorial".into(),
            "missing_reference",
        );
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    pub fn etag(&self) -> String {
        format!(
            "\"{:x}\"",
            Sha256::digest(serde_json::to_vec(self).expect("typed snapshot serializes"))
        )
    }
}
impl App {
    pub fn release(&self) -> Option<&Release> {
        self.releases
            .iter()
            .find(|r| Some(&r.id) == self.current_release.as_ref())
    }
    /// Evidence is descriptive, never approval or managed-install authority.
    pub fn evidence_status(&self, now: DateTime<Utc>) -> (TestResult, Freshness) {
        let current = self
            .evidence
            .iter()
            .filter(|e| Some(&e.release_id) == self.current_release.as_ref())
            .max_by(|a, b| a.tested_at.cmp(&b.tested_at));
        if let Some(e) = current {
            if let Ok(t) = DateTime::parse_from_rfc3339(&e.tested_at) {
                let age = now.signed_duration_since(t);
                if age.num_seconds() < 0 {
                    return (TestResult::NotTested, Freshness::RetestDue);
                }
                return (
                    e.result.clone(),
                    if age.num_days() >= 90 {
                        Freshness::RetestDue
                    } else {
                        Freshness::Current
                    },
                );
            }
        }
        (
            TestResult::NotTested,
            if self.evidence.is_empty() {
                Freshness::Current
            } else {
                Freshness::Superseded
            },
        )
    }
}

#[cfg(feature = "dev-fixtures")]
pub fn development_fixture() -> Snapshot {
    Snapshot::parse(
        include_bytes!("../../../tests/fixtures/catalogue.json"),
        true,
    )
    .expect("valid development fixture")
}
