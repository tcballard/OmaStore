use omastore_workflow::{now, Store};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, path] if command == "users" => println!(
            "{}",
            serde_json::to_string_pretty(&Store::open(Path::new(path))?.admin_users()?)?
        ),
        [command, path, user, role] if command == "grant" || command == "revoke" => {
            Store::open(Path::new(path))?.set_role(user, role, command == "grant", now())?;
            println!("Role updated. The next authenticated action uses the current role set.");
        }
        _ => {
            eprintln!(
                "Usage: omastore-admin users PRIVATE.db | grant/revoke PRIVATE.db USER_ID ROLE"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
