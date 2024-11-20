use mrdocument::watcher::{Watcher, WatcherEvent};
use mrdocument::error::{Result, Error};
use env_logger;
use env_logger::{Builder, Env};
use clap::Parser;
use tokio_stream::StreamExt;
use mrdocument::handler::handle_new_file;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg()]
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env = Env::default().filter_or("BLACKJACK_LOG_LEVEL", "info");
    Builder::from_env(env).init();

    let args = Cli::parse();
    let mut watcher = Watcher::new(args.path.into())?;
    loop {
        match watcher.queue.next().await {
            Some(event) => {
                match event {
                    WatcherEvent::Paths(paths) => {
                        for path in paths {
                            match handle_new_file(path.clone().into()).await {
                                Err(err) => {
                                    log::error!("Error handling file {path:?}: {err:?}");
                                }
                                _ => {}
                            }
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
