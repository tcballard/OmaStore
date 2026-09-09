use omastore_service::{router, AppState};
use omastore_workflow::{auth::GithubOAuth, net, Store};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("Usage: omastore-service CATALOGUE.json 127.0.0.1:PORT [--workspace PRIVATE.db] [--sandbox]");
        std::process::exit(2);
    }
    let addr: SocketAddr = args[1].parse()?;
    if !addr.ip().is_loopback() {
        return Err("bind must be loopback; terminate HTTPS at the operator proxy".into());
    }
    let mut state = AppState::public(PathBuf::from(&args[0]));
    let mut database = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--workspace" if index + 1 < args.len() => {
                database = Some(PathBuf::from(&args[index + 1]));
                index += 2;
            }
            #[cfg(feature = "development-workflow")]
            "--sandbox" => {
                state.sandbox = true;
                index += 1;
            }
            #[cfg(feature = "development-workflow")]
            "--commerce-test" => {
                state.commerce_test = true;
                index += 1;
            }
            _ => return Err("unsupported service argument".into()),
        }
    }
    if let Some(path) = database {
        #[cfg(feature = "development-workflow")]
        let store = if state.sandbox || state.commerce_test {
            Store::development(&path)?
        } else {
            Store::open(&path)?
        };
        #[cfg(not(feature = "development-workflow"))]
        let store = Store::open(&path)?;
        state.store = Some(store);
        state.objects = Some(omastore_workflow::media::LocalObjects::new(
            &path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("objects"),
        )?);
    }
    state.origin = std::env::var("OMASTORE_SERVICE_ORIGIN")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    if state.sandbox {
        state.origin = "https://sandbox.omastore.invalid".into();
        if state.store.is_none() {
            return Err("sandbox requires its own --workspace database".into());
        }
    } else if !state.origin.is_empty() {
        let origin = net::public_url(&state.origin)?;
        if origin.path() != "/" || origin.query().is_some() {
            return Err("service origin must be an HTTPS origin without a path".into());
        }
        if let (Ok(client_id), Ok(client_secret)) = (
            std::env::var("OMASTORE_GITHUB_CLIENT_ID"),
            std::env::var("OMASTORE_GITHUB_CLIENT_SECRET"),
        ) {
            if !client_id.is_empty() && !client_secret.is_empty() {
                state.oauth = Some(Arc::new(GithubOAuth {
                    client_id,
                    client_secret,
                    callback: format!("{}/api/v1/auth/callback", state.origin),
                }));
            }
        }
    }
    if !state.sandbox && !state.commerce_test && state.store.is_some() && !state.origin.is_empty() {
        match omastore_workflow::github::Config::from_env() {
            Ok(Some(config)) => {
                state.github = Some(omastore_workflow::github::Github::new(config));
                state.publication_state = "configured".into();
            }
            Ok(None) => {}
            Err(error) => state.publication_state = error.code.into(),
        }
    }
    if std::env::var("OMASTORE_PUBLICATION_PAUSED").ok().as_deref() == Some("1") {
        state.publication_state = "paused".into();
    }
    let publication_worker = if state.github.is_some() && state.publication_state == "configured" {
        Some(omastore_service::publication::start_worker(state.clone()))
    } else {
        None
    };
    state.read_catalogue()?;
    let monitoring_worker = if state.store.is_some()
        && !state.sandbox
        && !state.commerce_test
        && std::env::var("OMASTORE_MONITORING_PAUSED").ok().as_deref() != Some("1")
    {
        Some(omastore_service::monitoring::start_worker(state.clone()))
    } else {
        None
    };
    let worker = if state.store.is_some()
        && !state.sandbox
        && !state.commerce_test
        && std::env::var("OMASTORE_CHECKS_PAUSED").ok().as_deref() != Some("1")
    {
        Some(omastore_service::start_checks_worker(state.clone()))
    } else {
        None
    };
    let maintenance_worker = if state.store.is_some()
        && !state.sandbox
        && !state.commerce_test
        && std::env::var("OMASTORE_MAINTENANCE_PAUSED").ok().as_deref() != Some("1")
    {
        Some(omastore_service::start_maintenance_worker(state.clone()))
    } else {
        None
    };
    omastore_service::commerce::configure(&mut state)?;
    let commerce_worker = if state.commerce.mode() != "disabled" {
        Some(omastore_service::commerce::start_worker(state.clone()))
    } else {
        None
    };
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("LISTENING {}", listener.local_addr()?);
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    if let Some(worker) = commerce_worker {
        worker.abort();
    }
    if let Some(worker) = worker {
        worker.abort();
    }
    if let Some(worker) = publication_worker {
        worker.abort();
    }
    if let Some(worker) = maintenance_worker {
        worker.abort();
    }
    if let Some(worker) = monitoring_worker {
        worker.abort();
    }
    Ok(())
}
