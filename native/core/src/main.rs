mod catalogue;
mod setups;
mod workspace;
struct Runtime {
    catalogue: catalogue::Client,
    workspace: Option<workspace::Client>,
    demo: bool,
}
impl Runtime {
    fn new(demo: bool) -> Self {
        Self {
            catalogue: catalogue::Client::new(demo),
            workspace: None,
            demo,
        }
    }
}
use omastore_catalogue::query;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

const PROTOCOL_VERSION: u32 = 1;
const MAX_LINE_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol_version: u32,
    id: String,
    method: String,
    #[serde(default = "empty_params")]
    params: Value,
}

fn empty_params() -> Value {
    json!({})
}

fn error(id: Option<&str>, code: &str) -> Value {
    json!({"protocol_version": PROTOCOL_VERSION, "id": id,
        "ok": false, "error": {"code": code}})
}

fn respond(line: &[u8], runtime: &mut Runtime) -> Value {
    let request: Request = match serde_json::from_slice(line) {
        Ok(request) => request,
        Err(_) => return error(None, "invalid_request"),
    };
    if request.id.is_empty() || request.id.len() > 128 {
        return error(None, "invalid_id");
    }
    if request.protocol_version != PROTOCOL_VERSION {
        return error(Some(&request.id), "unsupported_protocol");
    }
    if let Some(method) = request.method.strip_prefix("workspace.") {
        let client = runtime
            .workspace
            .get_or_insert_with(|| workspace::Client::new(runtime.demo));
        return match client.dispatch(method, request.params) {
            Ok(result) => {
                json!({"protocol_version":PROTOCOL_VERSION,"id":request.id,"ok":true,"result":result})
            }
            Err(e) => error(Some(&request.id), e.code),
        };
    }
    let client = &mut runtime.catalogue;
    let now = chrono::Utc::now();
    let result: Result<Value, &str> = match request.method.as_str() {
        "candidate.prepare" => {
            serde_json::from_value::<omastore_catalogue::preparation::Fields>(request.params)
                .map(omastore_catalogue::preparation::prepare)
                .map_err(|_| "invalid_request")
        }
        "core.info" if request.params == json!({}) => Ok(
            json!({"service": "omastore-core", "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
            "capabilities": ["core.info", "catalogue.info", "catalogue.refresh", "apps.list", "apps.get", "makers.list", "makers.get", "editorial.list", "editorial.get", "setups.list", "setups.select", "setups.export", "setups.import", "apps.pick", "candidate.prepare", "workspace.state", "workspace.command", "workspace.drafts.get", "workspace.drafts.cache", "workspace.drafts.new", "workspace.drafts.preview", "workspace.revisions.get", "workspace.media.upload"]}),
        ),
        "catalogue.info" if request.params == json!({}) => Ok(client.info()),
        "catalogue.refresh" if request.params == json!({}) => Ok(client.refresh()),
        "apps.list" => serde_json::from_value::<query::Query>(request.params)
            .map_err(|_| "invalid_filter")
            .and_then(|q| query::list(&client.catalogue, &q, now)),
        "setups.list" => {
            serde_json::from_value::<omastore_catalogue::editorial::Browse>(request.params)
                .map_err(|_| "invalid_filter")
                .and_then(|q| omastore_catalogue::setups::list(&client.catalogue, &q))
        }
        "setups.select" => {
            serde_json::from_value::<omastore_catalogue::setups::Selection>(request.params)
                .map_err(|_| "invalid_request")
                .and_then(|q| omastore_catalogue::setups::select(&client.catalogue, &q, now))
        }
        "setups.export" => setups::export(&client.catalogue, request.params),
        "setups.import" => setups::import(&client.catalogue, request.params),
        "apps.pick" => serde_json::from_value::<query::Query>(request.params)
            .map_err(|_| "invalid_filter")
            .and_then(|q| query::list(&client.catalogue, &q, now)),
        "makers.list" => {
            serde_json::from_value::<omastore_catalogue::editorial::Browse>(request.params)
                .map_err(|_| "invalid_filter")
                .and_then(|q| omastore_catalogue::editorial::makers(&client.catalogue, &q, now))
        }
        "editorial.list" if request.params == json!({}) => Ok(
            omastore_catalogue::editorial::stories(&client.catalogue, now),
        ),
        "makers.get" | "editorial.get" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Lookup {
                id: String,
            }
            serde_json::from_value::<Lookup>(request.params)
                .map_err(|_| "invalid_request")
                .and_then(|p| {
                    if !omastore_catalogue::token(&p.id) {
                        return Err("invalid_id");
                    }
                    if request.method == "makers.get" {
                        omastore_catalogue::editorial::maker_detail(&client.catalogue, &p.id, now)
                    } else {
                        omastore_catalogue::editorial::story(&client.catalogue, &p.id, now)
                    }
                })
        }
        "apps.get" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Lookup {
                id: String,
            }
            serde_json::from_value::<Lookup>(request.params)
                .map_err(|_| "invalid_request")
                .and_then(|p| {
                    if !omastore_catalogue::token(&p.id) {
                        return Err("invalid_id");
                    }
                    query::app_detail(&client.catalogue, &p.id, now)
                })
        }
        "core.info" | "catalogue.info" | "catalogue.refresh" => Err("invalid_request"),
        _ => Err("unknown_method"),
    };
    match result {
        Ok(value) => {
            json!({"protocol_version": PROTOCOL_VERSION, "id": request.id, "ok": true, "result": value})
        }
        Err(code) => error(Some(&request.id), code),
    }
}

