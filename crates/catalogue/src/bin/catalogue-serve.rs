use omastore_catalogue::{api, Snapshot, MAX_BYTES};
use std::{io::Read, net::SocketAddr};
use tiny_http::{Header, Response, Server, StatusCode};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("Usage: catalogue-serve FILE 127.0.0.1:PORT");
        std::process::exit(2);
    }
    let address: SocketAddr = args[1].parse().expect("valid socket address");
    if !address.ip().is_loopback() {
        eprintln!("Bind to loopback behind an HTTPS reverse proxy.");
        std::process::exit(2);
    }
    let server = Server::http(address).expect("bind read service");
    println!("{}", server.server_addr());
    for request in server.incoming_requests() {
        let read = (|| {
            let mut bytes = Vec::new();
            std::fs::File::open(&args[0])
                .ok()?
                .take((MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .ok()?;
            Snapshot::parse(&bytes, cfg!(feature = "dev-fixtures")).ok()
        })();
        let Some(snapshot) = read else {
            let _ = request.respond(
                Response::from_string(api::failure("catalogue_unavailable").to_string())
                    .with_status_code(503),
            );
            continue;
        };
        let etag = snapshot.etag();
        let (path, qs) = request.url().split_once('?').unwrap_or((request.url(), ""));
        let result = if request.method().as_str() != "GET" {
            Err("method_not_allowed")
        } else {
            api::read(&snapshot, path, qs)
        };
        let (status, body) = match result {
            Ok(value) => {
                if request
                    .headers()
                    .iter()
                    .any(|h| h.field.equiv("If-None-Match") && h.value.as_str() == etag)
                {
                    (304, String::new())
                } else {
                    (200, value.to_string())
                }
            }
            Err(code) => (
                match code {
                    "not_found" => 404,
                    "method_not_allowed" => 405,
                    "snapshot_changed" => 409,
                    _ => 400,
                },
                api::failure(code).to_string(),
            ),
        };
        let response = Response::from_string(body)
            .with_status_code(StatusCode(status))
            .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
            .with_header(Header::from_bytes("ETag", etag).unwrap())
            .with_header(
                Header::from_bytes("X-Catalogue-Revision", snapshot.revision.as_str()).unwrap(),
            )
            .with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap());
        let _ = request.respond(response);
    }
}
