use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("ipc-contract-gen: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--root")) {
        return Err("usage: solosoul-ipc-contract-gen --root <tauri-dir>".into());
    }
    let root = PathBuf::from(args.next().ok_or("--root requires a directory")?);
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let generated = solosoul_ipc_contract_gen::generate(&root)?;
    println!(
        "{}",
        serde_json::to_string(&generated).map_err(|error| error.to_string())?
    );
    Ok(())
}
