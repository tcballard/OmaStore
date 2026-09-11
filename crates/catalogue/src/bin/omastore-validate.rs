use omastore_catalogue::{Catalogue, MAX_CATALOGUE_BYTES};
use std::{fs::File, io::Read};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (path, development) = match args.as_slice() {
        [path] => (path, false),
        [flag, path] if flag == "--development" => (path, true),
        _ => {
            eprintln!("Usage: omastore-validate [--development] CATALOGUE.json");
            std::process::exit(2);
        }
    };
    let mut bytes = Vec::new();
    if File::open(path)
        .and_then(|f| {
            f.take((MAX_CATALOGUE_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .is_err()
    {
        eprintln!("{{\"error\":\"catalogue_unreadable\"}}");
        std::process::exit(2);
    }
    match Catalogue::parse(&bytes, development) {
        Ok(c) => println!(
            "{}",
            serde_json::json!({"ok": true, "apps": c.apps.len(), "snapshot": c.snapshot_id()})
        ),
        Err(errors) => {
            eprintln!("{}", serde_json::json!({"ok": false, "errors": errors}));
            std::process::exit(1);
        }
    }
}
