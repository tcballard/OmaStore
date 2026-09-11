//! Bounded normalization and an explicit private/public object-storage boundary.
use crate::{
    bounded, digest, nonce,
    store::{audit, recheck},
    Actor, Error, Result, Store,
};
use image::{ImageFormat, ImageReader};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const ICON_LIMIT: usize = 1024 * 1024;
pub const SCREENSHOT_LIMIT: usize = 5 * 1024 * 1024;
pub const VIDEO_LIMIT: usize = 30 * 1024 * 1024;
pub struct Asset {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub content_type: &'static str,
    pub width: u32,
    pub height: u32,
    pub duration_ms: Option<i64>,
}
pub trait ObjectStorage: Send + Sync {
    fn put_private(&self, key: &str, bytes: &[u8]) -> Result<()>;
    fn read_private(&self, key: &str, limit: usize) -> Result<Vec<u8>>;
    fn promote_public(&self, key: &str) -> Result<()>;
    fn remove_private(&self, key: &str) -> Result<()>;
}
#[derive(Clone)]
pub struct LocalObjects {
    pub root: PathBuf,
}
impl LocalObjects {
    pub fn new(root: &Path) -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;
        for path in [
            root.to_owned(),
            root.join("private"),
            root.join("public"),
            root.join("work"),
        ] {
            if !path.exists() {
                fs::create_dir_all(&path)?;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
            }
            if fs::symlink_metadata(&path)?.file_type().is_symlink() {
                return Err(Error::new(500, "unsafe_media_storage"));
            }
        }
        Ok(Self {
            root: root.to_owned(),
        })
    }
    fn path(&self, visibility: &str, key: &str) -> Result<PathBuf> {
        let (sha, extension) = key
            .split_once('.')
            .ok_or(Error::new(422, "invalid_media_key"))?;
        if sha.len() != 64
            || !sha.bytes().all(|b| b.is_ascii_hexdigit())
            || !["png", "mp4", "webm"].contains(&extension)
        {
            return Err(Error::new(422, "invalid_media_key"));
        }
        let path = self.root.join(visibility).join(key);
        if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file()) {
            return Err(Error::new(500, "unsafe_media_storage"));
        }
        Ok(path)
    }
    pub fn read_public(&self, key: &str) -> Result<Vec<u8>> {
        read(&self.path("public", key)?, VIDEO_LIMIT)
    }
    pub fn keys(&self) -> Result<Vec<(String, i64)>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(self.root.join("private"))?.take(10000) {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            self.path("private", &name)?;
            let modified = entry
                .metadata()?
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::new(500, "invalid_media_timestamp"))?
                .as_secs() as i64;
            result.push((name, modified));
        }
        Ok(result)
    }
}
impl ObjectStorage for LocalObjects {
    fn put_private(&self, key: &str, bytes: &[u8]) -> Result<()> {
        let path = self.path("private", key)?;
        if !key.starts_with(&digest(bytes)) {
            return Err(Error::new(422, "media_digest_mismatch"));
        }
        if path.exists() {
            if read(&path, VIDEO_LIMIT)? != bytes {
                return Err(Error::new(500, "media_integrity_failure"));
            }
            return Ok(());
        }
        let mut temporary = tempfile::NamedTempFile::new_in(self.root.join("private"))?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(path)
            .map_err(|_| Error::new(500, "media_store_failed"))?;
        File::open(self.root.join("private"))?.sync_all()?;
        Ok(())
    }
    fn read_private(&self, key: &str, limit: usize) -> Result<Vec<u8>> {
        read(&self.path("private", key)?, limit)
    }
    fn promote_public(&self, key: &str) -> Result<()> {
        let bytes = self.read_private(key, VIDEO_LIMIT)?;
        if !key.starts_with(&digest(&bytes)) {
            return Err(Error::new(500, "media_integrity_failure"));
        }
        let path = self.path("public", key)?;
        if path.exists() {
            if read(&path, VIDEO_LIMIT)? == bytes {
                return Ok(());
            }
            return Err(Error::new(500, "media_integrity_failure"));
        }
        let mut temporary = tempfile::NamedTempFile::new_in(self.root.join("public"))?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        temporary
            .persist_noclobber(path)
            .map_err(|_| Error::new(500, "media_store_failed"))?;
        Ok(())
    }
    fn remove_private(&self, key: &str) -> Result<()> {
        match fs::remove_file(self.path("private", key)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(Error::new(413, "media_too_large"));
    }
    Ok(bytes)
}
pub fn normalize(kind: &str, bytes: &[u8], objects: &LocalObjects) -> Result<Asset> {
    let limit = match kind {
        "icon" => ICON_LIMIT,
        "screenshot" => SCREENSHOT_LIMIT,
        "demo" => VIDEO_LIMIT,
        _ => return Err(Error::new(422, "invalid_media_kind")),
    };
    if bytes.is_empty() || bytes.len() > limit {
        return Err(Error::new(413, "media_too_large"));
    }
    if kind == "demo" {
        return normalize_video(bytes, objects);
    }
    let format =
        image::guess_format(bytes).map_err(|_| Error::new(422, "unsupported_media_format"))?;
    if !matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    ) || (kind == "icon" && format == ImageFormat::Jpeg)
    {
        return Err(Error::new(422, "unsupported_media_format"));
    }
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| Error::new(422, "invalid_image"))?;
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || u64::from(width) * u64::from(height) > 16_000_000
        || (kind == "icon" && (width > 1024 || height > 1024))
    {
        return Err(Error::new(422, "image_dimensions_exceeded"));
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| Error::new(422, "invalid_image"))?;
    let mut normalized = Cursor::new(Vec::new());
    image
        .write_to(&mut normalized, ImageFormat::Png)
        .map_err(|_| Error::new(422, "image_normalization_failed"))?;
    let bytes = normalized.into_inner();
    if bytes.len() > limit {
        return Err(Error::new(413, "normalized_media_too_large"));
    }
    Ok(Asset {
        bytes,
        extension: "png",
        content_type: "image/png",
        width,
        height,
        duration_ms: None,
    })
}
fn sandbox_command(work: &Path) -> Result<Command> {
    let work = work.canonicalize()?;
    if fs::symlink_metadata(&work)?.file_type().is_symlink() {
        return Err(Error::new(500, "unsafe_media_storage"));
    }
    let mut command = Command::new("/usr/bin/bwrap");
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("OPENBLAS_NUM_THREADS", "1")
        .args([
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/usr",
            "/usr",
            "--ro-bind",
            "/etc/ld.so.cache",
            "/etc/ld.so.cache",
            "--ro-bind-try",
            "/etc/alternatives",
            "/etc/alternatives",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--bind",
        ])
        .arg(work)
        .args(["/work", "--chdir", "/work", "--", "/usr/bin/prlimit"]);
    Ok(command)
}
fn decoder(program: &str, args: &[&str], limit: usize, work: &Path) -> Result<Vec<u8>> {
    let mut child = sandbox_command(work)?
        .args([
            "--as=1073741824",
            "--cpu=15",
            "--fsize=31457280",
            "--nofile=64",
            "--",
            program,
        ])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| Error::new(503, "media_processor_unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(Error::new(503, "media_processor_unavailable"))?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let until = Instant::now() + Duration::from_secs(20);
    let result = loop {
        match child.try_wait()? {
            Some(status) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(Error::new(422, "media_processor_rejected"))
                }
            }
            None if Instant::now() < until => std::thread::sleep(Duration::from_millis(20)),
            None => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(Error::new(422, "media_processor_timeout"));
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| Error::new(500, "media_processor_failed"))??;
    result?;
    if bytes.len() > limit {
        return Err(Error::new(422, "media_processor_output_exceeded"));
    }
    Ok(bytes)
}
fn normalize_video(bytes: &[u8], objects: &LocalObjects) -> Result<Asset> {
    let (extension, format, content_type) = if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        ("mp4", "mov", "video/mp4")
    } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
        ("webm", "matroska,webm", "video/webm")
    } else {
        return Err(Error::new(422, "unsupported_media_format"));
    };
    let work = tempfile::tempdir_in(objects.root.join("work"))?;
    let input = work.path().join(format!("input.{extension}"));
    let output = work.path().join(format!("output.{extension}"));
    fs::write(&input, bytes)?;
    let host_output = output.clone();
    let input = format!("/work/input.{extension}");
    let output = format!("/work/output.{extension}");
    decoder("/usr/bin/true", &[], 1024, work.path())
        .map_err(|_| Error::new(503, "media_sandbox_unavailable"))?;
    let probe = decoder(
        "/usr/bin/ffprobe",
        &[
            "-v",
            "error",
            "-protocol_whitelist",
            "file",
            "-f",
            format,
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,width,height",
            "-of",
            "json",
            &input,
        ],
        64 * 1024,
        work.path(),
    )?;
    let probe: Value = serde_json::from_slice(&probe)?;
    let duration = probe["format"]["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v >= 15.0 && *v <= 45.05)
        .ok_or(Error::new(422, "demo_duration_must_be_15_to_45_seconds"))?;
    let stream = probe["streams"]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["codec_type"] == "video"))
        .ok_or(Error::new(422, "video_stream_missing"))?;
    let width = stream["width"].as_u64().unwrap_or(0);
    let height = stream["height"].as_u64().unwrap_or(0);
    if width == 0
        || height == 0
        || width > 8192
        || height > 8192
        || width * height > 16_000_000
        || !["h264", "vp8", "vp9", "av1"].contains(&stream["codec_name"].as_str().unwrap_or(""))
    {
        return Err(Error::new(422, "unsupported_video_stream"));
    }
    decoder(
        "/usr/bin/ffmpeg",
        &[
            "-v",
            "error",
            "-nostdin",
            "-n",
            "-protocol_whitelist",
            "file",
            "-f",
            format,
            "-i",
            &input,
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-c",
            "copy",
            &output,
        ],
        1024,
        work.path(),
    )?;
    let bytes = read(&host_output, VIDEO_LIMIT)?;
    Ok(Asset {
        bytes,
        extension,
        content_type,
        width: width as u32,
        height: height as u32,
        duration_ms: Some((duration * 1000.0).round() as i64),
    })
}

