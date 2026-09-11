use omastore_catalogue::{
    client::{CatalogueClient, DEFAULT_ORIGIN},
    query::{self, Query},
};
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
    #[serde(default)]
    params: Option<Value>,
}

fn error(id: Option<&str>, code: &str) -> Value {
    json!({"protocol_version": PROTOCOL_VERSION, "id": id,
        "ok": false, "error": {"code": code}})
}

fn respond(line: &[u8], client: &mut CatalogueClient) -> Value {
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
    let result = match request.method.as_str() {
        "core.info" => json!({"service":"omastore-core","version":env!("CARGO_PKG_VERSION"),
            "platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,
            "capabilities":["core.info","catalogue.info","catalogue.query","catalogue.detail","catalogue.refresh"]}),
        "catalogue.info" => {
            json!({"state":client.state,"count":client.snapshot.apps.len(),"channel":client.snapshot.channel})
        }
        "catalogue.refresh" => {
            client.refresh();
            json!({"state":client.state,"count":client.snapshot.apps.len()})
        }
        "catalogue.query" => {
            let mut params = serde_json::to_value(Query::default()).unwrap();
            if let Some(extra) = request.params {
                let Some(fields) = extra.as_object() else {
                    return error(Some(&request.id), "invalid_filter");
                };
                for (k, v) in fields {
                    params[k] = v.clone();
                }
            }
            let Ok(q) = serde_json::from_value::<Query>(params) else {
                return error(Some(&request.id), "invalid_filter");
            };
            match query::query(&client.snapshot, &q) {
                Ok(page) => serde_json::to_value(page).unwrap(),
                Err(code) => return error(Some(&request.id), code),
            }
        }
        "catalogue.detail" => {
            let id = request
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("");
            match client
                .snapshot
                .apps
                .iter()
                .find(|a| a.id == id || a.slug == id)
            {
                Some(app) => {
                    let mut value = query::detail(app);
                    value["makers"] = json!(client
                        .snapshot
                        .makers
                        .iter()
                        .filter(|m| app.maker_ids.contains(&m.id))
                        .collect::<Vec<_>>());
                    value
                }
                None => return error(Some(&request.id), "not_found"),
            }
        }
        _ => return error(Some(&request.id), "unknown_method"),
    };
    let reply =
        json!({"protocol_version":PROTOCOL_VERSION,"id":request.id,"ok":true,"result":result});
    if serde_json::to_vec(&reply).unwrap().len() + 1 > MAX_LINE_BYTES {
        return error(reply["id"].as_str(), "response_too_large");
    }
    reply
}

fn serve(
    mut input: impl BufRead,
    mut output: impl Write,
    client: &mut CatalogueClient,
) -> io::Result<()> {
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
            respond(&line, client)
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if oversized {
            // End this stream without allocating the rest of an unbounded request.
            return Ok(());
        }
    }
}

fn main() -> io::Result<()> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))
        .map(|p| p.join("omastore/catalogue.json"));
    let mut client = CatalogueClient::new(
        cache,
        std::env::var("OMASTORE_CATALOGUE_URL").unwrap_or_else(|_| DEFAULT_ORIGIN.into()),
    );
    #[cfg(feature = "dev-fixtures")]
    if std::env::var_os("OMASTORE_DEV_FIXTURES").is_some() {
        client.snapshot = omastore_catalogue::development_fixture();
        client.state.source = "development fixture".into();
        client.state.revision = client.snapshot.revision.clone();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => serve(io::stdin().lock(), io::stdout().lock(), &mut client),
        [arg] if arg == "--stdio" => serve(io::stdin().lock(), io::stdout().lock(), &mut client),
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
    fn client() -> CatalogueClient {
        CatalogueClient::new(None, DEFAULT_ORIGIN.into())
    }

    #[test]
    fn bad_request_does_not_desynchronise_the_next_reply() {
        let input =
            b"not json\n{\"protocol_version\":1,\"id\":\"next\",\"method\":\"core.info\"}\n";
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output, &mut client()).unwrap();
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
        serve(Cursor::new(input), &mut output, &mut client()).unwrap();
        assert!(output.len() < 200);
        let reply: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(reply["error"]["code"], "message_too_large");
    }

    #[test]
    fn unsupported_protocol_and_write_methods_are_rejected() {
        let reply = respond(
            br#"{"protocol_version":2,"id":"a","method":"core.info"}"#,
            &mut client(),
        );
        assert_eq!(reply["error"]["code"], "unsupported_protocol");
        let reply = respond(
            br#"{"protocol_version":1,"id":"b","method":"install"}"#,
            &mut client(),
        );
        assert_eq!(reply["error"]["code"], "unknown_method");
    }
}
