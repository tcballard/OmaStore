//! Reviewed editorial content and maker attribution. No payment signal participates in ordering.
use crate::{query, Catalogue, Claim, Maker};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Story,
    Pick,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Story {
    pub id: String,
    pub revision: String,
    pub kind: Kind,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub author_maker_id: String,
    pub app_ids: Vec<String>,
    pub publish_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
    pub rights: String,
}
impl Story {
    pub fn visible(&self, now: DateTime<Utc>) -> bool {
        DateTime::parse_from_rfc3339(&self.publish_at).is_ok_and(|d| d <= now)
            && self
                .end_at
                .as_ref()
                .is_none_or(|s| DateTime::parse_from_rfc3339(s).is_ok_and(|d| d > now))
    }
}
pub fn maker_claim(maker: &Maker, now: DateTime<Utc>) -> (&'static str, &'static str) {
    match maker.claim {
        Claim::Unclaimed => (
            "unclaimed",
            "Community listing · publisher has not claimed this profile",
        ),
        Claim::Expired => ("expired", "Project control needs renewal"),
        Claim::Suspended => ("suspended", "Project control is suspended"),
        Claim::Verified => {
            let current = maker
                .claim_verified_at
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .is_some_and(|t| t <= now)
                && maker
                    .claim_expires_at
                    .as_ref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .is_some_and(|t| t > now);
            if current {
                ("verified", "Scoped project control verified")
            } else {
                ("expired", "Project control is not currently verified")
            }
        }
    }
}
pub fn maker_summary(m: &Maker, now: DateTime<Utc>) -> Value {
    let (claim, label) = maker_claim(m, now);
    let mut value = json!(m);
    value["claim"] = json!(claim);
    value["claimLabel"] = json!(label);
    value
}
pub fn maker_detail(c: &Catalogue, id: &str, now: DateTime<Utc>) -> Result<Value, &'static str> {
    let maker = c
        .makers
        .iter()
        .find(|m| m.id == id || m.slug == id)
        .ok_or("not_found")?;
    let apps = c
        .apps
        .iter()
        .filter(|a| a.maker_ids.contains(&maker.id))
        .take(30)
        .map(|a| query::summary(a, now))
        .collect::<Vec<_>>();
    let stories = c
        .stories
        .iter()
        .filter(|s| s.author_maker_id == maker.id && s.visible(now))
        .take(10)
        .collect::<Vec<_>>();
    let mut value = maker_summary(maker, now);
    value["appCount"] = json!(c
        .apps
        .iter()
        .filter(|a| a.maker_ids.contains(&maker.id))
        .count());
    value["apps"] = json!(apps);
    value["stories"] = json!(stories);
    value["notice"] = json!(
        "Project control, store participation and software compatibility are separate facts."
    );
    Ok(value)
}
pub fn stories(c: &Catalogue, now: DateTime<Utc>) -> Value {
    let mut items = c
        .stories
        .iter()
        .filter(|s| s.visible(now))
        .collect::<Vec<_>>();
    items.sort_by(|a, b| b.publish_at.cmp(&a.publish_at).then(a.id.cmp(&b.id)));
    json!({"items":items.into_iter().take(30).collect::<Vec<_>>(),"asOf":now.to_rfc3339_opts(chrono::SecondsFormat::Secs,true),"notice":"Independent editorial selection. Payment does not buy placement."})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editorial_scheduling_and_claim_dates_do_not_invent_participation() {
        let mut c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let now = Utc::now();
        let mut story = Story {
            id: "story-one".into(),
            revision: "r1".into(),
            kind: Kind::Story,
            title: "A useful workflow".into(),
            summary: "A reviewed editorial fixture".into(),
            body: "Fictional text".into(),
            author_maker_id: c.makers[0].id.clone(),
            app_ids: vec![c.apps[0].id.clone()],
            publish_at: (now + chrono::Duration::seconds(10))
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            end_at: None,
            rights: "Fixture rights".into(),
        };
        assert!(!story.visible(now));
        assert!(story.visible(now + chrono::Duration::seconds(11)));
        story.end_at = Some(
            (now + chrono::Duration::seconds(20))
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        );
        assert!(!story.visible(now + chrono::Duration::seconds(21)));
        c.stories.push(story);
        assert!(c.validate(true).is_empty());
        assert_eq!(maker_claim(&c.makers[0], now).0, "unclaimed");
        c.makers[0].claim = Claim::Verified;
        assert_eq!(maker_claim(&c.makers[0], now).0, "expired");
        c.makers[0].claim_verified_at = Some(
            (now - chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        );
        c.makers[0].claim_expires_at = Some(
            (now + chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        );
        assert_eq!(maker_claim(&c.makers[0], now).0, "verified");
        assert_eq!(
            maker_claim(&c.makers[0], now + chrono::Duration::days(2)).0,
            "expired"
        );
        c.stories[0].app_ids = vec!["missing-app".into()];
        assert!(!c.validate(true).is_empty());
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Browse {
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub offset: usize,
    pub snapshot: Option<String>,
}
pub fn makers(c: &Catalogue, q: &Browse, now: DateTime<Utc>) -> Result<Value, &'static str> {
    if q.q.len() > 200 || q.offset > 10000 {
        return Err("invalid_filter");
    }
    let snapshot = c.snapshot_id();
    if q.offset > 0 && q.snapshot.as_deref() != Some(&snapshot) {
        return Err("snapshot_changed");
    }
    let needle = q.q.to_lowercase();
    let mut found = c
        .makers
        .iter()
        .filter(|m| {
            needle.is_empty()
                || format!("{} {}", m.name, m.bio)
                    .to_lowercase()
                    .contains(&needle)
        })
        .collect::<Vec<_>>();
    found.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    let total = found.len();
    let items = found
        .into_iter()
        .skip(q.offset)
        .take(30)
        .map(|m| maker_summary(m, now))
        .collect::<Vec<_>>();
    Ok(
        json!({"items":items,"total":total,"offset":q.offset,"nextOffset":if q.offset+30<total{Some(q.offset+30)}else{None},"snapshot":snapshot}),
    )
}
pub fn story(c: &Catalogue, id: &str, now: DateTime<Utc>) -> Result<Value, &'static str> {
    let s = c
        .stories
        .iter()
        .find(|s| s.id == id && s.visible(now))
        .ok_or("not_found")?;
    Ok(
        json!({"story":s,"maker":c.makers.iter().find(|m|m.id==s.author_maker_id).map(|m|maker_summary(m,now)),"apps":c.apps.iter().filter(|a|s.app_ids.contains(&a.id)).map(|a|query::summary(a,now)).collect::<Vec<_>>()}),
    )
}
