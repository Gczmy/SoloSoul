use solosoul_core::vault_service::VaultService;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: create_account <data-dir> <account-name> [password] [hint]");
        std::process::exit(1);
    }
    let data_dir = PathBuf::from(&args[1]);
    let name = &args[2];
    let password = args.get(3).map(|s| s.as_str()).unwrap_or("password123");
    let hint = args.get(4).map(|s| s.as_str());

    let svc = VaultService::with_base_path(data_dir);
    match svc.create_account(name, password, hint) {
        Ok(info) => println!("Created account: {}", info),
        Err(e) => {
            eprintln!("Failed to create account: {}", e);
            std::process::exit(1);
        }
    }
}
