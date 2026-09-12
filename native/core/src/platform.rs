//! Read-only, bounded observations of the system package manager.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub type Result<T> = std::result::Result<T, &'static str>;
pub fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash<T: Serialize>(value: &T) -> String {
    digest(serde_json::to_vec(value).expect("serializable system value"))
}
pub fn package_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"@._+-".contains(&c))
}
pub fn version(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"@._+:-~".contains(&c))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Host {
    pub state: String,
    pub reason: String,
    pub architecture: String,
    pub omarchy_version: Option<String>,
    pub configuration: String,
    pub sync_databases: String,
    pub repositories: BTreeMap<String, bool>,
    pub installed: BTreeMap<String, String>,
    #[serde(default)]
    pub updates: BTreeMap<String, PackageUpdate>,
    #[serde(default)]
    pub databases_stale: bool,
    pub update_required: bool,
    pub locked: bool,
    pub simulated: bool,
}
impl Host {
    pub fn fingerprint(&self) -> String {
        hash(self)
    }
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({"state":self.state,"reason":self.reason,"architecture":self.architecture,"omarchyVersion":self.omarchy_version,"repositories":self.repositories,"installedCount":self.installed.len(),"updateRequired":self.update_required,"locked":self.locked,"simulated":self.simulated,"fingerprint":self.fingerprint(),"configurationFingerprint":self.configuration,"databaseFingerprint":self.sync_databases})
    }
}

