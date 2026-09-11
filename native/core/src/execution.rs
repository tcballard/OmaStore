use crate::{
    library::{self, Store},
    lifecycle,
    planner::Plan,
    platform::{self, Result},
    Runtime,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    io::{IsTerminal, Read},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn environment(command: &mut Command) {
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C");
    for name in [
        "HOME",
        "XDG_RUNTIME_DIR",
        "XDG_STATE_HOME",
        "XDG_DATA_HOME",
        "XDG_CONFIG_HOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_CURRENT_DESKTOP",
        "OMASTORE_SERVICE_ORIGIN",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}
fn helper(name: &str) -> Result<String> {
    // Callers provide one of these fixed internal helper names, never a catalogue value.
    if !matches!(
        name,
        "omarchy-launch-terminal" | "omarchy" | "omarchy-pkg-install"
    ) {
        return Err("unsupported_system_helper");
    }
    for directory in ["/usr/bin", "/usr/share/omarchy/bin"] {
        let path = format!("{directory}/{name}");
        if platform::trusted_program(&path).is_ok() {
            return Ok(path);
        }
    }
    Err("omarchy_helper_unavailable")
}
pub fn launch(store: &Store, plan: &Plan) -> Result<()> {
    let state = lifecycle::status(store, &plan.digest)?;
    if state["state"] != "awaiting_user" || state["claimed"] == true {
        return Ok(());
    }
    if !plan.simulated && !lifecycle::live_enabled(plan) {
        return Err("omarchy_adapter_evidence_required");
    }
    let executable = std::env::current_exe().map_err(|_| "worker_unavailable")?;
    let mut command = if plan.simulated {
        let mut c = Command::new(&executable);
        c.args(["--operation-worker", &plan.digest, "--demo"]);
        c.process_group(0);
        c
    } else {
        platform::trusted_program(executable.to_str().ok_or("worker_unavailable")?)?;
        let mut c = Command::new(helper("omarchy-launch-terminal")?);
        c.arg(&executable)
            .args(["--operation-worker", &plan.digest]);
        c
    };
    environment(&mut command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| "worker_unavailable")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
pub fn arguments(plan: &Plan) -> Result<Vec<String>> {
    if matches!(plan.selection, crate::planner::Selection::Remove { .. }) {
        if plan.operations.len() != 1 || plan.operations[0].action != "remove" {
            return Err("invalid_removal_plan");
        }
        let package = plan.operations[0]
            .package
            .as_deref()
            .ok_or("invalid_package_target")?;
        if !platform::package_token(package) {
            return Err("invalid_package_target");
        }
        return Ok(vec![
            "-n".into(),
            "--".into(),
            "/usr/bin/pacman".into(),
            "-R".into(),
            "--noconfirm".into(),
            "--".into(),
            package.into(),
        ]);
    }
    let mut targets = BTreeSet::new();
    for op in &plan.operations {
        if op.action != "install" {
            continue;
        }
        let package = op.package.as_deref().ok_or("invalid_package_target")?;
        let repository = op.repository.as_deref().ok_or("invalid_package_target")?;
        if !platform::package_token(package)
            || !platform::package_token(repository)
            || !platform::version(&op.version)
        {
            return Err("invalid_package_target");
        }
        if !plan
            .packages
            .iter()
            .any(|p| p.name == package && p.repository == repository && p.version == op.version)
        {
            return Err("package_plan_mismatch");
        }
        targets.insert(format!("{repository}/{package}={}", op.version));
    }
    if targets.is_empty() || targets.len() > 100 {
        return Err("invalid_package_target");
    }
    let mut args = vec![
        "-n".into(),
        "--".into(),
        "/usr/bin/pacman".into(),
        "-S".into(),
        "--needed".into(),
        "--noconfirm".into(),
        "--".into(),
    ];
    args.extend(targets);
    Ok(args)
}
fn authenticate(store: &Store, id: &str, expires: i64) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err("system_terminal_required");
    }
    platform::trusted_program("/usr/bin/sudo")?;
    let mut command = Command::new("/usr/bin/sudo");
    command.arg("-v");
    environment(&mut command);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = command.spawn().map_err(|_| "privilege_unavailable")?;
    loop {
        let now = chrono::Utc::now().timestamp();
        let state = lifecycle::status(store, id)?;
        if now >= expires || state["state"] == "failed" || state["cancelRequested"] == true {
            // This is only the authentication prompt. A package process is never killed here.
            let _ = child.kill();
            let _ = child.wait();
            return Err("cancelled_before_execution");
        }
        let _ = lifecycle::heartbeat(store, id, now);
        match child.try_wait().map_err(|_| "privilege_unavailable")? {
            Some(exit) => {
                return if exit.success() {
                    Ok(())
                } else {
                    Err("privilege_denied")
                }
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}
fn drain(mut pipe: impl Read + Send + 'static) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let retain = (16384 - kept.len()).min(n);
                    kept.extend_from_slice(&buf[..retain]);
                }
            }
        }
        let _ = sender.send(kept);
    });
    receiver
}
fn run_packages(store: &Store, plan: &Plan) -> Result<(bool, &'static str)> {
    platform::trusted_program("/usr/bin/sudo")?;
    platform::trusted_program("/usr/bin/pacman")?;
    let mut command = Command::new("/usr/bin/sudo");
    command.args(arguments(plan)?).current_dir("/");
    environment(&mut command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| "package_process_unavailable")?;
    let stdout = drain(child.stdout.take().ok_or("package_output_unavailable")?);
    let stderr = drain(child.stderr.take().ok_or("package_output_unavailable")?);
    let mut heartbeat = Instant::now();
    let status = loop {
        // No timeout, cancellation kill, UI pipe or parent-owned QProcess controls this mutator.
        match child.try_wait() {
            Ok(Some(exit)) => break exit,
            Ok(None) => {}
            Err(_) => return Err("package_outcome_unknown"),
        }
        if heartbeat.elapsed() >= Duration::from_secs(1) {
            let _ = lifecycle::heartbeat(store, &plan.digest, chrono::Utc::now().timestamp());
            heartbeat = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let _ = stdout.recv_timeout(Duration::from_millis(500));
    let errors = stderr
        .recv_timeout(Duration::from_millis(500))
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&errors).to_lowercase();
    let code = if status.success() {
        "package_state_verified"
    } else if text.contains("unable to lock database") || text.contains("could not lock database") {
        "package_manager_locked"
    } else if text.contains("password is required") || text.contains("not in the sudoers") {
        "privilege_denied"
    } else if text.contains("failed retrieving file")
        || text.contains("could not resolve")
        || text.contains("failed to retrieve")
    {
        "repository_unavailable"
    } else {
        "package_command_failed"
    };
    Ok((status.success(), code))
}
pub fn worker(id: &str, demo: bool) -> Result<()> {
    let mut store = Store::open(demo)?;
    let plan = store.plan(id)?;
    if plan.simulated != demo || (!demo && !lifecycle::live_enabled(&plan)) {
        return Err("omarchy_adapter_evidence_required");
    }
    let directory = library::directory(demo)?;
    let _global = lifecycle::lock(&directory.join("package-worker.lock"))?;
    let _operation = lifecycle::lock(&directory.join(format!("{id}.lock")))?;
    if !lifecycle::claim(&mut store, id, chrono::Utc::now().timestamp())? {
        return Ok(());
    }
    // The standalone terminal worker survives window/core loss and ignores a terminal hangup.
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
    let result = (|| -> Result<(Runtime, crate::platform::Host, bool, &'static str)> {
        arguments(&plan)?;
        if !demo {
            println!("OmaStore package plan {}", plan.digest);
            println!("Administrator privileges are handled by sudo in this terminal.");
            for op in &plan.operations {
                if op.action == "install" {
                    println!(
                        "{} / {} = {}",
                        op.repository.as_deref().unwrap_or(""),
                        op.package.as_deref().unwrap_or(""),
                        op.version
                    );
                }
            }
            authenticate(&store, id, plan.expires_at)?;
        }
        let mut runtime = Runtime::new(demo);
        // Re-evaluate after the privilege prompt; consent cannot outlive an authentication pause.
        let fresh = crate::compute_plan(
            &mut runtime,
            plan.selection.clone(),
            chrono::Utc::now().timestamp(),
        )?;
        lifecycle::begin(&mut store, id, &fresh, chrono::Utc::now().timestamp())?;
        #[cfg(feature = "development-catalogue")]
        if demo {
            std::thread::sleep(Duration::from_millis(300));
            let tx = store
                .connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| "local_database_unavailable")?;
            for op in &plan.operations {
                if op.action == "remove" {
                    tx.execute(
                        "DELETE FROM sample_packages WHERE name=?1",
                        [op.package.as_deref().ok_or("invalid_package_target")?],
                    )
                    .map_err(|_| "local_database_unavailable")?;
                }
            }
            for package in &plan.packages {
                tx.execute("INSERT INTO sample_packages(name,version) VALUES(?1,?2) ON CONFLICT(name) DO UPDATE SET version=excluded.version",rusqlite::params![package.name,package.version]).map_err(|_|"local_database_unavailable")?;
            }
            tx.commit().map_err(|_| "local_database_unavailable")?;
            let host = crate::local_host(&runtime, &store)?;
            return Ok((runtime, host, true, "sample_package_state_verified"));
        }
        let (okay, code) = run_packages(&store, &plan)?;
        let host = crate::local_host(&runtime, &store)?;
        Ok((runtime, host, okay, code))
    })();
    match result {
        Ok((_runtime, host, okay, code)) => {
            lifecycle::finish(
                &mut store,
                &plan,
                &host,
                okay,
                code,
                chrono::Utc::now().timestamp(),
            )?;
            Ok(())
        }
        Err(code) => {
            let state = lifecycle::status(&store, id)?;
            if state["state"] == "running" {
                // A process/query failure after mutation began is never reported as a clean failure.
                let mut unknown = platform::probe();
                unknown.state = "unknown".into();
                lifecycle::finish(
                    &mut store,
                    &plan,
                    &unknown,
                    false,
                    "package_outcome_unknown",
                    chrono::Utc::now().timestamp(),
                )?;
            } else {
                lifecycle::fail(&mut store, id, code, chrono::Utc::now().timestamp())?;
            }
            Err(code)
        }
    }
}
pub fn system_handoff(kind: &str, demo: bool) -> Result<Value> {
    if !matches!(kind, "update" | "install") {
        return Err("unsupported_system_handoff");
    }
    if demo {
        return Ok(json!({"requested":true,"simulated":true,"kind":kind}));
    }
    let mut command = Command::new(helper("omarchy-launch-terminal")?);
    if kind == "update" {
        command.arg(helper("omarchy")?).arg("update");
    } else {
        command.arg(helper("omarchy-pkg-install")?);
    }
    environment(&mut command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| "system_handoff_unavailable")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(
        json!({"requested":true,"simulated":false,"kind":kind,"notice":"The system tool owns this operation. Refresh your local library after it finishes."}),
    )
}

#[cfg(all(test, feature = "development-catalogue"))]
mod tests {
    use super::*;
    #[test]
    fn execution_arguments_have_no_shell_or_option_injection() {
        let c: omastore_catalogue::Catalogue =
            serde_json::from_str(include_str!("../../../tests/fixtures/catalogue.json")).unwrap();
        let s = crate::planner::Selection::App {
            id: "demo-fieldnotes".into(),
        };
        let (h, status, p) =
            crate::planner::sample(&c, &s, Default::default(), 1_800_000_000).unwrap();
        let mut plan = crate::planner::build(&c, s, &h, &status, Ok(p), 1_800_000_000).unwrap();
        let args = arguments(&plan).unwrap();
        assert_eq!(args[2], "/usr/bin/pacman");
        assert!(args.last().unwrap().contains("fieldnotes="));
        assert!(!args
            .iter()
            .any(|s| s == "-Sy" || s == "-Syu" || s == "--nodeps"));
        plan.operations[0].package = Some("--root".into());
        assert!(arguments(&plan).is_err());
    }
}
