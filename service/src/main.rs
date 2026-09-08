use omastore_catalogue::{http, Catalogue, MAX_CATALOGUE_BYTES};
use std::{fs::File, io::Read, net::SocketAddr};
use tiny_http::{Header, Response, Server, StatusCode};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("Usage: omastore-service CATALOGUE.json 127.0.0.1:PORT\nServe public read routes behind an operator-managed HTTPS proxy.");
        std::process::exit(2);
    }
    let addr: SocketAddr = args[1].parse()?;
    if !addr.ip().is_loopback() {
        return Err("bind must be loopback; terminate HTTPS at the proxy".into());
    }
    let read = || -> Result<Catalogue, Box<dyn std::error::Error + Send + Sync>> {
        let mut bytes = Vec::new();
        File::open(&args[0])?
            .take((MAX_CATALOGUE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Catalogue::parse(&bytes, cfg!(feature = "development-catalogue"))
            .map_err(|_| "catalogue validation failed".into())
    };
    // Refuse to start with invalid data. Each request gets one complete snapshot.
    read()?;
    let server = Server::http(addr)?;
    println!("LISTENING {}", server.server_addr());
    for request in server.incoming_requests() {
        let response = match read() {
            Ok(catalogue) => {
                let etag = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("If-None-Match"))
                    .map(|h| h.value.as_str());
                http::handle(
                    &catalogue,
                    request.method().as_str(),
                    request.url(),
                    etag,
                    chrono::Utc::now(),
                )
            }
            Err(_) => http::Response {
                status: 503,
                etag: String::new(),
                body: br#"{"error":{"code":"catalogue_unavailable"}}"#.to_vec(),
            },
        };
        let mut out =
            Response::from_data(response.body).with_status_code(StatusCode(response.status));
        for (name, value) in [
            ("Content-Type", "application/json; charset=utf-8"),
            ("X-Content-Type-Options", "nosniff"),
            ("Cache-Control", "public, max-age=0, must-revalidate"),
        ] {
            out.add_header(Header::from_bytes(name, value).unwrap());
        }
        if !response.etag.is_empty() {
            out.add_header(Header::from_bytes("ETag", response.etag).unwrap());
        }
        if response.status == 405 {
            out.add_header(Header::from_bytes("Allow", "GET").unwrap());
        }
        let _ = request.respond(out);
    }
    Ok(())
}
