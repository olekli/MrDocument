use crate::error::{Error, Result};
use crate::paths::{Location};
use crate::handler::Handler;
use crate::watcher::{Watcher, WatcherEvent};
use tokio_stream::StreamExt;
use crate::profile::Profile;

pub async fn run_main_loop(profile: Profile) -> Result<()> {
    log::info!("Operating on {:?}", profile.paths.path);
    let inbox_path = profile.paths.make_root(Location::Inbox);
    let mut handler = Handler::new(profile, 4).await?;
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