pub struct Upload<'a> {
    pub draft_id: &'a str,
    pub version: i64,
    pub kind: &'a str,
    pub alt: &'a str,
    pub rights: &'a str,
    pub key: &'a str,
}
impl Store {
    pub fn attach_media(
        &self,
        actor: &Actor,
        upload: Upload<'_>,
        asset: &Asset,
        objects: &impl ObjectStorage,
        now: i64,
    ) -> Result<Value> {
        bounded(upload.alt, 2000)?;
        bounded(upload.rights, 2000)?;
        if upload.key.len() < 16
            || upload.key.len() > 128
            || !upload
                .key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(Error::new(422, "invalid_idempotency_key"));
        }
        let sha = digest(&asset.bytes);
        let object_key = format!("{sha}.{}", asset.extension);
        self.transaction(|t| {
            recheck(t,actor,"author")?;
            let hash=digest(serde_json::to_vec(&json!({"draft":upload.draft_id,"version":upload.version,"kind":upload.kind,"digest":sha,"alt":upload.alt,"rights":upload.rights}))?);
            let prior=t.query_row("SELECT digest,response FROM request_keys WHERE actor=?1 AND key=?2",params![actor.id,upload.key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
            if let Some((old,response))=prior {if old!=hash {return Err(Error::new(409,"idempotency_key_reused"));}return Ok(serde_json::from_str(&response)?);}
            crate::drafts::owned_version(t,actor,upload.draft_id,upload.version)?;
            let total=crate::operations::active_media_bytes(t,&actor.id,&sha)?;
            if total+asset.bytes.len() as i64>200*1024*1024 {return Err(Error::new(422,"media_quota_exceeded"));}
            let raw:String=t.query_row("SELECT candidate FROM drafts WHERE id=?1",[upload.draft_id],|r|r.get(0))?;
            let draft:Value=serde_json::from_str(&raw)?;
            let kind:String=t.query_row("SELECT kind FROM drafts WHERE id=?1",[upload.draft_id],|r|r.get(0))?;
            let collection=match kind.as_str(){"app"=>"apps","setup"=>"recipes",_=>return Err(Error::new(422,"media_not_supported_for_draft"))};
            let count=draft[collection][0]["media"].as_array().map(|m|m.iter().filter(|m|m["kind"]==upload.kind).count()).unwrap_or(0);
            if count>=if upload.kind=="screenshot"{5}else{1} {return Err(Error::new(422,"media_count_exceeded"));}
            objects.put_private(&object_key,&asset.bytes)?;
            let raw:String=t.query_row("SELECT candidate FROM drafts WHERE id=?1",[upload.draft_id],|r|r.get(0))?;let mut candidate:Value=serde_json::from_str(&raw)?;
            if candidate[collection][0]["media"].is_null(){candidate[collection][0]["media"]=json!([]);}
            let list=candidate[collection][0]["media"].as_array_mut().ok_or(Error::new(422,"draft_needs_media_fields"))?;
            let id=t.query_row("SELECT id FROM media WHERE draft_id=?1 AND digest=?2 AND kind=?3",params![upload.draft_id,sha,upload.kind],|r|r.get::<_,String>(0)).optional()?.unwrap_or(nonce()?);
            let item=json!({"kind":upload.kind,"url":format!("https://raw.githubusercontent.com/tcballard/OmaStore/catalogue-live/media/{object_key}"),"alt":upload.alt,"rights":upload.rights,"sha256":sha});list.push(item.clone());
            let candidate=serde_json::to_string(&candidate)?;if candidate.len()>crate::drafts::MAX_DRAFT_BYTES {return Err(Error::new(413,"candidate_too_large"));}
            t.execute("INSERT OR IGNORE INTO media VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,NULL)",params![id,upload.draft_id,actor.id,upload.kind,sha,asset.content_type,asset.bytes.len() as i64,asset.width,asset.height,asset.duration_ms,upload.alt,upload.rights,now])?;
            t.execute("UPDATE drafts SET candidate=?2,version=version+1,updated_at=?3,expiry_notice_at=NULL WHERE id=?1",params![upload.draft_id,candidate,now])?;
            audit(t,&actor.id,"media_attached",upload.draft_id,now,&json!({"media":id,"digest":sha}))?;
            let response=json!({"id":id,"draftId":upload.draft_id,"version":upload.version+1,"media":item});
            t.execute("INSERT INTO request_keys VALUES(?1,?2,?3,?4,?5)",params![actor.id,upload.key,hash,response.to_string(),now])?;Ok(response)
        })
    }
    pub fn private_media_key(&self, actor: &Actor, id: &str) -> Result<(String, String)> {
        let c = self.connection()?;
        recheck(&c, actor, "author")?;
        let row = c
            .query_row(
                "SELECT digest,content_type,owner,draft_id FROM media WHERE id=?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(Error::new(404, "media_unavailable"))?;
        let (sha, content_type, owner, draft) = row;
        if owner != actor.id {
            if recheck(&c, actor, "reviewer").is_err() {
                return Err(Error::new(404, "media_unavailable"));
            }
            let submitted:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM revisions r,json_each(r.candidate,'$.apps') a,json_each(a.value,'$.media') m WHERE r.draft_id=?1 AND json_extract(m.value,'$.sha256')=?2) OR EXISTS(SELECT 1 FROM revisions r,json_each(r.candidate,'$.recipes') a,json_each(a.value,'$.media') m WHERE r.draft_id=?1 AND json_extract(m.value,'$.sha256')=?2)",params![draft,sha],|r|r.get(0))?;
            if !submitted {
                return Err(Error::new(404, "media_unavailable"));
            }
        }
        let extension = match content_type.as_str() {
            "image/png" => "png",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            _ => return Err(Error::new(500, "media_integrity_failure")),
        };
        Ok((format!("{sha}.{extension}"), content_type))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_and_store_only_inert_bounded_media() {
        let dir = tempfile::tempdir().unwrap();
        let objects = LocalObjects::new(dir.path()).unwrap();
        assert!(normalize("icon", br#"<svg onload='alert(1)'/>"#, &objects).is_err());
        assert!(normalize("screenshot", b"not an image", &objects).is_err());
        let image = image::DynamicImage::new_rgba8(4, 4);
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        let asset = normalize("icon", bytes.get_ref(), &objects).unwrap();
        let key = format!("{}.png", digest(&asset.bytes));
        objects.put_private(&key, &asset.bytes).unwrap();
        assert!(objects.read_public(&key).is_err());
        objects.promote_public(&key).unwrap();
        assert_eq!(objects.read_public(&key).unwrap(), asset.bytes);
        assert!(objects.read_private("../../secret", 100).is_err());
        let too_large = image::DynamicImage::new_rgba8(1025, 1);
        let mut bytes = Cursor::new(Vec::new());
        too_large.write_to(&mut bytes, ImageFormat::Png).unwrap();
        assert!(normalize("icon", bytes.get_ref(), &objects).is_err());
    }
}

#[cfg(all(test, feature = "development-workflow"))]
mod setup_tests {
    use super::*;
    use crate::drafts::Command;
    #[test]
    fn recipe_media_stays_private_until_submission_and_reuses_published_app_context() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::development(&dir.path().join("workflow.db")).unwrap();
        let objects = LocalObjects::new(&dir.path().join("objects")).unwrap();
        let now = crate::now();
        let login = s.development_login("author", now).unwrap();
        let author = s.actor(login["token"].as_str().unwrap(), now).unwrap();
        let login = s.development_login("reviewer", now).unwrap();
        let reviewer = s.actor(login["token"].as_str().unwrap(), now).unwrap();
        let base: omastore_catalogue::Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        s.seed_sample_context(&base).unwrap();
        let mut candidate = base.clone();
        candidate.apps.clear();
        candidate.stories.clear();
        candidate.makers[0].id = "setup-author".into();
        candidate.makers[0].slug = "setup-author".into();
        candidate.recipes[0].id = "sample-setup".into();
        candidate.recipes[0].slug = "sample-setup".into();
        candidate.recipes[0].maker_id = "setup-author".into();
        candidate.recipes[0].parent = Some(base.recipes[0].id.clone());
        candidate.recipes[0].parent_revision = Some(base.recipes[0].revision.clone());
        let draft = s
            .command(
                &author,
                &nonce().unwrap(),
                Command::CreateDraft {
                    kind: "setup".into(),
                    candidate: json!(candidate),
                    base_revision: None,
                },
                now,
            )
            .unwrap();
        let id = draft["id"].as_str().unwrap();
        let mut forged = json!(candidate);
        forged["recipes"][0]["parent"] = Value::Null;
        assert_eq!(
            s.command(
                &author,
                &nonce().unwrap(),
                Command::SaveDraft {
                    id: id.into(),
                    version: 1,
                    candidate: forged
                },
                now
            )
            .unwrap_err()
            .code,
            "remix_attribution_changed"
        );
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(4, 4)
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let asset = normalize("icon", png.get_ref(), &objects).unwrap();
        let media = s
            .attach_media(
                &author,
                Upload {
                    draft_id: id,
                    version: 1,
                    kind: "icon",
                    alt: "Fictional setup icon",
                    rights: "Test fixture",
                    key: &nonce().unwrap(),
                },
                &asset,
                &objects,
                now,
            )
            .unwrap();
        let mid = media["id"].as_str().unwrap();
        assert!(s.private_media_key(&reviewer, mid).is_err());
        let preview = s
            .command(
                &author,
                &nonce().unwrap(),
                Command::PrepareDraft {
                    id: id.into(),
                    version: 2,
                },
                now,
            )
            .unwrap();
        assert_eq!(preview["errors"], json!([]));
        assert_eq!(
            preview["candidate"]["recipes"][0]["media"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(preview["candidate"]["apps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["media"].as_array().unwrap().is_empty()));
        let revision = s
            .command(
                &author,
                &nonce().unwrap(),
                Command::SubmitDraft {
                    id: id.into(),
                    version: 3,
                    confirm_public_preview: true,
                },
                now,
            )
            .unwrap();
        let rid = revision["id"].as_str().unwrap();
        assert!(s.private_media_key(&reviewer, mid).is_ok());
        let job = s.lease_checks(now).unwrap().unwrap();
        assert!(matches!(
            s.media_findings(&job).unwrap().result,
            crate::checks::Outcome::Pass
        ));
        let findings = ["schema", "links", "media"].map(|name| crate::checks::Finding {
            check: name.into(),
            result: crate::checks::Outcome::Pass,
            code: "fixture_review".into(),
            detail: "Explicit local fixture observation.".into(),
            private: false,
        });
        s.finish_checks(&job, &findings, now).unwrap();
        let version = s.revision(&author, rid).unwrap()["version"]
            .as_i64()
            .unwrap();
        let approved = s
            .command(
                &reviewer,
                &nonce().unwrap(),
                Command::ReviewDecision {
                    id: rid.into(),
                    version,
                    decision: "approve".into(),
                    reason: "Reviewed fictional setup rights and selective components".into(),
                    acknowledge_limits: true,
                },
                now,
            )
            .unwrap();
        s.command(
            &author,
            &nonce().unwrap(),
            Command::RequestPublication {
                id: rid.into(),
                version: approved["version"].as_i64().unwrap(),
            },
            now,
        )
        .unwrap();
        crate::sample_publication::rehearse(
            &s,
            &author,
            rid,
            &objects,
            &dir.path().join("catalogue.json"),
        )
        .unwrap();
        assert!(!s.release_feed(None).unwrap().contains("<item>"));
        assert_eq!(s.delivered_catalogue().unwrap().unwrap().recipes.len(), 2);
    }
}

#[cfg(test)]
mod sandbox_tests {
    use super::*;
    #[test]
    #[ignore = "requires Bubblewrap user namespaces and FFmpeg; mandatory deployment CI gate"]
    fn video_normalization_is_isolated_and_strips_metadata() {
        let d = tempfile::tempdir().unwrap();
        let o = LocalObjects::new(&d.path().join("objects")).unwrap();
        let work = tempfile::tempdir_in(o.root.join("work")).unwrap();
        let probe = sandbox_command(work.path())
            .unwrap()
            .args(["--", "/usr/bin/true"])
            .output()
            .unwrap();
        assert!(
            probe.status.success(),
            "Sandbox startup failed: {}",
            String::from_utf8_lossy(&probe.stderr[..probe.stderr.len().min(4096)])
        );
        let binary = sandbox_command(work.path())
            .unwrap()
            .args([
                "--as=1073741824",
                "--cpu=15",
                "--",
                "/usr/bin/ffprobe",
                "-version",
            ])
            .output()
            .unwrap();
        assert!(
            binary.status.success(),
            "Isolated decoder startup failed: {}",
            String::from_utf8_lossy(&binary.stderr[..binary.stderr.len().min(4096)])
        );
        let env = decoder("/usr/bin/env", &[], 4096, work.path()).unwrap();
        let env = String::from_utf8(env).unwrap();
        assert!(!env.contains("HOME="));
        assert!(!env.contains("TOKEN="));
        assert!(!env.contains("SECRET="));
        let outside = d.path().join("private-secret");
        fs::write(&outside, "private sentinel").unwrap();
        assert!(decoder(
            "/usr/bin/test",
            &["!", "-e", outside.to_str().unwrap()],
            1024,
            work.path()
        )
        .is_ok());
        let input = d.path().join("input.mp4");
        let status = Command::new("/usr/bin/ffmpeg")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .args([
                "-v",
                "error",
                "-nostdin",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=32x32:d=15:r=1",
                "-an",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-metadata",
                "title=PRIVATE_MEDIA_METADATA",
            ])
            .arg(&input)
            .status()
            .unwrap();
        assert!(status.success());
        let normalized = normalize_video(&fs::read(input).unwrap(), &o).unwrap();
        assert!(normalized.duration_ms.unwrap() >= 15000);
        assert!(!normalized
            .bytes
            .windows(b"PRIVATE_MEDIA_METADATA".len())
            .any(|w| w == b"PRIVATE_MEDIA_METADATA"));
    }
}
