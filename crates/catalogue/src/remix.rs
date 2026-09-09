//! Portable selected intent; never serialize local desktop state or inventory.
use crate::{
    setups::{self, SettingReference},
    Catalogue, Component, Media, Recipe,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parent {
    pub id: String,
    pub revision: String,
    pub maker_id: String,
    pub rights: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Remix {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub parent: Parent,
    pub rights: String,
    pub media: Vec<Media>,
    pub components: Vec<Component>,
    pub settings: Vec<SettingReference>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Create {
    pub selection: setups::Selection,
    pub settings: Vec<SettingReference>,
    pub name: String,
}
pub fn create(
    c: &Catalogue,
    input: Create,
    id: String,
    sample: bool,
) -> Result<Remix, &'static str> {
    let selected = setups::select(c, &input.selection, chrono::Utc::now())?;
    if selected["valid"] != true {
        return Err("setup_selection_blocked");
    }
    let p = c
        .recipes
        .iter()
        .find(|p| p.id == input.selection.id && p.revision == input.selection.revision)
        .ok_or("setup_not_found")?;
    let mut components = p
        .components
        .iter()
        .filter(|p| {
            selected["selected"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == &p.app_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let ids = components
        .iter()
        .map(|p| p.app_id.clone())
        .collect::<BTreeSet<_>>();
    for part in &mut components {
        part.conflicts.retain(|id| ids.contains(id));
    }
    let r = Remix {
        schema_version: 1,
        kind: "omastore_recipe_remix".into(),
        id,
        revision: 1,
        name: input.name,
        parent: Parent {
            id: p.id.clone(),
            revision: p.revision.clone(),
            maker_id: p.maker_id.clone(),
            rights: p.rights.clone(),
        },
        rights: p.rights.clone(),
        media: p.media.clone(),
        components,
        settings: input.settings,
    };
    validate(&r, sample)?;
    Ok(r)
}
pub fn as_recipe(r: &Remix, id: &str, maker: &str) -> Result<Recipe, &'static str> {
    if !crate::token(id) || !crate::token(maker) {
        return Err("invalid_recipe_identity");
    }
    Ok(Recipe {
        id: id.into(),
        slug: id.into(),
        name: r.name.clone(),
        summary: format!("A selected remix of {}", r.parent.id),
        description: format!(
            "Based on {} revision {} by {}. Review every component and setting before adoption.",
            r.parent.id, r.parent.revision, r.parent.maker_id
        ),
        revision: r.revision.to_string(),
        maker_id: maker.into(),
        parent: Some(r.parent.id.clone()),
        parent_revision: Some(r.parent.revision.clone()),
        rights: r.rights.clone(),
        media: r.media.clone(),
        components: r.components.clone(),
        settings: r.settings.clone(),
    })
}
pub fn validate(r: &Remix, sample: bool) -> Result<(), &'static str> {
    if r.schema_version != 1
        || r.kind != "omastore_recipe_remix"
        || !r.id.starts_with("local-")
        || !crate::token(&r.id)
        || r.revision == 0
        || r.name.is_empty()
        || r.name.len() > 160
        || r.name.chars().any(|c| c.is_control())
        || !crate::token(&r.parent.id)
        || !crate::token(&r.parent.revision)
        || !crate::token(&r.parent.maker_id)
        || r.rights != r.parent.rights
    {
        return Err("invalid_recipe_remix");
    }
    let recipe = as_recipe(r, &r.id, "local-curator")?;
    if !setups::validate(&recipe).is_empty() {
        return Err("invalid_recipe_remix");
    }
    let ids = r
        .components
        .iter()
        .map(|p| &p.app_id)
        .collect::<BTreeSet<_>>();
    if ids.len() != r.components.len()
        || r.components.iter().any(|p| {
            !crate::token(&p.app_id)
                || !crate::token(&p.release_id)
                || p.depends_on.iter().any(|id| !ids.contains(id))
        })
    {
        return Err("invalid_remix_components");
    }
    fn visit<'a>(
        id: &'a str,
        components: &'a [Component],
        visiting: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> bool {
        if done.contains(id) {
            return true;
        }
        if !visiting.insert(id) {
            return false;
        }
        let Some(part) = components.iter().find(|p| p.app_id == id) else {
            return false;
        };
        if part.depends_on.iter().collect::<BTreeSet<_>>().len() != part.depends_on.len()
            || !part
                .depends_on
                .iter()
                .all(|id| visit(id, components, visiting, done))
        {
            return false;
        }
        visiting.remove(id);
        done.insert(id);
        true
    }
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    if !r
        .components
        .iter()
        .all(|p| visit(&p.app_id, &r.components, &mut visiting, &mut done))
    {
        return Err("cyclic_remix_components");
    }
    for reference in &r.settings {
        crate::settings::resolve(reference, sample)?;
    }
    Ok(())
}
pub fn review(c: &Catalogue, r: &Remix, sample: bool) -> Result<Value, &'static str> {
    validate(r, sample)?;
    let mut differences = Vec::new();
    match c.recipes.iter().find(|p|p.id==r.parent.id&&p.revision==r.parent.revision){Some(p)=>{if p.maker_id!=r.parent.maker_id||p.rights!=r.parent.rights||p.media!=r.media{differences.push(json!({"kind":"parent_attribution_changed","id":r.parent.id}));}},None=>differences.push(json!({"kind":"parent_revision_unavailable","id":r.parent.id,"revision":r.parent.revision}))}
    for p in &r.components {
        let kind = match c.apps.iter().find(|a| a.id == p.app_id) {
            None => Some("component_missing"),
            Some(a) if !a.releases.iter().any(|v| v.id == p.release_id) => {
                Some("component_release_missing")
            }
            Some(a) if a.current_release_id != p.release_id => Some("component_release_changed"),
            _ => None,
        };
        if let Some(kind) = kind {
            differences.push(json!({"kind":kind,"id":p.app_id,"revision":p.release_id}));
        }
    }
    for part in &r.components {
        if !part.conflicts.is_empty() {
            differences.push(json!({"kind":"component_conflict","id":part.app_id}));
        }
    }
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(r).map_err(|_| "invalid_recipe_remix")?)
    );
    Ok(
        json!({"remix":r,"export":r,"digest":digest,"differences":differences,"ready":differences.is_empty(),"notice":"Saved locally. Export contains only selected intent and public attribution. This does not install, change settings or publish."}),
    )
}
pub fn import(bytes: &[u8], sample: bool) -> Result<Remix, &'static str> {
    if bytes.len() > 128 * 1024 {
        return Err("recipe_remix_too_large");
    }
    let r: Remix = serde_json::from_slice(bytes).map_err(|_| "invalid_recipe_remix")?;
    validate(&r, sample)?;
    Ok(r)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_intent_round_trips_without_private_fields_or_silent_substitutions() {
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let p = &c.recipes[0];
        let r = create(
            &c,
            Create {
                selection: setups::Selection {
                    id: p.id.clone(),
                    revision: p.revision.clone(),
                    chosen: Some(vec![p.components[0].app_id.clone()]),
                },
                settings: vec![],
                name: "My desk".into(),
            },
            "local-fixture".into(),
            true,
        )
        .unwrap();
        assert_eq!(r.media, p.media);
        assert!(r.settings.is_empty());
        assert_eq!(r.parent.id, p.id);
        let mut v = json!(r);
        v["localState"] = json!({"home":"/home/person","token":"credential","accountId":"private"});
        assert!(import(&serde_json::to_vec(&v).unwrap(), true).is_err());
        v.as_object_mut().unwrap().remove("localState");
        v["settings"] =
            json!([{"adapter":"omarchy-theme","valueId":"/home/person/token","revision":"1"}]);
        assert!(import(&serde_json::to_vec(&v).unwrap(), true).is_err());
        let imported = import(&serde_json::to_vec(&r).unwrap(), true).unwrap();
        assert_eq!(review(&c, &imported, true).unwrap()["ready"], true);
        let mut other = c.clone();
        other.apps.clear();
        let changed = review(&other, &imported, true).unwrap();
        assert_eq!(changed["ready"], false);
        assert_eq!(changed["remix"]["components"], json!(r.components));
    }
}
