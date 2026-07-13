use solosoul_core::vault_service::VaultService;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: unlock_account <data-dir> <account-id> <password>");
        std::process::exit(1);
    }
    let data_dir = PathBuf::from(&args[1]);
    let account_id = &args[2];
    let password = &args[3];

    let svc = VaultService::with_base_path(data_dir);
    match svc.unlock(account_id, password) {
        Ok(_) => println!("Unlocked successfully"),
        Err(e) => {
            eprintln!("Unlock failed: {}", e);
            std::process::exit(1);
        }
    }
}
