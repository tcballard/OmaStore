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
}

fn error(id: Option<&str>, code: &str) -> Value {
    json!({"protocol_version": PROTOCOL_VERSION, "id": id,
        "ok": false, "error": {"code": code}})
}

fn respond(line: &[u8]) -> Value {
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
    if request.method != "core.info" {
        return error(Some(&request.id), "unknown_method");
    }
    json!({"protocol_version": PROTOCOL_VERSION, "id": request.id, "ok": true,
        "result": {"service": "omastore-core", "version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
            "capabilities": ["core.info"]}})
}

fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
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
            respond(&line)
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => serve(io::stdin().lock(), io::stdout().lock()),
        [arg] if arg == "--stdio" => serve(io::stdin().lock(), io::stdout().lock()),
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
        serve(Cursor::new(input), &mut output).unwrap();
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
        serve(Cursor::new(input), &mut output).unwrap();
        assert!(output.len() < 200);
        let reply: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(reply["error"]["code"], "message_too_large");
    }

    #[test]
    fn unsupported_protocol_and_write_methods_are_rejected() {
        let reply = respond(br#"{"protocol_version":2,"id":"a","method":"core.info"}"#);
        assert_eq!(reply["error"]["code"], "unsupported_protocol");
        let reply = respond(br#"{"protocol_version":1,"id":"b","method":"install"}"#);
        assert_eq!(reply["error"]["code"], "unknown_method");
    }
}
