use clap::Parser;
use env_logger;
use env_logger::{Builder, Env};
use mrdocument::error::{Error, Result};
use mrdocument::main_loop::run_main_loop;
use std::env;
use std::path::PathBuf;
use which::which;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg()]
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = Env::default().filter_or("MRDOCUMENT_LOG_LEVEL", "info");
    Builder::from_env(env).init();
    env::var("OPENAI_API_KEY").map_err(|_| Error::NoApiKeyError)?;
    which("pdftoppm").map_err(|_| Error::DependencyMissingError("pdftoppm".to_string()))?;
    which("pdftk").map_err(|_| Error::DependencyMissingError("pdftk".to_string()))?;

    let args = Cli::parse();
    let path = PathBuf::from(args.path.clone()).canonicalize()?;

    run_main_loop(path).await
}
