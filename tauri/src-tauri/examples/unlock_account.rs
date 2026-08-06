use solosoul_core::vault_service::VaultService;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: unlock_account <data-dir> <account-id>");
        eprintln!(
            "       密码经 SOLOSOUL_TEST_PASSWORD 环境变量传入（不进 argv，避免进程列表泄漏）"
        );
        std::process::exit(1);
    }
    let data_dir = PathBuf::from(&args[1]);
    let account_id = &args[2];

    // R2-15: 主密码改从环境变量读取——argv 中的密码对同机进程可见（ps），
    // 违反敏感信息处理约定。环境变量为最小改动方案。
    let password = match env::var("SOLOSOUL_TEST_PASSWORD") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("错误：请设置 SOLOSOUL_TEST_PASSWORD 环境变量（主密码不进 argv）");
            std::process::exit(1);
        }
    };

    let svc = VaultService::with_base_path(data_dir);
    match svc.unlock(account_id, &password) {
        Ok(_) => println!("Unlocked successfully"),
        Err(e) => {
            eprintln!("Unlock failed: {}", e);
            std::process::exit(1);
        }
    }
}
