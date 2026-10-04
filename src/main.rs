use clap::Parser;
use opusab::{backend, cli::Cli, error::Failure};
fn main() {
    let json = std::env::args_os().any(|s| s == "--json");
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if json
                && !matches!(
                    e.kind(),
                    clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
                )
            {
                eprintln!(
                    "{}",
                    serde_json::json!({"schema_version":1,"error":{"code":"invalid_arguments","message":e.to_string()}})
                );
                std::process::exit(2);
            } else {
                e.exit();
            }
        }
    };
    let result = backend::install_signals().and_then(|_| opusab::cli::run(cli));
    if let Err(e) = result {
        let (code, exit) = e
            .downcast_ref::<Failure>()
            .map(|e| (e.code, e.exit))
            .unwrap_or(("operation_failed", 1));
        if json {
            eprintln!(
                "{}",
                serde_json::json!({"schema_version":1,"error":{"code":code,"message":format!("{e:#}")}})
            );
        } else {
            eprintln!("opusab: {e:#}");
        }
        std::process::exit(exit);
    }
}
