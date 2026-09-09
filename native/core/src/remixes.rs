use crate::{library::Store, platform::Result, settings::db};
use omastore_catalogue::{
    remix::{self, Remix},
    Catalogue,
};
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
};
pub fn migrate(c: &rusqlite::Connection) -> Result<()> {
    db(c.execute_batch("CREATE TABLE IF NOT EXISTS local_remixes(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,body TEXT NOT NULL,updated_at INTEGER NOT NULL); PRAGMA user_version=5;"))?;
    Ok(())
}
pub fn get(s: &Store, id: &str) -> Result<Remix> {
    let raw: Option<String> = db(s
        .connection
        .query_row("SELECT body FROM local_remixes WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional())?;
    remix::import(raw.ok_or("local_remix_not_found")?.as_bytes(), s.demo)
}
fn save(s: &Store, r: &Remix, expected: Option<u32>, now: i64) -> Result<()> {
    remix::validate(r, s.demo)?;
    let t = db(s.connection.unchecked_transaction())?;
    let old: Option<(u32, String)> = db(t
        .query_row(
            "SELECT revision,body FROM local_remixes WHERE id=?1",
            [&r.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional())?;
    let body = serde_json::to_string(r).map_err(|_| "invalid_recipe_remix")?;
    if old.as_ref().is_some_and(|(_, v)| v == &body) {
        return Ok(());
    }
    if old.as_ref().map(|(v, _)| *v) != expected
        || old
            .as_ref()
            .is_some_and(|(v, _)| r.revision != v.saturating_add(1))
    {
        return Err("local_remix_changed");
    }
    let count: u32 = db(t.query_row("SELECT count(*) FROM local_remixes", [], |r| r.get(0)))?;
    if old.is_none() && count >= 100 {
        return Err("local_remix_limit");
    }
    db(t.execute("INSERT INTO local_remixes VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,body=excluded.body,updated_at=excluded.updated_at",params![r.id,r.revision,body,now]))?;
    db(t.commit())
}
fn path(url: &str) -> Result<PathBuf> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .filter(|p| p.is_absolute())
        .ok_or("local_file_required")
}
fn review(c: &Catalogue, r: &Remix, s: &Store) -> Result<Value> {
    let mut v = remix::review(c, r, s.demo)?;
    v["installationSelection"] = json!({"kind":"apps","components":r.components.iter().map(|p|json!({"appId":p.app_id,"releaseId":p.release_id})).collect::<Vec<_>>()});
    v["settingsPreview"] = if r.settings.is_empty() {
        json!({"effects":[],"canApply":false})
    } else {
        serde_json::to_value(crate::settings::build(
            crate::settings::Request {
                settings: r.settings.clone(),
            },
            &crate::settings::environment(s)?,
            chrono::Utc::now().timestamp(),
        )?)
        .map_err(|_| "settings_unavailable")?
    };
    Ok(v)
}
pub fn request(s: &Store, c: &Catalogue, method: &str, value: Value, now: i64) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Lookup {
        id: String,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        file: String,
    }
    match method {
        "remixes.create" => {
            let r = remix::create(
                c,
                serde_json::from_value(value).map_err(|_| "invalid_remix_request")?,
                format!(
                    "local-{}",
                    &omastore_workflow::nonce().map_err(|_| "local_state_unavailable")?[..24]
                ),
                s.demo,
            )?;
            save(s, &r, None, now)?;
            review(c, &r, s)
        }
        "remixes.list" if value == json!({}) => {
            let mut q = db(s
                .connection
                .prepare("SELECT body FROM local_remixes ORDER BY updated_at DESC,id LIMIT 100"))?;
            let rows = db(q.query_map([], |r| r.get::<_, String>(0)))?
                .collect::<rusqlite::Result<Vec<_>>>();
            let mut items = Vec::new();
            for body in db(rows)? {
                let r = remix::import(body.as_bytes(), s.demo)?;
                items
                    .push(json!({"id":r.id,"revision":r.revision,"name":r.name,"parent":r.parent}));
            }
            Ok(json!({"items":items}))
        }
        "remixes.get" => {
            let p: Lookup = serde_json::from_value(value).map_err(|_| "invalid_remix_request")?;
            review(c, &get(s, &p.id)?, s)
        }
        "remixes.rename" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Rename {
                id: String,
                version: u32,
                name: String,
            }
            let p: Rename = serde_json::from_value(value).map_err(|_| "invalid_remix_request")?;
            let mut r = get(s, &p.id)?;
            if r.revision != p.version {
                return Err("local_remix_changed");
            }
            r.name = p.name;
            r.revision = r.revision.checked_add(1).ok_or("local_remix_limit")?;
            save(s, &r, Some(p.version), now)?;
            review(c, &r, s)
        }
        "remixes.export" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Export {
                id: String,
                digest: String,
                file: String,
            }
            let p: Export = serde_json::from_value(value).map_err(|_| "invalid_remix_request")?;
            let r = get(s, &p.id)?;
            let v = review(c, &r, s)?;
            if v["digest"] != p.digest {
                return Err("remix_export_changed");
            }
            let target = path(&p.file)?;
            if fs::symlink_metadata(&target).is_ok_and(|m| !m.is_file()) {
                return Err("regular_file_required");
            }
            let mut temp =
                tempfile::NamedTempFile::new_in(target.parent().ok_or("local_file_required")?)
                    .map_err(|_| "local_export_failed")?;
            temp.write_all(&serde_json::to_vec_pretty(&r).map_err(|_| "invalid_recipe_remix")?)
                .map_err(|_| "local_export_failed")?;
            temp.as_file()
                .sync_all()
                .map_err(|_| "local_export_failed")?;
            temp.persist(target).map_err(|_| "local_export_failed")?;
            Ok(json!({"exported":true,"id":p.id,"digest":p.digest}))
        }
        "remixes.import" => {
            let p: File = serde_json::from_value(value).map_err(|_| "invalid_remix_request")?;
            let f = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path(&p.file)?)
                .map_err(|_| "local_import_failed")?;
            let m = f.metadata().map_err(|_| "local_import_failed")?;
            if !m.is_file() || m.len() > 128 * 1024 {
                return Err("invalid_recipe_remix");
            }
            let mut bytes = Vec::new();
            f.take(128 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "local_import_failed")?;
            let r = remix::import(&bytes, s.demo)?;
            save(s, &r, None, now)?;
            review(c, &r, s)
        }
        _ => Err("unsupported_method"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_save_cross_machine_import_and_revision_conflicts() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(&d.path().join("first"), true).unwrap();
        let c: Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let p = &c.recipes[0];
        let v=request(&s,&c,"remixes.create",json!({"selection":{"id":p.id,"revision":p.revision,"chosen":[p.components[0].app_id]},"name":"My desk","settings":[]}),1000).unwrap();
        let r = &v["remix"];
        let out = d.path().join("selected.json");
        let url = url::Url::from_file_path(&out).unwrap();
        request(
            &s,
            &c,
            "remixes.export",
            json!({"id":r["id"],"digest":v["digest"],"file":url.as_str()}),
            1001,
        )
        .unwrap();
        assert!(
            !String::from_utf8_lossy(&fs::read(&out).unwrap()).contains(d.path().to_str().unwrap())
        );
        let other = Store::at(&d.path().join("second"), true).unwrap();
        assert_eq!(
            request(
                &other,
                &c,
                "remixes.import",
                json!({"file":url.as_str()}),
                1002
            )
            .unwrap()["remix"],
            *r
        );
        request(
            &s,
            &c,
            "remixes.rename",
            json!({"id":r["id"],"version":1,"name":"New desk"}),
            1003,
        )
        .unwrap();
        assert_eq!(
            request(&s, &c, "remixes.import", json!({"file":url.as_str()}), 1004).unwrap_err(),
            "local_remix_changed"
        );
        let count: u32 = s
            .connection
            .query_row("SELECT count(*) FROM operations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