fn serve(mut input: impl BufRead, mut output: impl Write, runtime: &mut Runtime) -> io::Result<()> {
    loop {
        let mut line = Vec::new();
        let bytes = input
            .by_ref()
            .take((MAX_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if bytes == 0 {
            return Ok(());
        }
        let oversized = line.len() > MAX_LINE_BYTES;
        let response = if oversized {
            error(None, "message_too_large")
        } else {
            respond(&line, runtime)
        };
        let bytes = serde_json::to_vec(&response)?;
        if bytes.len() + 1 > MAX_LINE_BYTES {
            serde_json::to_writer(
                &mut output,
                &error(response["id"].as_str(), "reply_too_large"),
            )?;
        } else {
            output.write_all(&bytes)?;
        }
        output.write_all(b"\n")?;
        output.flush()?;
        if oversized {
            // End this stream without allocating the rest of an unbounded request.
            return Ok(());
        }
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut client = Runtime::new(args.iter().any(|s| s == "--demo"));
    match args.as_slice() {
        [] => serve(io::stdin().lock(), io::stdout().lock(), &mut client),
        [arg] if arg == "--stdio" => serve(io::stdin().lock(), io::stdout().lock(), &mut client),
        #[cfg(feature = "development-catalogue")]
        [mode, demo] if mode == "--stdio" && demo == "--demo" => {
            serve(io::stdin().lock(), io::stdout().lock(), &mut client)
        }
        [arg] if arg == "--version" => {
            println!("omastore-core {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => {
            eprintln!("Usage: omastore-core [--stdio | --version]");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn bad_request_does_not_desynchronise_the_next_reply() {
        let input =
            b"not json\n{\"protocol_version\":1,\"id\":\"next\",\"method\":\"core.info\"}\n";
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output, &mut Runtime::new(false)).unwrap();
        let values: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["error"]["code"], "invalid_request");
        assert_eq!(values[1]["id"], "next");
        assert_eq!(values[1]["ok"], true);
    }

    #[test]
    fn oversized_input_ends_the_stream_with_a_bounded_error() {
        let input = vec![b'x'; MAX_LINE_BYTES + 100];
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output, &mut Runtime::new(false)).unwrap();
        assert!(output.len() < 200);
        let reply: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(reply["error"]["code"], "message_too_large");
    }

    #[test]
    fn unsupported_protocol_and_write_methods_are_rejected() {
        let reply = respond(
            br#"{"protocol_version":2,"id":"a","method":"core.info"}"#,
            &mut Runtime::new(false),
        );
        assert_eq!(reply["error"]["code"], "unsupported_protocol");
        let reply = respond(
            br#"{"protocol_version":1,"id":"b","method":"install"}"#,
            &mut Runtime::new(false),
        );
        assert_eq!(reply["error"]["code"], "unknown_method");
    }
}
