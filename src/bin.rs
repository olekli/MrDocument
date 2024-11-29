use env_logger;
use env_logger::{Builder, Env};
use mrdocument::error::{Error, Result};
use mrdocument::main_loop::run_main_loop;
use mrdocument::profile::Profile;
use std::path::PathBuf;
use which::which;

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
    Profile::init_default_profile().await?;

    mrdocument::api_key::init()?;
    which("pdftoppm").map_err(|_| Error::DependencyMissingError("pdftoppm".to_string()))?;
    which("pdftk").map_err(|_| Error::DependencyMissingError("pdftk".to_string()))?;

    let path = Profile::get_profile_dir();
    run_main_loop(path).await
}
