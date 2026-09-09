//! The public catalogue contract. Private workflow records must never use these types.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

pub mod editorial;
pub mod http;
pub mod preparation;
pub mod query;
pub mod remix;
pub mod settings;
pub mod setups;

pub const MAX_CATALOGUE_BYTES: usize = 8 * 1024 * 1024;

macro_rules! vocabulary {
    ($name:ident { $($variant:ident),+ }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
vocabulary!(Channel {
    Public,
    Development
});
vocabulary!(AppType {
    Desktop,
    Terminal,
    ShellPlugin,
    Web,
    Service
});
vocabulary!(Maturity {
    Development,
    Preview,
    Stable
});
vocabulary!(LicenceClass {
    OpenSource,
    SourceAvailable,
    Proprietary,
    Unknown
});
vocabulary!(Behaviour {
    Yes,
    No,
    Optional,
    Unknown
});
vocabulary!(Claim {
    Unclaimed,
    Verified,
    Expired,
    Suspended
});
vocabulary!(OfferModel {
    Free,
    Donation,
    PayWhatYouWant,
    Paid,
    Upgrade,
    PaidFeatures,
    Subscription,
    Service,
    WorkingPreview
});
vocabulary!(TaxInclusion {
    Included,
    Excluded,
    Unknown
});
vocabulary!(TestResult {
    Passes,
    Limitations,
    Fails,
    NotTested
});
vocabulary!(Freshness {
    Current,
    RetestDue,
    Superseded
});
vocabulary!(MediaKind {
    Icon,
    Screenshot,
    Demo
});

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalogue {
    pub schema_version: u32,
    pub channel: Channel,
    pub revision: String,
    pub build_revision: String,
    pub generated_at: String,
    pub apps: Vec<App>,
    pub makers: Vec<Maker>,
    pub recipes: Vec<Recipe>,
    pub editorial: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stories: Vec<editorial::Story>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct App {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: String,
    pub app_type: AppType,
    pub maturity: Maturity,
    pub licence: Licence,
    pub maker_ids: Vec<String>,
    pub homepage: String,
    pub source: Option<String>,
    pub support: String,
    pub media: Vec<Media>,
    pub capabilities: Vec<String>,
    pub current_release_id: String,
    pub releases: Vec<Release>,
    pub offers: Vec<Offer>,
    pub tests: Vec<TestRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Licence {
    pub class: LicenceClass,
    pub identifier: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Maker {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub bio: String,
    pub homepage: String,
    pub claim: Claim,
    pub claim_evidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_verified_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReleaseIdentity {
    SourceCommit {
        repository: String,
        commit: String,
    },
    BinaryArtifact {
        publisher: String,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallRoute {
    ArchPackage {
        repository: String,
        package: String,
        url: String,
    },
    AurExternal {
        package: String,
        url: String,
    },
    UpstreamExternal {
        url: String,
    },
    PluginExternal {
        url: String,
    },
}

impl InstallRoute {
    pub fn url(&self) -> &str {
        match self {
            Self::ArchPackage { url, .. }
            | Self::AurExternal { url, .. }
            | Self::UpstreamExternal { url }
            | Self::PluginExternal { url } => url,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::ArchPackage { .. } => "Arch repository",
            Self::AurExternal { .. } => "AUR · manual installation",
            Self::UpstreamExternal { .. } => "Developer website",
            Self::PluginExternal { .. } => "Plugin · manual installation",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Release {
    pub id: String,
    pub version: String,
    pub identity: ReleaseIdentity,
    pub architectures: Vec<String>,
    pub route: InstallRoute,
    pub notes: String,
    pub offline: Behaviour,
    pub account: Behaviour,
    pub activation: Behaviour,
    pub service_costs: String,
    pub privileges: Vec<String>,
    pub services: Vec<String>,
    pub removal: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Money {
    pub currency: String,
    pub minor_units: u64,
    pub exponent: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Offer {
    pub seller_id: String,
    pub model: OfferModel,
    pub url: String,
    pub price: Option<Money>,
    pub billing_interval: Option<String>,
    pub tax: TaxInclusion,
    pub checked_at: Option<String>,
    pub entitlement: String,
    pub terms: String,
    pub refund: Option<String>,
    pub cancellation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Media {
    pub kind: MediaKind,
    pub url: String,
    pub alt: String,
    pub rights: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestRecord {
    pub release_id: String,
    pub candidate_digest: String,
    pub executed_identity: ReleaseIdentity,
    pub executed_sha256: String,
    pub result: TestResult,
    pub freshness: Freshness,
    pub tested_at: String,
    pub environment: String,
    pub tool_version: String,
    pub actor: String,
    pub evidence: String,
    pub limitations: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recipe {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub revision: String,
    pub maker_id: String,
    pub parent: Option<String>,
    pub rights: String,
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<Media>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<setups::SettingReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub app_id: String,
    pub release_id: String,
    pub optional: bool,
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FieldError {
    pub path: String,
    pub code: String,
}

pub fn token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-._+".contains(&b))
}
fn digest(s: &str, length: usize) -> bool {
    s.len() == length
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn public_url(s: &str) -> bool {
    if s.len() > 2048 {
        return false;
    }
    Url::parse(s).is_ok_and(|u| u.scheme() == "https" && u.username().is_empty()
        && u.password().is_none() && u.port().is_none_or(|p| p == 443)
        && matches!(u.host(), Some(url::Host::Domain(h)) if h.contains('.') && !h.ends_with(".localhost") && !h.ends_with(".local")))
}
fn timestamp(s: &str) -> bool {
    s.ends_with('Z') && DateTime::parse_from_rfc3339(s).is_ok()
}

impl Catalogue {
    pub fn parse(bytes: &[u8], allow_development: bool) -> Result<Self, Vec<FieldError>> {
        if bytes.len() > MAX_CATALOGUE_BYTES {
            return Err(vec![FieldError {
                path: "$".into(),
                code: "catalogue_too_large".into(),
            }]);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| {
            vec![FieldError {
                path: "$".into(),
                code: "invalid_shape_or_unknown_field".into(),
            }]
        })?;
        let errors = value.validate(allow_development);
        if errors.is_empty() {
            Ok(value)
        } else {
            Err(errors)
        }
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut copy = self.clone();
        copy.apps.sort_by(|a, b| a.id.cmp(&b.id));
        copy.makers.sort_by(|a, b| a.id.cmp(&b.id));
        copy.recipes.sort_by(|a, b| a.id.cmp(&b.id));
        copy.stories.sort_by(|a, b| a.id.cmp(&b.id));
        serde_json::to_vec(&copy).expect("public types serialize")
    }

    pub fn snapshot_id(&self) -> String {
        format!("{:x}", Sha256::digest(self.canonical_bytes()))
    }

    pub fn validate(&self, allow_development: bool) -> Vec<FieldError> {
        let mut errors = Vec::new();
        let mut check = |condition: bool, path: String, code: &str| {
            if !condition && errors.len() < 100 {
                errors.push(FieldError {
                    path,
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
            "development_data_forbidden",
        );
        check(
            token(&self.revision) && token(&self.build_revision),
            "revision".into(),
            "invalid_revision",
        );
        check(
            timestamp(&self.generated_at),
            "generatedAt".into(),
            "invalid_utc_timestamp",
        );
        check(
            self.apps.len() <= 10_000 && self.makers.len() <= 10_000 && self.recipes.len() <= 1000,
            "$".into(),
            "too_many_records",
        );
        let mut ids = HashSet::new();
        let mut slugs = HashSet::new();
        let maker_ids: HashSet<&str> = self.makers.iter().map(|m| m.id.as_str()).collect();
        for (maker_index, maker) in self.makers.iter().enumerate() {
            check(
                token(&maker.id) && ids.insert(maker.id.clone()),
                format!("makers.{maker_index}.id"),
                "invalid_or_duplicate_id",
            );
            check(
                token(&maker.slug) && slugs.insert(maker.slug.clone()),
                format!("makers.{maker_index}.slug"),
                "invalid_or_duplicate_slug",
            );
            check(
                !maker.name.is_empty() && maker.name.len() <= 120 && maker.bio.len() <= 2000,
                format!("makers.{maker_index}"),
                "invalid_text_length",
            );
            check(
                public_url(&maker.homepage),
                format!("makers.{maker_index}.homepage"),
                "invalid_https_url",
            );
            check(
                maker.claim != Claim::Verified
                    || maker.claim_evidence.as_deref().is_some_and(public_url),
                format!("makers.{maker_index}.claim"),
                "missing_claim_evidence",
            );
        }
        for (index, maker) in self.makers.iter().enumerate() {
            let dates = match (&maker.claim_verified_at, &maker.claim_expires_at) {
                (None, None) => true,
                (Some(start), Some(end)) => {
                    timestamp(start)
                        && timestamp(end)
                        && DateTime::parse_from_rfc3339(start)
                            .ok()
                            .zip(DateTime::parse_from_rfc3339(end).ok())
                            .is_some_and(|(a, b)| b > a && (b - a).num_days() <= 31)
                }
                _ => false,
            };
            check(
                dates,
                format!("makers.{index}.claimExpiresAt"),
                "invalid_claim_window",
            );
            check(
                maker.support.as_deref().is_none_or(public_url),
                format!("makers.{index}.support"),
                "invalid_https_url",
            );
        }
        slugs.clear();
        for (app_index, app) in self.apps.iter().enumerate() {
            let p = format!("apps.{app_index}");
            check(
                token(&app.id) && ids.insert(app.id.clone()),
                format!("{p}.id"),
                "invalid_or_duplicate_id",
            );
            check(
                token(&app.slug) && slugs.insert(app.slug.clone()),
                format!("{p}.slug"),
                "invalid_or_duplicate_slug",
            );
            check(
                !app.name.is_empty()
                    && app.name.len() <= 120
                    && !app.summary.is_empty()
                    && app.summary.len() <= 240
                    && app.description.len() <= 12_000,
                p.clone(),
                "invalid_text_length",
            );
            check(
                serde_json::to_vec(app).is_ok_and(|b| b.len() <= 96 * 1024),
                p.clone(),
                "app_too_large",
            );
            check(
                token(&app.category) && app.tags.len() <= 20 && app.tags.iter().all(|t| token(t)),
                format!("{p}.tags"),
                "invalid_tags",
            );
            check(
                !app.maker_ids.is_empty()
                    && app
                        .maker_ids
                        .iter()
                        .all(|id| maker_ids.contains(id.as_str())),
                format!("{p}.makerIds"),
                "missing_maker",
            );
            check(
                public_url(&app.homepage)
                    && public_url(&app.support)
                    && app.source.as_deref().is_none_or(public_url)
                    && app.licence.url.as_deref().is_none_or(public_url),
                p.clone(),
                "invalid_https_url",
            );
            check(
                app.licence.class != LicenceClass::OpenSource
                    || app
                        .licence
                        .identifier
                        .as_ref()
                        .is_some_and(|s| !s.is_empty()),
                format!("{p}.licence"),
                "missing_licence_identifier",
            );
            let mut releases = HashSet::new();
            check(
                !app.releases.is_empty() && app.releases.len() <= 32,
                format!("{p}.releases"),
                "invalid_release_count",
            );
            for (release_index, release) in app.releases.iter().enumerate() {
                let rp = format!("{p}.releases.{release_index}");
                check(
                    token(&release.id) && releases.insert(release.id.as_str()),
                    rp.clone(),
                    "invalid_or_duplicate_release",
                );
                check(
                    !release.version.is_empty() && release.version.len() <= 128,
                    rp.clone(),
                    "invalid_version",
                );
                check(
                    valid_identity(&release.identity, &maker_ids),
                    rp.clone(),
                    "invalid_release_identity",
                );
                check(
                    !release.architectures.is_empty()
                        && release
                            .architectures
                            .iter()
                            .all(|a| ["x86_64", "aarch64", "any"].contains(&a.as_str())),
                    rp.clone(),
                    "unsupported_architecture",
                );
                check(
                    public_url(release.route.url()),
                    rp.clone(),
                    "invalid_https_url",
                );
                match &release.route {
                    InstallRoute::ArchPackage {
                        repository,
                        package,
                        ..
                    } => {
                        check(
                            token(repository) && token(package),
                            rp.clone(),
                            "unsafe_package_token",
                        );
                        if let ReleaseIdentity::RepositoryPackage {
                            repository: r,
                            package: n,
                            ..
                        } = &release.identity
                        {
                            check(
                                r == repository && n == package,
                                rp.clone(),
                                "route_identity_mismatch",
                            );
                        } else {
                            check(false, rp.clone(), "route_identity_mismatch");
                        }
                    }
                    InstallRoute::AurExternal { package, .. } => {
                        check(token(package), rp.clone(), "unsafe_package_token")
                    }
                    _ => {}
                }
            }
            check(
                releases.contains(app.current_release_id.as_str()),
                format!("{p}.currentReleaseId"),
                "missing_release",
            );
            for offer in &app.offers {
                check(
                    maker_ids.contains(offer.seller_id.as_str())
                        && app.maker_ids.contains(&offer.seller_id),
                    format!("{p}.offers"),
                    "invalid_seller",
                );
                check(
                    public_url(&offer.url)
                        && public_url(&offer.terms)
                        && offer.refund.as_deref().is_none_or(public_url)
                        && offer.cancellation.as_deref().is_none_or(public_url),
                    format!("{p}.offers"),
                    "invalid_https_url",
                );
                if let Some(money) = &offer.price {
                    check(
                        offer.model != OfferModel::Free || money.minor_units == 0,
                        format!("{p}.offers.price"),
                        "free_offer_has_price",
                    );
                    // Deliberately supported ISO currencies; expand with a reviewed exponent table.
                    let exponent = match money.currency.as_str() {
                        "USD" | "GBP" | "EUR" | "CAD" | "AUD" | "CHF" => Some(2),
                        "JPY" => Some(0),
                        "KWD" => Some(3),
                        _ => None,
                    };
                    check(
                        exponent == Some(money.exponent) && money.minor_units <= 1_000_000_000_000,
                        format!("{p}.offers.price"),
                        "invalid_money",
                    );
                }
                check(
                    offer.checked_at.as_deref().is_none_or(timestamp),
                    format!("{p}.offers.checkedAt"),
                    "invalid_utc_timestamp",
                );
                check(
                    offer.model != OfferModel::Subscription
                        || matches!(offer.billing_interval.as_deref(), Some("month" | "year")),
                    format!("{p}.offers.billingInterval"),
                    "invalid_billing_interval",
                );
            }
            check(app.media.len() <= 7, format!("{p}.media"), "too_many_media");
            for media in &app.media {
                check(
                    public_url(&media.url)
                        && digest(&media.sha256, 64)
                        && !media.alt.is_empty()
                        && !media.rights.is_empty(),
                    format!("{p}.media"),
                    "invalid_media_reference",
                );
            }
            for test in &app.tests {
                check(
                    digest(&test.executed_sha256, 64),
                    format!("{p}.tests"),
                    "missing_executed_byte_digest",
                );
                if let ReleaseIdentity::BinaryArtifact { sha256, .. } = &test.executed_identity {
                    check(
                        sha256 == &test.executed_sha256,
                        format!("{p}.tests"),
                        "executed_byte_digest_mismatch",
                    );
                }
                check(
                    releases.contains(test.release_id.as_str())
                        && valid_identity(&test.executed_identity, &maker_ids)
                        && digest(&test.candidate_digest, 64),
                    format!("{p}.tests"),
                    "invalid_evidence_identity",
                );
                check(
                    timestamp(&test.tested_at)
                        && !test.environment.is_empty()
                        && !test.tool_version.is_empty()
                        && !test.actor.is_empty()
                        && public_url(&test.evidence),
                    format!("{p}.tests"),
                    "incomplete_evidence",
                );
            }
        }
        slugs.clear();
        for (recipe_index, recipe) in self.recipes.iter().enumerate() {
            for error in setups::validate(recipe) {
                check(
                    false,
                    format!("recipes.{recipe_index}.{}", error.path),
                    &error.code,
                );
            }
            let p = format!("recipes.{recipe_index}");
            check(
                token(&recipe.id) && ids.insert(recipe.id.clone()) && token(&recipe.revision),
                p.clone(),
                "invalid_or_duplicate_id",
            );
            check(
                token(&recipe.slug) && slugs.insert(recipe.slug.clone()),
                p.clone(),
                "invalid_or_duplicate_slug",
            );
            check(
                maker_ids.contains(recipe.maker_id.as_str()),
                p.clone(),
                "missing_maker",
            );
            let mut components = HashSet::new();
            for component in &recipe.components {
                check(
                    components.insert(&component.app_id),
                    p.clone(),
                    "duplicate_component",
                );
                check(
                    self.apps.iter().any(|a| {
                        a.id == component.app_id
                            && a.releases.iter().any(|r| r.id == component.release_id)
                    }),
                    p.clone(),
                    "missing_component_release",
                );
                check(
                    component.depends_on.iter().all(|id| {
                        id != &component.app_id && recipe.components.iter().any(|c| &c.app_id == id)
                    }),
                    p.clone(),
                    "invalid_component_dependency",
                );
            }
            check(
                recipe.components.len() <= 128,
                p.clone(),
                "too_many_components",
            );
            if recipe.components.len() <= 128 {
                check(!recipe_has_cycle(recipe), p, "cyclic_components");
            }
        }
        check(
            self.editorial
                .iter()
                .all(|id| self.apps.iter().any(|a| &a.id == id)),
            "editorial".into(),
            "missing_editorial_app",
        );
        check(
            self.stories.len() <= 1000,
            "stories".into(),
            "too_many_stories",
        );
        for (index, story) in self.stories.iter().enumerate() {
            let p = format!("stories.{index}");
            check(
                token(&story.id) && ids.insert(story.id.clone()) && token(&story.revision),
                p.clone(),
                "invalid_or_duplicate_id",
            );
            check(
                !story.title.is_empty()
                    && story.title.len() <= 120
                    && story.summary.len() <= 280
                    && story.body.len() <= 6000
                    && !story.rights.is_empty()
                    && story.rights.len() <= 1000,
                p.clone(),
                "invalid_story_text",
            );
            check(
                maker_ids.contains(story.author_maker_id.as_str()),
                format!("{p}.authorMakerId"),
                "unknown_maker",
            );
            check(
                !story.app_ids.is_empty()
                    && story.app_ids.len() <= 12
                    && story
                        .app_ids
                        .iter()
                        .all(|id| self.apps.iter().any(|a| &a.id == id)),
                format!("{p}.appIds"),
                "missing_editorial_app",
            );
            check(
                timestamp(&story.publish_at)
                    && story.end_at.as_ref().is_none_or(|end| {
                        timestamp(end)
                            && DateTime::parse_from_rfc3339(end)
                                .ok()
                                .zip(DateTime::parse_from_rfc3339(&story.publish_at).ok())
                                .is_some_and(|(end, start)| end > start)
                    }),
                format!("{p}.publishAt"),
                "invalid_editorial_schedule",
            );
        }
        errors
    }
}

fn valid_identity(identity: &ReleaseIdentity, makers: &HashSet<&str>) -> bool {
    match identity {
        ReleaseIdentity::SourceCommit { repository, commit } => {
            public_url(repository) && (digest(commit, 40) || digest(commit, 64))
        }
        ReleaseIdentity::BinaryArtifact {
            publisher,
            version,
            sha256,
        } => makers.contains(publisher.as_str()) && !version.is_empty() && digest(sha256, 64),
        ReleaseIdentity::RepositoryPackage {
            repository,
            package,
            version,
            signature,
        } => {
            token(repository)
                && token(package)
                && !version.is_empty()
                && signature.as_deref().is_none_or(public_url)
        }
    }
}

fn recipe_has_cycle(recipe: &Recipe) -> bool {
    fn visit<'a>(
        id: &'a str,
        recipe: &'a Recipe,
        visiting: &mut HashSet<&'a str>,
        done: &mut HashSet<&'a str>,
    ) -> bool {
        if done.contains(id) {
            return false;
        }
        if !visiting.insert(id) {
            return true;
        }
        if let Some(c) = recipe.components.iter().find(|c| c.app_id == id) {
            if c.depends_on
                .iter()
                .any(|d| visit(d, recipe, visiting, done))
            {
                return true;
            }
        }
        visiting.remove(id);
        done.insert(id);
        false
    }
    let mut done = HashSet::new();
    recipe
        .components
        .iter()
        .any(|c| visit(&c.app_id, recipe, &mut HashSet::new(), &mut done))
}

impl App {
    pub fn candidate_digest(&self) -> String {
        let mut candidate = self.clone();
        candidate.tests.clear();
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&candidate).expect("public candidate serializes"))
        )
    }
    pub fn current_release(&self) -> &Release {
        self.releases
            .iter()
            .find(|r| r.id == self.current_release_id)
            .expect("validated release reference")
    }

    /// Evidence is tied to release bytes and elapsed time, never just a displayed date.
    pub fn evidence(&self, now: DateTime<Utc>) -> (&'static str, &'static str) {
        self.evidence_for_profile(now, None)
    }

    pub fn evidence_for_profile(
        &self,
        now: DateTime<Utc>,
        profile: Option<&str>,
    ) -> (&'static str, &'static str) {
        let release = self.current_release();
        let candidate = self.candidate_digest();
        let Some(test) = self
            .tests
            .iter()
            .filter(|t| {
                t.release_id == release.id
                    && t.executed_identity == release.identity
                    && t.candidate_digest == candidate
                    && profile.is_none_or(|p| t.environment == p)
            })
            .max_by(|a, b| a.tested_at.cmp(&b.tested_at))
        else {
            return (
                "not_tested",
                if self.tests.is_empty() {
                    "Not tested on Omarchy"
                } else {
                    "Only another release or environment was tested"
                },
            );
        };
        let date = DateTime::parse_from_rfc3339(&test.tested_at).expect("validated timestamp");
        if date > now
            || now.signed_duration_since(date).num_days() >= 90
            || test.freshness != Freshness::Current
        {
            return ("retest_due", "Previous evidence · retest required");
        }
        match test.result {
            TestResult::Passes => ("passes", "Tested on the recorded Omarchy environment"),
            TestResult::Limitations => ("limitations", "Tested with limitations"),
            TestResult::Fails => ("fails", "Does not pass the recorded test"),
            TestResult::NotTested => ("not_tested", "Not tested on Omarchy"),
        }
    }
}
