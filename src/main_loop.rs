use crate::error::Result;
use crate::handler::Handler;
use crate::paths::Location;
use crate::profile::Profile;
use crate::watcher::WatcherLoop;

pub async fn run_main_loop(profile: Profile) -> Result<()> {
    log::info!("Operating on {:?}", profile.paths.path);
    let inbox_path = profile.paths.make_root(Location::Inbox);
    let handler = Handler::new(profile, 4).await?;
    let watcher_loop =
        WatcherLoop::new(inbox_path, handler).await?;

    Ok(watcher_loop.wait().await?)
}
