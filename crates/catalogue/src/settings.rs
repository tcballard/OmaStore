//! Versioned public setting choices. No executable hooks, paths or arbitrary option keys.
use crate::setups::SettingReference;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Choice {
    Theme { theme_id: String },
    ClockPlacement { section: String, index: u8 },
}
pub fn resolve(reference: &SettingReference, sample: bool) -> Result<Choice, &'static str> {
    if reference.revision != "1" {
        return Err("unsupported_setting_revision");
    }
    match (reference.adapter.as_str(), reference.value_id.as_str()) {
        ("omarchy-theme", id)
            if if sample {
                ["sample-ink", "sample-sand"].contains(&id)
            } else {
                ["tokyo-night", "catppuccin"].contains(&id)
            } =>
        {
            Ok(Choice::Theme {
                theme_id: id.into(),
            })
        }
        ("omarchy-clock-placement", "left-start") => Ok(Choice::ClockPlacement {
            section: "left".into(),
            index: 0,
        }),
        ("omarchy-clock-placement", "center-start") => Ok(Choice::ClockPlacement {
            section: "center".into(),
            index: 0,
        }),
        ("omarchy-clock-placement", "right-start") => Ok(Choice::ClockPlacement {
            section: "right".into(),
            index: 0,
        }),
        _ => Err("unsupported_setting_choice"),
    }
}
pub fn adapters(sample: bool) -> Value {
    json!({"schemaVersion":1,"simulated":sample,"items":[
 {"id":"omarchy-theme","revision":"1","name":"Desktop theme","values":if sample{vec!["sample-ink","sample-sand"]}else{vec!["tokyo-night","catppuccin"]},"scope":"Omarchy theme selection and its generated application colours/background","restoration":"Reapplying the prior theme can have broad effects. It is not a backup of personalised application settings."},
 {"id":"omarchy-clock-placement","revision":"1","name":"Existing clock widget","values":["left-start","center-start","right-start"],"scope":"Placement of one existing omarchy.clock widget in the built-in bar","restoration":"Restore the previous placement only while the selected bar layout still matches the applied layout. Preserve widget options and unrelated settings."}
 ],"notice":if sample{"Fictional desktop fixture. Your real theme and bar are never changed."}else{"Live settings writes require measured adapter acceptance on the exact Omarchy version. Unknown choices remain manual."}})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_choices_are_typed_and_do_not_accept_paths_or_unknown_revisions() {
        let mut r = SettingReference {
            adapter: "omarchy-theme".into(),
            value_id: "tokyo-night".into(),
            revision: "1".into(),
        };
        assert!(resolve(&r, false).is_ok());
        assert!(resolve(&r, true).is_err());
        for value in [
            "../../private",
            "sk_live_key",
            "$(whoami)",
            "/home/person",
            "arbitrary-theme",
        ] {
            r.value_id = value.into();
            assert!(resolve(&r, false).is_err());
        }
        r.adapter = "omarchy-clock-placement".into();
        r.value_id = "right-start".into();
        assert!(matches!(
            resolve(&r, false),
            Ok(Choice::ClockPlacement { index: 0, .. })
        ));
        r.revision = "2".into();
        assert!(resolve(&r, false).is_err());
    }
}
