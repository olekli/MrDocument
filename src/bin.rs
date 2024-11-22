use mrdocument::watcher::{Watcher, WatcherEvent};
use mrdocument::error::{Result, Error};
use mrdocument::file::{Paths, Location, FileObject};
use env_logger;
use env_logger::{Builder, Env};
use clap::Parser;
use tokio_stream::StreamExt;
use mrdocument::handler::handle_file;
use std::env;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

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

    let args = Cli::parse();
    let path = PathBuf::from(args.path.clone()).canonicalize()?;
    let paths = Paths::new(path.clone());
    log::info!("Operating on {:?}", path);
    create_dir_all(paths.make_root(Location::Inbox)).await?;
    create_dir_all(paths.make_root(Location::Outbox)).await?;
    create_dir_all(paths.make_root(Location::Transit)).await?;
    create_dir_all(paths.make_root(Location::Processed)).await?;
    create_dir_all(paths.make_root(Location::Error)).await?;

    let mut watcher = Watcher::new(paths.make_root(Location::Inbox))?;
    loop {
        match watcher.queue.next().await {
            Some(event) => {
                match event {
                    WatcherEvent::Paths(observed_paths) => {
                        for path in observed_paths {
                            log::debug!("Found new file: {:?}", path);
                            tokio::spawn(handle_file(FileObject::new(paths.clone(), path)?));
                        }
                    }
                    WatcherEvent::Quit => {
                        log::info!("Received signal. Exiting.");
                        break Ok(());
                    }
                    WatcherEvent::Error(err) => {
                        log::error!("{err:?}");
                        break Err(err);
                    }
                }
            }
            None => {
                break Err(Error::StreamClosedError);
            }
        };
    }
}
