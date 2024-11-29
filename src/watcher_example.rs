use clap::Parser;
use env_logger;
use env_logger::{Builder, Env};
use mrdocument::error::{Result};
use mrdocument::handler::EventHandler;
use notify::Event;
use std::path::PathBuf;
use mrdocument::watcher::WatcherLoop;

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

    let result = main_log().await;
    if let Err(ref err) = result {
        log::error!("{err}");
    }

    result
}

struct Handler {
}

impl EventHandler for Handler {
    async fn handle_event(&mut self, event: Event) {
        log::info!("{event:?}");
    }
}


async fn main_log() -> Result<()> {
    let args = Cli::parse();
    let path = PathBuf::from(args.path.clone()).canonicalize()?;

    log::info!("Watching {:?}", path);
    let watcher_loop = WatcherLoop::new(path, Handler{}).await?;

    Ok(watcher_loop.wait().await?)
}
