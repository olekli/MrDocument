use crate::error::Result;
use crate::watcher::WatcherLoop;
use crate::profile_handler::ProfileHandler;
use std::path::PathBuf;

pub async fn run_main_loop(path: PathBuf) -> Result<()> {
    log::info!("Watching profiles: {:?}", path);
    let watcher_loop = WatcherLoop::new(path.clone(), ProfileHandler::new(path), false).await?;

    Ok(watcher_loop.wait().await?)
}
