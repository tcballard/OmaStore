use omastore_catalogue::{setups, Catalogue};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
fn local_file(value: &str) -> Result<PathBuf, &'static str> {
    url::Url::parse(value)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .filter(|p| p.is_absolute())
        .ok_or("local_file_required")
}
pub fn export(c: &Catalogue, params: Value) -> Result<Value, &'static str> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        selection: setups::Selection,
        snapshot: String,
        file: String,
    }
    let r: Request = serde_json::from_value(params).map_err(|_| "invalid_request")?;
    if r.snapshot != c.snapshot_id() {
        return Err("snapshot_changed");
    }
    let value = setups::select(c, &r.selection, chrono::Utc::now())?;
    if value["valid"] != true {
        return Err("setup_selection_blocked");
    }
    let path = local_file(&r.file)?;
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
        return Err("regular_file_required");
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("local_file_required")?)
        .map_err(|_| "local_export_failed")?;
    file.write_all(
        &serde_json::to_vec_pretty(&value["export"]).map_err(|_| "invalid_selection_export")?,
    )
    .map_err(|_| "local_export_failed")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "local_export_failed")?;
    file.persist(path).map_err(|_| "local_export_failed")?;
    Ok(json!({"exported":true,"id":r.selection.id,"revision":r.selection.revision}))
}
pub fn import(c: &Catalogue, params: Value) -> Result<Value, &'static str> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        file: String,
    }
    let r: Request = serde_json::from_value(params).map_err(|_| "invalid_request")?;
    let path = local_file(&r.file)?;
    let meta = std::fs::symlink_metadata(&path).map_err(|_| "local_import_failed")?;
    if !meta.is_file() || meta.len() > 128 * 1024 {
        return Err("invalid_selection_export");
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "local_import_failed")?
        .take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "local_import_failed")?;
    let mut value = setups::restore(c, &bytes, chrono::Utc::now())?;
    value["imported"] = json!(true);
    Ok(value)
}
