use crate::error::{Error, Result};
use crate::file::{Location, Paths};
use crate::handler::Handler;
use crate::watcher::{Watcher, WatcherEvent};
use std::path::PathBuf;
use tokio_stream::StreamExt;

pub async fn run_main_loop(path: PathBuf) -> Result<()> {
    log::info!("Operating on {:?}", path);
    let paths = Paths::new(path.clone());
    let inbox_path = paths.make_root(Location::Inbox);
    let mut handler = Handler::new(paths, 8).await?;
    let mut watcher = Watcher::new(inbox_path)?;
    loop {
        match watcher.queue.next().await {
            Some(event) => match event {
                WatcherEvent::Paths(observed_paths) => {
                    for path in observed_paths {
                        log::debug!("Found new file: {:?}", path);
                        handler.handle_file(path).await;
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
            },
            None => {
                break Err(Error::StreamClosedError);
            }
        };
    }
}