pub struct Output {
    pub code: i32,
    pub text: String,
    pub error_output: bool,
}
/// Only fixed program paths reach this function. It is not an IPC command API.
pub fn read_command(program: &str, arguments: &[String]) -> Result<Output> {
    trusted_program(program)?;
    let mut child = Command::new(program)
        .args(arguments)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "probe_unavailable")?;
    let out = child.stdout.take().ok_or("probe_unavailable")?;
    let err = child.stderr.take().ok_or("probe_unavailable")?;
    fn bounded(mut input: impl Read) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        input
            .by_ref()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    }
    let stdout = std::thread::spawn(move || bounded(out));
    let stderr = std::thread::spawn(move || bounded(err));
    let start = Instant::now();
    let exit = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let bytes = stdout
        .join()
        .map_err(|_| "probe_unavailable")?
        .map_err(|_| "probe_unavailable")?;
    let errors = stderr
        .join()
        .map_err(|_| "probe_unavailable")?
        .map_err(|_| "probe_unavailable")?;
    if bytes.len() > 2 * 1024 * 1024 || errors.len() > 2 * 1024 * 1024 {
        return Err("probe_too_large");
    }
    Ok(Output {
        code: exit.ok_or("probe_timeout")?.code().unwrap_or(-1),
        text: String::from_utf8(bytes).map_err(|_| "probe_invalid")?,
        error_output: !errors.is_empty(),
    })
}
pub fn trusted_program(program: &str) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let resolved = fs::canonicalize(program).map_err(|_| "program_unavailable")?;
    if !resolved.starts_with("/usr") {
        return Err("untrusted_program");
    }
    for path in resolved.ancestors() {
        let meta = fs::metadata(path).map_err(|_| "untrusted_program")?;
        if meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            return Err("untrusted_program");
        }
    }
    if !resolved.is_file() {
        return Err("untrusted_program");
    }
    Ok(())
}
fn read(program: &str, args: &[&str]) -> Result<String> {
    let output = read_command(
        program,
        &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
    )?;
    if output.code != 0 {
        return Err("probe_unavailable");
    }
    Ok(output.text)
}
pub fn installed(text: &str) -> Result<BTreeMap<String, String>> {
    let mut packages = BTreeMap::new();
    for line in text.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 2
            || !package_token(parts[0])
            || !version(parts[1])
            || packages.insert(parts[0].into(), parts[1].into()).is_some()
            || packages.len() > 20000
        {
            return Err("invalid_package_state");
        }
    }
    Ok(packages)
}
/// pacman -Qu uses `name installed -> available`, not the -Q record shape.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageUpdate {
    pub installed_version: String,
    pub available_version: String,
    pub ignored: bool,
}
pub fn parse_updates(text: &str) -> Result<BTreeMap<String, PackageUpdate>> {
    let mut updates = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        if !(f.len() == 4 || (f.len() == 5 && f[4] == "[ignored]"))
            || !package_token(f[0])
            || !version(f[1])
            || f[2] != "->"
            || !version(f[3])
        {
            return Err("invalid_update_state");
        }
        let update = PackageUpdate {
            installed_version: f[1].into(),
            available_version: f[3].into(),
            ignored: f.len() == 5,
        };
        if updates.insert(f[0].into(), update).is_some() || updates.len() > 20000 {
            return Err("invalid_update_state");
        }
    }
    Ok(updates)
}
fn sync_digest(repositories: &BTreeMap<String, bool>) -> Result<(String, bool)> {
    let mut hash = Sha256::new();
    let mut stale = false;
    let mut total = 0;
    for name in repositories.keys() {
        let path = Path::new("/var/lib/pacman/sync").join(format!("{name}.db"));
        let meta = fs::symlink_metadata(&path).map_err(|_| "package_database_unavailable")?;
        if !meta.is_file() || meta.len() > 64 * 1024 * 1024 {
            return Err("package_database_unavailable");
        }
        total += meta.len();
        if total > 128 * 1024 * 1024 {
            return Err("package_database_too_large");
        }
        stale |= meta
            .modified()
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_none_or(|age| age > Duration::from_secs(48 * 3600));
        hash.update(name.as_bytes());
        let mut file = fs::File::open(path).map_err(|_| "package_database_unavailable")?;
        let mut buffer = [0u8; 65536];
        let mut count = 0;
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|_| "package_database_unavailable")?;
            if n == 0 {
                break;
            }
            count += n;
            if count > 64 * 1024 * 1024 {
                return Err("package_database_too_large");
            }
            hash.update(&buffer[..n]);
        }
    }
    Ok((format!("{:x}", hash.finalize()), stale))
}
pub fn probe() -> Host {
    let mut host = Host {
        state: "unsupported".into(),
        reason: "Omarchy on Arch x86_64 is required".into(),
        architecture: std::env::consts::ARCH.into(),
        omarchy_version: None,
        configuration: String::new(),
        sync_databases: String::new(),
        repositories: BTreeMap::new(),
        installed: BTreeMap::new(),
        updates: BTreeMap::new(),
        databases_stale: false,
        update_required: false,
        locked: Path::new("/var/lib/pacman/db.lck").exists(),
        simulated: false,
    };
    let os = fs::read_to_string("/etc/os-release").unwrap_or_default();
    if std::env::consts::OS != "linux"
        || host.architecture != "x86_64"
        || !os.lines().any(|s| s == "ID=arch" || s == "ID=\"arch\"")
    {
        return host;
    }
    host.state = "unknown".into();
    let observe = || -> Result<Host> {
        let mut h = host.clone();
        h.installed = installed(&read("/usr/bin/pacman", &["-Q"])?)?;
        h.omarchy_version = h
            .installed
            .get("omarchy")
            .or_else(|| h.installed.get("omarchy-dev"))
            .cloned();
        if h.omarchy_version.is_none() {
            return Err("omarchy_package_missing");
        }
        let config = read("/usr/bin/pacman-conf", &[])?;
        h.configuration = digest(&config);
        // Alternative roots/dbs cannot accidentally target another installation.
        if read("/usr/bin/pacman-conf", &["RootDir"])?.trim() != "/"
            || read("/usr/bin/pacman-conf", &["DBPath"])?
                .trim()
                .trim_end_matches('/')
                != "/var/lib/pacman"
        {
            return Err("custom_package_root_unsupported");
        }
        for name in read("/usr/bin/pacman-conf", &["--repo-list"])?.lines() {
            if !package_token(name) || h.repositories.len() >= 32 {
                return Err("invalid_repository_configuration");
            }
            let policy = read("/usr/bin/pacman-conf", &["-r", name, "SigLevel"])?;
            let fields: Vec<_> = policy.split_whitespace().collect();
            let required = fields.contains(&"PackageRequired") || fields.contains(&"Required");
            let trusted = !fields.iter().any(|s| {
                matches!(
                    *s,
                    "Never"
                        | "Optional"
                        | "PackageNever"
                        | "PackageOptional"
                        | "TrustAll"
                        | "PackageTrustAll"
                )
            });
            h.repositories.insert(name.into(), required && trusted);
        }
        let (sync, stale) = sync_digest(&h.repositories)?;
        h.sync_databases = sync;
        let updates = read_command("/usr/bin/pacman", &["-Qu".into()])?;
        if updates.code != 0
            && !(updates.code == 1 && updates.text.is_empty() && !updates.error_output)
        {
            return Err("update_status_unavailable");
        }
        h.updates = parse_updates(&updates.text)?;
        if h.updates
            .iter()
            .any(|(name, update)| h.installed.get(name) != Some(&update.installed_version))
        {
            return Err("package_state_changed");
        }
        h.databases_stale = stale;
        h.update_required = stale || !h.updates.is_empty();
        h.state = "supported".into();
        h.reason = if h.update_required {
            "Run Omarchy's updater before planning an install"
        } else {
            "Read-only package queries available; live execution still needs adapter evidence"
        }
        .into();
        Ok(h)
    };
    match observe() {
        Ok(h) => h,
        Err(code) => {
            host.reason = code.into();
            host
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    pub repository: String,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub architecture: String,
    pub download_bytes: u64,
    pub conflicts: String,
    pub replaces: String,
}
pub fn parse_transaction(text: &str) -> Result<Vec<Package>> {
    let mut packages = BTreeMap::new();
    for line in text.lines() {
        let f: Vec<_> = line.split('\t').collect();
        if f.len() != 8
            || !package_token(f[0])
            || !package_token(f[1])
            || !version(f[2])
            || f[3].len() != 64
            || !f[3].bytes().all(|c| c.is_ascii_hexdigit())
            || !matches!(f[4], "any" | "x86_64")
            || f[6].len() > 2000
            || f[7].len() > 2000
        {
            return Err("unsupported_package_transaction");
        }
        let p = Package {
            repository: f[0].into(),
            name: f[1].into(),
            version: f[2].into(),
            sha256: f[3].into(),
            architecture: f[4].into(),
            download_bytes: f[5].parse().map_err(|_| "invalid_package_size")?,
            conflicts: f[6].into(),
            replaces: f[7].into(),
        };
        if packages.insert(p.name.clone(), p).is_some() || packages.len() > 256 {
            return Err("package_transaction_too_large");
        }
    }
    Ok(packages.into_values().collect())
}
pub fn resolve(targets: &[String]) -> Result<Vec<Package>> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }
    if targets.len() > 100
        || targets.iter().any(|t| {
            let f: Vec<_> = t.split('/').collect();
            f.len() != 2 || !package_token(f[0]) || !package_token(f[1])
        })
    {
        return Err("invalid_package_target");
    }
    let mut args = vec![
        "-S".into(),
        "--needed".into(),
        "--print".into(),
        "--print-format".into(),
        "%r\t%n\t%v\t%h\t%a\t%s\t%H\t%R".into(),
        "--".into(),
    ];
    args.extend_from_slice(targets);
    let output = read_command("/usr/bin/pacman", &args)?;
    if output.code != 0 {
        return Err("package_resolution_unavailable");
    }
    parse_transaction(&output.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_rows_keep_versions_and_reject_ambiguous_output() {
        let updates =
            parse_updates("alpha 1:2.0-1 -> 1:2.1-1\nbeta 3-1 -> 4-1 [ignored]\n").unwrap();
        assert_eq!(updates["alpha"].available_version, "1:2.1-1");
        assert!(updates["beta"].ignored);
        assert!(parse_updates("").unwrap().is_empty());
        for bad in [
            "alpha 1-1",
            "alpha 1-1 => 2-1",
            "alpha 1 -> 2 junk",
            "--root 1 -> 2",
            "alpha 1 -> 2\nalpha 1 -> 3",
            "alpha 1 -> 2 [ignored] extra",
        ] {
            assert!(parse_updates(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn package_queries_reject_options_and_ambiguous_solver_output() {
        assert!(!package_token("--root"));
        assert!(!package_token("x;touch"));
        assert!(!package_token("../root"));
        assert!(package_token("lib32-gcc-libs"));
        assert!(version("2:1.2-3"));
        assert!(installed("x 1\nx 2\n").is_err());
        let line = format!("extra\tx\t1.0-1\t{}\tx86_64\t123\t\t\n", "a".repeat(64));
        assert_eq!(parse_transaction(&line).unwrap()[0].download_bytes, 123);
        assert!(parse_transaction(&line.replace("extra", "--root")).is_err());
        assert!(resolve(&["extra/--root".into()]).is_err());
    }
}
