use clap::Parser;
use env_logger;
use env_logger::{Builder, Env};
use mrdocument::error::{Error, Result};
use mrdocument::main_loop::run_main_loop;
use std::path::PathBuf;
use which::which;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg()]
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = Env::default().filter_or("MRDOCUMENT_LOG_LEVEL", "info");
    Builder::from_env(env).init();

    let result = main_log().await;
    if let Err(ref err) = result {
        log::error!("{err}");
    }

    result
}

async fn main_log() -> Result<()> {
    let args = Cli::parse();
    let path = if let Some(path) = args.path {
        PathBuf::from(path).canonicalize()?
    } else {
        dirs::config_local_dir().ok_or(Error::SkelError)?.join("MrDocument").join("profile")
    };

    mrdocument::api_key::init()?;
    which("pdftoppm").map_err(|_| Error::DependencyMissingError("pdftoppm".to_string()))?;
    which("pdftk").map_err(|_| Error::DependencyMissingError("pdftk".to_string()))?;

    run_main_loop(path).await
}
