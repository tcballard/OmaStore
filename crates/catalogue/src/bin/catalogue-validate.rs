use omastore_catalogue::{Snapshot, MAX_BYTES};
use std::io::Read;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Usage: catalogue-validate FILE");
        std::process::exit(2);
    }
    let bytes = std::fs::File::open(&args[0]).and_then(|f| {
        let mut bytes = Vec::new();
        f.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    let result = bytes
        .map_err(|_| "{\"code\":\"read_failed\"}".to_string())
        .and_then(|bytes| {
            Snapshot::parse(&bytes, cfg!(feature = "dev-fixtures"))
                .map_err(|e| serde_json::to_string(&e).unwrap())
        });
    match result {
        Ok(s) => println!(
            "Valid catalogue v1: {} apps, revision {}",
            s.apps.len(),
            s.revision
        ),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
