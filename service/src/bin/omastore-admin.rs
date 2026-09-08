use omastore_workflow::{now, Store};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, path, objects] if command == "maintenance" => println!(
            "{}",
            Store::open(Path::new(path))?.maintenance(
                &omastore_workflow::media::LocalObjects::new(Path::new(objects))?,
                now()
            )?
        ),
        [command, path, objects, destination] if command == "backup" => println!(
            "{}",
            Store::open(Path::new(path))?.backup_bundle(
                &omastore_workflow::media::LocalObjects::new(Path::new(objects))?,
                Path::new(destination)
            )?
        ),
        [command, bundle, database, objects] if command == "restore" => println!(
            "{}",
            omastore_workflow::operations::restore_bundle(
                Path::new(bundle),
                Path::new(database),
                Path::new(objects)
            )?
        ),
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
                "Usage: omastore-admin users DB | grant/revoke DB USER ROLE | maintenance DB OBJECTS | backup DB OBJECTS NEW_BUNDLE | restore BUNDLE NEW_DB NEW_OBJECTS"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
