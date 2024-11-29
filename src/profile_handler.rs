use crate::error::Result;
use crate::handler::{EventHandler, Handler};
use crate::paths::Location;
use crate::profile::Profile;
use crate::watcher::WatcherLoop;
use notify::{Event, EventKind};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

pub struct ProfileHandler {
    path: PathBuf,
    profiles: HashMap<PathBuf, (Profile, WatcherLoop)>,
}

impl ProfileHandler {
    pub fn new(path: PathBuf) -> ProfileHandler {
        ProfileHandler {
            path,
            profiles: HashMap::new(),
        }
    }

    async fn on_start_impl(&mut self) -> Result<()> {
        log::debug!("Pinging all profiles");
        let mut dir_entries = fs::read_dir(self.path.clone()).await?;
        while let Some(entry) = dir_entries.next_entry().await? {
            let entry_path = entry.path();
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                let _file = fs::File::open(entry_path).await?;
            }
        }
        Ok(())
    }

    async fn handle_profile(&mut self, path: PathBuf, event: EventKind) -> Result<()> {
        if self.profiles.contains_key(&path) {
            match event {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    let profile = Profile::new_from_file(path.clone()).await?;
                    let (running_profile, _) = self.profiles.get(&path).unwrap();
                    if *running_profile != profile {
                        let running_profile = running_profile.clone();
                        let (_, running_loop) = self.profiles.remove(&path).unwrap();
                        running_loop.shutdown().await.inspect_err(|e| {
                            log::warn!(
                                "Cannot shutdown running watcher: {running_profile:?}: {e:?}"
                            )
                        })?;
                    }
                }
                _ => {
                    log::trace!("Ignoring {path:?}: {event:?}");
                }
            }
        }
        if !self.profiles.contains_key(&path) {
            let profile = Profile::new_from_file(path.clone()).await?;
            log::info!("Starting watcher on {:?}", profile.paths.path);
            let inbox_path = profile.paths.make_root(Location::Inbox);
            let handler = Handler::new(profile.clone(), 4).await?;
            let watcher_loop = WatcherLoop::new(inbox_path, handler).await?;
            self.profiles.insert(path, (profile, watcher_loop));
        } else {
            log::trace!("Watcher for {path:?} already running");
        }

        Ok(())
    }
}

impl EventHandler for ProfileHandler {
    async fn on_start(&mut self) {
        self.on_start_impl()
            .await
            .inspect_err(|e| log::warn!("Unable to ping profile files: {e:?}"))
            .ok();
    }

    async fn handle_event(&mut self, event: Event) {
        match event {
            Event { kind, paths, .. } => {
                let existing_paths: Vec<_> =
                    paths.into_iter().filter(|path| path.is_file()).collect();
                for path in existing_paths {
                    self.handle_profile(path.clone(), kind)
                        .await
                        .inspect_err(|e| log::error!("Unable to run profile: {path:?}: {e:?}"))
                        .ok();
                }
            }
        };
    }
}
